use super::*;

fn fixture() -> Arc<DesktopService> {
    let store = Arc::new(
        storage::SqliteMailStore::open_in_memory(
            &storage::DatabaseKey::from_bytes([45; 32]),
            Default::default(),
        )
        .unwrap(),
    );
    let vault: Arc<dyn vault::SecretStore> = Arc::new(vault::EncryptedFallbackStore::in_memory(
        Arc::new(vault::Argon2AesGcmCipher::from_key([46; 32])),
    ));
    DesktopService::new(
        store,
        vault.clone(),
        Arc::new(crate::remote::NativeRemoteMail::new(vault)),
        None,
    )
}

fn input(key: &str) -> IncomingApiKey {
    IncomingApiKey(Zeroizing::new(key.into()))
}

#[tokio::test]
async fn settings_persist_without_exposing_keys_or_reusing_them_at_new_endpoints() {
    let service = fixture();
    let defaults = service.get_settings().await.unwrap();
    assert!(!defaults.ai.enabled);
    assert!(!defaults.has_api_key);
    let mut settings = AiSettings {
        enabled: true,
        provider: AiProvider::Byok,
        endpoint: "https://ai.example.com".into(),
        model: "test-model".into(),
    };
    service
        .save_settings(settings.clone(), input("synthetic-private-value"))
        .await
        .unwrap();
    let response = serde_json::to_string(&service.get_settings().await.unwrap()).unwrap();
    assert!(!response.contains("synthetic-private-value"));
    assert!(service.get_settings().await.unwrap().has_api_key);
    service
        .save_settings(settings.clone(), input(""))
        .await
        .unwrap();
    settings.endpoint = "https://other.example.com".into();
    assert_eq!(
        service.save_settings(settings.clone(), input("")).await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service.read_ai_settings().unwrap().endpoint,
        "https://ai.example.com"
    );
    settings.enabled = false;
    service.save_settings(settings, input("")).await.unwrap();
    assert!(!service.get_settings().await.unwrap().has_api_key);
    // Recreating the service demonstrates vault persistence, without copying UI state.
    let reopened = DesktopService::new(
        service.store.clone(),
        service.vault.clone(),
        service.remote.clone(),
        None,
    );
    assert_eq!(
        reopened.read_ai_settings().unwrap().endpoint,
        "https://other.example.com"
    );
    service
        .save_settings(AiSettings::default(), input(""))
        .await
        .unwrap();
    assert!(!service.get_settings().await.unwrap().has_api_key);
}

#[tokio::test]
async fn consent_and_settings_snapshot_gate_dispatch_before_network() {
    let service = fixture();
    assert_eq!(
        service
            .summarize_message("missing".into(), false, AiSettings::default())
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service
            .summarize_message("missing".into(), true, AiSettings::default())
            .await,
        Err(IpcError::InvalidRequest)
    );
    let enabled = AiSettings {
        enabled: true,
        ..Default::default()
    };
    service
        .save_settings(enabled.clone(), input(""))
        .await
        .unwrap();
    assert_eq!(
        service
            .summarize_message("missing".into(), true, AiSettings::default())
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service
            .summarize_message("missing".into(), true, enabled)
            .await,
        Err(IpcError::NotFound)
    );
    let request = crate::model_download::ModelDownloadRequest {
        model_url: "https://models.example.com/model.onnx".into(),
        tokenizer_url: "https://models.example.com/tokenizer.json".into(),
        consent: false,
    };
    assert_eq!(
        service.download_model(request).await,
        Err(IpcError::InvalidRequest)
    );
}

#[test]
fn secret_parse_errors_and_endpoint_validation_fail_closed() {
    let error = serde_json::from_str::<IncomingApiKey>("{\"synthetic-private-value\":1}")
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains("synthetic-private-value"));
    for endpoint in [
        "https://user:pass@ai.example.com",
        "https://ai.example.com?key=private",
        "http://ai.example.com",
        "http://192.168.1.1:11434",
    ] {
        let settings = AiSettings {
            endpoint: endpoint.into(),
            ..Default::default()
        };
        assert!(settings.client_config(String::new()).is_err());
    }
}

#[tokio::test]
async fn explicit_summary_sends_only_selected_stored_content_to_loopback() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let service = fixture();
    service
        .store
        .insert_account(&storage::AccountRecord {
            id: "a".into(),
            email: "reader@example.com".into(),
            auth_type: "AppPassword".into(),
            created_at_epoch_ms: 0,
            updated_at_epoch_ms: 0,
        })
        .unwrap();
    let record: storage::MessageRecord = serde_json::from_value(serde_json::json!({
        "id":"m", "account_id":"a", "folder":"INBOX", "uid":1,
        "subject":"Selected subject", "sender":"sender@example.com", "recipients":"reader@example.com",
        "received_at_epoch_ms":0,"is_read":false,"has_attachments":false,
        "raw_headers":"private-header-not-for-summary", "text_body":"Selected plain body",
        "html_body":"<p>html-not-for-summary</p>", "one_click_unsubscribe_supported":false,
        "created_at_epoch_ms":0, "attachments":[], "tags":[]
    })).unwrap();
    service.store.insert_message(&record).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        let mut buffer = [0; 2048];
        loop {
            let size = socket.read(&mut buffer).await.unwrap();
            assert!(size > 0);
            received.extend_from_slice(&buffer[..size]);
            if let Some(end) = received.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&received[..end]).to_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if received.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let body = r#"{"response":"Local summary"}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        String::from_utf8(received).unwrap()
    });
    let settings = AiSettings {
        enabled: true,
        endpoint,
        ..Default::default()
    };
    service
        .save_settings(settings.clone(), input(""))
        .await
        .unwrap();
    assert_eq!(
        service
            .summarize_message("m".into(), true, settings)
            .await
            .unwrap(),
        "Local summary"
    );
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/generate "));
    assert!(request.contains("Selected subject"));
    assert!(request.contains("Selected plain body"));
    assert!(!request.contains("private-header-not-for-summary"));
    assert!(!request.contains("html-not-for-summary"));
}

#[tokio::test]
async fn experimental_semantic_search_persists_in_settings() {
    let service = fixture();
    let initial = service.get_settings().await.unwrap();
    assert!(!initial.experimental_semantic_search);

    service
        .set_experimental_semantic_search(true)
        .await
        .unwrap();
    let updated = service.get_settings().await.unwrap();
    assert!(updated.experimental_semantic_search);

    // Saving AI settings preserves the experimental toggle
    let ai = AiSettings::default();
    service.save_settings(ai, input("")).await.unwrap();
    let preserved = service.get_settings().await.unwrap();
    assert!(preserved.experimental_semantic_search);

    service
        .set_experimental_semantic_search(false)
        .await
        .unwrap();
    let disabled = service.get_settings().await.unwrap();
    assert!(!disabled.experimental_semantic_search);
}
