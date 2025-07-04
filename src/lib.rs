pub mod error;
pub mod macos;
pub mod window;

use error::{AppInfoError, Result};
use std::path::PathBuf;

/// Application information
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// Application name
    pub name: String,
    /// Application version
    pub version: Option<String>,
    /// Application path
    pub path: PathBuf,
    /// Application icon (RGBA format)
    pub icon: Option<Icon>,
    /// Application bundle identifier (macOS) or ProductCode (Windows)
    pub identifier: Option<String>,
    /// Developer/Publisher
    pub publisher: Option<String>,
    /// Installation date
    pub install_date: Option<String>,
}

/// Icon data
#[derive(Debug, Clone)]
pub struct Icon {
    /// Icon width in pixels
    pub width: u32,
    /// Icon height in pixels
    pub height: u32,
    /// Pixel data in RGBA format
    pub pixels: Vec<u8>,
}

/// Gets all installed applications.
///
/// # Arguments
///
/// * `icon_size` - The desired icon size. If 0, no icon will be fetched.
///
/// # Returns
///
/// A vector containing information about all installed applications.
pub fn get_installed_apps(icon_size: u16) -> Result<Vec<AppInfo>> {
    #[cfg(target_os = "macos")]
    return macos::get_installed_apps(icon_size);

    #[cfg(target_os = "windows")]
    return window::get_installed_apps(icon_size);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Err(AppInfoError::UnsupportedPlatform)
}

/// Finds a specific application by its name.
///
/// # Arguments
///
/// * `name` - The name of the application to find.
/// * `icon_size` - The desired icon size. If 0, no icon will be fetched.
///
/// # Returns
///
/// Information about the matched application.
pub fn find_app_by_name(name: &str, icon_size: u16) -> Result<AppInfo> {
    let apps = get_installed_apps(icon_size)?;
    apps.into_iter()
        .find(|app| app.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| AppInfoError::AppNotFound {
            name: name.to_string(),
        })
}

/// Gets the icon for a given file path.
pub fn get_file_icon(path: impl AsRef<std::path::Path>, size: u16) -> Result<Icon> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(AppInfoError::FileIconError(
            error::FileIconError::PathDoesNotExist,
        ));
    }

    if size == 0 {
        return Err(AppInfoError::FileIconError(
            error::FileIconError::NullIconSize,
        ));
    }

    #[cfg(target_os = "macos")]
    return macos::get_file_icon(path, size);

    #[cfg(target_os = "windows")]
    return window::get_file_icon(path, size);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Err(AppInfoError::FileIconError(
        error::FileIconError::PlatformNotSupported,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_installed_apps() {
        // Test without icons
        let apps = get_installed_apps(0).expect("Failed to get installed apps");
        assert!(!apps.is_empty(), "Should find at least one application");
        println!("Applications found: {:?}", apps);

        // Test with icons
        let apps_with_icons =
            get_installed_apps(32).expect("Failed to get installed apps with icons");
        assert!(!apps_with_icons.is_empty());

        // On supported platforms, at least one app should have an icon
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let app_with_icon = apps_with_icons.iter().find(|app| app.icon.is_some());
            assert!(
                app_with_icon.is_some(),
                "At least one app should have an icon"
            );

            if let Some(app) = app_with_icon {
                let icon = app.icon.as_ref().unwrap();
                assert_eq!(icon.width, 32);
                assert_eq!(icon.height, 32);
                assert!(!icon.pixels.is_empty());
                assert_eq!(icon.pixels.len(), (32 * 32 * 4) as usize);
            }
        }
    }

    #[test]
    fn test_find_app_by_name() {
        let apps = get_installed_apps(0).unwrap();
        if apps.is_empty() {
            // Skip this test if no apps are installed
            return;
        }

        // Test with the first app from the list
        let first_app_name = &apps[0].name;
        let found_app = find_app_by_name(first_app_name, 0).unwrap();
        assert_eq!(found_app.name, *first_app_name);

        // Test case-insensitivity
        let found_app_lower = find_app_by_name(&first_app_name.to_lowercase(), 0).unwrap();
        assert_eq!(found_app_lower.name, *first_app_name);

        // Test for an app that doesn't exist
        let not_found_result = find_app_by_name("ThisAppSurelyDoesNotExist12345", 0);
        assert!(matches!(
            not_found_result,
            Err(AppInfoError::AppNotFound { .. })
        ));
    }

    #[test]
    fn test_get_file_icon() {
        // Choose a path that is likely to exist on different platforms
        let path_to_test = if cfg!(target_os = "macos") {
            "/System/Applications/Calculator.app"
        } else if cfg!(target_os = "windows") {
            "C:\\Windows\\System32\\notepad.exe"
        } else {
            // On other OS, we don't have a default path, so skip the rest of the test
            return;
        };

        let path = std::path::Path::new(path_to_test);

        // Test with size 0, should return an error
        let result = get_file_icon(path, 0);
        assert!(matches!(
            result,
            Err(AppInfoError::FileIconError(
                error::FileIconError::NullIconSize
            ))
        ));

        // Test with a non-existent path
        let result = get_file_icon("/path/to/non/existent/file", 32);
        assert!(matches!(
            result,
            Err(AppInfoError::FileIconError(
                error::FileIconError::PathDoesNotExist
            ))
        ));

        // If the test path exists, test getting a real icon
        if path.exists() {
            let icon = get_file_icon(path, 64).expect("Failed to get file icon");
            assert_eq!(icon.width, 64);
            assert_eq!(icon.height, 64);
            assert!(!icon.pixels.is_empty());
            assert_eq!(icon.pixels.len(), (64 * 64 * 4) as usize);
        }
    }
}
