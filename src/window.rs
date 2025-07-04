#[cfg(target_os = "windows")]
use crate::{error::AppInfoError, AppInfo, Icon, Result};
#[cfg(target_os = "windows")]
use scopeguard::defer;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use windows::{
    core::{HSTRING, PWSTR},
    Win32::{
        Foundation::SIZE,
        Graphics::{
            Gdi::DeleteObject,
            Imaging::{
                CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA,
                GUID_WICPixelFormat32bppRGBA, IWICImagingFactory, WICBitmapUseAlpha, WICRect,
            },
        },
        System::{
            Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL},
            Registry::{
                RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE,
                KEY_READ,
            },
        },
        UI::Shell::{
            IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_SCALEUP,
        },
    },
};

/// Gets all installed applications on Windows by scanning the registry.
#[cfg(target_os = "windows")]
pub fn get_installed_apps(icon_size: u16) -> Result<Vec<AppInfo>> {
    let mut apps = Vec::new();

    // Search for installed programs in the registry
    // HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
    let uninstall_key = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    apps.extend(scan_registry_key(uninstall_key, icon_size)?);

    // For 64-bit systems, also search for 32-bit programs
    #[cfg(target_pointer_width = "64")]
    {
        let uninstall_key_wow64 =
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
        apps.extend(scan_registry_key(uninstall_key_wow64, icon_size)?);
    }

    Ok(apps)
}

/// Scans a registry key for application information.
#[cfg(target_os = "windows")]
fn scan_registry_key(key_path: &str, icon_size: u16) -> Result<Vec<AppInfo>> {
    use windows::Win32::System::Registry::HKEY;

    let mut apps = Vec::new();
    let mut hkey: HKEY = HKEY::default();

    // Open the registry key
    let key_name = HSTRING::from(key_path);
    let result = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, &key_name, 0, KEY_READ, &mut hkey) };

    if result.is_err() {
        return Ok(apps);
    }

    defer!(unsafe {
        let _ = RegCloseKey(hkey);
    });

    // Enumerate subkeys
    let mut index = 0u32;
    loop {
        let mut subkey_name = [0u16; 256];
        let mut subkey_name_len = subkey_name.len() as u32;

        let result = unsafe {
            RegEnumKeyExW(
                hkey,
                index,
                PWSTR(subkey_name.as_mut_ptr()),
                &mut subkey_name_len,
                Some(std::ptr::null()),
                PWSTR::null(),
                Some(std::ptr::null_mut()),
                Some(std::ptr::null_mut()),
            )
        };

        if result.is_err() {
            break;
        }

        // Construct the subkey path
        let subkey_path = format!(
            "{}\\{}",
            key_path,
            String::from_utf16_lossy(&subkey_name[..subkey_name_len as usize])
        );

        // Parse application info
        if let Ok(app_info) = parse_registry_app(&subkey_path, icon_size) {
            apps.push(app_info);
        }

        index += 1;
    }

    Ok(apps)
}

/// Parses application information from a specific registry key.
#[cfg(target_os = "windows")]
fn parse_registry_app(key_path: &str, icon_size: u16) -> Result<AppInfo> {
    use windows::Win32::System::Registry::HKEY;

    let mut hkey: HKEY = HKEY::default();
    let key_name = HSTRING::from(key_path);

    let result = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, &key_name, 0, KEY_READ, &mut hkey) };

    if result.is_err() {
        return Err(AppInfoError::RegistryError(
            "Failed to open registry key".to_string(),
        ));
    }

    defer!(unsafe {
        let _ = RegCloseKey(hkey);
    });

    // Read application information
    let display_name = read_registry_string(hkey, "DisplayName")?;
    let version = read_registry_string(hkey, "DisplayVersion").ok();
    let publisher = read_registry_string(hkey, "Publisher").ok();
    let install_location = read_registry_string(hkey, "InstallLocation").ok();
    let install_date = read_registry_string(hkey, "InstallDate").ok();
    let display_icon_path = read_registry_string(hkey, "DisplayIcon").ok();

    // Determine the path for the application and its icon
    let (app_path, icon_path) = if let Some(icon_str) = display_icon_path {
        // DisplayIcon can be "path,index" or just "path"
        let path_part = icon_str.split(',').next().unwrap_or("").trim();
        let path = PathBuf::from(path_part);
        if path.exists() {
            (path.clone(), Some(path))
        } else {
            // DisplayIcon path doesn't exist, fallback to InstallLocation
            install_location
                .as_ref()
                .and_then(|loc| find_main_executable(&PathBuf::from(loc)))
                .map_or((PathBuf::new(), None), |p| (p.clone(), Some(p)))
        }
    } else {
        // No DisplayIcon, search in InstallLocation
        install_location
            .as_ref()
            .and_then(|loc| find_main_executable(&PathBuf::from(loc)))
            .map_or((PathBuf::new(), None), |p| (p.clone(), Some(p)))
    };

    // Get the icon
    let icon = if icon_size > 0 {
        icon_path.and_then(|path| get_file_icon(&path, icon_size).ok())
    } else {
        None
    };

    Ok(AppInfo {
        name: display_name,
        version,
        path: app_path,
        icon,
        identifier: None, // Windows typically uses a ProductCode, simplified here
        publisher,
        install_date,
    })
}

