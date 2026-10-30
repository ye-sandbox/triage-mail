#![forbid(unsafe_code)]

//! Comprehensive end-to-end integration tests for Tier 1 Deterministic Triage pipeline.
//!
//! Uses synthetic RFC 2606 email fixtures (`@example.com`, `@example.org`) with zero real secrets.

use triage_ai::{
    compute_crc16_ccitt_false, modulo10, modulo11_barcode, DefaultDeterministicTriage,
    DeterministicTriage, TriageBadge, TriageCategory,
};

#[test]
fn test_integration_2fa_verification_code_extraction_and_security_category() {
    let triage = DefaultDeterministicTriage::new();
    let subject = "Seu código de verificação para acesso seguro";
    let sender = "no-reply@auth.example.com";
    let body = r#"
        Olá usuário,
        
        Você solicitou acesso à sua conta.
        Seu código de verificação é: 749201.
        
        Este código expira em 10 minutos. Se você não solicitou, altere sua senha.
        
        © 2026 Example Corp. Todos os direitos reservados.
        Estilo CSS: #FFFFFF / #000000.
        Central de ajuda: +55 11 4004-0000.
    "#;

    let result = triage.analyze(subject, sender, body, false);

    assert_eq!(result.category, TriageCategory::Security);
    assert_eq!(
        result.badges,
        vec![TriageBadge::OtpCode {
            code: "749201".to_string()
        }]
    );
}

#[test]
fn test_integration_febraban_bank_boleto_extraction_and_financial_category() {
    let triage = DefaultDeterministicTriage::new();

    // Construct valid 47-digit bank boleto (Itaú 341)
    // Field 1: 341901043 (DV = 8)
    // Field 2: 5100491020 (DV = 8)
    // Field 3: 1500000000 (DV = 1)
    // Factor: 1365 (Post-2025 rollover: 2025-02-22 + 365 days = 2026-02-22)
    // Amount: R$ 250,50 (0000025050)
    let f1_data = "341901043";
    let dv1 = modulo10(f1_data);
    let f2_data = "5100491020";
    let dv2 = modulo10(f2_data);
    let f3_data = "1500000000";
    let dv3 = modulo10(f3_data);

    let factor_str = "1365";
    let value_str = "0000025050";
    let campo_livre = format!("{}{}{}", &f1_data[4..9], f2_data, f3_data);
    let barcode_no_dac = format!("3419{factor_str}{value_str}{campo_livre}");
    let dac = modulo11_barcode(&barcode_no_dac);

    let linha_digitavel =
        format!("{f1_data}{dv1}{f2_data}{dv2}{f3_data}{dv3}{dac}{factor_str}{value_str}");

    let subject = "Fatura mensal de serviços - Vencimento próximo";
    let sender = "cobranca@financeiro.example.org";
    let body = format!(
        "Prezado cliente,\n\nSegue o boleto para quitação da sua mensalidade:\n\n{linha_digitavel}\n\nValor: R$ 250,50."
    );

    let result = triage.analyze(subject, sender, &body, false);

    assert_eq!(result.category, TriageCategory::Financial);
    assert_eq!(
        result.badges,
        vec![TriageBadge::FebrabanBoleto {
            digitable_line: linha_digitavel,
            amount_cents: Some(25050),
            due_date_iso: Some("2026-02-22".to_string()),
        }]
    );
}

#[test]
fn test_integration_pix_emv_copia_e_cola_extraction() {
    let triage = DefaultDeterministicTriage::new();

    let base_payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913Exemplo Pagto6008SAO PAULO62070503***6304";
    let crc = compute_crc16_ccitt_false(base_payload.as_bytes());
    let full_pix = format!("{base_payload}{crc:04X}");

    let subject = "Cobrança via Pix - Pedido #84920";
    let sender = "pagamentos@loja.example.com";
    let body = format!(
        "Olá! Para pagar instantaneamente, utilize o Pix Copia e Cola abaixo:\n\n{full_pix}\n\nObrigado!"
    );

    let result = triage.analyze(subject, sender, &body, false);

    assert_eq!(result.category, TriageCategory::Financial);
    assert_eq!(
        result.badges,
        vec![TriageBadge::PixPayload { payload: full_pix }]
    );
}

#[test]
fn test_integration_nfe_access_key_extraction() {
    let triage = DefaultDeterministicTriage::new();

    // Valid 44-digit NF-e key (UF 35 SP, Oct 2026, mod 55)
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
    let nfe_key = format!("{prefix}{dv}");

    let subject = "Emissão de Nota Fiscal Eletrônica NF-e #100";
    let sender = "nfe@faturamento.example.com";
    let body = format!(
        "Sua nota fiscal eletrônica foi autorizada pela SEFAZ.\nChave de Acesso: {nfe_key}\nConsulte os detalhes no portal nacional."
    );

    let result = triage.analyze(subject, sender, &body, false);

    assert_eq!(result.category, TriageCategory::Financial);
    assert_eq!(
        result.badges,
        vec![TriageBadge::NfeInvoice {
            access_key: nfe_key
        }]
    );
}

#[test]
fn test_integration_package_shipping_tracking_code_and_shopping_category() {
    let triage = DefaultDeterministicTriage::new();

    // Correios S10: NL123456785BR
    let subject = "Seu pedido foi enviado! Acompanhe a entrega";
    let sender = "envios@ecommerce.example.org";
    let body = "Boas notícias! Seu pedido #19283 foi despachado via Correios. Código de rastreamento: NL123456785BR.";

    let result = triage.analyze(subject, sender, body, false);

    assert_eq!(result.category, TriageCategory::Shopping);
    assert_eq!(
        result.badges,
        vec![TriageBadge::TrackingCode {
            carrier: "Correios".to_string(),
            code: "NL123456785BR".to_string(),
        }]
    );
}

#[test]
fn test_integration_marketing_newsletter_with_rfc8058_one_click() {
    let triage = DefaultDeterministicTriage::new();

    let subject = "As novidades da tecnologia neste outono";
    let sender = "newsletter@techdigest.example.com";
    let body =
        "Confira nossos artigos semanais sobre desenvolvimento de software local-first e Rust.";

    let result = triage.analyze(subject, sender, body, true);

    assert_eq!(result.category, TriageCategory::Newsletter);
    assert_eq!(
        result.badges,
        vec![TriageBadge::UnsubscribeAvailable {
            supports_one_click: true
        }]
    );
}

#[test]
fn test_integration_personal_correspondence_without_badges() {
    let triage = DefaultDeterministicTriage::new();

    let subject = "Café na sexta-feira?";
    let sender = "maria@example.org";
    let body = "Oi! Como estão as coisas por aí? Vamos marcar aquele café na sexta à tarde?";

    let result = triage.analyze(subject, sender, body, false);

    assert_eq!(result.category, TriageCategory::Personal);
    assert!(result.badges.is_empty());
}
