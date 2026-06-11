// SPDX-License-Identifier: GPL-3.0-or-later

use regex::Regex;
use std::sync::OnceLock;

/// Returns true when text looks too sensitive to store in clipboard history.
///
/// This is intentionally conservative. False positives are better than storing secrets.
pub fn looks_sensitive(text: &str) -> bool {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return false;
    }

    if trimmed.len() > 4096 && high_entropy_ratio(trimmed) > 0.70 {
        return true;
    }

    for pattern in sensitive_patterns() {
        if pattern.is_match(trimmed) {
            return true;
        }
    }

    false
}

fn sensitive_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    PATTERNS.get_or_init(|| {
        [
            // Common assignment-style secrets.
            r#"(?i)(password|passwd|pwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)\s*[:=]\s*['\"]?[^\s'\"]{8,}"#,
            // PEM private keys.
            r#"-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----"#,
            // GitHub tokens.
            r#"gh[pousr]_[A-Za-z0-9_]{36,255}"#,
            // Generic bearer JWT.
            r#"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+"#,
            // AWS access key id.
            r#"AKIA[0-9A-Z]{16}"#,
            // Slack tokens.
            r#"xox[baprs]-[A-Za-z0-9-]{10,}"#,
            // 12/18/24-word recovery phrases; heuristic only.
            r#"(?i)\b([a-z]{3,12}\s+){11,23}[a-z]{3,12}\b"#,
            // One-time codes shown alone.
            r#"^\s*\d{6,8}\s*$"#,
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("sensitive regex must compile"))
        .collect()
    })
}

fn high_entropy_ratio(text: &str) -> f64 {
    let interesting = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '_' | '-'))
        .count();
    interesting as f64 / text.chars().count().max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::looks_sensitive;

    #[test]
    fn catches_assignment_secret() {
        assert!(looks_sensitive("API_KEY=abcdef1234567890"));
    }

    #[test]
    fn catches_private_key() {
        assert!(looks_sensitive("-----BEGIN OPENSSH PRIVATE KEY-----"));
    }

    #[test]
    fn allows_normal_text() {
        assert!(!looks_sensitive("copy this normal sentence"));
    }
}
