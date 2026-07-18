use ports::error::PortError;
use url::Url;

pub(crate) fn validate_url(url_str: &str) -> Result<Url, PortError> {
    let parsed = Url::parse(url_str).map_err(|_| PortError::InvalidSource {
        message: "Invalid URL".to_string(),
    })?;

    if parsed.scheme() != "https" {
        return Err(PortError::InvalidSource {
            message: "Only secure HTTPS URLs are supported".to_string(),
        });
    }

    let host = parsed.host_str().ok_or_else(|| PortError::InvalidSource {
        message: "Missing host".to_string(),
    })?;

    let host_lower = host.to_lowercase();
    let is_allowed = host_lower == "youtube.com"
        || host_lower.ends_with(".youtube.com")
        || host_lower == "youtu.be";

    if !is_allowed {
        return Err(PortError::InvalidSource {
            message: "Host not allowed".to_string(),
        });
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(PortError::InvalidSource {
            message: "User info is not allowed in the URL".to_string(),
        });
    }

    if parsed.port().map(|p| p != 443).unwrap_or(false) {
        return Err(PortError::InvalidSource {
            message: "Non-standard port not allowed".to_string(),
        });
    }

    Ok(parsed)
}
