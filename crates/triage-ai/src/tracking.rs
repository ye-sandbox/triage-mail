//! Deterministic package and courier tracking code extractor.
//!
//! Supports:
//! - UPU S10 / Correios (13 characters, Modulo 11 check digit verified)
//! - UPS (1Z format, 18 characters)
//! - FedEx (12 or 15 digits in logistics context)
//! - DHL (10 digits in DHL context)

use regex::Regex;
use std::sync::LazyLock;

static UPU_S10_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z]{2}[0-9]{9}[A-Z]{2})\b").expect("valid upu s10 regex"));

static UPS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(1Z[0-9A-Z]{16})\b").expect("valid ups regex"));

static FEDEX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bfedex[^\d\n]{1,20}([0-9]{12}|[0-9]{15})\b").expect("valid fedex regex")
});

static DHL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bdhl[^\d\n]{1,20}([0-9]{10})\b").expect("valid dhl regex"));

/// Extracted tracking code details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingDetails {
    pub carrier: String,
    pub code: String,
}

/// Validates UPU S10 standard check digit (8 digits with weights [8, 6, 4, 2, 3, 5, 9, 7]).
pub fn is_valid_upu_s10(code: &str) -> bool {
    if code.len() != 13 {
        return false;
    }

    let chars: Vec<char> = code.chars().collect();
    if !chars[0].is_ascii_alphabetic()
        || !chars[1].is_ascii_alphabetic()
        || !chars[11].is_ascii_alphabetic()
        || !chars[12].is_ascii_alphabetic()
    {
        return false;
    }

    let digits: Vec<u32> = chars[2..10].iter().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 8 {
        return false;
    }

    let expected_check_digit = match chars[10].to_digit(10) {
        Some(d) => d,
        None => return false,
    };

    let weights = [8, 6, 4, 2, 3, 5, 9, 7];
    let sum: u32 = digits
        .iter()
        .zip(weights.iter())
        .map(|(&d, &w)| d * w)
        .sum();
    let rem = sum % 11;

    let computed_check_digit = match rem {
        0 => 5,
        1 => 0,
        _ => 11 - rem,
    };

    computed_check_digit == expected_check_digit
}

/// Extracts all valid courier and postal tracking codes from text.
pub fn extract_tracking_codes(text: &str) -> Vec<TrackingDetails> {
    let mut results = Vec::new();

    // 1. UPU S10 / Correios
    for cap in UPU_S10_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let code = m.as_str().to_uppercase();
            if is_valid_upu_s10(&code) {
                let carrier = if code.ends_with("BR") {
                    "Correios".to_string()
                } else {
                    "Postal (UPU)".to_string()
                };

                if !results.iter().any(|t: &TrackingDetails| t.code == code) {
                    results.push(TrackingDetails { carrier, code });
                }
            }
        }
    }

    // 2. UPS
    for cap in UPS_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let code = m.as_str().to_uppercase();
            if !results.iter().any(|t: &TrackingDetails| t.code == code) {
                results.push(TrackingDetails {
                    carrier: "UPS".to_string(),
                    code,
                });
            }
        }
    }

    // 3. FedEx
    for cap in FEDEX_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let code = m.as_str().to_string();
            if !results.iter().any(|t: &TrackingDetails| t.code == code) {
                results.push(TrackingDetails {
                    carrier: "FedEx".to_string(),
                    code,
                });
            }
        }
    }

    // 4. DHL
    for cap in DHL_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let code = m.as_str().to_string();
            if !results.iter().any(|t: &TrackingDetails| t.code == code) {
                results.push(TrackingDetails {
                    carrier: "DHL".to_string(),
                    code,
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_validates_correios_s10_code() {
        // Construct valid S10 code:
        // Prefix: NL, Digits: 12345678, Suffix: BR
        // Weights: [8, 6, 4, 2, 3, 5, 9, 7]
        // 1*8 + 2*6 + 3*4 + 4*2 + 5*3 + 6*5 + 7*9 + 8*7 = 8 + 12 + 12 + 8 + 15 + 30 + 63 + 56 = 204
        // 204 % 11 = 6. Check digit = 11 - 6 = 5.
        // Full code: NL123456785BR
        let email = "Seu pedido foi despachado! Código de rastreamento: NL123456785BR.";
        let codes = extract_tracking_codes(email);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].carrier, "Correios");
        assert_eq!(codes[0].code, "NL123456785BR");
    }

    #[test]
    fn rejects_corrupted_s10_code() {
        let email = "Rastreio: NL123456789BR"; // Wrong check digit 9 instead of 5
        let codes = extract_tracking_codes(email);
        assert!(codes.is_empty());
    }

    #[test]
    fn extracts_ups_tracking_code() {
        let email =
            "Your package has shipped via UPS tracking 1Z9999999999999999. Track delivery online.";
        let codes = extract_tracking_codes(email);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].carrier, "UPS");
        assert_eq!(codes[0].code, "1Z9999999999999999");
    }

    #[test]
    fn extracts_fedex_tracking_code_in_context() {
        let email = "FedEx tracking number: 123456789012 for your order.";
        let codes = extract_tracking_codes(email);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].carrier, "FedEx");
        assert_eq!(codes[0].code, "123456789012");
    }
}
