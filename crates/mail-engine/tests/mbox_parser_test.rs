use std::io::Cursor;

use mail_engine::{
    extract_gmail_labels, parse_eml_bytes, parse_eml_reader, MboxFlavor, MboxReadConfig, MboxReader,
};

#[test]
fn streams_multi_message_takeout_and_thunderbird_mbox_with_labels() {
    let mbox_data = concat!(
        "From 1812345678901234567@xxx Mon Nov 16 10:00:00 +0000 2026\n",
        "From: Billing <billing@example.org>\n",
        "To: user@example.com\n",
        "Subject: Invoice #100\n",
        "Message-ID: <inv-100@example.org>\n",
        "Date: Mon, 16 Nov 2026 10:00:00 +0000\n",
        "X-Gmail-Labels: Inbox,Important,\"Receipts, Taxes\",Unread\n",
        "\n",
        "Hello,\n",
        ">From our billing department:\n",
        ">>From yesterday's statement, no balance due.\n",
        "From 2026 onwards this prose line stays intact.\n",
        "\n",
        "From - Mon Nov 16 11:30:00 2026\r\n",
        "From: Alerts <alerts@example.com>\r\n",
        "To: user@example.com\r\n",
        "Subject: =?UTF-8?Q?Nota_Fiscal_Emitida?=\r\n",
        "Message-ID: <nfe-200@example.com>\r\n",
        "Date: Mon, 16 Nov 2026 11:30:00 +0000\r\n",
        "X-Gmail-Labels: =?UTF-8?Q?Caixa_de_entrada?=,Starred\r\n",
        "\r\n",
        "Your receipt is ready.\r\n",
        "\r\n"
    );

    let reader = MboxReader::new(Cursor::new(mbox_data.as_bytes())).expect("valid reader");
    let items: Vec<_> = reader
        .collect::<Result<Vec<_>, _>>()
        .expect("all messages parse");

    assert_eq!(items.len(), 2);

    let first = &items[0];
    assert_eq!(first.message.uid, 1);
    assert_eq!(first.message.subject, "Invoice #100");
    assert_eq!(first.message.sender, "Billing <billing@example.org>");
    assert_eq!(
        first.message.message_id_header.as_deref(),
        Some("inv-100@example.org")
    );
    assert_eq!(
        first.gmail_labels,
        vec!["Inbox", "Important", "Receipts, Taxes", "Unread"]
    );
    assert!(first.has_unread_label());
    assert!(!first.has_starred_label());
    let body0 = first.message.text_plain.as_deref().expect("body");
    assert!(body0.contains("From our billing department:"));
    assert!(body0.contains(">From yesterday's statement, no balance due."));
    assert!(body0.contains("From 2026 onwards this prose line stays intact."));

    let second = &items[1];
    assert_eq!(second.message.uid, 2);
    assert_eq!(second.message.subject, "Nota Fiscal Emitida");
    assert_eq!(second.gmail_labels, vec!["Caixa de entrada", "Starred"]);
    assert!(!second.has_unread_label());
    assert!(second.has_starred_label());
    assert_eq!(
        second.message.text_plain.as_deref(),
        Some("Your receipt is ready.")
    );
}

#[test]
fn mboxo_flavor_only_unescapes_single_gt_from() {
    let mbox_data = concat!(
        "From sender@example.com Mon Nov 16 12:00:00 2026\n",
        "From: sender@example.com\n",
        "To: recipient@example.org\n",
        "Subject: Mboxo Test\n",
        "\n",
        ">From first\n",
        ">>From second\n",
    );

    let config = MboxReadConfig {
        flavor: MboxFlavor::Mboxo,
        starting_uid: 10,
        ..MboxReadConfig::default()
    };
    let mut reader = MboxReader::with_config(Cursor::new(mbox_data.as_bytes()), config).unwrap();
    let msg = reader.next_message().unwrap().unwrap();
    assert_eq!(msg.message.uid, 10);
    let body = msg.message.text_plain.as_deref().unwrap();
    assert!(body.contains("From first"));
    assert!(body.contains(">>From second"));
}

#[test]
fn rejects_oversized_mbox_message_and_halts_stream() {
    let mbox_data = concat!(
        "From sender@example.com Mon Nov 16 12:00:00 2026\n",
        "From: sender@example.com\n",
        "Subject: Small\n",
        "\n",
        "ok\n",
        "\n",
        "From sender@example.com Mon Nov 16 12:05:00 2026\n",
        "From: sender@example.com\n",
        "Subject: Huge\n",
        "\n",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
    );

    let config = MboxReadConfig {
        max_message_bytes: 96,
        ..MboxReadConfig::default()
    };
    let mut reader = MboxReader::with_config(Cursor::new(mbox_data.as_bytes()), config).unwrap();

    let first = reader.next_message().expect("first fits").expect("some");
    assert_eq!(first.message.subject, "Small");

    let second = reader.next_message();
    assert!(second.is_err(), "oversized second message must fail");
    assert_eq!(reader.next_message().unwrap(), None);
}

#[test]
fn rejects_non_mbox_input_missing_initial_from_delimiter() {
    let eml_data = b"From: alice@example.com\r\nSubject: Not an mbox\r\n\r\nBody\r\n";
    let mut reader = MboxReader::new(Cursor::new(eml_data.as_slice())).unwrap();
    assert!(reader.next_message().is_err());
}

#[test]
fn empty_mbox_yields_zero_messages() {
    let mut reader = MboxReader::new(Cursor::new(b"\n\r\n")).unwrap();
    assert_eq!(reader.next_message().unwrap(), None);
}

#[test]
fn parses_standalone_eml_with_lf_and_attachments() {
    let eml_data = concat!(
        "From: billing@example.org\n",
        "To: user@example.com\n",
        "Subject: Standalone EML\n",
        "Message-ID: <eml-1@example.org>\n",
        "X-Gmail-Labels: Archive,Important\n",
        "MIME-Version: 1.0\n",
        "Content-Type: multipart/mixed; boundary=\"BND\"\n",
        "\n",
        "--BND\n",
        "Content-Type: text/plain; charset=utf-8\n",
        "\n",
        "Attached document inside.\n",
        "--BND\n",
        "Content-Type: application/pdf; name=\"doc.pdf\"\n",
        "Content-Disposition: attachment; filename=\"doc.pdf\"\n",
        "Content-Transfer-Encoding: base64\n",
        "\n",
        "JVBERi0xLjQ=\n",
        "--BND--\n",
    );

    let parsed = parse_eml_bytes(eml_data.as_bytes(), 7, 1024 * 1024).expect("eml parses");
    assert_eq!(parsed.message.uid, 7);
    assert_eq!(parsed.message.subject, "Standalone EML");
    assert_eq!(parsed.gmail_labels, vec!["Archive", "Important"]);
    assert_eq!(parsed.message.attachments.len(), 1);
    assert_eq!(parsed.message.attachments[0].filename, "doc.pdf");

    let via_reader =
        parse_eml_reader(Cursor::new(eml_data.as_bytes()), 7, 1024 * 1024).expect("reader parses");
    assert_eq!(via_reader, parsed);

    assert!(parse_eml_reader(Cursor::new(eml_data.as_bytes()), 7, 64).is_err());
}

#[test]
fn extracts_duplicate_and_quoted_gmail_labels() {
    let raw =
        b"X-Gmail-Labels: Inbox, \"Work, Urgent\", Inbox, \"Quoted \"\"Label\"\"\"\r\n\r\nBody";
    assert_eq!(
        extract_gmail_labels(raw),
        vec!["Inbox", "Work, Urgent", "Quoted \"Label\""]
    );
}
