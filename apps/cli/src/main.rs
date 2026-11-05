#![forbid(unsafe_code)]

mod args;
mod unlock;

use std::io::{self, Write};

use serde::Serialize;
use storage::cached::CachedMailReader;

use args::{Command, Options};

const HELP: &str = "triage-mail-cli --data-dir PATH [--unlock-stdin] COMMAND [OPTIONS]
Commands: accounts | search | show
accounts/search: --limit 1..500 (default 100), --offset N (default 0)
search: --query TEXT (default empty), --account OPAQUE_ID
show: --message-id OPAQUE_ID
--unlock-stdin reads an existing fallback-vault passphrase from non-terminal stdin.
Outputs bounded JSON containing private cached mail; no sync or network requests.
Exit codes: 0 success, 2 arguments, 3 vault, 4 cache/schema, 5 not found, 6 output/input.
";

#[derive(Clone, Copy, Debug)]
pub(crate) enum Error {
    Arguments,
    Vault,
    Cache,
    NotFound,
    Output,
}

impl Error {
    fn status(self) -> u8 {
        match self {
            Self::Arguments => 2,
            Self::Vault => 3,
            Self::Cache => 4,
            Self::NotFound => 5,
            Self::Output => 6,
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Arguments => "invalid arguments; use --help",
            Self::Vault => "existing vault unavailable",
            Self::Cache => "cached mail unavailable or unsupported schema",
            Self::NotFound => "cached message not found",
            Self::Output => "input or output unavailable or exceeds limits",
        }
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{}", error.message());
            std::process::ExitCode::from(error.status())
        }
    }
}

fn run() -> Result<(), Error> {
    let options = args::parse(std::env::args_os().skip(1))?;
    let Some(options) = options else {
        return io::stdout()
            .lock()
            .write_all(HELP.as_bytes())
            .map_err(|_| Error::Output);
    };
    let key = unlock::existing_key(&options)?;
    let reader = CachedMailReader::open(&options.data_dir.join("mail.db"), &key)
        .map_err(|_| Error::Cache)?;
    execute(&reader, options)
}

fn execute(reader: &CachedMailReader, options: Options) -> Result<(), Error> {
    let bytes = match options.command {
        Command::Accounts => json(
            &reader
                .accounts(options.limit, options.offset)
                .map_err(|_| Error::Cache)?,
        )?,
        Command::Search { query, account } => json(
            &reader
                .search(query, account, options.limit, options.offset)
                .map_err(|_| Error::Cache)?,
        )?,
        Command::Show { id } => json(
            &reader
                .show(&id)
                .map_err(|_| Error::Cache)?
                .ok_or(Error::NotFound)?,
        )?,
    };
    // INVARIANT: Complete bounded serialization precedes stdout, so oversized responses
    // cannot expose a partial JSON mail payload. JSON escapes terminal control bytes.
    io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|_| Error::Output)
}

struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > (4 * 1024 * 1024usize).saturating_sub(self.0.len()) {
            return Err(io::Error::other("response too large"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn json(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    let mut output = BoundedJson(Vec::new());
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, TerminalSafeJson);
    value
        .serialize(&mut serializer)
        .map_err(|_| Error::Output)?;
    output.write_all(b"\n").map_err(|_| Error::Output)?;
    Ok(output.0)
}

struct TerminalSafeJson;
impl serde_json::ser::Formatter for TerminalSafeJson {
    fn write_string_fragment<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        // INVARIANT: JSON's default escaping covers C0, but leaves DEL/C1 controls
        // literal. Escape every Unicode control so cached mail cannot emit them.
        let mut start = 0;
        for (index, character) in fragment.char_indices().filter(|(_, c)| c.is_control()) {
            writer.write_all(&fragment.as_bytes()[start..index])?;
            write!(writer, "\\u{:04x}", character as u32)?;
            start = index + character.len_utf8();
        }
        writer.write_all(&fragment.as_bytes()[start..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serialization_is_bounded_and_escapes_controls() {
        assert_eq!(
            json(&"\u{1b}[31m\n\0").unwrap(),
            b"\"\\u001b[31m\\n\\u0000\"\n"
        );
        assert!(matches!(
            json(&"x".repeat(4 * 1024 * 1024)),
            Err(Error::Output)
        ));
        let controls: String = (0..=0x9f)
            .filter_map(char::from_u32)
            .filter(|c| c.is_control())
            .collect();
        let bytes = json(&controls).unwrap();
        let output = std::str::from_utf8(&bytes).unwrap().trim_end_matches('\n');
        assert!(!output.chars().any(char::is_control));
        assert_eq!(serde_json::from_slice::<String>(&bytes).unwrap(), controls);
    }
}
