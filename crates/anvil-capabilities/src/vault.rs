// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Secret redaction vault — prevents accidental leaking of API keys,
//! passwords, and other secrets to LLM providers.

use anvil_core::types::CompletionRequest;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A redaction rule that matches a pattern in text.
#[derive(Debug, Clone)]
struct RedactionRule {
    name: &'static str,
    pattern: Regex,
    replacement: &'static str,
}

/// Record of a redaction that was applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionHit {
    pub label: String,
    pub count: usize,
}

/// Vault configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub enabled: bool,
    pub redact_local: bool,
    #[serde(default)]
    pub custom_patterns: Vec<CustomPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPattern {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            redact_local: false,
            custom_patterns: Vec::new(),
        }
    }
}

/// Built-in redaction rules covering common secret formats.
static RULES: Lazy<Vec<RedactionRule>> = Lazy::new(|| {
    vec![
        RedactionRule {
            name: "AWS Access Key",
            pattern: Regex::new(r"(?i)(AKIA[0-9A-Z]{16})").unwrap(),
            replacement: "[REDACTED_AWS_KEY]",
        },
        RedactionRule {
            name: "GitHub PAT",
            pattern: Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap(),
            replacement: "[REDACTED_GITHUB_PAT]",
        },
        RedactionRule {
            name: "GitHub OAuth",
            pattern: Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap(),
            replacement: "[REDACTED_GITHUB_OAUTH]",
        },
        RedactionRule {
            name: "OpenAI API Key",
            pattern: Regex::new(r"sk-[A-Za-z0-9]{20,}T3BlbkFJ[A-Za-z0-9]{20}").unwrap(),
            replacement: "[REDACTED_OPENAI_KEY]",
        },
        RedactionRule {
            name: "OpenAI Short Key",
            pattern: Regex::new(r"sk-[A-Za-z0-9]{48,}").unwrap(),
            replacement: "[REDACTED_API_KEY]",
        },
        RedactionRule {
            name: "Anthropic API Key",
            pattern: Regex::new(r"sk-ant-[A-Za-z0-9\-]{90,}").unwrap(),
            replacement: "[REDACTED_ANTHROPIC_KEY]",
        },
        RedactionRule {
            name: "Slack Bot Token",
            pattern: Regex::new(r"xoxb-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24,36}").unwrap(),
            replacement: "[REDACTED_SLACK_TOKEN]",
        },
        RedactionRule {
            name: "Slack User Token",
            pattern: Regex::new(r"xoxp-[0-9]{10,13}-[0-9]{10,13}-[0-9]{10,13}-[a-z0-9]{32}").unwrap(),
            replacement: "[REDACTED_SLACK_TOKEN]",
        },
        RedactionRule {
            name: "PEM Private Key",
            pattern: Regex::new(r"-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |DSA )?PRIVATE KEY-----").unwrap(),
            replacement: "[REDACTED_PRIVATE_KEY]",
        },
        RedactionRule {
            name: "JWT Token",
            pattern: Regex::new(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_.+/=]+").unwrap(),
            replacement: "[REDACTED_JWT]",
        },
        RedactionRule {
            name: "Generic Password Assignment",
            pattern: Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"]?([^\s'"<>]{8,})['"]?"#).unwrap(),
            replacement: "[REDACTED_PASSWORD]",
        },
        RedactionRule {
            name: "Generic API Key Assignment",
            pattern: Regex::new(r#"(?i)(api[_-]?key|apikey|secret[_-]?key|access[_-]?key)\s*[:=]\s*['"]?([^\s'"<>]{16,})['"]?"#).unwrap(),
            replacement: "[REDACTED_KEY]",
        },
    ]
});

