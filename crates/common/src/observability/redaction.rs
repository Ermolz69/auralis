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

        let e2 = DiagnosticError {
            kind: "Unknown",
            code: None,
            retryable: false,
        };
        assert_eq!(
            format!("{}", e2),
            "kind=Unknown, code=none, retryable=false"
        );
    }
}
