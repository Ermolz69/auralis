use std::sync::{
    LazyLock,
    atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(1);
static SESSION: LazyLock<String> = LazyLock::new(|| {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or_else(
            |error| error.duration().as_nanos(),
            |duration| duration.as_nanos(),
        );
    format!("{timestamp:x}-{:x}", std::process::id())
});

pub(crate) fn next_request_id() -> String {
    format!(
        "{}-{:x}",
        *SESSION,
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

pub(crate) fn request_id(request: &tauri::ipc::Request<'_>) -> String {
    validated_id(
        request
            .headers()
            .get("x-auralis-request-id")
            .and_then(|value| value.to_str().ok()),
    )
    .unwrap_or_else(next_request_id)
}

fn validated_id(value: Option<&str>) -> Option<String> {
    let value = value?;
    if value.len() != 36 {
        return None;
    }
    value
        .bytes()
        .enumerate()
        .all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
        .then(|| value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_uuid_correlation_without_request_content() {
        assert!(validated_id(Some("https://secret.invalid/?token=password")).is_none());
        assert!(validated_id(Some(&"a".repeat(10000))).is_none());
        assert!(validated_id(None).is_none());
        let id = "a1111111-1111-4111-8111-111111111111".to_string();
        assert_eq!(validated_id(Some(&id)), Some(id));
        assert_ne!(next_request_id(), next_request_id());
    }
}
