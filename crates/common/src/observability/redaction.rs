#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::fmt::{Debug, Display};

pub struct Redacted<T>(pub T);

impl<T> Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T> Display for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

pub struct SanitizedUrl<'a>(pub &'a str);

impl<'a> Display for SanitizedUrl<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match url::Url::parse(self.0) {
            Ok(mut parsed) => {
                let _ = parsed.set_username("");
                let _ = parsed.set_password(None);
                parsed.set_query(None);
                parsed.set_fragment(None);
                write!(f, "{}", parsed)
            }
            Err(_) => write!(f, "<invalid-url>"),
        }
    }
}

impl<'a> Debug for SanitizedUrl<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

pub struct SanitizedUserPath<'a>(pub Option<&'a std::path::Path>);

impl<'a> Display for SanitizedUserPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_some() {
            write!(f, "<user-path-redacted>")
        } else {
            write!(f, "None")
        }
    }
}

impl<'a> Debug for SanitizedUserPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

pub struct DiagnosticError<'a> {
    pub kind: &'a str,
    pub code: Option<&'a str>,
    pub retryable: bool,
}

impl<'a> Display for DiagnosticError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "kind={}, code={}, retryable={}",
            self.kind,
            self.code.unwrap_or("none"),
            self.retryable
        )
    }
}

impl<'a> Debug for DiagnosticError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

fn trim_word_punctuation(w: &str) -> &str {
    w.trim_matches(|c: char| {
        c == '('
            || c == ')'
            || c == '{'
            || c == '}'
            || c == '"'
            || c == '\''
            || c == ','
            || c == '.'
            || c == ';'
            || c == ':'
            || c == '!'
            || c == '?'
            || c == '<'
            || c == '>'
    })
}

