//! Deterministic Brazilian NF-e / NFC-e 44-digit access key extractor and validator.
//!
//! Validates:
//! - 44 numerical digits (formatted in 4-digit blocks or unformatted)
//! - State UF code (IBGE 11..53)
//! - Emission month (01..12)
//! - Fiscal model (55 = NF-e, 65 = NFC-e, 57 = CT-e)
//! - SEFAZ Modulo 11 check digit (weights 2..9 from right to left).

use regex::Regex;
use std::sync::LazyLock;

static NFE_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4}\s?[0-9]{4})\b")
        .expect("valid nfe regex")
});

/// Validates SEFAZ Modulo 11 check digit for a 44-digit NF-e key.
pub fn is_valid_nfe_key(raw_key: &str) -> bool {
    let digits: Vec<u32> = raw_key.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 44 {
        return false;
    }

    // 1. Validate IBGE UF code (digits 0..2)
    let uf = digits[0] * 10 + digits[1];
    let valid_ufs = [
        11, 12, 13, 14, 15, 16, 17, // Norte
        21, 22, 23, 24, 25, 26, 27, 28, 29, // Nordeste
        31, 32, 33, 35, // Sudeste
        41, 42, 43, // Sul
        50, 51, 52, 53, // Centro-Oeste
    ];
    if !valid_ufs.contains(&uf) {
        return false;
    }

    // 2. Validate Month (digits 4..6)
    let month = digits[4] * 10 + digits[5];
    if !(1..=12).contains(&month) {
        return false;
    }

    // 3. Validate Fiscal Model (digits 20..22)
    let model = digits[20] * 10 + digits[21];
    if model != 55 && model != 65 && model != 57 {
        return false;
    }

    // 4. Compute Modulo 11 check digit on first 43 digits
    let mut sum = 0;
    let mut weight = 2;
    for &d in digits[0..43].iter().rev() {
        sum += d * weight;
        weight += 1;
        if weight > 9 {
            weight = 2;
        }
    }

    let rem = sum % 11;
    let expected_dv = if rem == 0 || rem == 1 { 0 } else { 11 - rem };

    digits[43] == expected_dv
}

/// Extracts all valid NF-e access keys from text.
pub fn extract_nfe_keys(text: &str) -> Vec<String> {
    let mut results = Vec::new();

    for cap in NFE_KEY_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let candidate: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            if is_valid_nfe_key(&candidate) && !results.contains(&candidate) {
                results.push(candidate);
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_extracts_real_nfe_key() {
        // Construct valid 44-digit NF-e key
        // UF: 35 (SP), AAMM: 2610 (Oct 2026), CNPJ: 00000000000191, Mod: 55, Serie: 001, nNF: 000000100, tpEmis: 1, cNF: 12345678
        // First 43 digits:
        let prefix = "3526100000000000019155001000000100112345678";
        let mut sum = 0;
        let mut weight = 2;
        for c in prefix.chars().rev() {
            let d = c.to_digit(10).unwrap();
            sum += d * weight;
            weight = if weight == 9 { 2 } else { weight + 1 };
        }
        let rem = sum % 11;
        let dv = if rem == 0 || rem == 1 { 0 } else { 11 - rem };
        let full_key = format!("{prefix}{dv}");

        let email = format!(
            "Sua Nota Fiscal Eletrônica foi emitida. Chave de acesso:\n{}\nConsulte no portal da SEFAZ.",
            full_key
        );

        let keys = extract_nfe_keys(&email);
        assert_eq!(keys, vec![full_key]);
    }

    #[test]
    fn rejects_invalid_nfe_check_digit() {
        // Valid DV for this prefix is 9; using 8 makes it definitely invalid
        let invalid_key = "35261000000000000191550010000001001123456788";
        let keys = extract_nfe_keys(invalid_key);
        assert!(keys.is_empty());
    }

    #[test]
    fn rejects_invalid_uf() {
        let bad_uf_key = "99261000000000000191550010000001001123456780";
        let keys = extract_nfe_keys(bad_uf_key);
        assert!(keys.is_empty());
    }
}