/// Reads a string value from the registry.
#[cfg(target_os = "windows")]
fn read_registry_string(
    hkey: windows::Win32::System::Registry::HKEY,
    value_name: &str,
) -> Result<String> {
    use windows::Win32::System::Registry::REG_VALUE_TYPE;

    let value_name = HSTRING::from(value_name);
    let mut data_type: REG_VALUE_TYPE = REG_VALUE_TYPE(0);
    let mut data_size = 0u32;

    // First, get the size of the data
    let result = unsafe {
        RegQueryValueExW(
            hkey,
            &value_name,
            None,
            Some(&mut data_type),
            None,
            Some(&mut data_size),
        )
    };

    if result.is_err() || data_size == 0 {
        return Err(AppInfoError::RegistryError(
            "Failed to read registry value".to_string(),
        ));
    }

    // Allocate a buffer and read the data
    let mut buffer = vec![0u8; data_size as usize];
    let result = unsafe {
        RegQueryValueExW(
            hkey,
            &value_name,
            None,
            Some(&mut data_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut data_size),
        )
    };

    if result.is_err() {
        return Err(AppInfoError::RegistryError(
            "Failed to read registry value".to_string(),
        ));
    }

    // Convert to string
    let wide_chars =
        unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u16, buffer.len() / 2) };

    // Remove trailing null characters
    let end = wide_chars
        .iter()
        .position(|&x| x == 0)
        .unwrap_or(wide_chars.len());
    Ok(String::from_utf16_lossy(&wide_chars[..end]))
}

/// Finds the main executable file in an installation directory.
#[cfg(target_os = "windows")]
fn find_main_executable(install_dir: &Path) -> Option<PathBuf> {
    use std::fs;

    if !install_dir.exists() {
        return None;
    }

    // Look for .exe files, but avoid uninstaller files
    if let Ok(entries) = fs::read_dir(install_dir) {
        let mut exe_files: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("exe") {
                exe_files.push(path);
            }
        }

        // Filter out likely uninstaller executables
        let filtered_exes: Vec<PathBuf> = exe_files
            .into_iter()
            .filter(|path| {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let name_lower = name.to_lowercase();
                    // Skip common uninstaller patterns
                    !name_lower.contains("unins")
                        && !name_lower.contains("uninst")
                        && !name_lower.contains("uninstall")
                        && !name_lower.contains("remove")
                        && !name_lower.starts_with("un")
                } else {
                    true
                }
            })
            .collect();

        // Return the first non-uninstaller executable, or first one if none found
        filtered_exes.into_iter().next().or_else(|| {
            // If all were filtered out, try to get any exe file as fallback
            if let Ok(entries) = fs::read_dir(install_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("exe") {
                        return Some(path);
                    }
                }
            }
            None
        })
    } else {
        None
    }
}

/// Gets the icon for a given file path on Windows.
#[cfg(target_os = "windows")]
pub fn get_file_icon(path: &Path, size: u16) -> Result<Icon> {
    // Helper struct to ensure CoUninitialize is called.
    struct InitializationToken;

    impl Drop for InitializationToken {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }

    // Initialize COM
    let _token = if unsafe { CoInitialize(None) }.is_ok() {
        Some(InitializationToken)
    } else {
        None
    };

    // Create a Shell item
    let path_string = HSTRING::from(path.to_string_lossy().as_ref());
    let image_factory: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(&path_string, None) }
            .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

    // Set the icon size
    let bitmap_size = SIZE {
        cx: size as i32,
        cy: size as i32,
    };

    // Get the icon bitmap
    let bitmap = unsafe { image_factory.GetImage(bitmap_size, SIIGBF_ICONONLY | SIIGBF_SCALEUP) }
        .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

    // Ensure the bitmap is deleted when the function ends
    defer!(unsafe {
        let _ = DeleteObject(bitmap);
    });

    // Create a WIC factory
    let imaging_factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL) }
            .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

    // Create a WIC bitmap from the HBITMAP
    let wic_bitmap =
        unsafe { imaging_factory.CreateBitmapFromHBITMAP(bitmap, None, WICBitmapUseAlpha) }
            .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

    // Define the source rectangle
    let source_rectangle = WICRect {
        X: 0,
        Y: 0,
        Width: size as i32,
        Height: size as i32,
    };

    // Get and process pixel data
    let pixel_format = unsafe { wic_bitmap.GetPixelFormat() }
        .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

    #[allow(non_upper_case_globals)]
    let pixels = match pixel_format {
        GUID_WICPixelFormat32bppBGRA | GUID_WICPixelFormat32bppRGBA => {
            let mut pixels = vec![0u8; size as usize * size as usize * 4];
            unsafe { wic_bitmap.CopyPixels(&source_rectangle, size as u32 * 4, &mut pixels) }
                .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;

            // If the format is BGRA, convert it to RGBA
            if pixel_format == GUID_WICPixelFormat32bppBGRA {
                for chunk in pixels.chunks_exact_mut(4) {
                    chunk.swap(0, 2);
                }
            }
            pixels
        }
        _ => {
            return Err(AppInfoError::FileIconError(
                crate::error::FileIconError::Failed,
            ))
        }
    };

    Ok(Icon {
        width: size as u32,
        height: size as u32,
        pixels,
    })
}
