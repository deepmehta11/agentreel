use regex::Regex;
use std::sync::LazyLock;

/// Patterns that match common secrets and PII.
static SECRET_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Anthropic API keys
        (Regex::new(r"sk-ant-[a-zA-Z0-9\-]{20,}").unwrap(), "[REDACTED_ANTHROPIC_KEY]"),
        // OpenAI API keys
        (Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(), "[REDACTED_API_KEY]"),
        // GitHub tokens
        (Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
        (Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
        (Regex::new(r"ghu_[a-zA-Z0-9]{36}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
        (Regex::new(r"ghs_[a-zA-Z0-9]{36}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
        (Regex::new(r"github_pat_[a-zA-Z0-9_]{22,}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
        // Stripe keys
        (Regex::new(r"sk_live_[a-zA-Z0-9]{20,}").unwrap(), "[REDACTED_STRIPE_KEY]"),
        (Regex::new(r"sk_test_[a-zA-Z0-9]{20,}").unwrap(), "[REDACTED_STRIPE_KEY]"),
        (Regex::new(r"rk_live_[a-zA-Z0-9]{20,}").unwrap(), "[REDACTED_STRIPE_KEY]"),
        (Regex::new(r"rk_test_[a-zA-Z0-9]{20,}").unwrap(), "[REDACTED_STRIPE_KEY]"),
        // AWS
        (Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), "[REDACTED_AWS_KEY]"),
        (Regex::new(r"(?i)(aws_secret_access_key\s*[:=]\s*)[a-zA-Z0-9/+=]{40}").unwrap(), "${1}[REDACTED]"),
        // Google API keys
        (Regex::new(r"AIza[a-zA-Z0-9_\-]{35}").unwrap(), "[REDACTED_GOOGLE_KEY]"),
        // Slack tokens
        (Regex::new(r"xoxb-[a-zA-Z0-9\-]{20,}").unwrap(), "[REDACTED_SLACK_TOKEN]"),
        (Regex::new(r"xoxp-[a-zA-Z0-9\-]{20,}").unwrap(), "[REDACTED_SLACK_TOKEN]"),
        (Regex::new(r"xoxo-[a-zA-Z0-9\-]{20,}").unwrap(), "[REDACTED_SLACK_TOKEN]"),
        // Generic API key/token/password assignments
        (Regex::new(r#"(?i)(api[_-]?key\s*[:=]\s*)['"]?([a-zA-Z0-9_\-]{20,})['"]?"#).unwrap(), "${1}[REDACTED]"),
        (Regex::new(r"(?i)(bearer\s+)([a-zA-Z0-9_\-.]{20,})").unwrap(), "${1}[REDACTED]"),
        (Regex::new(r#"(?i)(password\s*[:=]\s*)['"]?([^\s'"]{4,})['"]?"#).unwrap(), "${1}[REDACTED]"),
        (Regex::new(r#"(?i)(token\s*[:=]\s*)['"]?([a-zA-Z0-9_\-.]{20,})['"]?"#).unwrap(), "${1}[REDACTED]"),
        (Regex::new(r#"(?i)(secret\s*[:=]\s*)['"]?([a-zA-Z0-9_\-.]{20,})['"]?"#).unwrap(), "${1}[REDACTED]"),
        // Private keys
        (Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(), "[REDACTED_PRIVATE_KEY]"),
        // Email addresses
        (Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(), "[REDACTED_EMAIL]"),
        // IP addresses (IPv4)
        (Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b").unwrap(), "[REDACTED_IP]"),
    ]
});

/// Redact secrets and PII from a string.
pub fn redact(input: &str) -> String {
    let mut result = input.to_string();
    for (pattern, replacement) in SECRET_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_openai_key() {
        let input = "Using key sk-abcdefghijklmnopqrstuvwxyz123456";
        let result = redact(input);
        assert!(result.contains("[REDACTED_API_KEY]"));
        assert!(!result.contains("sk-abcdef"));
    }

    #[test]
    fn test_redact_anthropic_key() {
        let input = "key: sk-ant-abc123-xyzxyzxyzxyzxyzxyzxyz";
        let result = redact(input);
        assert!(result.contains("[REDACTED_ANTHROPIC_KEY]"));
    }

    #[test]
    fn test_redact_github_token() {
        let input = "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let result = redact(input);
        assert!(result.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn test_redact_github_pat() {
        let input = "token: github_pat_abcdefghijklmnopqrstuvwx";
        let result = redact(input);
        assert!(result.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn test_redact_stripe_key() {
        // Build the key dynamically to avoid GitHub push protection
        let prefix = "sk_live_";
        let suffix = "x".repeat(24);
        let input = format!("STRIPE_KEY={}{}", prefix, suffix);
        let result = redact(input.as_str());
        assert!(result.contains("[REDACTED_STRIPE_KEY]"));
    }

    #[test]
    fn test_redact_stripe_test_key() {
        let prefix = "sk_test_";
        let suffix = "x".repeat(24);
        let input = format!("STRIPE_KEY={}{}", prefix, suffix);
        let result = redact(input.as_str());
        assert!(result.contains("[REDACTED_STRIPE_KEY]"));
    }

    #[test]
    fn test_redact_google_key() {
        let input = "key: AIzaSyB-abcdefghijklmnopqrstuvwxyz12345";
        let result = redact(input);
        assert!(result.contains("[REDACTED_GOOGLE_KEY]"));
    }

    #[test]
    fn test_redact_slack_token() {
        let input = "SLACK_TOKEN=xoxb-123456789012-abcdefghij";
        let result = redact(input);
        assert!(result.contains("[REDACTED_SLACK_TOKEN]"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let result = redact(input);
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_aws_key() {
        let input = "AWS key: AKIAIOSFODNN7EXAMPLE";
        let result = redact(input);
        assert!(result.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn test_redact_private_key() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpA...";
        let result = redact(input);
        assert!(result.contains("[REDACTED_PRIVATE_KEY]"));
    }

    #[test]
    fn test_redact_email() {
        let input = "Contact user@example.com for details";
        let result = redact(input);
        assert!(result.contains("[REDACTED_EMAIL]"));
        assert!(!result.contains("user@example.com"));
    }

    #[test]
    fn test_redact_ip_address() {
        let input = "Server at 192.168.1.100 is down";
        let result = redact(input);
        assert!(result.contains("[REDACTED_IP]"));
        assert!(!result.contains("192.168.1.100"));
    }

    #[test]
    fn test_redact_secret_assignment() {
        let input = "secret = my-super-secret-value-here-1234";
        let result = redact(input);
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_no_false_positive_on_short_strings() {
        let input = "The api returned 200 OK";
        let result = redact(input);
        assert_eq!(result, input);
    }
}
