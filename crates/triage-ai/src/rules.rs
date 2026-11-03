//! Pure local rule matching and action planning; no storage or delivery effects.
use serde::{Deserialize, Serialize};

pub const MAX_RULES: usize = 200;
pub const MAX_SIMULATION_MESSAGES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub version: u32,
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub enabled: bool,
    pub position: u32,
    pub all: Vec<Condition>,
    pub any: Vec<Condition>,
    pub except: Vec<Condition>,
    pub actions: Vec<RuleAction>,
    pub stop_processing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    Account(String),
    Sender(String),
    Domain(String),
    DomainSuffix(String),
    Subject(String),
    Body(String),
    AttachmentName(String),
    AttachmentMime(String),
    AmountAtLeast(u64),
    AmountAtMost(u64),
    DueOnOrAfter(String),
    DueOnOrBefore(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    Tag(String),
    Star,
    Archive,
    Alert(String),
    UrgentAlert(String),
}

/// Normalized facts are supplied by the ingestion/storage adapter, never by rule scripts.
#[derive(Debug, Clone, Default)]
pub struct MessageFacts {
    pub account_id: String,
    pub sender_address: Option<String>,
    pub subject: String,
    pub body: Option<String>,
    pub attachments: Vec<(String, String)>,
    pub amounts_cents: Vec<u64>,
    pub due_dates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionResult {
    Matched,
    NotMatched,
    MissingFact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleOutcome {
    Matched,
    NotMatched,
    Excluded,
    Disabled,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleExplanation {
    pub rule_id: String,
    pub outcome: RuleOutcome,
    pub all: Vec<ConditionResult>,
    pub any: Vec<ConditionResult>,
    pub except: Vec<ConditionResult>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePlan {
    pub actions: Vec<RuleAction>,
    pub explanations: Vec<RuleExplanation>,
}

pub fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = value[..4].parse().unwrap_or(0);
    let month: usize = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..].parse().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        0,
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && month <= 12 && day > 0 && day <= days[month]
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

impl Rule {
    pub fn validate(&self) -> bool {
        self.version == 1
            && valid_text(&self.id, 128)
            && valid_text(&self.name, 120)
            && self.revision < i32::MAX as u32
            && self.all.len() + self.any.len() > 0
            && self.all.len() + self.any.len() + self.except.len() <= 64
            && !self.actions.is_empty()
            && self.actions.len() <= 16
            && self
                .all
                .iter()
                .chain(&self.any)
                .chain(&self.except)
                .all(Condition::validate)
            && self.actions.iter().all(|a| match a {
                RuleAction::Tag(tag) => valid_text(tag, 80),
                RuleAction::Alert(id) | RuleAction::UrgentAlert(id) => crate::alerts::valid_id(id),
                _ => true,
            })
    }
}

impl Condition {
    fn validate(&self) -> bool {
        match self {
            Self::DueOnOrAfter(v) | Self::DueOnOrBefore(v) => valid_date(v),
            Self::AmountAtLeast(v) | Self::AmountAtMost(v) => *v <= 9_007_199_254_740_991,
            Self::Sender(v) => valid_text(v, 320) && v.contains('@'),
            Self::Domain(v) | Self::DomainSuffix(v) => {
                valid_text(v, 253)
                    && v.split('.').all(|p| {
                        !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    })
            }
            Self::Account(v) => valid_text(v, 128),
            Self::Subject(v)
            | Self::Body(v)
            | Self::AttachmentName(v)
            | Self::AttachmentMime(v) => valid_text(v, 512),
        }
    }

    fn matches(&self, facts: &MessageFacts) -> ConditionResult {
        use ConditionResult::{Matched, MissingFact, NotMatched};
        let contains = |s: &str, v: &str| s.to_lowercase().contains(&v.to_lowercase());
        let domain = facts
            .sender_address
            .as_deref()
            .and_then(|a| a.rsplit_once('@').map(|(_, d)| d.to_ascii_lowercase()));
        let matched = match self {
            Self::Account(v) => facts.account_id == *v,
            Self::Sender(v) => match &facts.sender_address {
                Some(a) => a.eq_ignore_ascii_case(v),
                None => return MissingFact,
            },
            Self::Domain(v) => match domain {
                Some(d) => d.eq_ignore_ascii_case(v),
                None => return MissingFact,
            },
            Self::DomainSuffix(v) => match domain {
                Some(d) => {
                    d == v.to_ascii_lowercase()
                        || d.ends_with(&format!(".{}", v.to_ascii_lowercase()))
                }
                None => return MissingFact,
            },
            Self::Subject(v) => contains(&facts.subject, v),
            Self::Body(v) => match &facts.body {
                Some(b) => contains(b, v),
                None => return MissingFact,
            },
            Self::AttachmentName(v) => facts.attachments.iter().any(|(n, _)| contains(n, v)),
            Self::AttachmentMime(v) => facts.attachments.iter().any(|(_, m)| contains(m, v)),
            Self::AmountAtLeast(v) | Self::AmountAtMost(v) => {
                if facts.amounts_cents.is_empty() {
                    return MissingFact;
                }
                facts.amounts_cents.iter().any(|a| {
                    if matches!(self, Self::AmountAtLeast(_)) {
                        a >= v
                    } else {
                        a <= v
                    }
                })
            }
            Self::DueOnOrAfter(v) | Self::DueOnOrBefore(v) => {
                if facts.due_dates.is_empty() {
                    return MissingFact;
                }
                facts.due_dates.iter().filter(|d| valid_date(d)).any(|d| {
                    if matches!(self, Self::DueOnOrAfter(_)) {
                        d >= v
                    } else {
                        d <= v
                    }
                })
            }
        };
        if matched {
            Matched
        } else {
            NotMatched
        }
    }
}

/// INVARIANT: All rules see the same facts; actions never feed back into matching.
pub fn evaluate(rules: &[Rule], facts: &MessageFacts) -> RulePlan {
    let mut ordered: Vec<_> = rules.iter().collect();
    ordered.sort_by(|a, b| (a.position, &a.id).cmp(&(b.position, &b.id)));
    let mut plan = RulePlan::default();
    let mut stopped = false;
    for rule in ordered {
        let mut explanation = RuleExplanation {
            rule_id: rule.id.clone(),
            outcome: RuleOutcome::NotMatched,
            all: vec![],
            any: vec![],
            except: vec![],
        };
        if stopped {
            explanation.outcome = RuleOutcome::Stopped;
        } else if !rule.enabled {
            explanation.outcome = RuleOutcome::Disabled;
        } else if rule.validate() {
            explanation.all = rule.all.iter().map(|c| c.matches(facts)).collect();
            explanation.any = rule.any.iter().map(|c| c.matches(facts)).collect();
            explanation.except = rule.except.iter().map(|c| c.matches(facts)).collect();
            let matched = explanation
                .all
                .iter()
                .all(|r| *r == ConditionResult::Matched)
                && (explanation.any.is_empty()
                    || explanation.any.contains(&ConditionResult::Matched));
            if explanation.except.contains(&ConditionResult::Matched) {
                explanation.outcome = RuleOutcome::Excluded;
            } else if matched {
                explanation.outcome = RuleOutcome::Matched;
                for action in &rule.actions {
                    if !plan.actions.contains(action) {
                        plan.actions.push(action.clone());
                    }
                }
                stopped = rule.stop_processing;
            }
        }
        plan.explanations.push(explanation);
    }
    // INVARIANT: Stars target the source UID before any archive MOVE changes that UID.
    plan.actions.sort_by_key(|a| match a {
        RuleAction::Tag(_) => 0,
        RuleAction::Star => 1,
        RuleAction::Archive => 2,
        RuleAction::UrgentAlert(_) => 3,
        RuleAction::Alert(_) => 4,
    });
    plan
}

/// Parse exactly one mailbox; display-name text is never treated as an address.
pub fn sender_address(sender: &str) -> Option<String> {
    if sender.contains(['\r', '\n']) {
        return None;
    }
    let header = format!("From: {sender}\r\n\r\n");
    let message = mail_parser::MessageParser::default().parse(header.as_bytes())?;
    let list = message.from()?;
    let addresses: Vec<_> = list.mailboxes().collect();
    if addresses.len() != 1 {
        return None;
    }
    let address = addresses[0].address()?;
    let (local, domain) = address.rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    Some(address.to_ascii_lowercase())
}
