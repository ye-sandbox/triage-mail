use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

use crate::{MasterCipher, VaultError};

const MAGIC_PASSPHRASE_V1: &[u8; 4] = b"TMV1";
const MAGIC_DIRECT_KEY_V1: &[u8; 4] = b"TMK1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const MIN_PASSPHRASE_ENVELOPE_LEN: usize = 4 + SALT_LEN + NONCE_LEN + TAG_LEN;
const MIN_DIRECT_KEY_ENVELOPE_LEN: usize = 4 + NONCE_LEN + TAG_LEN;

/// Cryptographic cipher providing authenticated AES-256-GCM encryption with Argon2id KDF.
pub struct Argon2AesGcmCipher {
    secret: CipherSecret,
    params: Params,
}

enum CipherSecret {
    Passphrase(Zeroizing<Vec<u8>>),
    DirectKey(Zeroizing<[u8; 32]>),
}

impl Argon2AesGcmCipher {
    /// Constructs a cipher derived from a master passphrase using standard Argon2id parameters.
    pub fn from_passphrase(passphrase: &[u8]) -> Self {
        // INVARIANT: Use OWASP-aligned Argon2id parameters (19 MiB memory, 2 passes, 1 lane).
        let params =
            Params::new(19 * 1024, 2, 1, Some(32)).expect("valid default Argon2id parameters");
        Self::from_passphrase_with_params(passphrase, params)
    }

    /// Constructs a cipher derived from a master passphrase with custom Argon2id parameters.
    pub fn from_passphrase_with_params(passphrase: &[u8], params: Params) -> Self {
        Self {
            secret: CipherSecret::Passphrase(Zeroizing::new(passphrase.to_vec())),
            params,
        }
    }

    /// Constructs a cipher directly from a 256-bit symmetric key.
    pub fn from_key(key: [u8; 32]) -> Self {
        let params =
            Params::new(19 * 1024, 2, 1, Some(32)).expect("valid default Argon2id parameters");
        Self {
            secret: CipherSecret::DirectKey(Zeroizing::new(key)),
            params,
        }
    }

    fn derive_key(
        &self,
        salt: &[u8; SALT_LEN],
        passphrase: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, VaultError> {
        let mut derived = Zeroizing::new([0u8; 32]);
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone());
        argon2
            .hash_password_into(passphrase, salt, derived.as_mut())
            .map_err(|err| VaultError::CryptoError(format!("key derivation failed: {err}")))?;
        Ok(derived)
    }

    fn generate_random_bytes<const N: usize>() -> Result<[u8; N], VaultError> {
        let mut bytes = [0u8; N];
        getrandom::getrandom(&mut bytes)
            .map_err(|err| VaultError::CryptoError(format!("entropy generation failed: {err}")))?;
        Ok(bytes)
    }
}

impl MasterCipher for Argon2AesGcmCipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let nonce_bytes = Self::generate_random_bytes::<NONCE_LEN>()?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        match &self.secret {
            CipherSecret::Passphrase(passphrase) => {
                let salt_bytes = Self::generate_random_bytes::<SALT_LEN>()?;
                let derived_key = self.derive_key(&salt_bytes, passphrase)?;
                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(derived_key.as_ref()));

                let ciphertext = cipher
                    .encrypt(nonce, plaintext)
                    .map_err(|err| VaultError::CryptoError(format!("encryption failed: {err}")))?;

