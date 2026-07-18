#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::containment::verify_containment;
use super::error::YtDlpError;
use super::validation::validate_url;
use ports::error::PortError;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_url_validation_success() {
    assert!(validate_url("https://youtube.com/watch?v=123").is_ok());
    assert!(validate_url("https://www.youtube.com/watch?v=123").is_ok());
    assert!(validate_url("https://m.youtube.com/watch?v=123").is_ok());
    assert!(validate_url("https://youtu.be/123").is_ok());
    assert!(validate_url("HTTPS://YOUTUBE.COM/watch?v=123").is_ok());
    assert!(validate_url("https://youtube.com:443/watch?v=123").is_ok());
}

#[test]
fn test_url_validation_invalid_scheme() {
    assert!(validate_url("http://youtube.com/watch?v=123").is_err());
    assert!(validate_url("file:///etc/passwd").is_err());
    assert!(validate_url("ftp://youtube.com/").is_err());
}

#[test]
fn test_url_validation_host_deception() {
    assert!(validate_url("https://youtube.com.attacker.tld/watch?v=123").is_err());
    assert!(validate_url("https://attackeryoutube.com/watch?v=123").is_err());
    assert!(validate_url("https://youtube.com@attacker.tld/").is_err());
}

#[test]
fn test_url_validation_userinfo() {
    assert!(validate_url("https://user:pass@youtube.com/watch?v=123").is_err());
    assert!(validate_url("https://user@youtube.com/watch?v=123").is_err());
}

#[test]
fn test_url_validation_non_standard_port() {
    assert!(validate_url("https://youtube.com:8443/watch?v=123").is_err());
}

#[test]
fn test_containment_normal() {
    let target = tempdir().unwrap();
    let file_path = target.path().join("video.mp4");
    fs::write(&file_path, "content").unwrap();

    assert!(verify_containment(target.path(), &file_path).is_ok());
}

#[test]
fn test_containment_not_a_dir_fails() {
    let target = tempdir().unwrap();
    let not_a_dir = target.path().join("file.txt");
    fs::write(&not_a_dir, "content").unwrap();

    let output_file = target.path().join("video.mp4");
    fs::write(&output_file, "content").unwrap();

    assert!(verify_containment(&not_a_dir, &output_file).is_err());
}

#[test]
fn test_containment_same_path_fails() {
    let target = tempdir().unwrap();
    assert!(verify_containment(target.path(), target.path()).is_err());
}

#[test]
fn test_containment_traversal_fails() {
    let target = tempdir().unwrap();
    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("video.mp4");
    fs::write(&outside_file, "content").unwrap();

    // Check traversal
    assert!(verify_containment(target.path(), &outside_file).is_err());
}

#[cfg(unix)]
#[test]
fn test_containment_symlink_output_fails() {
    let target = tempdir().unwrap();
    let outside_file = target.path().join("../outside.mp4");
    fs::write(&outside_file, "content").unwrap();

    let link_path = target.path().join("link.mp4");
    std::os::unix::fs::symlink(&outside_file, &link_path).unwrap();

    assert!(verify_containment(target.path(), &link_path).is_err());
}

#[cfg(unix)]
#[test]
fn test_containment_intermediate_symlink_fails() {
    let target = tempdir().unwrap();
    let outside_dir = tempdir().unwrap();
    let link_dir = target.path().join("evil_dir");
    std::os::unix::fs::symlink(outside_dir.path(), &link_dir).unwrap();

    let file_path = link_dir.join("video.mp4");
    fs::write(&file_path, "content").unwrap();

    assert!(verify_containment(target.path(), &file_path).is_err());
}

#[test]
fn test_error_sanitization_does_not_leak_secrets() {
    let secret = "marker-secret-12345";

    // Command failed error with secret stderr
    let error1 = YtDlpError::CommandFailed {
        code: Some(1),
        stderr: format!("error with {}", secret),
    };
    let port_err1: PortError = error1.into();
    assert!(!port_err1.to_string().contains(secret));

    // Downloaded file missing error with secret path
    let error2 = YtDlpError::DownloadedFileMissing {
        path: format!("/path/with/{}", secret),
    };
    let port_err2: PortError = error2.into();
    assert!(!port_err2.to_string().contains(secret));

    // Create dir failed error with secret path
    let error3 = YtDlpError::CreateDownloadDirFailed {
        path: format!("/path/with/{}", secret),
        source: std::io::Error::other("io error"),
    };
    let port_err3: PortError = error3.into();
    assert!(!port_err3.to_string().contains(secret));

    // Spawn failed with secret candidate
    let error4 = YtDlpError::SpawnFailed {
        candidate: format!("/executable/{}", secret),
        source: std::io::Error::other("io error"),
    };
    let port_err4: PortError = error4.into();
    assert!(!port_err4.to_string().contains(secret));

    // Unsupported source with secret message
    let error5 = YtDlpError::UnsupportedSource {
        message: format!("unsupported {}", secret),
    };
    let port_err5: PortError = error5.into();
    assert!(!port_err5.to_string().contains(secret));
}
