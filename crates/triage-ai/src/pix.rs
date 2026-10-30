//! Deterministic Pix EMV Copia-e-Cola payload and Pix key extractor.
//!
//! Validates:
//! - Pix EMV `000201` payload structure with `BR.GOV.BCB.PIX` GUI
//! - CRC-16/CCITT-FALSE (`0x1021`, initial `0xFFFF`) checksum verification
//! - Pix keys (CPF/CNPJ with Modulo 11 check digits, EVP/UUID, email, phone) in Pix context.

use regex::Regex;
use std::sync::LazyLock;

static PIX_EMV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(000201[\x20-\x7E]{20,}?6304[0-9a-fA-F]{4})").expect("valid pix emv regex")
});

static PIX_CONTEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:chave\s+pix|pagamento\s+via\s+pix|pix\s*:\s*|copia\s+e\s+cola\s+pix|transfira\s+por\s+pix)\b")
        .expect("valid pix context regex")
});

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\b")
        .expect("valid uuid regex")
});

static CPF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([0-9]{3}\.[0-9]{3}\.[0-9]{3}-[0-9]{2}|[0-9]{11})\b").expect("valid cpf regex")
});

static CNPJ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([0-9]{2}\.[0-9]{3}\.[0-9]{3}/[0-9]{4}-[0-9]{2}|[0-9]{14})\b")
        .expect("valid cnpj regex")
});

static PHONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\+55\s?[0-9]{2}\s?9?[0-9]{4}-?[0-9]{4})\b").expect("valid phone regex")
});

/// Computes CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF, no reflection, final XOR 0x0000).
pub fn compute_crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Validates Brazilian CPF checksum (Modulo 11).
fn is_valid_cpf(cpf: &str) -> bool {
    let digits: Vec<u32> = cpf.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 11 {
        return false;
    }
    // Reject repeated digits like 111.111.111-11
    if digits.iter().all(|&d| d == digits[0]) {
        return false;
    }

    // Digit 1
    let sum1: u32 = digits[0..9]
        .iter()
        .enumerate()
        .map(|(idx, &d)| d * (10 - idx as u32))
        .sum();
    let rem1 = sum1 % 11;
    let expected_d1 = if rem1 < 2 { 0 } else { 11 - rem1 };
    if digits[9] != expected_d1 {
        return false;
    }

    // Digit 2
    let sum2: u32 = digits[0..10]
        .iter()
        .enumerate()
        .map(|(idx, &d)| d * (11 - idx as u32))
        .sum();
    let rem2 = sum2 % 11;
    let expected_d2 = if rem2 < 2 { 0 } else { 11 - rem2 };
    digits[10] == expected_d2
}

/// Validates Brazilian CNPJ checksum (Modulo 11).
fn is_valid_cnpj(cnpj: &str) -> bool {
    let digits: Vec<u32> = cnpj.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 14 {
        return false;
    }
    if digits.iter().all(|&d| d == digits[0]) {
        return false;
    }

    let weights1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum1: u32 = digits[0..12]
        .iter()
        .zip(weights1.iter())
        .map(|(&d, &w)| d * w)
        .sum();
    let rem1 = sum1 % 11;
    let expected_d1 = if rem1 < 2 { 0 } else { 11 - rem1 };
    if digits[12] != expected_d1 {
        return false;
    }

    let weights2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum2: u32 = digits[0..13]
        .iter()
        .zip(weights2.iter())
        .map(|(&d, &w)| d * w)
        .sum();
    let rem2 = sum2 % 11;
    let expected_d2 = if rem2 < 2 { 0 } else { 11 - rem2 };
    digits[13] == expected_d2
}

/// Extracts validated Pix EMV payload or contextual Pix keys.
pub fn extract_pix_payload(text: &str) -> Option<String> {
    // 1. Try extracting standard EMV Copia e Cola payload
    for cap in PIX_EMV_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let candidate = m.as_str().replace(['\r', '\n'], "");
            if candidate.contains("BR.GOV.BCB.PIX") {
                // Must end with 6304XXXX
                if let Some(pos) = candidate.rfind("6304") {
                    if pos + 8 == candidate.len() {
                        let payload_for_crc = &candidate[..pos + 4];
                        let expected_crc_str = &candidate[pos + 4..];
                        let calculated_crc = compute_crc16_ccitt_false(payload_for_crc.as_bytes());
                        let calculated_crc_str = format!("{calculated_crc:04X}");

                        if calculated_crc_str.eq_ignore_ascii_case(expected_crc_str) {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    // 2. Try contextual Pix Key extraction if explicit Pix phrase is found
    if PIX_CONTEXT_RE.is_match(text) {
        // Look for EVP UUID
        if let Some(cap) = UUID_RE.captures(text) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().to_string());
            }
        }

        // Look for valid CNPJ
        for cap in CNPJ_RE.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str();
                if is_valid_cnpj(candidate) {
                    return Some(candidate.to_string());
                }
            }
        }

        // Look for valid CPF
        for cap in CPF_RE.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str();
                if is_valid_cpf(candidate) {
                    return Some(candidate.to_string());
                }
            }
        }

        // Look for Phone
        if let Some(cap) = PHONE_RE.captures(text) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_validates_valid_pix_emv_payload() {
        let base_payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913Exemplo Nome6008SAO PAULO62070503***6304";
        let crc = compute_crc16_ccitt_false(base_payload.as_bytes());
        let full_payload = format!("{base_payload}{crc:04X}");

        let email = format!("Pague através do Pix Copia e Cola:\n{full_payload}\nObrigado!");
        assert_eq!(extract_pix_payload(&email), Some(full_payload));
    }

    #[test]
    fn rejects_corrupted_pix_emv_crc() {
        let base_payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913Exemplo Nome6008SAO PAULO62070503***6304FFFF";
        let email = format!("Pix: {base_payload}");
        assert_eq!(extract_pix_payload(&email), None);
    }

    #[test]
    fn extracts_pix_evp_random_key_in_context() {
        let email =
            "Chave Pix para depósito: 123e4567-e89b-12d3-a456-426614174000 no Banco Central.";
        assert_eq!(
            extract_pix_payload(email),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
    }

    #[test]
    fn extracts_valid_cpf_pix_key_and_ignores_invalid_cpf() {
        // Valid CPF: 111.444.777-35 (modulo 11 matches)
        let email = "Pagamento via Pix. Chave Pix: 111.444.777-35.";
        assert_eq!(
            extract_pix_payload(email),
            Some("111.444.777-35".to_string())
        );

        let invalid_cpf_email = "Chave Pix: 123.456.789-00"; // Invalid checksum
        assert_eq!(extract_pix_payload(invalid_cpf_email), None);
    }
}
