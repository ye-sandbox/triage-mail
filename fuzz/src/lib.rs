#![forbid(unsafe_code)]

//! Offline-only parser assertions shared by libFuzzer targets and seed replay tests.
mod stream;

use mail_engine::{
    extract_body_structure_metadata, parse_bodystructure_str, protocol::CommandResult, EngineError,
    ImapSession, RawFetchedMessage,
};
use stream::Fragmented;

pub const MAX_INPUT: usize = 64 * 1024;
const COMMAND: &str = "UID FETCH 1 (UID BODY.PEEK[HEADER] BODY.PEEK[1] BODY.PEEK[2])";

thread_local! {
    static RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time().build().expect("in-memory fuzz runtime");
}

pub fn bodystructure(data: &[u8]) {
    if data.len() > MAX_INPUT {
        return;
    }
    // INVARIANT: The public metadata API accepts str; invalid UTF-8 is not repaired.
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let first = parse_bodystructure_str(input);
    assert_eq!(first, parse_bodystructure_str(input));
    if let Ok(structure) = first {
        let metadata = extract_body_structure_metadata(&structure);
        assert_eq!(metadata, extract_body_structure_metadata(&structure));
    }
}

pub fn fetch(data: &[u8]) {
    if data.len() > MAX_INPUT {
        return;
    }
    RUNTIME.with(|runtime| {
        runtime.block_on(async {
            // Arbitrary control bytes exercise malformed framing, lengths and EOF.
            let baseline = run_commands(data, vec![4096]).await;
            let fragmented = run_commands(data, vec![1, 2, 3, 5, 7]).await;
            assert_eq!(baseline, fragmented, "fragmentation changed parsing");

            // INVARIANT: Independently encoded lengths and expected bytes make valid
            // literals an oracle, rather than merely comparing a parser to itself.
            let (wire, expected) = structured_fetch(data);
            for chunks in [vec![4096], vec![1], vec![2, 3, 5, 7]] {
                let (first, next) = run_commands(&wire, chunks).await;
                let first = first.expect("generated valid FETCH");
                assert_eq!(first.fetched_messages, vec![expected.clone()]);
                assert_eq!(first.message, "complete");
                let next = next
                    .expect("first command succeeded")
                    .expect("next command");
                assert_eq!(next.message, "next-command");
                assert!(next.fetched_messages.is_empty());
            }
        });
    });
}

type Outcome = Result<CommandResult, EngineError>;

async fn run_commands(wire: &[u8], chunks: Vec<usize>) -> (Outcome, Option<Outcome>) {
    let mut session = ImapSession::new(Fragmented::new(wire, chunks));
    let first = execute(&mut session, COMMAND).await;
    let next = if first.is_ok() {
        Some(execute(&mut session, "NOOP").await)
    } else {
        None
    };
    (first, next)
}

async fn execute(session: &mut ImapSession<Fragmented<'_>>, command: &str) -> Outcome {
    // Synchronous CPU/recursion faults require libFuzzer's process-level limits.
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        session.execute_command(command),
    )
    .await
    .expect("parser pending beyond timeout")
}

fn structured_fetch(data: &[u8]) -> (Vec<u8>, RawFetchedMessage) {
    // Leave room for framing while retaining a 64 KiB total wire limit.
    let payload = &data[..data.len().min(60 * 1024)];
    let header = b"From: sender@example.org\r\nSubject: synthetic\r\n\r\n";
    let structure = r#"("TEXT" "PLAIN" NIL NIL "café (quoted)" "7BIT" 12 1)"#;
    let mut wire = format!(
        "* 1 FETCH (FLAGS (\\Seen) MODSEQ (99) BODYSTRUCTURE {structure} BODY[HEADER] {{{}}}\r\n",
        header.len()
    )
    .into_bytes();
    wire.extend_from_slice(header);
    wire.extend_from_slice(format!(" BODY[1] {{{}}}\r\n", payload.len()).as_bytes());
    wire.extend_from_slice(payload);
    wire.extend_from_slice(
        b" BODY[2] {0}\r\n UID 101)\r\nA0001 OK complete\r\nA0002 OK next-command\r\n",
    );
    let expected = RawFetchedMessage {
        seq: 1,
        uid: 101,
        modseq: Some(99),
        flags: vec!["\\Seen".into()],
        header_bytes: header.to_vec(),
        bodystructure: Some(structure.into()),
        sections: [("1".into(), payload.to_vec()), ("2".into(), vec![])].into(),
        ..Default::default()
    };
    (wire, expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_reviewed_seeds() {
        for (target, run) in [
            ("bodystructure", bodystructure as fn(&[u8])),
            ("fetch", fetch),
        ] {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("seeds")
                .join(target);
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                run(&std::fs::read(path).unwrap());
            }
            run(&[]);
        }
    }

    #[test]
    fn structured_literal_size_boundaries() {
        for size in [0, 1, 4095, 4096, 4097, MAX_INPUT] {
            let payload: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            fetch(&payload);
        }
        bodystructure(&vec![b'('; MAX_INPUT + 1]);
        fetch(&vec![0; MAX_INPUT + 1]);
    }
}
