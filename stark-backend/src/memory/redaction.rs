//! PII and secret redaction for memory content.
//!
//! Automatically strips sensitive data (private keys, API keys, tokens, passwords)
//! before memories are persisted. Uses compiled regex patterns via a lazy singleton.

use once_cell::sync::Lazy;
use regex::Regex;

/// A single redaction pattern with its label.
struct RedactionPattern {
    label: &'static str,
    regex: Regex,
}

/// Compiled set of redaction patterns, initialized once.
struct MemoryRedactor {
    patterns: Vec<RedactionPattern>,
}

/// Result of redacting content.
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// Content with secrets replaced by `[REDACTED:<type>]`
    pub content: String,
    /// Number of redactions applied
    pub redaction_count: usize,
    /// Types of secrets that were redacted
    pub redacted_types: Vec<String>,
}

/// Global singleton — patterns are compiled once on first access.
static REDACTOR: Lazy<MemoryRedactor> = Lazy::new(|| {
    // Order matters: more specific patterns first to avoid partial matches
    let patterns = vec![
        // Ethereum private keys (64 hex chars prefixed with 0x)
        RedactionPattern {
            label: "eth_private_key",
            regex: Regex::new(r"0x[0-9a-fA-F]{64}").unwrap(),
        },
        // AWS access key IDs
        RedactionPattern {
            label: "aws_access_key",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        // JWT tokens (three base64url segments)
        RedactionPattern {
            label: "jwt_token",
            regex: Regex::new(r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}").unwrap(),
        },
        // Bearer tokens
        RedactionPattern {
            label: "bearer_token",
            regex: Regex::new(r"Bearer\s+[A-Za-z0-9_\-\.]{20,}").unwrap(),
        },
        // Generic API keys (sk_*, pk_*, api_*, api-*)
        RedactionPattern {
            label: "api_key",
            regex: Regex::new(r"(?:sk|pk|api)[_-][A-Za-z0-9]{20,}").unwrap(),
        },
        // Base58 private keys (Bitcoin WIF format)
        RedactionPattern {
            label: "base58_private_key",
            regex: Regex::new(r"[5KL][1-9A-HJ-NP-Za-km-z]{50,51}").unwrap(),
        },
        // Passwords in context (password: xxx, secret=xxx, etc.)
        RedactionPattern {
            label: "password",
            regex: Regex::new(r"(?i)(password|passwd|secret)\s*[:=]\s*\S+").unwrap(),
        },
        // Credit card numbers (with optional separators)
        RedactionPattern {
            label: "credit_card",
            regex: Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap(),
        },
    ];

    MemoryRedactor { patterns }
});

/// Redact sensitive content from a string.
///
/// Scans the input against all known secret patterns and replaces matches
/// with `[REDACTED:<type>]` placeholders.
pub fn redact_content(content: &str) -> RedactionResult {
    let mut result = content.to_string();
    let mut count = 0usize;
    let mut types = Vec::new();

    for pattern in &REDACTOR.patterns {
        let before = result.clone();
        let replacement = format!("[REDACTED:{}]", pattern.label);
        result = pattern.regex.replace_all(&result, replacement.as_str()).to_string();

        if result != before {
            count += 1;
            types.push(pattern.label.to_string());
        }
    }

    RedactionResult {
        content: result,
        redaction_count: count,
        redacted_types: types,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_private_key_redacted() {
        let input = "My key is 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:eth_private_key]"));
        assert!(!result.content.contains("abcdef1234567890"));
        assert_eq!(result.redaction_count, 1);
    }

    #[test]
    fn test_api_key_redacted() {
        let input = "Use key api_FAKEFAKEFAKEFAKE1234567890";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:api_key]"));
        assert!(!result.content.contains("api_FAKE"));
    }

    #[test]
    fn test_password_redacted() {
        let input = "Set password: mysecretpass123";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:password]"));
        assert!(!result.content.contains("mysecretpass123"));
    }

    #[test]
    fn test_no_redaction_needed() {
        let input = "Just a normal memory about what happened today.";
        let result = redact_content(input);
        assert_eq!(result.content, input);
        assert_eq!(result.redaction_count, 0);
        assert!(result.redacted_types.is_empty());
    }

    #[test]
    fn test_multiple_redactions() {
        let input = "Key: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 and password=hunter2";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:eth_private_key]"));
        assert!(result.content.contains("[REDACTED:password]"));
        assert_eq!(result.redaction_count, 2);
    }

    #[test]
    fn test_jwt_redacted() {
        let input = "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:jwt_token]"));
    }

    #[test]
    fn test_credit_card_redacted() {
        let input = "Card number: 4111-1111-1111-1111";
        let result = redact_content(input);
        assert!(result.content.contains("[REDACTED:credit_card]"));
        assert!(!result.content.contains("4111"));
    }
}
