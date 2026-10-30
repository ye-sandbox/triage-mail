// INVARIANT: Provider quirk normalization isolates IMAP dialect differences (Gmail labels,
// Outlook CONDSTORE absence, Proton Bridge loopback) behind an agnostic domain layer,
// preventing duplicate ingestion and protocol crashes.

use serde::{Deserialize, Serialize};

/// High-level provider dialect recognized by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Gmail,
    Outlook,
    ProtonBridge,
    GenericRfc,
}

impl ProviderKind {
    /// Detects the provider dialect based on host and server capabilities.
    pub fn detect(host: &str, capabilities: Option<&ServerCapabilities>) -> Self {
        let host_lower = host.trim().to_ascii_lowercase();

        if let Some(caps) = capabilities {
            if caps.x_gm_ext1 {
                return Self::Gmail;
            }
        }

        if host_lower.contains("gmail.com") || host_lower.contains("googlemail.com") {
            Self::Gmail
        } else if host_lower.contains("outlook.")
            || host_lower.contains("office365.com")
            || host_lower.contains("hotmail.com")
            || host_lower.contains("live.com")
        {
            Self::Outlook
        } else if host_lower == "127.0.0.1" || host_lower == "localhost" || host_lower == "::1" {
            Self::ProtonBridge
        } else {
            Self::GenericRfc
        }
    }
}

/// Normalized server capabilities parsed from IMAP `CAPABILITY` response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub condstore: bool,
    pub special_use: bool,
    pub move_cmd: bool,
    pub idle: bool,
    pub x_gm_ext1: bool,
    pub xoauth2: bool,
    pub oauthbearer: bool,
    pub raw_capabilities: Vec<String>,
}

impl ServerCapabilities {
    /// Parses capabilities from a list of tokens returned in an IMAP response.
    pub fn from_tokens<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut caps = Self::default();

        for token in tokens {
            let token_str = token.as_ref().trim();
            if token_str.is_empty() {
                continue;
            }
            let upper = token_str.to_ascii_uppercase();
            caps.raw_capabilities.push(upper.clone());

            match upper.as_str() {
                "CONDSTORE" => caps.condstore = true,
                "SPECIAL-USE" => caps.special_use = true,
                "MOVE" => caps.move_cmd = true,
                "IDLE" => caps.idle = true,
                "X-GM-EXT-1" => caps.x_gm_ext1 = true,
                "AUTH=XOAUTH2" => caps.xoauth2 = true,
                "AUTH=OAUTHBEARER" => caps.oauthbearer = true,
                _ => {}
            }
        }

        caps
    }

    /// Parses capabilities from a raw IMAP response line (e.g. `* CAPABILITY IMAP4rev1 CONDSTORE ...`).
    pub fn from_capability_line(line: &str) -> Self {
        let trimmed = line.trim();
        let payload = if let Some(stripped) = trimmed.strip_prefix("* CAPABILITY ") {
            stripped
        } else if let Some(stripped) = trimmed.strip_prefix("CAPABILITY ") {
            stripped
        } else {
            trimmed
        };

        Self::from_tokens(payload.split_whitespace())
    }

    /// Returns whether the server supports CONDSTORE (`HIGHESTMODSEQ` / `CHANGEDSINCE`).
    pub fn supports_condstore(&self) -> bool {
        self.condstore
    }
}

/// Standardized special-use mailbox roles (RFC 6154) with localized name fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialFolderRole {
    Inbox,
    Archive,
    Trash,
    Junk,
    Sent,
    Drafts,
    AllMail,
}

