use std::{
    io::{self, IsTerminal, Read},
    path::Path,
    sync::Arc,
};

use vault::{SecretStore, VaultKey};
use zeroize::Zeroizing;

use crate::{args::Options, Error};

pub fn existing_key(options: &Options) -> Result<storage::DatabaseKey, Error> {
    require_directory(&options.data_dir)?;
    require_file(&options.data_dir.join("mail.db"))?;
    let recovered = match std::fs::symlink_metadata(options.data_dir.join("recovered-local-vault"))
    {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => true,
        Ok(_) => return Err(Error::Vault),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(_) => return Err(Error::Vault),
    };
    let id = VaultKey::for_account("application", "database-key").map_err(|_| Error::Vault)?;
    let raw = if options.unlock_stdin {
        // Explicit fallback unlock selects only that vault, preventing a keyring entry
        // from overriding the supplied passphrase or a restored installation's identity.
        let passphrase = read_passphrase(io::stdin().lock(), io::stdin().is_terminal())?;
        let path = options.data_dir.join("vault.enc");
        require_file(&path).map_err(|_| Error::Vault)?;
        let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(&passphrase));
        vault::existing::ExistingVaultReader::open(&path, cipher)
            .and_then(|reader| reader.get_secret(&id))
            .map_err(|_| Error::Vault)?
    } else {
        if recovered {
            return Err(Error::Vault);
        }
        Zeroizing::new(
            vault::KeyringStore::new()
                .get_secret(&id)
                .map_err(|_| Error::Vault)?,
        )
    };
    let mut bytes = Zeroizing::new([0u8; 32]);
    if raw.len() != bytes.len() {
        return Err(Error::Vault);
    }
    bytes.copy_from_slice(&raw);
    Ok(storage::DatabaseKey::from_bytes(*bytes))
}

fn require_directory(path: &Path) -> Result<(), Error> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Cache)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(Error::Cache);
    }
    Ok(())
}

fn require_file(path: &Path) -> Result<(), Error> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Cache)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(Error::Cache);
    }
    Ok(())
}

fn read_passphrase(input: impl Read, terminal: bool) -> Result<Zeroizing<Vec<u8>>, Error> {
    // INVARIANT: Refuse terminal input to avoid echo; never accept secrets in argv/env.
    if terminal {
        return Err(Error::Output);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    input
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Output)?;
    if bytes.len() > 4096 {
        return Err(Error::Output);
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.is_empty()
        || bytes.iter().any(|b| matches!(b, 0 | b'\n' | b'\r'))
        || std::str::from_utf8(&bytes).is_err()
    {
        return Err(Error::Vault);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_stdin_and_preserves_spaces() {
        assert_eq!(
            &*read_passphrase(&b" synthetic phrase \r\n"[..], false).unwrap(),
            b" synthetic phrase "
        );
        for value in [
            vec![],
            vec![b'x'; 4097],
            b"a\nb".to_vec(),
            vec![0xff],
            vec![0],
        ] {
            assert!(read_passphrase(value.as_slice(), false).is_err());
        }
        assert!(read_passphrase(&b"synthetic"[..], true).is_err());
    }
}
