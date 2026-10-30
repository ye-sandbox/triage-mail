---
id: "k-005"
title: "FEBRABAN Boleto Post-2025 Factor Rollover and Brazilian Financial Extractor Invariants"
date: 2026-10-30
tags:
  - "febraban"
  - "boleto"
  - "pix"
  - "nfe"
  - "triage"
  - "regex"
severity: "medium"
affected_components:
  - "crates/triage-ai"
symptoms:
  - "Incorrect boleto due date (year 2000 instead of 2025/2026)"
  - "Pix EMV Copia e Cola CRC validation failure on payloads containing spaces or plus signs"
  - "False positive OTP code matching common Portuguese words like 'para'"
---

# [k-005] FEBRABAN Boleto Post-2025 Factor Rollover and Brazilian Financial Extractor Invariants

## Problem / Background
Brazilian financial and logistics communications follow standardized deterministic formats that require exact mathematical validation:

1. **FEBRABAN Bank Boletos (47 digits *Linha Digitável*):**
   - Base factor `1000` originally anchored at `2000-07-03`.
   - On `2025-02-21`, the 4-digit factor reached `9999`.
   - On `2025-02-22`, the factor rolled over back to `1000`.
   - *Gotcha:* Naive date calculators anchored to `2000-07-03` will compute a 2000-era date for modern post-2025 boletos. For systems operating in 2025+, factors `>= 1000` calculate from `2025-02-22`.
   - Value calculation: Field 5 last 10 digits represent exact cents (`amount_cents`).

2. **FEBRABAN Utility / Concessionária Boletos (48 digits, starting with '8'):**
   - 4 blocks of 11 data digits + 1 check digit each.
   - The 3rd digit dictates check digit algorithm: `6` or `7` uses Modulo 10; `8` or `9` uses Modulo 11.
   - For currencies `6` and `8`, positions 4..15 of the 44-digit reconstructed barcode contain the value in cents.

3. **Pix EMV "Copia e Cola" Payloads:**
   - Starts with `000201`, contains `BR.GOV.BCB.PIX` in tag 26, and ends with `6304` + 4-hex-char CRC-16.
   - Checksum is CRC-16/CCITT-FALSE (polynomial `0x1021`, initial `0xFFFF`, no reflection, final XOR `0x0000`).
   - *Gotcha:* Merchant names (tag 59) and city names (tag 60) can legitimately contain spaces, and phone keys contain `+`. Strip only newline characters (`\r\n`), never spaces inside valid EMV payload tags, otherwise CRC validation fails.

4. **2FA / OTP False-Positive Guards:**
   - OTP codes are either purely numeric (4 to 8 digits) or alphanumeric tokens.
   - Lowercase ordinary alphabetic words (e.g. Portuguese `para`, `login`, `senha`, `conta`) must be strictly rejected as false positives.

---

## Approved Solution & Verification
All extractors in `crates/triage-ai` validate mathematically before surfacing badges:
- `is_valid_nfe_key`: SEFAZ Modulo 11 check digit on 44th digit with weights 2..9 right-to-left.
- `is_valid_upu_s10`: UPU S10 Correios tracking check digit with weights `[8, 6, 4, 2, 3, 5, 9, 7]`.
- `compute_crc16_ccitt_false`: Evaluates EMV string up to `6304`.
- `extract_boletos`: Validates Modulo 10 on fields 1..3 and Modulo 11 on the barcode DAC.

```bash
cargo test -p triage-ai
```
