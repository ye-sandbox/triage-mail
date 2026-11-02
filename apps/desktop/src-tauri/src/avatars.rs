//! Native-only, bounded local image normalization. Never fetches images or accepts paths.
use std::{io::Cursor, sync::Arc};

use image::{imageops::FilterType, ImageFormat, ImageReader, Limits};
use storage::AccountAvatar;

use crate::{DesktopService, IpcError};

pub const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;

pub fn normalize_avatar(bytes: &[u8]) -> Result<AccountAvatar, IpcError> {
    if bytes.is_empty() || bytes.len() > MAX_AVATAR_BYTES {
        return Err(IpcError::InvalidRequest);
    }
    let format = image::guess_format(bytes).map_err(|_| IpcError::InvalidRequest)?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(IpcError::InvalidRequest);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(1024);
    limits.max_image_height = Some(1024);
    limits.max_alloc = Some(32 * 1024 * 1024);
    // INVARIANT: Install dimension/allocation limits before decoding untrusted local bytes.
    reader.limits(limits);
    let image = reader.decode().map_err(|_| IpcError::InvalidRequest)?;
    if image.width() == 0 || image.height() == 0 {
        return Err(IpcError::InvalidRequest);
    }
    // INVARIANT: A fresh RGBA buffer and PNG encoder carry no original EXIF/text/profile metadata.
    let image = image
        .resize_to_fill(128, 128, FilterType::Triangle)
        .to_rgba8();
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| IpcError::InvalidRequest)?;
    Ok(AccountAvatar {
        bytes: output.into_inner(),
        mime_type: "image/png".into(),
        width: 128,
        height: 128,
    })
}

impl DesktopService {
    pub async fn get_account_avatar(
        self: &Arc<Self>,
        account_id: String,
    ) -> Result<Option<AccountAvatar>, IpcError> {
        self.account(account_id.clone()).await?;
        self.blocking(move |s| {
            s.store
                .account_avatar(&account_id)
                .map_err(|_| IpcError::StorageUnavailable)
        })
        .await
    }

    pub async fn set_account_avatar(
        self: &Arc<Self>,
        account_id: String,
        bytes: Vec<u8>,
    ) -> Result<AccountAvatar, IpcError> {
        if bytes.is_empty() || bytes.len() > MAX_AVATAR_BYTES {
            return Err(IpcError::InvalidRequest);
        }
        self.account(account_id.clone()).await?;
        self.blocking(move |s| {
            let avatar = normalize_avatar(&bytes)?;
            // INVARIANT: Validation completes before the upsert, preserving a photo on invalid replacement.
            s.store
                .save_account_avatar(&account_id, &avatar)
                .map_err(|_| IpcError::StorageUnavailable)?;
            Ok(avatar)
        })
        .await
    }

    pub async fn remove_account_avatar(
        self: &Arc<Self>,
        account_id: String,
    ) -> Result<(), IpcError> {
        self.account(account_id.clone()).await?;
        self.blocking(move |s| {
            s.store
                .remove_account_avatar(&account_id)
                .map_err(|_| IpcError::StorageUnavailable)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GenericImageView};

    #[test]
    fn accepts_rasters_and_normalizes_dimensions_and_format() {
        for format in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP] {
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::new_rgb8(80, 40)
                .write_to(&mut bytes, format)
                .unwrap();
            let avatar = normalize_avatar(bytes.get_ref()).unwrap();
            assert_eq!(avatar.mime_type, "image/png");
            assert_eq!(
                image::load_from_memory(&avatar.bytes).unwrap().dimensions(),
                (128, 128)
            );
        }
    }

    #[test]
    fn rejects_empty_invalid_unsupported_oversized_and_large_dimensions() {
        for bytes in [
            vec![],
            b"<svg/>".to_vec(),
            b"https://example.com/photo.png".to_vec(),
            vec![0; MAX_AVATAR_BYTES + 1],
            b"GIF89a".to_vec(),
        ] {
            assert_eq!(normalize_avatar(&bytes), Err(IpcError::InvalidRequest));
        }
        for (w, h) in [(1025, 1), (1, 1025)] {
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::new_rgb8(w, h)
                .write_to(&mut bytes, ImageFormat::Png)
                .unwrap();
            assert_eq!(
                normalize_avatar(bytes.get_ref()),
                Err(IpcError::InvalidRequest)
            );
        }
    }

    #[test]
    fn strips_jpeg_comment_and_exif_metadata() {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(32, 32)
            .write_to(&mut bytes, ImageFormat::Jpeg)
            .unwrap();
        let mut original = bytes.into_inner();
        let comment = b"private-synthetic-metadata";
        let mut segment = vec![0xff, 0xfe, 0, (comment.len() + 2) as u8];
        segment.extend_from_slice(comment);
        original.splice(2..2, segment);
        let exif = b"Exif\0\0private-synthetic-exif";
        let mut app1 = vec![0xff, 0xe1, 0, (exif.len() + 2) as u8];
        app1.extend_from_slice(exif);
        original.splice(2..2, app1);
        let normalized = normalize_avatar(&original).unwrap();
        assert!(!normalized
            .bytes
            .windows(exif.len())
            .any(|part| part == exif));
        assert!(!normalized
            .bytes
            .windows(comment.len())
            .any(|part| part == comment));
    }
}

#[cfg(test)]
mod service_tests {
    use super::*;
    use crate::{dto::AccountConfigDto, remote::NativeRemoteMail, service::IncomingCredentials};
    use vault::{AccountAuthMaterial, SecretStore};

    #[tokio::test]
    async fn photos_validate_account_and_preserve_previous_photo_without_remote_operations() {
        let store = Arc::new(
            storage::SqliteMailStore::open_in_memory(
                &storage::DatabaseKey::from_bytes([29; 32]),
                Default::default(),
            )
            .unwrap(),
        );
        let vault: Arc<dyn SecretStore> = Arc::new(vault::EncryptedFallbackStore::in_memory(
            Arc::new(vault::Argon2AesGcmCipher::from_key([27; 32])),
        ));
        let service = DesktopService::new(
            store,
            vault.clone(),
            Arc::new(NativeRemoteMail::new(vault)),
            None,
        );
        let config = AccountConfigDto {
            account_id: "a".into(),
            email_display: "reader@example.com".into(),
            imap_host: "imap.example.com".into(),
            imap_port: 993,
            auth_method: mail_engine::AuthMethod::AppPassword,
        };
        service
            .add_account(
                config,
                IncomingCredentials(AccountAuthMaterial::AppPassword {
                    secret: "synthetic-password".into(),
                }),
            )
            .await
            .unwrap();
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(64, 32)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        let original = bytes.into_inner();
        assert_eq!(service.get_account_avatar("a".into()).await.unwrap(), None);
        assert_eq!(
            service
                .set_account_avatar("unknown".into(), original.clone())
                .await,
            Err(IpcError::NotFound)
        );
        assert_eq!(
            service.get_account_avatar("unknown".into()).await,
            Err(IpcError::NotFound)
        );
        assert_eq!(
            service.remove_account_avatar("unknown".into()).await,
            Err(IpcError::NotFound)
        );
        let photo = service
            .set_account_avatar("a".into(), original)
            .await
            .unwrap();
        assert_eq!(
            service
                .set_account_avatar("a".into(), b"<svg/>".to_vec())
                .await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(
            service.get_account_avatar("a".into()).await.unwrap(),
            Some(photo)
        );
        assert!(service.store.pending_mutations().unwrap().is_empty());
        service.remove_account_avatar("a".into()).await.unwrap();
        assert_eq!(service.get_account_avatar("a".into()).await.unwrap(), None);
    }
}