                let mut envelope = Vec::with_capacity(4 + SALT_LEN + NONCE_LEN + ciphertext.len());
                envelope.extend_from_slice(MAGIC_PASSPHRASE_V1);
                envelope.extend_from_slice(&salt_bytes);
                envelope.extend_from_slice(&nonce_bytes);
                envelope.extend_from_slice(&ciphertext);
                Ok(envelope)
            }
            CipherSecret::DirectKey(key) => {
                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
                let ciphertext = cipher
                    .encrypt(nonce, plaintext)
                    .map_err(|err| VaultError::CryptoError(format!("encryption failed: {err}")))?;

                let mut envelope = Vec::with_capacity(4 + NONCE_LEN + ciphertext.len());
                envelope.extend_from_slice(MAGIC_DIRECT_KEY_V1);
                envelope.extend_from_slice(&nonce_bytes);
                envelope.extend_from_slice(&ciphertext);
                Ok(envelope)
            }
        }
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, VaultError> {
        if ciphertext.len() < 4 {
            return Err(VaultError::CryptoError(
                "ciphertext payload too short".into(),
            ));
        }

        let magic = &ciphertext[0..4];
        if magic == MAGIC_PASSPHRASE_V1 {
            if ciphertext.len() < MIN_PASSPHRASE_ENVELOPE_LEN {
                return Err(VaultError::CryptoError(
                    "passphrase envelope truncated".into(),
                ));
            }

            let passphrase = match &self.secret {
                CipherSecret::Passphrase(p) => p.as_slice(),
                CipherSecret::DirectKey(_) => {
                    return Err(VaultError::CryptoError(
                        "cannot decrypt passphrase-derived envelope with direct key cipher".into(),
                    ));
                }
            };

            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&ciphertext[4..20]);
            let nonce = Nonce::from_slice(&ciphertext[20..32]);
            let payload = &ciphertext[32..];

            let derived_key = self.derive_key(&salt, passphrase)?;
            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(derived_key.as_ref()));

            cipher.decrypt(nonce, payload).map_err(|_| {
                VaultError::CryptoError(
                    "authentication tag verification failed or corrupted ciphertext".into(),
                )
            })
        } else if magic == MAGIC_DIRECT_KEY_V1 {
            if ciphertext.len() < MIN_DIRECT_KEY_ENVELOPE_LEN {
                return Err(VaultError::CryptoError(
                    "direct key envelope truncated".into(),
                ));
            }

            let key = match &self.secret {
                CipherSecret::DirectKey(k) => k.as_slice(),
                CipherSecret::Passphrase(_) => {
                    return Err(VaultError::CryptoError(
                        "cannot decrypt direct-key envelope with passphrase cipher".into(),
                    ));
                }
            };

            let nonce = Nonce::from_slice(&ciphertext[4..16]);
            let payload = &ciphertext[16..];

            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
            cipher.decrypt(nonce, payload).map_err(|_| {
                VaultError::CryptoError(
                    "authentication tag verification failed or corrupted ciphertext".into(),
                )
            })
        } else {
            Err(VaultError::CryptoError(
                "unknown envelope magic identifier".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> Params {
        // Fast parameters for unit tests (1 MiB memory, 1 pass, 1 lane)
        Params::new(1024, 1, 1, Some(32)).expect("valid test params")
    }

    #[test]
    fn roundtrip_passphrase_encryption_and_decryption() {
        let cipher = Argon2AesGcmCipher::from_passphrase_with_params(
            b"super-secret-passphrase",
            test_params(),
        );
        let plaintext = b"Hello, local-first secure triage!";

        let encrypted = cipher.encrypt(plaintext).expect("encryption succeeds");
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.starts_with(MAGIC_PASSPHRASE_V1));

        let decrypted = cipher.decrypt(&encrypted).expect("decryption succeeds");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn roundtrip_direct_key_encryption_and_decryption() {
        let key = [42u8; 32];
        let cipher = Argon2AesGcmCipher::from_key(key);
        let plaintext = b"Database SQLCipher key material";

        let encrypted = cipher.encrypt(plaintext).expect("encryption succeeds");
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.starts_with(MAGIC_DIRECT_KEY_V1));

        let decrypted = cipher.decrypt(&encrypted).expect("decryption succeeds");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn fails_on_corrupted_magic() {
        let cipher = Argon2AesGcmCipher::from_passphrase_with_params(b"pass", test_params());
        let mut encrypted = cipher.encrypt(b"data").expect("encrypt");
        encrypted[0] = b'X';

        let result = cipher.decrypt(&encrypted);
        assert_eq!(
            result,
            Err(VaultError::CryptoError(
                "unknown envelope magic identifier".into()
            ))
        );
    }

    #[test]
    fn fails_on_bit_flip_tampering() {
        let cipher = Argon2AesGcmCipher::from_passphrase_with_params(b"pass", test_params());
        let mut encrypted = cipher.encrypt(b"sensitive data").expect("encrypt");

        // Corrupt a byte in the encrypted payload / tag area
        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0x01;

        let result = cipher.decrypt(&encrypted);
        assert_eq!(
            result,
            Err(VaultError::CryptoError(
                "authentication tag verification failed or corrupted ciphertext".into()
            ))
        );
    }

    #[test]
    fn fails_on_truncated_envelope() {
        let cipher = Argon2AesGcmCipher::from_passphrase_with_params(b"pass", test_params());
        let encrypted = cipher.encrypt(b"secret").expect("encrypt");
        let truncated = &encrypted[..15];

        let result = cipher.decrypt(truncated);
        assert_eq!(
            result,
            Err(VaultError::CryptoError(
                "passphrase envelope truncated".into()
            ))
        );
    }

    #[test]
    fn fails_on_incorrect_passphrase() {
        let cipher1 =
            Argon2AesGcmCipher::from_passphrase_with_params(b"correct-password", test_params());
        let cipher2 =
            Argon2AesGcmCipher::from_passphrase_with_params(b"wrong-password", test_params());

        let encrypted = cipher1.encrypt(b"private email auth").expect("encrypt");
        let result = cipher2.decrypt(&encrypted);
        assert_eq!(
            result,
            Err(VaultError::CryptoError(
                "authentication tag verification failed or corrupted ciphertext".into()
            ))
        );
    }
}
