#![forbid(unsafe_code)]

mod args;
mod unlock;

use std::io::{self, Write};

use serde::Serialize;
use storage::cached::CachedMailReader;

use args::{Command, Options};

const HELP: &str = "triage-mail-cli --data-dir PATH [--unlock-stdin] COMMAND [OPTIONS]
Commands: accounts | search | show | sync | daemon
accounts/search: --limit 1..500 (default 100), --offset N (default 0)
search: --query TEXT (default empty), --account OPAQUE_ID
show: --message-id OPAQUE_ID
sync: [--account OPAQUE_ID]
daemon: [--interval-secs 1..86400 (default 60)] [--account OPAQUE_ID] [--max-iterations N]
--unlock-stdin reads an existing fallback-vault passphrase from non-terminal stdin.
Outputs bounded JSON containing private cached mail; accounts/search/show perform no network requests.
Exit codes: 0 success, 2 arguments, 3 vault, 4 cache/schema, 5 not found, 6 output/input, 7 lock contention, 8 remote error.
";

#[derive(Clone, Copy, Debug)]
pub(crate) enum Error {
    Arguments,
    Vault,
    Cache,
    NotFound,
    Output,
    Contention,
    Remote,
}

impl Error {
    fn status(self) -> u8 {
        match self {
            Self::Arguments => 2,
            Self::Vault => 3,
            Self::Cache => 4,
            Self::NotFound => 5,
            Self::Output => 6,
            Self::Contention => 7,
            Self::Remote => 8,
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Arguments => "invalid arguments; use --help",
            Self::Vault => "existing vault unavailable",
            Self::Cache => "cached mail unavailable or unsupported schema",
            Self::NotFound => "cached message or account not found",
            Self::Output => "input or output unavailable or exceeds limits",
            Self::Contention => "installation already owned by another writer",
            Self::Remote => "remote IMAP sync failed",
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
    if matches!(options.command, Command::Sync { .. }) {
        return sync(options);
    }
    if matches!(options.command, Command::Daemon { .. }) {
        return daemon(options);
    }
    let key = unlock::existing_key(&options)?;
    let reader = CachedMailReader::open(&options.data_dir.join("mail.db"), &key)
        .map_err(|_| Error::Cache)?;
    execute(&reader, options)
}

fn sync(options: Options) -> Result<(), Error> {
    let account = match options.command {
        Command::Sync { account } => account,
        _ => None,
    };
    let ownership = match sync_runtime::ownership::WriterOwnership::acquire(&options.data_dir) {
        Ok(guard) => std::sync::Arc::new(guard),
        Err(sync_runtime::ownership::OwnershipError::Busy) => return Err(Error::Contention),
        Err(sync_runtime::ownership::OwnershipError::Unavailable) => return Err(Error::Cache),
    };
    let passphrase = if options.unlock_stdin {
        Some(unlock::read_stdin_passphrase()?)
    } else {
        None
    };
    let installation = sync_runtime::bootstrap::open(&options.data_dir, passphrase, ownership)
        .map_err(|e| match e {
            sync_runtime::bootstrap::BootstrapError::VaultUnavailable => Error::Vault,
            sync_runtime::bootstrap::BootstrapError::StorageUnavailable => Error::Cache,
            sync_runtime::bootstrap::BootstrapError::InvalidRequest => Error::Arguments,
            sync_runtime::bootstrap::BootstrapError::Internal => Error::Cache,
        })?;
    let transport = sync_runtime::DefaultSyncTransport::new(installation.vault.clone());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Cache)?;
    let summary = rt
        .block_on(async {
            sync_runtime::sync_accounts(&installation.store, &transport, account.as_deref()).await
        })
        .map_err(|e| match e {
            sync_runtime::HeadlessSyncError::AccountNotFound(_) => Error::NotFound,
            sync_runtime::HeadlessSyncError::InvalidConfig(_) => Error::Arguments,
            sync_runtime::HeadlessSyncError::Storage(_)
            | sync_runtime::HeadlessSyncError::Ingest(_)
            | sync_runtime::HeadlessSyncError::Json(_) => Error::Cache,
            sync_runtime::HeadlessSyncError::Engine(_) => Error::Remote,
        })?;

    let bytes = json(&summary)?;
    io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|_| Error::Output)?;

    let all_failed =
        !summary.accounts.is_empty() && summary.accounts.iter().all(|a| a.error.is_some());
    if all_failed {
        return Err(Error::Remote);
    }
    Ok(())
}

fn daemon(options: Options) -> Result<(), Error> {
    let (interval_secs, account, max_iterations) = match options.command {
        Command::Daemon {
            interval_secs,
            account,
            max_iterations,
        } => (interval_secs, account, max_iterations),
        _ => return Err(Error::Arguments),
    };

    let ownership = match sync_runtime::ownership::WriterOwnership::acquire(&options.data_dir) {
        Ok(guard) => std::sync::Arc::new(guard),
        Err(sync_runtime::ownership::OwnershipError::Busy) => return Err(Error::Contention),
        Err(sync_runtime::ownership::OwnershipError::Unavailable) => return Err(Error::Cache),
    };
    let passphrase = if options.unlock_stdin {
        Some(unlock::read_stdin_passphrase()?)
    } else {
        None
    };
    let installation = sync_runtime::bootstrap::open(&options.data_dir, passphrase, ownership)
        .map_err(|e| match e {
            sync_runtime::bootstrap::BootstrapError::VaultUnavailable => Error::Vault,
            sync_runtime::bootstrap::BootstrapError::StorageUnavailable => Error::Cache,
            sync_runtime::bootstrap::BootstrapError::InvalidRequest => Error::Arguments,
            sync_runtime::bootstrap::BootstrapError::Internal => Error::Cache,
        })?;
    let transport = sync_runtime::DefaultSyncTransport::new(installation.vault.clone());

    let config = sync_runtime::DaemonConfig::default()
        .with_account_filter(account)
        .with_max_iterations(max_iterations);
    let config = sync_runtime::DaemonConfig {
        interval_secs,
        account_filter: config.account_filter,
        max_iterations: config.max_iterations,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Cache)?;

    let sig_tx = shutdown_tx.clone();
    rt.spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).ok();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = async {
                    if let Some(ref mut term) = sigterm {
                        term.recv().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        let _ = sig_tx.send(true);
    });

    rt.block_on(async {
        sync_runtime::run_daemon(
            &installation.store,
            &transport,
            config,
            shutdown_rx,
            |event| {
                if let Ok(line) = serde_json::to_string(&event) {
                    let mut stdout = io::stdout().lock();
                    let _ = writeln!(stdout, "{line}");
                }
            },
        )
        .await
    })
    .map_err(|e| match e {
        sync_runtime::HeadlessSyncError::AccountNotFound(_) => Error::NotFound,
        sync_runtime::HeadlessSyncError::InvalidConfig(_) => Error::Arguments,
        sync_runtime::HeadlessSyncError::Storage(_)
        | sync_runtime::HeadlessSyncError::Ingest(_)
        | sync_runtime::HeadlessSyncError::Json(_) => Error::Cache,
        sync_runtime::HeadlessSyncError::Engine(_) => Error::Remote,
    })?;

    Ok(())
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
        Command::Sync { .. } | Command::Daemon { .. } => unreachable!(),
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
