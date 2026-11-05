//! Bounded deterministic properties, not exhaustive or coverage-guided fuzzing.
use std::pin::Pin;
use std::task::{Context, Poll};

use mail_engine::{
    parse_bodystructure_str, protocol::CommandResult, BodyPartStructure, EngineError, ImapSession,
    RawFetchedMessage,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const SEEDS: [u64; 4] = [1, 42, 4242, 0x5eed];

struct Fragmented {
    bytes: Vec<u8>,
    position: usize,
    chunks: Vec<usize>,
    chunk_index: usize,
    chunk_remaining: usize,
    reads: usize,
}

impl Fragmented {
    fn new(bytes: Vec<u8>, chunks: Vec<usize>) -> Self {
        assert!(bytes.len() <= 65536);
        assert!(!chunks.is_empty() && chunks.iter().all(|&n| n > 0));
        Self {
            bytes,
            position: 0,
            chunks,
            chunk_index: 0,
            chunk_remaining: 0,
            reads: 0,
        }
    }
}

impl AsyncRead for Fragmented {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // INVARIANT: Finite input, immediate EOF and a read budget bound every generated run
        // even if a parser accidentally retries EOF without yielding to Tokio's timeout.
        self.reads += 1;
        assert!(self.reads <= 100_000, "parser read budget exhausted");
        if self.chunk_remaining == 0 {
            self.chunk_remaining = self.chunks[self.chunk_index % self.chunks.len()];
            self.chunk_index += 1;
        }
        let n = self
            .chunk_remaining
            .min(buf.remaining())
            .min(self.bytes.len() - self.position);
        buf.put_slice(&self.bytes[self.position..self.position + n]);
        self.position += n;
        self.chunk_remaining -= n;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for Fragmented {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn fragmentations(seed: u64, wire: &[u8]) -> Vec<Vec<usize>> {
    let mut state = seed;
    let mut previous = 0;
    let boundaries = wire
        .windows(2)
        .enumerate()
        .filter_map(|(i, pair)| {
            if pair != b"\r\n" {
                return None;
            }
            let size = i + 2 - previous;
            previous = i + 2;
            Some(size)
        })
        .collect::<Vec<_>>();
    vec![
        if boundaries.is_empty() {
            vec![1]
        } else {
            boundaries
        },
        vec![1],
        vec![2, 3, 5, 7],
        vec![4096],
        (0..32)
            .map(|_| (next(&mut state) % 97 + 1) as usize)
            .collect(),
    ]
}

fn append_literal(wire: &mut Vec<u8>, section: &str, bytes: &[u8]) {
    wire.extend_from_slice(format!("BODY[{section}] {{{}}}\r\n", bytes.len()).as_bytes());
    wire.extend_from_slice(bytes);
}

fn generated_fetch(seed: u64) -> (Vec<u8>, Vec<RawFetchedMessage>) {
    let mut state = seed;
    let mut wire = Vec::new();
    let mut expected = Vec::new();
    let structure = r#"("TEXT" "PLAIN" NIL NIL "quoted ( \"" "7BIT" 12 1)"#;
    for seq in 1..=8 {
        let uid = seq + 100;
        let metadata = if seq % 2 == 0 {
            "FLAGS (\\Seen) MODSEQ (99)".into()
        } else {
            format!("UID {uid} MODSEQ (99) FLAGS (\\Seen)")
        };
        wire.extend_from_slice(
            format!("* {seq} FETCH ({metadata} BODYSTRUCTURE {structure} ").as_bytes(),
        );
        let header =
            format!("From: sender@example.org\r\nSubject: seed {seed}\r\n\r\n").into_bytes();
        let size = [0, 1, 2, 4095, 4096, 4097, 127, 256][seq as usize - 1];
        let mut body: Vec<u8> = (0..size).map(|_| (next(&mut state) >> 32) as u8).collect();
        let marker = b"\0\xff\r\n)\r\nA0001 OK forged\r\n* 9 FETCH (UID 999)\r\n";
        if size >= marker.len() {
            body[size - marker.len()..].copy_from_slice(marker);
        }
        append_literal(&mut wire, "HEADER", &header);
        wire.extend_from_slice(b" ");
        append_literal(&mut wire, "1", &body);
        wire.extend_from_slice(b" ");
        append_literal(&mut wire, "2", &[]);
        let suffix = if seq % 2 == 0 {
            format!(" UID {uid})\r\n")
        } else {
            ")\r\n".into()
        };
        wire.extend_from_slice(suffix.as_bytes());
        expected.push(RawFetchedMessage {
            seq,
            uid,
            modseq: Some(99),
            flags: vec!["\\Seen".into()],
            header_bytes: header,
            bodystructure: Some(structure.into()),
            sections: [("1".into(), body), ("2".into(), vec![])].into(),
            ..Default::default()
        });
    }
    wire.extend_from_slice(b"A0001 OK fetched\r\nA0002 OK next-command\r\n");
    (wire, expected)
}

#[tokio::test]
async fn generated_fetch_preserves_bytes_and_command_boundaries() {
    for seed in SEEDS {
        let (wire, expected) = generated_fetch(seed);
        for chunks in fragmentations(seed, &wire) {
            let mut session = ImapSession::new(Fragmented::new(wire.clone(), chunks.clone()));
            let result = execute(
                &mut session,
                "UID FETCH 101:108 (UID BODY.PEEK[HEADER] BODY.PEEK[1] BODY.PEEK[2])",
            )
            .await
            .unwrap();
            assert_eq!(
                result.fetched_messages, expected,
                "seed={seed}, chunks={chunks:?}"
            );
            assert_eq!(result.message, "fetched");
            let next = execute(&mut session, "NOOP").await.unwrap();
            assert_eq!(
                next.message, "next-command",
                "seed={seed}, chunks={chunks:?}"
            );
        }
    }
}

#[tokio::test]
async fn malformed_fetch_fails_deterministically() {
    let cases: Vec<Vec<u8>> = vec![
        b"* 1 FETCH (UID 1 BODY[1] {5}\r\nabc".to_vec(),
        b"* 1 FETCH (UID 1 BODY[1] {0}\r\n".to_vec(),
        b"* 1 FETCH (UID 1 BODY[1] {184467440737095516160}\r\nA0001 OK false-success\r\n".to_vec(),
        b"* 1 FETCH (UID 1 BODY[1] {oops}\r\nA0001 OK false-success\r\n".to_vec(),
        b"* 1 FETCH (UID 1 BODY[1] {1}\r\nx\r\nA0001 OK false-success\r\n".to_vec(),
        b"* 1 FETCH (UID 1)\xff\r\nA0001 OK invalid-control\r\n".to_vec(),
        b"* 1 FETCH (UID 1))\r\nA0001 OK extra-parenthesis\r\n".to_vec(),
        b"* 1 FETCH (UID 1 X \"unclosed)\r\nA0001 OK invalid-quote\r\n".to_vec(),
    ];
    let mut accepted = Vec::new();
    for (case, wire) in cases.into_iter().enumerate() {
        let mut first = None;
        for chunks in fragmentations(42, &wire) {
            let mut session = ImapSession::new(Fragmented::new(wire.clone(), chunks));
            let result = execute(&mut session, "UID FETCH 1 (BODY.PEEK[1])").await;
            if !matches!(result, Err(EngineError::ImapProtocol(_))) {
                accepted.push(case);
                break;
            }
            let error = result.unwrap_err().to_string();
            assert_eq!(first.get_or_insert(error.clone()), &error, "case={case}");
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted malformed FETCH cases: {accepted:?}"
    );
}

fn single(description: &str) -> String {
    format!("(\"TEXT\" \"PLAIN\" NIL NIL {description} \"7BIT\" 12 1)")
}

#[test]
fn generated_bodystructure_preserves_quoted_and_literal_strings() {
    for seed in SEEDS {
        for depth in 0..=16 {
            let description = format!("seed {seed} café \\\" (depth {depth})");
            let quoted = format!(
                "\"{}\"",
                description.replace('\\', "\\\\").replace('"', "\\\"")
            );
            let literal = format!("{{{}}}\r\n{description}", description.len());
            for encoded in [quoted, literal] {
                let mut input = single(&encoded);
                for _ in 0..depth {
                    input = format!("({input} \"MIXED\")");
                }
                let mut structure = parse_bodystructure_str(&input).unwrap();
                for _ in 0..depth {
                    let BodyPartStructure::Multipart(mut part) = structure else {
                        panic!("seed={seed}, depth={depth}")
                    };
                    assert_eq!(part.parts.len(), 1);
                    structure = part.parts.remove(0);
                }
                let BodyPartStructure::Single(part) = structure else {
                    panic!("expected leaf")
                };
                assert_eq!(
                    part.description.as_deref(),
                    Some(description.as_str()),
                    "seed={seed}, depth={depth}"
                );
                assert_eq!(part.octets, 12);
            }
        }
    }
}

#[test]
fn malformed_bodystructure_is_rejected() {
    let valid = single("NIL");
    let cases = [
        format!("{valid})"),
        format!("{valid} NIL"),
        valid[..valid.len() - 1].into(),
        "(\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" 12 1 \"unterminated)".into(),
        "(\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" 12 1 \"escape\\".into(),
        "(\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" 12 1 {100}\r\nx)".into(),
        "(\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" 12 1 {18446744073709551615}\r\nx)".into(),
        single("{3}abc"),
        single("{3}\nabc"),
        single("{1}\r\né"),
    ];
    let failures: Vec<_> = cases
        .iter()
        .enumerate()
        .filter_map(|(case, input)| {
            match std::panic::catch_unwind(|| parse_bodystructure_str(input)) {
                Ok(Err(_)) => None,
                _ => Some(case),
            }
        })
        .collect();
    assert!(
        failures.is_empty(),
        "accepted or panicked cases: {failures:?}"
    );
}

// The read budget additionally guards non-yielding EOF loops; timeout guards pending IO.
async fn execute(
    session: &mut ImapSession<Fragmented>,
    command: &str,
) -> Result<CommandResult, EngineError> {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        session.execute_command(command),
    )
    .await
    .expect("bounded parser execution")
}

#[tokio::test]
async fn every_truncated_fetch_prefix_fails() {
    let wire = b"* 1 FETCH (UID 1 BODY[1] {4}\r\nx\0\xffy)\r\nA0001 OK complete\r\n";
    for end in 0..wire.len() {
        for chunks in fragmentations(4242, &wire[..end]) {
            let mut session = ImapSession::new(Fragmented::new(wire[..end].to_vec(), chunks));
            assert!(
                execute(&mut session, "UID FETCH 1 (BODY.PEEK[1])")
                    .await
                    .is_err(),
                "prefix={end}"
            );
        }
    }
}

#[test]
fn bounded_generated_bodystructure_inputs_never_panic() {
    let alphabet = b"(){}\"\\0129 NIL\r\n";
    for seed in SEEDS {
        let mut state = seed;
        for case in 0..256 {
            let size = (next(&mut state) % 128) as usize;
            let input: String = (0..size)
                .map(|_| alphabet[(next(&mut state) >> 32) as usize % alphabet.len()] as char)
                .collect();
            let first = parse_bodystructure_str(&input);
            assert_eq!(
                first,
                parse_bodystructure_str(&input),
                "seed={seed}, case={case}"
            );
        }
    }
}
