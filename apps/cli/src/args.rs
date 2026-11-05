use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

use crate::Error;

pub enum Command {
    Accounts,
    Search {
        query: String,
        account: Option<String>,
    },
    Show {
        id: String,
    },
}

pub struct Options {
    pub data_dir: PathBuf,
    pub unlock_stdin: bool,
    pub limit: usize,
    pub offset: usize,
    pub command: Command,
}

pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Option<Options>, Error> {
    let args = args
        .take(33)
        .map(|arg| arg.into_string().map_err(|_| Error::Arguments))
        .collect::<Result<Vec<_>, _>>()?;
    if args == ["--help"] {
        return Ok(None);
    }
    if args.is_empty() || args.len() > 32 || args.iter().any(|arg| arg.len() > 32768) {
        return Err(Error::Arguments);
    }
    let (mut flags, command) = flags(&args)?;
    let data_dir = PathBuf::from(flags.remove("--data-dir").ok_or(Error::Arguments)?);
    if data_dir.as_os_str().is_empty() {
        return Err(Error::Arguments);
    }
    let unlock_stdin = flags.remove("--unlock-stdin").is_some();
    let (limit, offset) = if command == "show" {
        (100, 0)
    } else {
        (
            number(flags.remove("--limit"), 100)?,
            number(flags.remove("--offset"), 0)?,
        )
    };
    if limit == 0
        || limit > 500
        || offset
            .checked_add(limit)
            .is_none_or(|n| i64::try_from(n).is_err())
    {
        return Err(Error::Arguments);
    }
    let command = match command.as_str() {
        "accounts" => Command::Accounts,
        "search" => Command::Search {
            query: flags.remove("--query").unwrap_or_default(),
            account: flags.remove("--account").map(identity).transpose()?,
        },
        "show" => Command::Show {
            id: identity(flags.remove("--message-id").ok_or(Error::Arguments)?)?,
        },
        _ => return Err(Error::Arguments),
    };
    if !flags.is_empty() {
        return Err(Error::Arguments);
    }
    Ok(Some(Options {
        data_dir,
        unlock_stdin,
        limit,
        offset,
        command,
    }))
}

fn flags(args: &[String]) -> Result<(BTreeMap<String, String>, String), Error> {
    let mut flags = BTreeMap::new();
    let mut command = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') {
            if command.replace(arg.clone()).is_some() {
                return Err(Error::Arguments);
            }
            continue;
        }
        let value = match arg.as_str() {
            "--unlock-stdin" => String::new(),
            "--data-dir" | "--query" | "--account" | "--message-id" | "--limit" | "--offset" => {
                iter.next().ok_or(Error::Arguments)?.clone()
            }
            _ => return Err(Error::Arguments),
        };
        if flags.insert(arg.clone(), value).is_some() {
            return Err(Error::Arguments);
        }
    }
    Ok((flags, command.ok_or(Error::Arguments)?))
}

fn number(value: Option<String>, default: usize) -> Result<usize, Error> {
    match value {
        None => Ok(default),
        Some(value) if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
            value.parse().map_err(|_| Error::Arguments)
        }
        Some(_) => Err(Error::Arguments),
    }
}

fn identity(value: String) -> Result<String, Error> {
    if value.is_empty()
        || value.len() > 256
        || value.contains('@')
        || value.chars().any(char::is_control)
    {
        return Err(Error::Arguments);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parsed(args: &[&str]) -> Result<Option<Options>, Error> {
        parse(args.iter().map(OsString::from))
    }
    #[test]
    fn rejects_ambiguous_or_unbounded_arguments() {
        for args in [
            vec![],
            vec!["accounts"],
            vec!["--help", "accounts"],
            vec!["--data-dir", "cache", "search", "--limit", "0"],
            vec!["--data-dir", "cache", "search", "--limit", "501"],
            vec![
                "--data-dir",
                "cache",
                "search",
                "--offset",
                "18446744073709551615",
            ],
            vec!["--data-dir", "cache", "accounts", "--query", "private"],
            vec![
                "--data-dir",
                "cache",
                "show",
                "--message-id",
                "x",
                "--limit",
                "2",
            ],
            vec![
                "--data-dir",
                "cache",
                "search",
                "--account",
                "reader@example.org",
            ],
            vec![
                "--data-dir",
                "cache",
                "accounts",
                "--unlock-stdin",
                "--unlock-stdin",
            ],
            vec!["--data-dir", "cache", "accounts", "--secret", "synthetic"],
        ] {
            assert!(parsed(&args).is_err());
        }
        assert!(parsed(&["--help"]).unwrap().is_none());
        let options = parsed(&[
            "search",
            "--data-dir",
            "cache",
            "--account",
            "a",
            "--query",
            "boleto",
            "--limit",
            "5",
            "--offset",
            "10",
        ])
        .unwrap()
        .unwrap();
        assert_eq!((options.limit, options.offset), (5, 10));
    }
}
