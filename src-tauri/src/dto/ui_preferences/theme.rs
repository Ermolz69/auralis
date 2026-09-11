use ports::ui_preferences::StoredTheme;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredThemeDto {
    value: String,
    revision: u64,
}
impl From<StoredTheme> for StoredThemeDto {
    fn from(theme: StoredTheme) -> Self {
        Self {
            value: theme.value,
            revision: theme.revision,
        }
    }
}
