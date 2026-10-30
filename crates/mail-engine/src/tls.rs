// INVARIANT: Self-signed TLS certificates are permitted strictly when connecting to loopback
// (`127.0.0.1` or `::1`) for local Proton Mail Bridge, never for remote IMAP hosts.

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, DigitallySignedStruct, Error as RustlsError, RootCertStore, SignatureScheme,
};
use std::sync::Arc;
use tokio_rustls::TlsConnector;

use crate::{EngineError, ImapEndpointConfig};

/// Custom verifier allowing self-signed certificates strictly on local loopback hosts (Proton Mail Bridge).
#[derive(Debug)]
struct LoopbackCertVerifier;

impl ServerCertVerifier for LoopbackCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

/// Builds a `tokio_rustls::TlsConnector` configured according to endpoint TLS policy.
pub fn create_tls_connector(endpoint: &ImapEndpointConfig) -> Result<TlsConnector, EngineError> {
    endpoint.validate_tls_policy()?;

    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| EngineError::ImapProtocol(format!("TLS protocol config error: {e}")))?;

    let client_config = if endpoint.allow_loopback_self_signed_tls {
        if !endpoint.is_loopback_host() {
            return Err(EngineError::InsecureTlsRejected(endpoint.host.clone()));
        }

        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(LoopbackCertVerifier))
            .with_no_client_auth()
    } else {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        builder
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    Ok(TlsConnector::from(Arc::new(client_config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthMethod;

    #[test]
    fn allows_loopback_self_signed_connector() {
        let loopback = ImapEndpointConfig {
            account_id: "proton".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1143,
            auth_method: AuthMethod::ProtonBridgeLocal,
            allow_loopback_self_signed_tls: true,
        };
        let connector = create_tls_connector(&loopback);
        assert!(connector.is_ok());
    }

    #[test]
    fn rejects_remote_self_signed_connector() {
        let remote = ImapEndpointConfig {
            account_id: "remote".to_string(),
            host: "mail.example.org".to_string(),
            port: 993,
            auth_method: AuthMethod::AppPassword,
            allow_loopback_self_signed_tls: true,
        };
        match create_tls_connector(&remote) {
            Err(err) => {
                assert_eq!(
                    err,
                    EngineError::InsecureTlsRejected("mail.example.org".to_string())
                );
            }
            Ok(_) => panic!("expected insecure TLS to be rejected for remote host"),
        }
    }
}
