use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct WorkspaceKey(String);

impl WorkspaceKey {
    pub fn new(key: impl Into<String>) -> Result<Self, String> {
        let key = key.into();
        if key.is_empty() {
            return Err("WorkspaceKey cannot be empty".to_string());
        }
        if key.contains('\\') {
            return Err("WorkspaceKey must use '/' as separator, '\\' is not allowed".to_string());
        }
        if key.starts_with('/') {
            return Err("WorkspaceKey cannot be absolute (must not start with '/')".to_string());
        }
        if key.contains('\0') {
            return Err("WorkspaceKey cannot contain NUL byte".to_string());
        }

        // Check for Windows drive letters like "C:/"
        if key.len() >= 2
            && key.chars().nth(1) == Some(':')
            && key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        {
            return Err("WorkspaceKey cannot contain Windows drive letters".to_string());
        }

        // Check components
        for comp in key.split('/') {
            if comp == "." || comp == ".." {
                return Err(format!("WorkspaceKey cannot contain '{}' components", comp));
            }
            if comp.is_empty() {
                return Err("WorkspaceKey cannot contain empty components (e.g. '//')".to_string());
            }
            // check for invalid filename characters (windows reserved)
            if comp
                .chars()
                .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
            {
                return Err(format!(
                    "WorkspaceKey component '{}' contains invalid characters",
                    comp
                ));
            }
        }

        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Custom deserialize ensures valid state from DB
impl<'de> Deserialize<'de> for WorkspaceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        WorkspaceKey::new(s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for WorkspaceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
