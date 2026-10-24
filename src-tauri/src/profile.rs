use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProfileDirectory(pub Option<PathBuf>);

impl ProfileDirectory {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, &'static str> {
        let mut args = args.into_iter();
        let mut root = None;
        while let Some(arg) = args.next() {
            if arg == "--profile-dir" {
                if root.is_some() {
                    return Err("--profile-dir must occur once");
                }
                let path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("--profile-dir requires an absolute directory")?;
                if !path.is_absolute()
                    || path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                {
                    return Err(
                        "--profile-dir requires an absolute directory without parent traversal",
                    );
                }
                root = Some(path);
            } else if arg.to_string_lossy().starts_with("--profile-dir=") {
                return Err("use --profile-dir followed by an absolute directory");
            }
        }
        if cfg!(feature = "native-e2e") && root.is_some() {
            return Err("--profile-dir cannot be combined with the native-e2e build");
        }
        Ok(Self(root))
    }

    pub fn configure(
        &self,
        context: &mut tauri::Context<tauri::Wry>,
    ) -> Vec<tauri::utils::config::WindowConfig> {
        if self.0.is_none() {
            return Vec::new();
        }
        let windows = context
            .config()
            .app
            .windows
            .iter()
            .filter(|window| window.create)
            .cloned()
            .collect();
        for window in &mut context.config_mut().app.windows {
            window.create = false;
        }
        windows
    }

    pub fn create_windows(
        &self,
        app: &tauri::App,
        windows: &[tauri::utils::config::WindowConfig],
    ) -> tauri::Result<()> {
        if let Some(root) = &self.0 {
            for config in windows {
                tauri::WebviewWindowBuilder::from_config(app, config)?
                    .data_directory(root.join("webview2").join(&config.label))
                    .build()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<ProfileDirectory, &'static str> {
        ProfileDirectory::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn default_keeps_the_existing_profile() {
        assert_eq!(parse(&[]), Ok(ProfileDirectory::default()));
    }

    #[test]
    fn rejects_missing_relative_duplicate_and_traversal_arguments() {
        assert!(parse(&["--profile-dir"]).is_err());
        assert!(parse(&["--profile-dir", "relative"]).is_err());
        assert!(parse(&["--profile-dir=relative"]).is_err());
        let root = std::env::temp_dir();
        let root = root.to_string_lossy();
        assert!(parse(&["--profile-dir", &root, "--profile-dir", &root]).is_err());
        let traversal = std::env::temp_dir().join("..").join("escape");
        assert!(parse(&["--profile-dir", &traversal.to_string_lossy()]).is_err());
    }

    #[test]
    fn selects_an_absolute_root_without_creating_it() {
        let root = std::env::temp_dir().join("auralis-profile-argument-only");
        let selected = parse(&["--profile-dir", &root.to_string_lossy()]);
        if cfg!(feature = "native-e2e") {
            assert!(selected.is_err());
        } else {
            assert_eq!(selected, Ok(ProfileDirectory(Some(root))));
        }
    }
}