/// Classifies a folder name and RFC 6154 attributes into a standardized special folder role.
pub fn classify_folder<S1: AsRef<str>, S2: AsRef<str>>(
    folder_name: S1,
    attributes: &[S2],
) -> Option<SpecialFolderRole> {
    // 1. Check strict RFC 6154 attributes first
    for attr in attributes {
        let attr_ref = attr.as_ref().trim().to_ascii_lowercase();
        match attr_ref.as_str() {
            "\\inbox" => return Some(SpecialFolderRole::Inbox),
            "\\archive" => return Some(SpecialFolderRole::Archive),
            "\\trash" => return Some(SpecialFolderRole::Trash),
            "\\junk" => return Some(SpecialFolderRole::Junk),
            "\\sent" => return Some(SpecialFolderRole::Sent),
            "\\drafts" => return Some(SpecialFolderRole::Drafts),
            "\\all" => return Some(SpecialFolderRole::AllMail),
            _ => {}
        }
    }

    // 2. Multilingual heuristic fallback (Portuguese, English, Spanish)
    let name = folder_name.as_ref().trim().to_ascii_lowercase();
    match name.as_str() {
        "inbox" | "caixa de entrada" | "entrada" | "bandeja de entrada" => {
            Some(SpecialFolderRole::Inbox)
        }
        "archive" | "arquivo" | "arquivados" => Some(SpecialFolderRole::Archive),
        "[gmail]/all mail"
        | "[gmail]/todos os e-mails"
        | "[gmail]/todos"
        | "all mail"
        | "todos os e-mails" => Some(SpecialFolderRole::AllMail),
        "trash" | "lixeira" | "itens excluídos" | "deleted items" | "papelera"
        | "[gmail]/trash" | "[gmail]/lixeira" | "[gmail]/papelera" => {
            Some(SpecialFolderRole::Trash)
        }
        "junk" | "spam" | "lixo eletrônico" | "correo no deseado" | "[gmail]/spam" => {
            Some(SpecialFolderRole::Junk)
        }
        "sent" | "sent messages" | "enviados" | "itens enviados" | "elementos enviados"
        | "[gmail]/sent mail" | "[gmail]/enviados" => Some(SpecialFolderRole::Sent),
        "drafts" | "rascunhos" | "borradores" | "[gmail]/drafts" | "[gmail]/rascunhos" => {
            Some(SpecialFolderRole::Drafts)
        }
        _ => None,
    }
}

/// Identifies if a folder is Gmail's virtual "All Mail" container.
pub fn is_gmail_virtual_all_mail(folder_name: &str) -> bool {
    let lower = folder_name.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "[gmail]/all mail" | "[gmail]/todos os e-mails" | "[gmail]/todos"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_provider_from_host_and_capabilities() {
        assert_eq!(
            ProviderKind::detect("imap.gmail.com", None),
            ProviderKind::Gmail
        );
        assert_eq!(
            ProviderKind::detect("outlook.office365.com", None),
            ProviderKind::Outlook
        );
        assert_eq!(
            ProviderKind::detect("127.0.0.1", None),
            ProviderKind::ProtonBridge
        );
        assert_eq!(
            ProviderKind::detect("mail.example.org", None),
            ProviderKind::GenericRfc
        );

        let caps = ServerCapabilities::from_tokens(["X-GM-EXT-1", "CONDSTORE"]);
        assert_eq!(
            ProviderKind::detect("custom-proxy.internal", Some(&caps)),
            ProviderKind::Gmail
        );
    }

    #[test]
    fn parses_capabilities_correctly() {
        let line = "* CAPABILITY IMAP4rev1 CONDSTORE SPECIAL-USE IDLE MOVE AUTH=XOAUTH2";
        let caps = ServerCapabilities::from_capability_line(line);
        assert!(caps.supports_condstore());
        assert!(caps.special_use);
        assert!(caps.idle);
        assert!(caps.move_cmd);
        assert!(caps.xoauth2);
        assert!(!caps.x_gm_ext1);
    }

    #[test]
    fn classifies_special_use_folders_and_localizations() {
        // RFC 6154 attributes
        assert_eq!(
            classify_folder("CustomArchive", &["\\Archive"]),
            Some(SpecialFolderRole::Archive)
        );
        assert_eq!(
            classify_folder("Bin", &["\\Trash"]),
            Some(SpecialFolderRole::Trash)
        );

        // Fallback names (Portuguese & English)
        assert_eq!(
            classify_folder("Caixa de Entrada", &[] as &[&str]),
            Some(SpecialFolderRole::Inbox)
        );
        assert_eq!(
            classify_folder("Lixeira", &[] as &[&str]),
            Some(SpecialFolderRole::Trash)
        );
        assert_eq!(
            classify_folder("Itens Enviados", &[] as &[&str]),
            Some(SpecialFolderRole::Sent)
        );
        assert_eq!(
            classify_folder("[Gmail]/All Mail", &[] as &[&str]),
            Some(SpecialFolderRole::AllMail)
        );

        assert!(is_gmail_virtual_all_mail("[Gmail]/All Mail"));
        assert!(is_gmail_virtual_all_mail("[Gmail]/Todos os e-mails"));
        assert!(!is_gmail_virtual_all_mail("INBOX"));
    }
}
