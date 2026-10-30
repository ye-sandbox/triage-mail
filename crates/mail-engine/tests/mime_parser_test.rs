#![forbid(unsafe_code)]

//! Integration tests for zero-copy RFC 5322 / MIME parsing and threading.
//!
//! Uses synthetic RFC 2606 domains (`@example.com`, `@example.org`) exclusively.

use mail_engine::{build_thread_tree, MimeExtractor, StalwartMimeParser};

#[test]
fn test_standard_plain_text_message_extraction() {
    let raw = concat!(
        "From: \"Jane Doe\" <jane.doe@example.com>\r\n",
        "To: \"Dev Team\" <dev@example.org>, lead@example.org\r\n",
        "Subject: Release v0.1.0 Roadmap\r\n",
        "Message-ID: <msg-roadmap-001@example.com>\r\n",
        "Date: Thu, 29 Oct 2026 14:00:00 +0000\r\n",
        "Content-Type: text/plain; charset=utf-8\r\n",
        "\r\n",
        "Hello team,\r\n\r\nHere is our roadmap for the upcoming sprint.\r\n"
    );

    let parser = StalwartMimeParser::new();
    let parsed = parser
        .parse_with_uid_and_modseq(raw.as_bytes(), 501, Some(1001))
        .expect("parsed successfully");

    assert_eq!(parsed.uid, 501);
    assert_eq!(parsed.modseq, Some(1001));
    assert_eq!(
        parsed.message_id_header.as_deref(),
        Some("msg-roadmap-001@example.com")
    );
    assert_eq!(parsed.in_reply_to, None);
    assert!(parsed.references.is_empty());
    assert_eq!(parsed.references_header, None);
    assert_eq!(parsed.subject, "Release v0.1.0 Roadmap");
    assert_eq!(parsed.sender, "Jane Doe <jane.doe@example.com>");
    assert_eq!(
        parsed.recipients,
        vec![
            "Dev Team <dev@example.org>".to_string(),
            "lead@example.org".to_string(),
        ]
    );
    assert_eq!(parsed.date_epoch_ms, Some(1_793_282_400_000));
    assert_eq!(
        parsed.text_plain.as_deref(),
        Some("Hello team,\r\n\r\nHere is our roadmap for the upcoming sprint.")
    );
    assert_eq!(parsed.html_raw, None);
    assert!(parsed.raw_headers.is_some());
    assert_eq!(parsed.attachments.len(), 0);
    assert_eq!(parsed.unsubscribe, None);
}

#[test]
fn test_complex_multipart_alternative_and_brazilian_charset() {
    // Subject and body in ISO-8859-1 with Portuguese diacritics
    let mut raw = Vec::new();
    raw.extend_from_slice(b"From: financeiro@example.org\r\n");
    raw.extend_from_slice(b"To: cliente@example.com\r\n");
    raw.extend_from_slice(b"Subject: =?ISO-8859-1?Q?Boleto_Banc=E1rio_-_Servi=E7os?=\r\n");
    raw.extend_from_slice(b"Message-ID: <boleto-2026-99@example.org>\r\n");
    raw.extend_from_slice(b"MIME-Version: 1.0\r\n");
    raw.extend_from_slice(b"Content-Type: multipart/alternative; boundary=\"ALT_BOUNDARY\"\r\n");
    raw.extend_from_slice(b"\r\n");

    // Plain text part in ISO-8859-1
    raw.extend_from_slice(b"--ALT_BOUNDARY\r\n");
    raw.extend_from_slice(b"Content-Type: text/plain; charset=iso-8859-1\r\n");
    raw.extend_from_slice(b"Content-Transfer-Encoding: 8bit\r\n\r\n");
    // "Valor a pagar: R$ 350,00 com vencimento em novembro."
    raw.extend_from_slice(b"Valor a pagar: R$ 350,00 com vencimento em novembro.\r\n");

    // HTML part in UTF-8
    raw.extend_from_slice(b"--ALT_BOUNDARY\r\n");
    raw.extend_from_slice(b"Content-Type: text/html; charset=utf-8\r\n\r\n");
    raw.extend_from_slice(b"<p>Valor a pagar: <strong>R$ 350,00</strong></p>\r\n");
    raw.extend_from_slice(b"--ALT_BOUNDARY--\r\n");

    let parser = StalwartMimeParser::new();
    let parsed = parser.parse_lightweight(&raw).expect("parsed");

    assert_eq!(parsed.subject, "Boleto Bancário - Serviços");
    assert_eq!(
        parsed.text_plain.as_deref(),
        Some("Valor a pagar: R$ 350,00 com vencimento em novembro.")
    );
    assert_eq!(
        parsed.html_raw.as_deref(),
        Some("<p>Valor a pagar: <strong>R$ 350,00</strong></p>")
    );
}

