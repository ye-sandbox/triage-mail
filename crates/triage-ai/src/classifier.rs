//! Deterministic category classification for inbound emails.
//!
//! Maps emails into `TriageCategory` using extracted badges and contextual keyword rules:
//! - `Security`: 2FA/OTP codes, password resets, suspicious logins.
//! - `Financial`: Boletos, Pix payments, invoices, banking statements.
//! - `Shopping`: Package tracking, dispatch confirmations, order updates.
//! - `Newsletter`: RFC 8058 One-Click Unsubscribe, marketing blasts, newsletters.
//! - `Personal`: Direct personal communications (default).

use crate::{TriageBadge, TriageCategory};

/// Determines the high-level category of an email based on badges, subject, sender, and text.
pub fn classify_message(
    subject: &str,
    sender: &str,
    body: &str,
    badges: &[TriageBadge],
    has_rfc8058_one_click: bool,
) -> TriageCategory {
    // 1. Check for definitive Security triggers
    let has_otp = badges
        .iter()
        .any(|b| matches!(b, TriageBadge::OtpCode { .. }));
    if has_otp {
        return TriageCategory::Security;
    }

    let subj_lower = subject.to_lowercase();
    let body_lower = body.to_lowercase();
    let sender_lower = sender.to_lowercase();

    let is_security_text = subj_lower.contains("redefinição de senha")
        || subj_lower.contains("redefinir senha")
        || subj_lower.contains("password reset")
        || subj_lower.contains("reset your password")
        || subj_lower.contains("alerta de segurança")
        || subj_lower.contains("security alert")
        || subj_lower.contains("tentativa de login")
        || subj_lower.contains("suspicious sign-in")
        || subj_lower.contains("novo acesso à sua conta")
        || subj_lower.contains("código de verificação")
        || subj_lower.contains("verification code");

    if is_security_text {
        return TriageCategory::Security;
    }

    // 2. Check for definitive Financial triggers
    let has_financial_badge = badges.iter().any(|b| {
        matches!(
            b,
            TriageBadge::FebrabanBoleto { .. }
                | TriageBadge::PixPayload { .. }
                | TriageBadge::NfeInvoice { .. }
        )
    });
    if has_financial_badge {
        return TriageCategory::Financial;
    }

    let is_financial_text = subj_lower.contains("fatura")
        || subj_lower.contains("extrato")
        || subj_lower.contains("comprovante de pagamento")
        || subj_lower.contains("invoice")
        || subj_lower.contains("recibo de pagamento")
        || subj_lower.contains("pagamento aprovado")
        || subj_lower.contains("transferência recebida")
        || subj_lower.contains("boleto bancário")
        || subj_lower.contains("bank statement")
        || subj_lower.contains("seu boleto");

    if is_financial_text {
        return TriageCategory::Financial;
    }

    // 3. Check for definitive Shopping / Logistics triggers
    let has_tracking = badges
        .iter()
        .any(|b| matches!(b, TriageBadge::TrackingCode { .. }));
    if has_tracking {
        return TriageCategory::Shopping;
    }

    let is_shopping_text = subj_lower.contains("seu pedido foi")
        || subj_lower.contains("pedido confirmado")
        || subj_lower.contains("order confirmed")
        || subj_lower.contains("saiu para entrega")
        || subj_lower.contains("entrega prevista")
        || subj_lower.contains("rastreamento")
        || subj_lower.contains("tracking number")
        || subj_lower.contains("despachado")
        || subj_lower.contains("compra aprovada");

    if is_shopping_text {
        return TriageCategory::Shopping;
    }

    // 4. Check for Newsletter / Marketing triggers
    if has_rfc8058_one_click {
        return TriageCategory::Newsletter;
    }

    let is_newsletter_text = subj_lower.contains("newsletter")
        || subj_lower.contains("boletim")
        || body_lower.contains("descadastre-se")
        || body_lower.contains("cancelar inscrição")
        || body_lower.contains("unsubscribe")
        || sender_lower.contains("marketing")
        || sender_lower.contains("news");

    if is_newsletter_text {
        return TriageCategory::Newsletter;
    }

    // 5. Default: Personal
    TriageCategory::Personal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_otp_badge_as_security() {
        let badges = vec![TriageBadge::OtpCode {
            code: "123456".to_string(),
        }];
        let cat = classify_message(
            "Login code",
            "auth@example.com",
            "Code 123456",
            &badges,
            false,
        );
        assert_eq!(cat, TriageCategory::Security);
    }

    #[test]
    fn classifies_boleto_badge_as_financial() {
        let badges = vec![TriageBadge::FebrabanBoleto {
            digitable_line: "34190104355100491020815000000001110000000015000".to_string(),
            amount_cents: Some(15000),
            due_date_iso: Some("2025-02-22".to_string()),
        }];
        let cat = classify_message(
            "Conta de telefone",
            "cobranca@example.org",
            "Segue boleto",
            &badges,
            false,
        );
        assert_eq!(cat, TriageCategory::Financial);
    }

    #[test]
    fn classifies_tracking_code_as_shopping() {
        let badges = vec![TriageBadge::TrackingCode {
            carrier: "Correios".to_string(),
            code: "NL123456785BR".to_string(),
        }];
        let cat = classify_message(
            "Seu pacote está a caminho",
            "loja@example.com",
            "Rastreio NL123456785BR",
            &badges,
            false,
        );
        assert_eq!(cat, TriageCategory::Shopping);
    }

    #[test]
    fn classifies_one_click_unsubscribe_as_newsletter() {
        let badges = vec![TriageBadge::UnsubscribeAvailable {
            supports_one_click: true,
        }];
        let cat = classify_message(
            "Novidades da semana",
            "digest@example.com",
            "Confira nossas matérias",
            &badges,
            true,
        );
        assert_eq!(cat, TriageCategory::Newsletter);
    }

    #[test]
    fn classifies_direct_message_as_personal() {
        let badges = vec![];
        let cat = classify_message(
            "Reunião amanhã às 14h",
            "amigo@example.com",
            "Podemos nos encontrar no café?",
            &badges,
            false,
        );
        assert_eq!(cat, TriageCategory::Personal);
    }
}
