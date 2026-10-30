//! Deterministic FEBRABAN Boleto extractor and validator (47-digit bank & 48-digit utility boletos).
//!
//! Validates:
//! - Modulo 10 check digits on Linha Digitável fields
//! - Modulo 11 barcode DAC check digit
//! - Exact BRL value in cents (`amount_cents`)
//! - Expiration due date (`due_date_iso`) handling the post-2025 factor rollover.

use regex::Regex;
use std::sync::LazyLock;

static BOLETO_CANDIDATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches candidate sequences of numbers formatted or unformatted (47 or 48 digits)
    Regex::new(r"(?x)
        \b(
            # Bank boleto formatted: 34191.79001 01043.510047 91020.150008 5 99990000010000
            [0-9]{5}[\.\s]?[0-9]{5}[\s]?[0-9]{5}[\.\s]?[0-9]{6}[\s]?[0-9]{5}[\.\s]?[0-9]{6}[\s]?[0-9][\s]?[0-9]{14}
            |
            # Concessionária / Utility formatted: 84670000001-4 43590024020-9 02405020243-7 00010101202-4
            8[0-9]{10}[-\s]?[0-9][\s]?[0-9]{11}[-\s]?[0-9][\s]?[0-9]{11}[-\s]?[0-9][\s]?[0-9]{11}[-\s]?[0-9]
            |
            # Unformatted 47 or 48 contiguous digits
            [0-9]{47,48}
        )\b
    ").expect("valid boleto regex")
});

/// Extracted FEBRABAN Boleto metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoletoDetails {
    pub digitable_line: String,
    pub amount_cents: Option<u64>,
    pub due_date_iso: Option<String>,
}

