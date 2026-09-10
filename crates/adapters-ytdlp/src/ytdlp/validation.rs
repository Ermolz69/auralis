use ports::error::PortError;
use url::Url;

const ALLOWED_YOUTUBE_HOSTS: &[&str] = &[
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
];

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

    let host_lower = host.to_ascii_lowercase();
    let is_allowed = ALLOWED_YOUTUBE_HOSTS.contains(&host_lower.as_str());

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

#[cfg(feature = "native-e2e")]
pub(crate) fn validate_native_e2e_url(url_str: &str) -> Result<Url, PortError> {
    let parsed = Url::parse(url_str).map_err(|_| PortError::InvalidSource {
        message: "Invalid native E2E URL".to_string(),
    })?;

    if parsed.scheme() != "http"
        || parsed.host_str() != Some("127.0.0.1")
        || parsed.port().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(PortError::InvalidSource {
            message: "Native E2E media must use an isolated loopback HTTP URL".to_string(),
        });
    }

    Ok(parsed)
}