/// Redact secrets from a text string. Returns the redacted text and a list of
/// what was found (labels + counts, never the matched values).
pub fn redact_text(input: &str, config: &VaultConfig) -> (String, Vec<RedactionHit>) {
    if !config.enabled {
        return (input.to_string(), vec![]);
    }

    let mut output = input.to_string();
    let mut hits = Vec::new();

    for rule in RULES.iter() {
        let matches: Vec<_> = rule.pattern.find_iter(&output).collect();
        if !matches.is_empty() {
            let count = matches.len();
            // Replace from end to preserve offsets
            let mut new_output = output.clone();
            for m in matches.iter().rev() {
                new_output.replace_range(m.range(), rule.replacement);
            }
            output = new_output;
            hits.push(RedactionHit {
                label: rule.name.to_string(),
                count,
            });
        }
    }

    // Apply custom patterns
    for custom in &config.custom_patterns {
        if let Ok(re) = Regex::new(&custom.pattern) {
            let matches: Vec<_> = re.find_iter(&output).collect();
            if !matches.is_empty() {
                let count = matches.len();
                let mut new_output = output.clone();
                for m in matches.iter().rev() {
                    new_output.replace_range(m.range(), &custom.replacement);
                }
                output = new_output;
                hits.push(RedactionHit {
                    label: custom.name.clone(),
                    count,
                });
            }
        }
    }

    (output, hits)
}

/// Redact secrets from every message in a completion request, in place.
/// Returns the aggregated list of what was found (labels + counts, never the
/// matched values) across all messages.
pub fn redact_request(request: &mut CompletionRequest, config: &VaultConfig) -> Vec<RedactionHit> {
    let mut hits = Vec::new();
    for message in &mut request.messages {
        let (redacted, message_hits) = redact_text(&message.content, config);
        message.content = redacted;
        hits.extend(message_hits);
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_key() {
        let input = "My key is sk-abc123def456ghi789jkl012mno345pqr678stu901vwx234T3BlbkFJtest1234567890abcdef";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert!(!redacted.contains("sk-"));
        assert!(hits
            .iter()
            .any(|h| h.label == "OpenAI Short Key" || h.label == "OpenAI API Key"));
    }

    #[test]
    fn redacts_github_pat() {
        let input = "token: ghp_abcdefghijklmnopqrstuvwxyz1234567890";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert!(!redacted.contains("ghp_"));
        assert!(hits.iter().any(|h| h.label == "GitHub PAT"));
    }

    #[test]
    fn redacts_anthropic_key() {
        let input = "key=sk-ant-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrs";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert!(!redacted.contains("sk-ant-"));
        assert!(hits.iter().any(|h| h.label == "Anthropic API Key"));
    }

    #[test]
    fn redacts_private_key() {
        let input =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQ...\n-----END RSA PRIVATE KEY-----";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert!(!redacted.contains("PRIVATE KEY"));
        assert!(hits.iter().any(|h| h.label == "PEM Private Key"));
    }

    #[test]
    fn redacts_jwt() {
        let input = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123signature";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert!(!redacted.contains("eyJ"));
        assert!(hits.iter().any(|h| h.label == "JWT Token"));
    }

    #[test]
    fn redacts_password_assignment() {
        let input = r#"password = "super_secret_123""#;
        let (redacted, _hits) = redact_text(input, &VaultConfig::default());
        assert!(redacted.contains("[REDACTED_PASSWORD]") || !redacted.contains("super_secret_123"));
    }

    #[test]
    fn no_false_positives_on_normal_code() {
        let input = "fn main() {\n    let x = 42;\n    println!(\"{x}\");\n}";
        let (redacted, hits) = redact_text(input, &VaultConfig::default());
        assert_eq!(redacted, input);
        assert!(hits.is_empty());
    }

    #[test]
    fn redact_request_scrubs_all_messages() {
        use anvil_core::types::{ChatMessage, Role};
        let mut request = CompletionRequest {
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: "you are helpful".into(),
                },
                ChatMessage {
                    role: Role::User,
                    content: "here is my key: ghp_abcdefghijklmnopqrstuvwxyz1234567890".into(),
                },
            ],
            model: None,
            max_tokens: 2048,
            temperature: 0.2,
            stream: true,
        };
        let hits = redact_request(&mut request, &VaultConfig::default());
        assert!(!hits.is_empty());
        assert!(!request.messages[1].content.contains("ghp_"));
        assert_eq!(request.messages[0].content, "you are helpful");
    }

    #[test]
    fn disabled_vault_returns_input() {
        let input = "sk-abc123def456ghi789jkl012mno345pqr678stu901vwx234";
        let cfg = VaultConfig {
            enabled: false,
            ..Default::default()
        };
        let (redacted, hits) = redact_text(input, &cfg);
        assert_eq!(redacted, input);
        assert!(hits.is_empty());
    }
}
