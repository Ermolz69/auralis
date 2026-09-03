#![allow(clippy::unwrap_used)]

use super::{Project, ProjectTitle};
use crate::error::DomainError;

#[test]
fn title_and_project_creation_reject_blank_titles() {
    for title in ["", "   ", "\t\r\n", "\u{a0}\u{2003}\u{3000}"] {
        let expected = DomainError::ValidationError("Project title cannot be empty".into());
        assert_eq!(ProjectTitle::new(title.into()), Err(expected.clone()));
        assert_eq!(Project::new(title.into()), Err(expected));
    }
}

#[test]
fn title_trims_surrounding_whitespace_and_preserves_content() {
    let title = ProjectTitle::new("\u{a0} Проект  Auralis 🎬 \t\n".into()).unwrap();
    assert_eq!(title.as_str(), "Проект  Auralis 🎬");
    assert_eq!(
        title,
        ProjectTitle::new("Проект  Auralis 🎬".into()).unwrap()
    );
}

#[test]
fn project_normalizes_titles_on_creation_rename_and_restore() {
    let mut project = Project::new("  Original  ".into()).unwrap();
    assert_eq!(project.title(), "Original");
    assert_eq!(project.to_snapshot().title, "Original");

    project.set_title("\t Renamed \n".into()).unwrap();
    assert_eq!(project.title(), "Renamed");
    assert_eq!(project.to_snapshot().title, "Renamed");

    let mut snapshot = project.to_snapshot();
    snapshot.title = "\u{2003} Restored \u{3000}".into();
    let restored = Project::from_snapshot(snapshot).unwrap();
    assert_eq!(restored.title(), "Restored");
    assert_eq!(restored.to_snapshot().title, "Restored");
    assert_eq!(
        Project::from_snapshot(restored.to_snapshot()).unwrap(),
        restored
    );
}

#[test]
fn invalid_rename_leaves_project_unchanged() {
    let mut project = Project::new("Original".into()).unwrap();
    let original = project.to_snapshot();
    for title in ["", "   ", "\t\r\n", "\u{a0}\u{2003}\u{3000}"] {
        assert!(matches!(
            project.set_title(title.into()),
            Err(DomainError::ValidationError(_))
        ));
        assert_eq!(project.to_snapshot(), original);
    }
}