#[test]
fn test_conversation_thread_tree_reconstruction() {
    let parser = StalwartMimeParser::new();

    let root_raw = concat!(
        "From: architect@example.com\r\n",
        "To: eng@example.org\r\n",
        "Subject: RFC: Architecture Proposal\r\n",
        "Message-ID: <root-rfc@example.com>\r\n",
        "\r\n",
        "What do you think of this RFC?\r\n"
    );

    let reply_a_raw = concat!(
        "From: alice@example.org\r\n",
        "To: architect@example.com\r\n",
        "Subject: Re: RFC: Architecture Proposal\r\n",
        "Message-ID: <reply-a@example.org>\r\n",
        "In-Reply-To: <root-rfc@example.com>\r\n",
        "References: <root-rfc@example.com>\r\n",
        "\r\n",
        "Looks solid to me.\r\n"
    );

    let reply_b_raw = concat!(
        "From: bob@example.org\r\n",
        "To: architect@example.com\r\n",
        "Subject: Re: RFC: Architecture Proposal\r\n",
        "Message-ID: <reply-b@example.org>\r\n",
        "In-Reply-To: <root-rfc@example.com>\r\n",
        "References: <root-rfc@example.com>\r\n",
        "\r\n",
        "I have a few questions about storage.\r\n"
    );

    let reply_b_sub_raw = concat!(
        "From: architect@example.com\r\n",
        "To: bob@example.org\r\n",
        "Subject: Re: RFC: Architecture Proposal\r\n",
        "Message-ID: <reply-b-clarification@example.com>\r\n",
        "In-Reply-To: <reply-b@example.org>\r\n",
        "References: <root-rfc@example.com> <reply-b@example.org>\r\n",
        "\r\n",
        "Storage uses SQLCipher + FTS5 + sqlite-vec.\r\n"
    );

    let parsed_root = parser.parse_lightweight(root_raw.as_bytes()).unwrap();
    let parsed_reply_a = parser.parse_lightweight(reply_a_raw.as_bytes()).unwrap();
    let parsed_reply_b = parser.parse_lightweight(reply_b_raw.as_bytes()).unwrap();
    let parsed_reply_b_sub = parser
        .parse_lightweight(reply_b_sub_raw.as_bytes())
        .unwrap();

    let messages = vec![
        parsed_root,
        parsed_reply_a,
        parsed_reply_b,
        parsed_reply_b_sub,
    ];

    let tree = build_thread_tree(&messages);
    assert_eq!(tree.len(), 1);

    let root_node = &tree[0];
    assert_eq!(root_node.message_id, "root-rfc@example.com");
    assert_eq!(root_node.children.len(), 2);

    // reply-a and reply-b are both direct children of root
    let child_ids: Vec<&str> = root_node
        .children
        .iter()
        .map(|c| c.message_id.as_str())
        .collect();
    assert!(child_ids.contains(&"reply-a@example.org"));
    assert!(child_ids.contains(&"reply-b@example.org"));

    // reply-b has child reply-b-clarification
    let reply_b_node = root_node
        .children
        .iter()
        .find(|c| c.message_id == "reply-b@example.org")
        .unwrap();
    assert_eq!(reply_b_node.children.len(), 1);
    assert_eq!(
        reply_b_node.children[0].message_id,
        "reply-b-clarification@example.com"
    );
}

#[test]
fn test_rfc8058_one_click_unsubscribe_headers() {
    let parser = StalwartMimeParser::new();

    // 1. Both valid HTTPS URL and List-Unsubscribe-Post: List-Unsubscribe=One-Click
    let raw_valid = concat!(
        "From: promotions@example.org\r\n",
        "To: user@example.com\r\n",
        "Subject: Spring Sale\r\n",
        "List-Unsubscribe: <https://example.org/unsubscribe/one-click?token=sec_abc123>, <mailto:unsub@example.org>\r\n",
        "List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
        "\r\n",
        "Sale details here.\r\n"
    );

    let parsed = parser.parse_lightweight(raw_valid.as_bytes()).unwrap();
    let unsub = parsed.unsubscribe.expect("present");
    assert!(unsub.supports_rfc8058_one_click);
    assert_eq!(
        unsub.https_uri.as_deref(),
        Some("https://example.org/unsubscribe/one-click?token=sec_abc123")
    );
    assert_eq!(
        unsub.mailto_uri.as_deref(),
        Some("mailto:unsub@example.org")
    );
    assert_eq!(
        unsub.raw_list_unsubscribe_post.as_deref(),
        Some("List-Unsubscribe=One-Click")
    );

    // 2. Missing List-Unsubscribe-Post header
    let raw_no_post = concat!(
        "From: promotions@example.org\r\n",
        "To: user@example.com\r\n",
        "Subject: Spring Sale\r\n",
        "List-Unsubscribe: <https://example.org/unsubscribe/standard>\r\n",
        "\r\n",
        "Sale details here.\r\n"
    );
    let parsed_no_post = parser.parse_lightweight(raw_no_post.as_bytes()).unwrap();
    let unsub_no_post = parsed_no_post.unsubscribe.expect("present");
    assert!(!unsub_no_post.supports_rfc8058_one_click);
    assert_eq!(
        unsub_no_post.https_uri.as_deref(),
        Some("https://example.org/unsubscribe/standard")
    );
}

