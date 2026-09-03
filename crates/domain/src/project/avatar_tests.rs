#![allow(clippy::unwrap_used)]

use super::avatar::{MAX_AVATAR_BYTES, ProjectAvatar};
use base64::{Engine, engine::general_purpose::STANDARD};

#[test]
fn accepts_allowlisted_image_headers_and_signatures() {
    for (mime, bytes) in [
        ("png", b"\x89PNG\r\n\x1a\n".as_slice()),
        ("jpeg", b"\xff\xd8\xff"),
        ("gif", b"GIF89a"),
        ("webp", b"RIFF0000WEBP"),
    ] {
        let value = format!("data:image/{mime};base64,{}", STANDARD.encode(bytes));
        assert_eq!(ProjectAvatar::new(value.clone()).unwrap().as_str(), value);
    }
}

#[test]
fn rejects_unsafe_types_invalid_encoding_and_mislabeled_data() {
    for value in [
        "data:image/svg+xml;base64,PHN2Zz4=",
        "data:image/png;base64,not base64",
        "data:image/png;base64,R0lGODlh",
        "data:image/png;base64,",
        "https://example.com/avatar.png",
    ] {
        assert!(ProjectAvatar::new(value.into()).is_err());
    }
}

#[test]
fn enforces_decoded_size_limit() {
    let mut bytes = vec![0; MAX_AVATAR_BYTES];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    let encode = |bytes: &[u8]| format!("data:image/png;base64,{}", STANDARD.encode(bytes));
    assert!(ProjectAvatar::new(encode(&bytes)).is_ok());
    bytes.push(0);
    assert!(ProjectAvatar::new(encode(&bytes)).is_err());
}
