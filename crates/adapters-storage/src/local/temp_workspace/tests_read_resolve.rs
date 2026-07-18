#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use tempfile::tempdir;

use super::port::LocalTempWorkspace;
use ports::workspace::TempWorkspacePort;

#[tokio::test]
async fn read_workspace_file_normal() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, "WEBVTT\n\nHello World!").unwrap();

    let content = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 1024)
        .await
        .unwrap();

    assert_eq!(content, "WEBVTT\n\nHello World!");
}

#[tokio::test]
async fn read_workspace_file_multibyte_utf8() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, "WEBVTT\n\nПривет, мир! 🇺🇦").unwrap();

    let content = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 1024)
        .await
        .unwrap();

    assert_eq!(content, "WEBVTT\n\nПривет, мир! 🇺🇦");
}

#[tokio::test]
async fn read_workspace_file_missing_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "missing.vtt", 1024)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_oversize_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, "WEBVTT\n\nHello World!").unwrap();

    // Check size limit failure (file size is 19 bytes, limit is 5 bytes)
    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 5)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_size_boundary() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, "12345").unwrap();

    // Limit exactly equal to size
    let content = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 5)
        .await
        .unwrap();
    assert_eq!(content, "12345");

    // Limit exactly equal to size - 1
    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 4)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_u64_max_does_not_overflow() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, "12345").unwrap();

    // Limit is u64::MAX
    let content = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", u64::MAX)
        .await
        .unwrap();
    assert_eq!(content, "12345");
}

#[tokio::test]
async fn read_workspace_file_invalid_utf8_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let file_path = alloc.absolute_path.join("subs.vtt");
    fs::write(&file_path, vec![0, 159, 146, 150]).unwrap(); // Invalid UTF-8 bytes

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "subs.vtt", 1024)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_directory_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let dir_path = alloc.absolute_path.join("nested_dir");
    fs::create_dir(&dir_path).unwrap();

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "nested_dir", 1024)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_traversal_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    // Check traversal via relative component
    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "../outside.txt", 1024)
        .await;
    assert!(result.is_err());

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "./../outside.txt", 1024)
        .await;
    assert!(result.is_err());

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "/", 1024)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_workspace_file_sibling_escape_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc1 = workspace
        .create_allocation(&project_id, "sub1")
        .await
        .unwrap();
    let alloc2 = workspace
        .create_allocation(&project_id, "sub2")
        .await
        .unwrap();

    let file_path2 = alloc2.absolute_path.join("file.txt");
    fs::write(&file_path2, "hello").unwrap();

    // Get folder name of alloc2
    let folder2 = alloc2.absolute_path.file_name().unwrap().to_string_lossy();
    let relative_sibling_path = format!("../{}/file.txt", folder2);

    let result = workspace
        .read_workspace_file_to_string(&alloc1.workspace_key, &relative_sibling_path, 1024)
        .await;

    assert!(result.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn read_workspace_file_symlink_fails() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_id = domain::project::ProjectId::new();
    let alloc = workspace
        .create_allocation(&project_id, "subtitles")
        .await
        .unwrap();

    let outside_file = workspace_dir.path().join("outside.txt");
    fs::write(&outside_file, "secret").unwrap();

    let link_path = alloc.absolute_path.join("link.txt");
    std::os::unix::fs::symlink(&outside_file, &link_path).unwrap();

    let result = workspace
        .read_workspace_file_to_string(&alloc.workspace_key, "link.txt", 1024)
        .await;

    assert!(result.is_err());
}