pub fn redact_secrets(message: &str) -> String {
    let raw_words: Vec<&str> = message.split_whitespace().collect();
    let mut redacted_words = Vec::new();
    let mut skip_next = false;

    for i in 0..raw_words.len() {
        if skip_next {
            skip_next = false;
            continue;
        }

        let word = raw_words[i];
        let clean_word = trim_word_punctuation(word);

        if clean_word.is_empty() {
            redacted_words.push(word.to_string());
            continue;
        }

        // Bearer token check
        let lower_clean = clean_word.to_lowercase();
        if lower_clean == "bearer" && i + 1 < raw_words.len() {
            redacted_words.push(word.to_string());
            let next_word = raw_words[i + 1];
            let next_clean = trim_word_punctuation(next_word);
            let sanitized_next = next_word.replace(next_clean, "<redacted>");
            redacted_words.push(sanitized_next);
            skip_next = true;
            continue;
        }

        // URL check
        if clean_word.starts_with("http://") || clean_word.starts_with("https://") {
            let sanitized = format!("{}", SanitizedUrl(clean_word));
            let restored = word.replace(clean_word, &sanitized);
            redacted_words.push(restored);
            continue;
        }

        // Path check
        let is_win_path = (clean_word.len() >= 3
            && clean_word.as_bytes()[1] == b':'
            && (clean_word.as_bytes()[2] == b'\\' || clean_word.as_bytes()[2] == b'/'))
            || clean_word.starts_with(r"\\")
            || clean_word.starts_with(r"\\?\");
        let is_unix_path =
            clean_word.starts_with('/') && clean_word.chars().filter(|&c| c == '/').count() >= 2;

        if is_win_path || is_unix_path {
            let restored = word.replace(clean_word, "<path-redacted>");
            redacted_words.push(restored);
            continue;
        }

        // Credentials check (key=val or key:val)
        let mut credential_redacted = false;
        if let Some(pos) = word.find('=') {
            let (key, _val) = word.split_at(pos);
            let lower_key = key.to_lowercase();
            if lower_key.contains("token")
                || lower_key.contains("password")
                || lower_key.contains("key")
                || lower_key.contains("secret")
                || lower_key.contains("auth_")
                || lower_key.contains("_auth")
                || lower_key == "auth"
                || lower_key.contains("pass")
                || lower_key.contains("pwd")
            {
                let sanitized_pair = format!("{}=<redacted>", key);
                redacted_words.push(sanitized_pair);
                credential_redacted = true;
            }
        } else if let Some(pos) = word.find(':') {
            let (key, _val) = word.split_at(pos);
            let lower_key = key.to_lowercase();
            if (lower_key.contains("token")
                || lower_key.contains("password")
                || lower_key.contains("key")
                || lower_key.contains("secret")
                || lower_key.contains("auth_")
                || lower_key.contains("_auth")
                || lower_key == "auth"
                || lower_key.contains("pass")
                || lower_key.contains("pwd"))
                && !lower_key.starts_with("http")
                && !lower_key.starts_with("https")
            {
                let sanitized_pair = format!("{}:<redacted>", key);
                redacted_words.push(sanitized_pair);
                credential_redacted = true;
            }
        }

        if credential_redacted {
            continue;
        }

        redacted_words.push(word.to_string());
    }

    redacted_words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_redacted() {
        let r = Redacted("sensitive data");
        assert_eq!(format!("{}", r), "[REDACTED]");
        assert_eq!(format!("{:?}", r), "[REDACTED]");
    }

    #[test]
    fn test_sanitized_url() {
        let u = SanitizedUrl("https://user:pass@example.com/path?query=1#frag");
        assert_eq!(format!("{}", u), "https://example.com/path");
        assert!(!format!("{}", u).contains("user"));
        assert!(!format!("{}", u).contains("pass"));
        assert!(!format!("{}", u).contains("query"));
        assert!(!format!("{}", u).contains("frag"));

        let u_invalid = SanitizedUrl("not a url");
        assert_eq!(format!("{}", u_invalid), "<invalid-url>");
    }

    #[test]
    fn test_sanitized_user_path() {
        let p = SanitizedUserPath(Some(Path::new("/home/sensitive_name/secret.txt")));
        assert_eq!(format!("{}", p), "<user-path-redacted>");
        assert!(!format!("{}", p).contains("sensitive_name"));
        assert!(!format!("{}", p).contains("secret.txt"));

        let p_none = SanitizedUserPath(None);
        assert_eq!(format!("{}", p_none), "None");
    }

    #[test]
    fn test_diagnostic_error() {
        let e1 = DiagnosticError {
            kind: "IoError",
            code: Some("ENOENT"),
            retryable: true,
        };
        assert_eq!(
            format!("{}", e1),
            "kind=IoError, code=ENOENT, retryable=true"
        );
    }

    #[test]
    fn test_redact_secrets_fixtures() {
        // C:\Users\secret\file.mp4
        assert_eq!(
            redact_secrets("Failed to open C:\\Users\\secret\\file.mp4"),
            "Failed to open <path-redacted>"
        );

        // UNC and extended Windows paths
        assert_eq!(
            redact_secrets("Load path \\\\server\\share\\file.txt"),
            "Load path <path-redacted>"
        );
        assert_eq!(
            redact_secrets("Read file \\\\?\\C:\\path\\file.txt"),
            "Read file <path-redacted>"
        );

        // /home/user/secret
        assert_eq!(
            redact_secrets("Delete directory /home/user/secret"),
            "Delete directory <path-redacted>"
        );

        // URL query and fragment
        assert_eq!(
            redact_secrets("Fetch https://user:pass@example.com/path?token=abc#frag"),
            "Fetch https://example.com/path"
        );

        // token=, access_token=, password=
        assert_eq!(
            redact_secrets("DB connection password=secretpwd token=123 access_token=456"),
            "DB connection password=<redacted> token=<redacted> access_token=<redacted>"
        );

        // Authorization: Bearer ...
        assert_eq!(
            redact_secrets("Authorization: Bearer sct_token"),
            "Authorization: Bearer <redacted>"
        );

        // Case insensitivity of keys
        assert_eq!(
            redact_secrets("auth_key=mykey PASSWORD=pwd"),
            "auth_key=<redacted> PASSWORD=<redacted>"
        );

        // No panic on malformed input
        assert_eq!(redact_secrets("token="), "token=<redacted>");
        assert_eq!(redact_secrets("password:"), "password:<redacted>");
    }
}
