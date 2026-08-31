//! Pure, deterministic product model. This crate performs no filesystem mutation.

use globset::Glob;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;
use uuid::Uuid;

pub const SCHEMA_VERSION: u8 = 1;
pub const MAX_RULES: usize = 1_000;
pub const MAX_REASON_BYTES: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileFacts {
    pub extension: Option<String>,
    pub verified_mime: Option<String>,
    pub filename: String,
    pub source_root_id: Uuid,
    pub source_directory: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Extension { value: String },
    VerifiedMime { value: String },
    FilenameGlob { value: String },
    SourceRoot { id: Uuid },
    SourceDirectory { value: String },
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Rule {
    pub id: Uuid,
    pub priority: i32,
    pub predicates: Vec<Predicate>,
    pub destination_root_id: Uuid,
    pub destination_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Destination {
    pub root_id: Uuid,
    pub directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleDecision {
    Destination {
        destination: Destination,
        rule_ids: Vec<Uuid>,
        reason: String,
    },
    Tie {
        destinations: Vec<Destination>,
        rule_ids: Vec<Uuid>,
        reason: String,
    },
    Abstain {
        reason: String,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("too many rules")]
    TooManyRules,
    #[error("invalid rule: {0}")]
    InvalidRule(String),
}

pub fn evaluate_rules(rules: &[Rule], facts: &FileFacts) -> Result<RuleDecision, RuleError> {
    if rules.len() > MAX_RULES {
        return Err(RuleError::TooManyRules);
    }
    let mut matches = Vec::new();
    for rule in rules {
        validate_rule(rule)?;
        if rule.predicates.iter().all(|p| predicate_matches(p, facts)) {
            matches.push(rule);
        }
    }
    let Some(top_priority) = matches.iter().map(|r| r.priority).max() else {
        return Ok(RuleDecision::Abstain {
            reason: "No deterministic rule matched".into(),
        });
    };
    let top: Vec<_> = matches
        .into_iter()
        .filter(|rule| rule.priority == top_priority)
        .collect();
    let destinations: BTreeSet<_> = top
        .iter()
        .map(|rule| Destination {
            root_id: rule.destination_root_id,
            directory: rule.destination_directory.clone(),
        })
        .collect();
    let rule_ids = top.iter().map(|r| r.id).collect::<Vec<_>>();
    if destinations.len() == 1 {
        Ok(RuleDecision::Destination {
            destination: destinations.into_iter().next().expect("one destination"),
            rule_ids,
            reason: format!("Unique destination from priority {top_priority} rule set"),
        })
    } else {
        Ok(RuleDecision::Tie {
            destinations: destinations.into_iter().collect(),
            rule_ids,
            reason: format!("Conflicting destinations at priority {top_priority}"),
        })
    }
}

fn validate_rule(rule: &Rule) -> Result<(), RuleError> {
    if rule.predicates.is_empty() {
        return Err(RuleError::InvalidRule(
            "predicates must not be empty".into(),
        ));
    }
    validate_relative_directory(&rule.destination_directory)?;
    for predicate in &rule.predicates {
        match predicate {
            Predicate::FilenameGlob { value } => {
                Glob::new(value).map_err(|_| RuleError::InvalidRule("invalid glob".into()))?;
            }
            Predicate::Extension { value } | Predicate::VerifiedMime { value }
                if value.is_empty() || value.len() > 255 =>
            {
                return Err(RuleError::InvalidRule("invalid predicate value".into()));
            }
            Predicate::SourceDirectory { value } => validate_relative_directory(value)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn validate_relative_directory(value: &str) -> Result<(), RuleError> {
    if value == "." {
        return Ok(());
    }
    if value.len() > 4_096
        || value.starts_with('/')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(RuleError::InvalidRule("unsafe relative directory".into()));
    }
    Ok(())
}

fn predicate_matches(predicate: &Predicate, facts: &FileFacts) -> bool {
    match predicate {
        Predicate::Extension { value } => facts
            .extension
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case(value)),
        Predicate::VerifiedMime { value } => facts.verified_mime.as_deref() == Some(value),
        Predicate::FilenameGlob { value } => Glob::new(value)
            .ok()
            .map(|g| g.compile_matcher().is_match(&facts.filename))
            .unwrap_or(false),
        Predicate::SourceRoot { id } => facts.source_root_id == *id,
        Predicate::SourceDirectory { value } => facts.source_directory == *value,
        Predicate::Completed => facts.completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> FileFacts {
        FileFacts {
            extension: Some("pdf".into()),
            verified_mime: Some("application/pdf".into()),
            filename: "report.pdf".into(),
            source_root_id: Uuid::nil(),
            source_directory: "downloads".into(),
            completed: true,
        }
    }

    fn rule(priority: i32, directory: &str) -> Rule {
        Rule {
            id: Uuid::new_v4(),
            priority,
            predicates: vec![Predicate::Extension {
                value: "pdf".into(),
            }],
            destination_root_id: Uuid::nil(),
            destination_directory: directory.into(),
        }
    }

    #[test]
    fn higher_priority_wins() {
        let decision = evaluate_rules(&[rule(1, "old"), rule(2, "docs")], &facts()).unwrap();
        assert!(
            matches!(decision, RuleDecision::Destination { destination, .. } if destination.directory == "docs")
        );
    }

    #[test]
    fn equal_priority_different_destinations_tie() {
        let decision = evaluate_rules(&[rule(2, "a"), rule(2, "b")], &facts()).unwrap();
        assert!(matches!(decision, RuleDecision::Tie { .. }));
    }

    #[test]
    fn mime_does_not_trust_extension() {
        let mut mime_rule = rule(3, "pdfs");
        mime_rule.predicates = vec![Predicate::VerifiedMime {
            value: "image/png".into(),
        }];
        assert!(matches!(
            evaluate_rules(&[mime_rule], &facts()).unwrap(),
            RuleDecision::Abstain { .. }
        ));
    }

    #[test]
    fn traversal_is_rejected() {
        assert!(validate_relative_directory("../escape").is_err());
    }
}
