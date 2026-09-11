use ports::ui_preferences::{ProjectPin, ProjectPins};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPinDto {
    project_id: String,
    pinned: bool,
    revision: u64,
}
impl From<ProjectPin> for ProjectPinDto {
    fn from(pin: ProjectPin) -> Self {
        Self {
            project_id: pin.project_id.to_string(),
            pinned: pin.pinned,
            revision: pin.revision,
        }
    }
}

#[derive(Serialize)]
pub struct ProjectPinsDto {
    entries: Vec<ProjectPinDto>,
    migrated: bool,
}
impl From<ProjectPins> for ProjectPinsDto {
    fn from(pins: ProjectPins) -> Self {
        Self {
            entries: pins.entries.into_iter().map(Into::into).collect(),
            migrated: pins.migrated,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPinDto {
    pub project_id: String,
    pub pinned: bool,
}
