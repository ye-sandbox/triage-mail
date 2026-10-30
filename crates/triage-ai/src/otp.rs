//! Deterministic 2FA/OTP verification code extractor with false-positive guards.

use regex::Regex;
use std::sync::LazyLock;

static OTP_CONTEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:c[oó]digo(?:\s+de)?(?:\s+(?:verifica[cç][aã]o|confirma[cç][aã]o|seguran[cç]a|acesso))?|verification\s+code|security\s+code|confirmation\s+code|one-time\s+password|2fa|otp|passcode|token|senha\s+tempor[aá]ria|c[oó]digo\s+[eé]\b)[:\s]*([a-zA-Z0-9]{4,8})\b")
        .expect("valid otp context regex")
});

static EXPLICIT_CODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:seu\s+c[oó]digo(?:\s+[eé])?|your\s+code\s+is|use\s+the\s+code)\s*[:\s]*([a-zA-Z0-9]{4,8})\b")
        .expect("valid explicit code regex")
});

static STANDALONE_DIGITS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([0-9]{4,8})\b").expect("valid standalone digits regex"));

/// Evaluates if candidate string is a false positive (years, postal codes, CSS colors, currencies, common words).
fn is_false_positive(candidate: &str, surrounding_text: &str) -> bool {
    // 0. Reject if candidate is entirely alphabetic and all lowercase (ordinary word, e.g. "para", "login")
    if candidate.chars().all(|c| c.is_ascii_alphabetic())
        && candidate.chars().all(|c| c.is_ascii_lowercase())
    {
        return true;
    }

    // 0b. Reject common stop words case-insensitively
    let candidate_lower = candidate.to_lowercase();
    let stop_words = [
        "para", "from", "with", "your", "this", "that", "user", "login", "code", "senha", "conta",
        "access", "email", "mail", "pass", "time", "date", "link", "help", "info", "site", "safe",
        "only", "onde", "como", "mais", "esse", "esta", "pelo", "pela",
    ];
    if stop_words.contains(&candidate_lower.as_str()) {
        return true;
    }

    // 1. Check if candidate appears as a CSS hex color in surrounding text
    let hex_needle = format!("#{candidate}");
    if surrounding_text.contains(&hex_needle) {
        return true;
    }

    // 2. Reject years (1900..2099) unless surrounded by strong explicit code markers
    if candidate.len() == 4
        && (candidate.starts_with("19") || candidate.starts_with("20"))
        && candidate.chars().all(|c| c.is_ascii_digit())
    {
        // If surrounding text contains date words near the candidate, it's a year
        let lower = surrounding_text.to_lowercase();
        if lower.contains("copyright")
            || lower.contains("ano")
            || lower.contains("year")
            || lower.contains("©")
            || lower.contains(&format!("/{candidate}"))
            || lower.contains(&format!("-{candidate}"))
        {
            return true;
        }
    }

    // 3. Reject Brazilian CEP (8 digits with or without hyphen if matched)
    if candidate.len() == 8 && candidate.chars().all(|c| c.is_ascii_digit()) {
        let prefix = &candidate[0..5];
        let suffix = &candidate[5..8];
        let hyphenated = format!("{prefix}-{suffix}");
        if surrounding_text.contains(&hyphenated) {
            return true;
        }
    }

    // 4. Reject if candidate is part of a monetary amount (e.g. R$ 1234 or $1234)
    let cur_pt = format!("R$ {candidate}");
    let cur_pt2 = format!("R${candidate}");
    let cur_en = format!("${candidate}");
    if surrounding_text.contains(&cur_pt)
        || surrounding_text.contains(&cur_pt2)
        || surrounding_text.contains(&cur_en)
    {
        return true;
    }

    // 5. Reject if preceded by telephone indicator
    let phone_plus = format!("+{candidate}");
    if surrounding_text.contains(&phone_plus) {
        return true;
    }

    false
}

/// Extracts a 2FA/OTP verification code from subject or message body.
///
/// INVARIANT: Sub-millisecond evaluation with strict false-positive filtering.
pub fn extract_otp(subject: &str, body: &str) -> Option<String> {
    let combined = format!("{subject}\n{body}");

    // Priority 1: Check explicit phrase match ("seu código é 123456", "your code is 49210")
    for cap in EXPLICIT_CODE_RE.captures_iter(&combined) {
        if let Some(m) = cap.get(1) {
            let candidate = m.as_str().trim();
            if !is_false_positive(candidate, &combined) {
                return Some(candidate.to_string());
            }
        }
    }

    // Priority 2: Check context match ("código de verificação: 849201")
    for cap in OTP_CONTEXT_RE.captures_iter(&combined) {
        if let Some(m) = cap.get(1) {
            let candidate = m.as_str().trim();
            if !is_false_positive(candidate, &combined) {
                return Some(candidate.to_string());
            }
        }
    }

    // Priority 3: If subject itself strongly indicates 2FA ("Your verification code", "Código de acesso")
    let subject_lower = subject.to_lowercase();
    let is_subject_otp = subject_lower.contains("código")
        || subject_lower.contains("codigo")
        || subject_lower.contains("verification code")
        || subject_lower.contains("security code")
        || subject_lower.contains("2fa")
        || subject_lower.contains("otp");

    if is_subject_otp {
        // Look for 4 to 8 digit code in subject first
        for cap in STANDALONE_DIGITS_RE.captures_iter(subject) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str().trim();
                if !is_false_positive(candidate, subject) {
                    return Some(candidate.to_string());
                }
            }
        }

        // Look for prominent code in first 500 chars of body
        let body_prefix: String = body.chars().take(500).collect();
        for cap in STANDALONE_DIGITS_RE.captures_iter(&body_prefix) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str().trim();
                if !is_false_positive(candidate, &body_prefix) {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_portuguese_verification_code() {
        let subject = "Seu código de verificação de conta";
        let body = "Olá! Use o código de verificação: 481920 para acessar seu aplicativo.";
        assert_eq!(extract_otp(subject, body), Some("481920".to_string()));
    }

    #[test]
    fn extracts_english_explicit_otp_code() {
        let subject = "Security alert";
        let body = "Your code is 938104. Do not share this code with anyone.";
        assert_eq!(extract_otp(subject, body), Some("938104".to_string()));
    }

    #[test]
    fn extracts_alphanumeric_code() {
        let subject = "Two-factor authentication token";
        let body = "Confirmation code: A7X9K2. Valid for 10 minutes.";
        assert_eq!(extract_otp(subject, body), Some("A7X9K2".to_string()));
    }

    #[test]
    fn ignores_years_and_copyright_dates() {
        let subject = "Boletim mensal da empresa";
        let body = "Todos os direitos reservados. Copyright © 2026 Empresa Exemplo.";
        assert_eq!(extract_otp(subject, body), None);
    }

    #[test]
    fn ignores_css_hex_colors() {
        let subject = "Newsletter semanal";
        let body = "Estilo de fundo: #FFFFFF e texto #333333.";
        assert_eq!(extract_otp(subject, body), None);
    }

    #[test]
    fn ignores_brazilian_cep_postal_code() {
        let subject = "Confirmação de entrega";
        let body = "Endereço registrado: Av. Paulista, 1000 - CEP 01310-100.";
        assert_eq!(extract_otp(subject, body), None);
    }

    #[test]
    fn ignores_currency_amounts() {
        let subject = "Resumo financeiro";
        let body = "Seu saldo atual é de R$ 5400,00 na conta corrente.";
        assert_eq!(extract_otp(subject, body), None);
    }
}