/// Checks if a year is a leap year in the Gregorian calendar.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns days in a given month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Adds `days_to_add` to a Gregorian date `(year, month, day)` and returns formatted `YYYY-MM-DD`.
fn add_days(mut year: i32, mut month: u32, mut day: u32, mut days_to_add: u32) -> String {
    while days_to_add > 0 {
        let dim = days_in_month(year, month);
        let remaining_in_month = dim - day;
        if days_to_add <= remaining_in_month {
            day += days_to_add;
            break;
        } else {
            days_to_add -= remaining_in_month + 1;
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    format!("{year:04}-{month:02}-{day:02}")
}

/// Computes FEBRABAN Modulo 10 check digit for a string of digits.
pub fn modulo10(input: &str) -> u32 {
    let mut sum = 0;
    let mut weight = 2;
    for ch in input.chars().rev() {
        let digit = ch.to_digit(10).unwrap_or(0);
        let mut prod = digit * weight;
        if prod >= 10 {
            prod = (prod / 10) + (prod % 10);
        }
        sum += prod;
        weight = if weight == 2 { 1 } else { 2 };
    }
    let rem = sum % 10;
    if rem == 0 {
        0
    } else {
        10 - rem
    }
}

/// Computes FEBRABAN Modulo 11 check digit for a 43-digit barcode (weights 2..9).
pub fn modulo11_barcode(barcode_without_dac: &str) -> u32 {
    let mut sum = 0;
    let mut weight = 2;
    for ch in barcode_without_dac.chars().rev() {
        let digit = ch.to_digit(10).unwrap_or(0);
        sum += digit * weight;
        weight += 1;
        if weight > 9 {
            weight = 2;
        }
    }
    let rem = sum % 11;
    let dac = 11 - rem;
    if dac == 0 || dac == 10 || dac == 11 {
        1
    } else {
        dac
    }
}

/// Validates and parses a 47-digit bank boleto.
fn parse_bank_boleto(digits: &str) -> Option<BoletoDetails> {
    if digits.len() != 47 {
        return None;
    }

    // Field 1: 9 digits data + 1 check digit
    let f1_data = &digits[0..9];
    let f1_dv = digits[9..10].parse::<u32>().ok()?;
    if modulo10(f1_data) != f1_dv {
        return None;
    }

    // Field 2: 10 digits data + 1 check digit
    let f2_data = &digits[10..20];
    let f2_dv = digits[20..21].parse::<u32>().ok()?;
    if modulo10(f2_data) != f2_dv {
        return None;
    }

    // Field 3: 10 digits data + 1 check digit
    let f3_data = &digits[21..31];
    let f3_dv = digits[31..32].parse::<u32>().ok()?;
    if modulo10(f3_data) != f3_dv {
        return None;
    }

    // Field 4: Barcode general DAC (1 digit)
    let f4_dac = digits[32..33].parse::<u32>().ok()?;

    // Field 5: Factor (4 digits) + Value (10 digits)
    let factor_str = &digits[33..37];
    let value_str = &digits[37..47];

    // Reconstruct 43-digit barcode (excluding DAC at position 5)
    // Barcode: [0..4 Bank + Currency] + [DAC] + [Factor 4] + [Value 10] + [Campo Livre 25]
    let bank_curr = &digits[0..4];
    let campo_livre = format!("{}{}{}", &digits[4..9], f2_data, f3_data);
    let barcode_without_dac = format!("{bank_curr}{factor_str}{value_str}{campo_livre}");

    if modulo11_barcode(&barcode_without_dac) != f4_dac {
        return None;
    }

    // Parse amount
    let raw_amount = value_str.parse::<u64>().ok()?;
    let amount_cents = if raw_amount > 0 {
        Some(raw_amount)
    } else {
        None
    };

    // Calculate due date
    let factor = factor_str.parse::<u32>().ok()?;
    let due_date_iso = if factor >= 1000 {
        // INVARIANT: Post-rollover FEBRABAN rule: on 2025-02-22 factor rolled over from 9999 to 1000.
        // For current system date >= 2025, factor 1000 corresponds to 2025-02-22.
        let days_from_base = factor - 1000;
        Some(add_days(2025, 2, 22, days_from_base))
    } else {
        None
    };

    Some(BoletoDetails {
        digitable_line: digits.to_string(),
        amount_cents,
        due_date_iso,
    })
}

/// Validates and parses a 48-digit utility/concessionária boleto.
fn parse_utility_boleto(digits: &str) -> Option<BoletoDetails> {
    if digits.len() != 48 || !digits.starts_with('8') {
        return None;
    }

    let currency_code = digits.chars().nth(2)?.to_digit(10)?;
    let use_modulo10 = currency_code == 6 || currency_code == 7;

    // Check 4 blocks of 11 data digits + 1 check digit
    let mut barcode_data = String::with_capacity(44);
    for i in 0..4 {
        let start = i * 12;
        let data = &digits[start..start + 11];
        let expected_dv = digits[start + 11..start + 12].parse::<u32>().ok()?;

        let calculated_dv = if use_modulo10 {
            modulo10(data)
        } else {
            // Modulo 11 for utility
            let mut sum = 0;
            let mut weight = 2;
            for ch in data.chars().rev() {
                let d = ch.to_digit(10).unwrap_or(0);
                sum += d * weight;
                weight += 1;
                if weight > 9 {
                    weight = 2;
                }
            }
            let rem = sum % 11;
            if rem == 0 || rem == 1 {
                0
            } else if rem == 10 {
                1
            } else {
                11 - rem
            }
        };

        if calculated_dv != expected_dv {
            return None;
        }

        barcode_data.push_str(data);
    }

    // In 44-digit utility barcode, positions 4..15 contain the 11-digit value in cents if currency_code is 6 or 8
    let amount_cents = if currency_code == 6 || currency_code == 8 {
        let val_slice = &barcode_data[4..15];
        let raw_val = val_slice.parse::<u64>().ok()?;
        if raw_val > 0 {
            Some(raw_val)
        } else {
            None
        }
    } else {
        None
    };

    Some(BoletoDetails {
        digitable_line: digits.to_string(),
        amount_cents,
        due_date_iso: None,
    })
}

/// Extracts all valid FEBRABAN Boletos from text.
pub fn extract_boletos(text: &str) -> Vec<BoletoDetails> {
    let mut results = Vec::new();

    for cap in BOLETO_CANDIDATE_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let raw = m.as_str();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();

            if digits.len() == 47 {
                if let Some(boleto) = parse_bank_boleto(&digits) {
                    if !results
                        .iter()
                        .any(|b: &BoletoDetails| b.digitable_line == boleto.digitable_line)
                    {
                        results.push(boleto);
                    }
                }
            } else if digits.len() == 48 {
                if let Some(boleto) = parse_utility_boleto(&digits) {
                    if !results
                        .iter()
                        .any(|b: &BoletoDetails| b.digitable_line == boleto.digitable_line)
                    {
                        results.push(boleto);
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_bank_boleto_with_value_and_post_2025_due_date() {
        let f1_data = "341901043";
        let dv1 = modulo10(f1_data);
        let f2_data = "5100491020";
        let dv2 = modulo10(f2_data);
        let f3_data = "1500000000";
        let dv3 = modulo10(f3_data);

        let factor_str = "1000";
        let value_str = "0000015000";
        let campo_livre = format!("{}{}{}", &f1_data[4..9], f2_data, f3_data);
        let barcode_without_dac = format!("3419{factor_str}{value_str}{campo_livre}");
        let dac = modulo11_barcode(&barcode_without_dac);

        let digits =
            format!("{f1_data}{dv1}{f2_data}{dv2}{f3_data}{dv3}{dac}{factor_str}{value_str}");

        let email = format!("Segue seu boleto para pagamento: {digits} com vencimento.");
        let boletos = extract_boletos(&email);
        assert_eq!(boletos.len(), 1);
        let b = &boletos[0];
        assert_eq!(b.amount_cents, Some(15000));
        assert_eq!(b.due_date_iso, Some("2025-02-22".to_string()));
    }

    #[test]
    fn parses_formatted_utility_boleto_with_cents() {
        // 48 digits utility boleto starting with 8, currency 6 (modulo 10)
        // Block 1: 84670000001 -> DV
        let b1 = "84670000001";
        let dv1 = modulo10(b1);
        let b2 = "43590024020";
        let dv2 = modulo10(b2);
        let b3 = "02405020243";
        let dv3 = modulo10(b3);
        let b4 = "00010101202";
        let dv4 = modulo10(b4);

        let formatted = format!("{b1}-{dv1} {b2}-{dv2} {b3}-{dv3} {b4}-{dv4}");
        let email = format!("Conta de luz disponível para pagamento:\n{formatted}");
        let boletos = extract_boletos(&email);
        assert_eq!(boletos.len(), 1);
        assert_eq!(boletos[0].digitable_line.len(), 48);
    }

    #[test]
    fn rejects_corrupted_boleto_check_digits() {
        let invalid = "34190104395100491020815000000001110000000015000"; // DV1 tampered (9 instead of 5)
        let boletos = extract_boletos(invalid);
        assert!(boletos.is_empty());
    }
}
