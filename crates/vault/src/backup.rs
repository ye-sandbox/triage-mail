//! Version-1 bounded authenticated record stream. No secret store is exported.
use std::io::{Read, Write};

use argon2::Argon2;
use zeroize::Zeroizing;

use crate::{Argon2AesGcmCipher, MasterCipher, VaultError};

const MAGIC: &[u8; 8] = b"TMBACK01";
pub const MAX_RECORD: usize = 64 * 1024 * 1024;
fn invalid() -> VaultError {
    VaultError::CryptoError("invalid backup, unsupported version or incorrect passphrase".into())
}
fn cipher(passphrase: &[u8], salt: &[u8; 16]) -> Result<Argon2AesGcmCipher, VaultError> {
    if passphrase.len() < 12 || passphrase.len() > 4096 {
        return Err(invalid());
    }
    let mut key = Zeroizing::new([0u8; 32]);
    // INVARIANT: V1 fixes Argon2id v19 parameters (19MiB, t=2, p=1); the file
    // cannot request attacker-controlled KDF work. A format change needs a new magic.
    let params = argon2::Params::new(19 * 1024, 2, 1, Some(32)).map_err(|_| invalid())?;
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(passphrase, salt, key.as_mut())
        .map_err(|_| invalid())?;
    Ok(Argon2AesGcmCipher::from_key(*key))
}

pub struct BackupWriter<W: Write> {
    output: W,
    cipher: Argon2AesGcmCipher,
    sequence: u64,
}
impl<W: Write> BackupWriter<W> {
    pub fn new(mut output: W, passphrase: &[u8]) -> Result<Self, VaultError> {
        let mut salt = [0; 16];
        getrandom::getrandom(&mut salt).map_err(|_| invalid())?;
        let cipher = cipher(passphrase, &salt)?;
        output
            .write_all(MAGIC)
            .and_then(|()| output.write_all(&salt))
            .map_err(|_| invalid())?;
        Ok(Self {
            output,
            cipher,
            sequence: 0,
        })
    }
    pub fn record(&mut self, data: &[u8]) -> Result<(), VaultError> {
        self.frame(data, false)
    }
    fn frame(&mut self, data: &[u8], final_record: bool) -> Result<(), VaultError> {
        if data.len() > MAX_RECORD {
            return Err(invalid());
        }
        let mut plain = Zeroizing::new(Vec::with_capacity(data.len() + 9));
        plain.extend_from_slice(&self.sequence.to_be_bytes());
        plain.push(u8::from(final_record));
        plain.extend_from_slice(data);
        let encrypted = self.cipher.encrypt(&plain)?;
        self.output
            .write_all(&(encrypted.len() as u32).to_be_bytes())
            .and_then(|()| self.output.write_all(&encrypted))
            .map_err(|_| invalid())?;
        self.sequence = self.sequence.checked_add(1).ok_or_else(invalid)?;
        Ok(())
    }
    pub fn finish(mut self) -> Result<W, VaultError> {
        self.frame(&[], true)?;
        self.output.flush().map_err(|_| invalid())?;
        Ok(self.output)
    }
}

pub struct BackupReader<R: Read> {
    input: R,
    cipher: Argon2AesGcmCipher,
    sequence: u64,
    finished: bool,
}
impl<R: Read> BackupReader<R> {
    pub fn new(mut input: R, passphrase: &[u8]) -> Result<Self, VaultError> {
        let mut magic = [0; 8];
        let mut salt = [0; 16];
        input
            .read_exact(&mut magic)
            .and_then(|()| input.read_exact(&mut salt))
            .map_err(|_| invalid())?;
        if &magic != MAGIC {
            return Err(invalid());
        }
        Ok(Self {
            input,
            cipher: cipher(passphrase, &salt)?,
            sequence: 0,
            finished: false,
        })
    }
    pub fn record(&mut self) -> Result<Option<Zeroizing<Vec<u8>>>, VaultError> {
        if self.finished {
            return Ok(None);
        }
        let mut length = [0; 4];
        self.input.read_exact(&mut length).map_err(|_| invalid())?;
        let length = u32::from_be_bytes(length) as usize;
        if !(41..=MAX_RECORD + 41).contains(&length) {
            return Err(invalid());
        }
        let mut encrypted = vec![0; length];
        self.input
            .read_exact(&mut encrypted)
            .map_err(|_| invalid())?;
        let plain = Zeroizing::new(self.cipher.decrypt(&encrypted)?);
        if plain.len() < 9 || plain[..8] != self.sequence.to_be_bytes() {
            return Err(invalid());
        }
        self.sequence = self.sequence.checked_add(1).ok_or_else(invalid)?;
        // INVARIANT: The authenticated sequence and mandatory final frame detect
        // removed/reordered frames and truncation, including at a record boundary.
        match plain[8] {
            0 => Ok(Some(Zeroizing::new(plain[9..].to_vec()))),
            1 if plain.len() == 9 => {
                if self.input.read(&mut [0]).map_err(|_| invalid())? != 0 {
                    return Err(invalid());
                }
                self.finished = true;
                Ok(None)
            }
            _ => Err(invalid()),
        }
    }
}

/// Fresh opaque identity without exposing key material.
pub fn random_id() -> Result<String, VaultError> {
    use std::fmt::Write;
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| invalid())?;
    let mut id = String::with_capacity(32);
    for byte in bytes {
        write!(id, "{byte:02x}").map_err(|_| invalid())?;
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PASS: &[u8] = b"synthetic backup passphrase";
    fn artifact() -> Vec<u8> {
        let mut w = BackupWriter::new(Vec::new(), PASS).unwrap();
        w.record(b"synthetic mail").unwrap();
        w.record(b"second").unwrap();
        w.finish().unwrap()
    }
    fn read(bytes: &[u8], pass: &[u8]) -> Result<Vec<Vec<u8>>, VaultError> {
        let mut reader = BackupReader::new(bytes, pass)?;
        let mut rows = Vec::new();
        while let Some(row) = reader.record()? {
            rows.push(row.to_vec());
        }
        Ok(rows)
    }
    #[test]
    fn roundtrip_and_randomized_encryption() {
        let a = artifact();
        assert_ne!(a, artifact());
        assert_eq!(
            read(&a, PASS).unwrap(),
            vec![b"synthetic mail".to_vec(), b"second".to_vec()]
        );
    }
    #[test]
    fn rejects_wrong_password_corruption_versions_and_truncation() {
        let a = artifact();
        assert!(read(&a, b"wrong synthetic passphrase").is_err());
        for n in [0, 24, a.len() - 1, a.len() - 45] {
            assert!(read(&a[..n], PASS).is_err());
        }
        for pos in [0, 8, 28, a.len() - 1] {
            let mut b = a.clone();
            b[pos] ^= 1;
            assert!(read(&b, PASS).is_err());
        }
        let mut b = a.clone();
        b.push(0);
        assert!(read(&b, PASS).is_err());
        let mut b = a.clone();
        b[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(read(&b, PASS).is_err());
    }
    #[test]
    fn rejects_reordered_or_duplicated_frames() {
        let a = artifact();
        let end = 28 + u32::from_be_bytes(a[24..28].try_into().unwrap()) as usize;
        let mut b = a[..end].to_vec();
        b.extend_from_slice(&a[24..]);
        assert!(read(&b, PASS).is_err());
    }
}
