#[test]
fn create_project_command_does_not_manage_project_directories() {
    let source = include_str!("../project.rs");
    let create_command = source
        .split("pub async fn create_project_cmd")
        .nth(1)
        .and_then(|source| source.split("pub async fn rename_project_cmd").next())
        .expect("create project command source");

    assert!(!create_command.contains("AppPaths"));
    assert!(!create_command.contains("create_dir_all"));
}

#[test]
fn open_project_folder_command_delegates_without_accepting_a_path() {
    let source = include_str!("../project.rs");
    let open_folder_command = source
        .split("pub async fn open_project_folder_cmd")
        .nth(1)
        .and_then(|source| {
            source
                .split("pub async fn create_project_from_youtube_cmd")
                .next()
        })
        .expect("open project folder command source");

    assert!(open_folder_command.contains("open_project_folder"));
    assert!(open_folder_command.contains("parse_project_id(&project_id)"));
    assert!(!open_folder_command.contains("AppPaths"));
    assert!(!open_folder_command.contains("create_dir_all"));
    assert!(!open_folder_command.contains("path: String"));
}

#[test]
fn delete_project_command_does_not_delete_project_files_directly() {
    let source = include_str!("../project.rs");
    let delete_command = source
        .split("pub async fn delete_project_cmd")
        .nth(1)
        .and_then(|source| {
            source
                .split("pub async fn start_project_mock_pipeline_cmd")
                .next()
        })
        .expect("delete project command source");

    assert!(!delete_command.contains("AppPaths"));
    assert!(!delete_command.contains("remove_dir"));
    assert!(!delete_command.contains("delete_project_dir"));
}
