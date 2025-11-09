//! PII detection and redaction

use regex::Regex;

/// PII redactor that removes sensitive information
pub struct PIIRedactor {
    ssn_regex: Regex,
    email_regex: Regex,
    credit_card_regex: Regex,
}

impl Default for PIIRedactor {
    fn default() -> Self {
        Self::new()
    }
}

impl PIIRedactor {
    pub fn new() -> Self {
        Self {
            ssn_regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
                .unwrap(),
            credit_card_regex: Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap(),
        }
    }

    /// Redacts PII from text
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Redact SSN
        result = self
            .ssn_regex
            .replace_all(&result, "[SSN_REDACTED]")
            .to_string();

        // Redact email
        result = self
            .email_regex
            .replace_all(&result, "[EMAIL_REDACTED]")
            .to_string();

        // Redact credit card
        result = self
            .credit_card_regex
            .replace_all(&result, "[CC_REDACTED]")
            .to_string();

        result
    }

    /// Detects if text contains PII
    pub fn contains_pii(&self, text: &str) -> bool {
        self.ssn_regex.is_match(text)
            || self.email_regex.is_match(text)
            || self.credit_card_regex.is_match(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_redaction() {
        let redactor = PIIRedactor::new();

        let text = "My SSN is 123-45-6789 and email is test@example.com";
        let redacted = redactor.redact(text);

        assert!(redacted.contains("[SSN_REDACTED]"));
        assert!(redacted.contains("[EMAIL_REDACTED]"));
        assert!(!redacted.contains("123-45-6789"));
        assert!(!redacted.contains("test@example.com"));
    }

    #[test]
    fn test_pii_detection() {
        let redactor = PIIRedactor::new();

        assert!(redactor.contains_pii("My SSN is 123-45-6789"));
        assert!(redactor.contains_pii("Email: test@example.com"));
        assert!(!redactor.contains_pii("No PII here"));
    }
}
