use super::LocalTempWorkspace;
use domain::outbox::WorkspaceKey;
use filetime::{FileTime, set_file_mtime};
use ports::TempWorkspacePort;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn resolve_normal_tmp_key() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let key = WorkspaceKey::new("tmp/project-1/file.txt".to_string()).unwrap();
    let resolved = workspace.resolve_key(&key).await.unwrap();

    let canonical_workspace = tokio::fs::canonicalize(workspace_dir.path())
        .await
        .unwrap_or_else(|_| workspace_dir.path().to_path_buf());
    assert!(resolved.starts_with(&canonical_workspace));
    assert_eq!(
        resolved.to_string_lossy().replace('\\', "/"),
        canonical_workspace
            .join("tmp/project-1/file.txt")
            .to_string_lossy()
            .replace('\\', "/")
    );
}

#[tokio::test]
async fn resolve_key_missing_tmp_prefix_fails() {
    let workspace_dir = tempdir().unwrap();
    let _workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let key = WorkspaceKey::new("staging/project-1/file.txt".to_string()).unwrap();
    let result = _workspace.resolve_key(&key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn resolve_non_existent_target_under_safe_parent() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let project_dir = workspace_dir.path().join("tmp").join("project-1");
    fs::create_dir_all(&project_dir).unwrap();

    let key = WorkspaceKey::new("tmp/project-1/missing.txt".to_string()).unwrap();
    let resolved = workspace.resolve_key(&key).await;
    assert!(resolved.is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_terminal_symlink_escape() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();

    let project_dir = workspace_dir.path().join("tmp").join("project-1");
    fs::create_dir_all(&project_dir).unwrap();

    // Create symlink pointing outside
    std::os::unix::fs::symlink(&outside_file, project_dir.join("link.txt")).unwrap();

    let key = WorkspaceKey::new("tmp/project-1/link.txt".to_string()).unwrap();
    let result = workspace.resolve_key(&key).await;

    assert!(result.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_intermediate_parent_symlink_escape() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let outside_dir = tempdir().unwrap();

    let tmp_dir = workspace_dir.path().join("tmp");
    fs::create_dir_all(&tmp_dir).unwrap();

    // tmp/evil_dir -> outside_dir
    std::os::unix::fs::symlink(outside_dir.path(), tmp_dir.join("evil_dir")).unwrap();

    let key = WorkspaceKey::new("tmp/evil_dir/file.txt".to_string()).unwrap();
    let result = workspace.resolve_key(&key).await;

    assert!(result.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_symlink_into_root_is_safe() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    let tmp_dir = workspace_dir.path().join("tmp");
    fs::create_dir_all(&tmp_dir).unwrap();

    let safe_target = tmp_dir.join("safe.txt");
    fs::write(&safe_target, "safe").unwrap();

    // tmp/link -> tmp/safe.txt
    std::os::unix::fs::symlink(&safe_target, tmp_dir.join("link.txt")).unwrap();

    let key = WorkspaceKey::new("tmp/link.txt".to_string()).unwrap();
    let result = workspace.resolve_key(&key).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_workspace_janitor() {
    let workspace_dir = tempdir().unwrap();
    let workspace = LocalTempWorkspace::new(workspace_dir.path().to_path_buf());

    // Create old allocation
    let old_project_id = Uuid::new_v4();
    let old_project_dir = workspace_dir
        .path()
        .join("tmp")
        .join(old_project_id.to_string());
    let old_dir = old_project_dir.join("purpose_1");
    fs::create_dir_all(&old_dir).unwrap();
    let old_file = old_dir.join("old.txt");
    fs::write(&old_file, "old").unwrap();

    // Create fresh allocation
    let fresh_project_id = Uuid::new_v4();
    let fresh_project_dir = workspace_dir
        .path()
        .join("tmp")
        .join(fresh_project_id.to_string());
    let fresh_dir = fresh_project_dir.join("purpose_2");
    fs::create_dir_all(&fresh_dir).unwrap();
    let fresh_file = fresh_dir.join("fresh.txt");
    fs::write(&fresh_file, "fresh").unwrap();

    // Modify old allocation time to be older than threshold
    let two_hours_ago = FileTime::from_unix_time(chrono::Utc::now().timestamp() - 7200, 0);
    set_file_mtime(&old_file, two_hours_ago).unwrap();
    set_file_mtime(&old_dir, two_hours_ago).unwrap();
    set_file_mtime(&old_project_dir, two_hours_ago).unwrap();

    #[cfg(unix)]
    {
        // Add symlink escape (which should be skipped or untraversed)
        let outside_dir = tempdir().unwrap();
        let outside_file = outside_dir.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();
        std::os::unix::fs::symlink(&outside_file, old_dir.join("link.txt")).unwrap();
        // Since the parent (old_dir) is old, janitor will remove_dir_all(old_dir),
        // which removes the symlink itself, but NOT the external target.
    }

    // Also create empty project directory
    let empty_project_id = Uuid::new_v4();
    let empty_dir = workspace_dir
        .path()
        .join("tmp")
        .join(empty_project_id.to_string());
    fs::create_dir_all(&empty_dir).unwrap();
    set_file_mtime(&empty_dir, two_hours_ago).unwrap();

    // Run Janitor
    let report = workspace
        .cleanup_stale_allocations(std::time::Duration::from_secs(3600))
        .await
        .unwrap();

    assert!(report.failed_count == 0, "Janitor failed: {:?}", report); // Should succeed
    assert!(!old_dir.exists());
    assert!(!empty_dir.exists());
    assert!(fresh_dir.exists());
    assert!(fresh_file.exists());
}