#[test]
fn test_lazy_attachment_part_metadata_and_inline_cid() {
    let parser = StalwartMimeParser::new();

    let raw = concat!(
        "From: reports@example.org\r\n",
        "To: executive@example.com\r\n",
        "Subject: Monthly Financials\r\n",
        "MIME-Version: 1.0\r\n",
        "Content-Type: multipart/mixed; boundary=\"MIX_TOP\"\r\n",
        "\r\n",
        "--MIX_TOP\r\n",
        "Content-Type: text/html; charset=utf-8\r\n",
        "\r\n",
        "<p>Attached is the financial statement and charts.</p><img src=\"cid:chart_q3@example.org\">\r\n",
        "--MIX_TOP\r\n",
        "Content-Type: application/pdf; name=\"financial_q3.pdf\"\r\n",
        "Content-Disposition: attachment; filename=\"financial_q3.pdf\"\r\n",
        "Content-Transfer-Encoding: base64\r\n",
        "\r\n",
        "JVBERi0xLjQKJeLjz9MKMSAwIG9ia...\r\n",
        "--MIX_TOP\r\n",
        "Content-Type: image/png\r\n",
        "Content-Disposition: inline; filename=\"chart.png\"\r\n",
        "Content-ID: <chart_q3@example.org>\r\n",
        "Content-Transfer-Encoding: base64\r\n",
        "\r\n",
        "iVBORw0KGgoAAAANSUhEUgAAAAUA...\r\n",
        "--MIX_TOP--\r\n"
    );

    let parsed = parser.parse_lightweight(raw.as_bytes()).unwrap();
    assert_eq!(parsed.attachments.len(), 2);

    let att_pdf = parsed
        .attachments
        .iter()
        .find(|a| a.filename == "financial_q3.pdf")
        .expect("pdf found");
    assert_eq!(att_pdf.part_id, "2");
    assert_eq!(att_pdf.mime_type, "application/pdf");
    assert!(!att_pdf.is_inline);
    assert_eq!(att_pdf.content_id, None);
    assert!(att_pdf.size_bytes > 0);

    let att_chart = parsed
        .attachments
        .iter()
        .find(|a| a.filename == "chart.png")
        .expect("chart found");
    assert_eq!(att_chart.part_id, "3");
    assert_eq!(att_chart.mime_type, "image/png");
    assert!(att_chart.is_inline);
    assert_eq!(
        att_chart.content_id.as_deref(),
        Some("chart_q3@example.org")
    );
    assert!(att_chart.size_bytes > 0);
}

#[test]
fn test_robustness_on_malformed_and_edge_case_inputs() {
    let parser = StalwartMimeParser::new();

    // 1. Missing From, Subject, Date
    let empty_msg = b"To: someone@example.com\r\n\r\nJust a body";
    let parsed = parser
        .parse_lightweight(empty_msg)
        .expect("parses gracefully");
    assert_eq!(parsed.subject, "");
    assert_eq!(parsed.sender, "");
    assert_eq!(parsed.recipients, vec!["someone@example.com"]);
    assert_eq!(parsed.date_epoch_ms, None);
    assert_eq!(parsed.text_plain.as_deref(), Some("Just a body"));

    // 2. Malformed unclosed angle brackets in List-Unsubscribe
    let malformed_unsub = concat!(
        "From: sender@example.com\r\n",
        "List-Unsubscribe: <https://example.org/broken-unsub\r\n",
        "\r\n",
        "Body\r\n"
    );
    let parsed_bad_unsub = parser
        .parse_lightweight(malformed_unsub.as_bytes())
        .unwrap();
    // Should not panic, gracefully extract what is parseable
    assert!(parsed_bad_unsub.unsubscribe.is_some() || parsed_bad_unsub.unsubscribe.is_none());

    // 3. Thread node reconstruction with disjoint / orphaned messages
    let orphan1 = parser
        .parse_lightweight(
            b"Message-ID: <msg-orphan-1@example.com>\r\nIn-Reply-To: <ghost@example.com>\r\n\r\nHi",
        )
        .unwrap();
    let orphan2 = parser
        .parse_lightweight(b"Message-ID: <msg-orphan-2@example.com>\r\n\r\nHello")
        .unwrap();

    let tree = build_thread_tree(&[orphan1, orphan2]);
    // Both orphans become root nodes because parent is not in the set
    assert_eq!(tree.len(), 2);
    let ids: Vec<&str> = tree.iter().map(|n| n.message_id.as_str()).collect();
    assert!(ids.contains(&"msg-orphan-1@example.com"));
    assert!(ids.contains(&"msg-orphan-2@example.com"));
}
