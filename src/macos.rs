#[cfg(target_os = "macos")]
use crate::{error::AppInfoError, AppInfo, Icon, Result};
#[cfg(target_os = "macos")]
use objc2::{
    class,
    msg_send_id,
    rc::{Allocated, Id},
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSBitmapImageRep, NSCompositingOperation, NSGraphicsContext, NSImage, NSWorkspace,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{CGFloat, CGPoint, CGRect, CGSize, NSString};
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

/// Gets all installed applications on macOS by scanning standard application directories.
#[cfg(target_os = "macos")]
pub fn get_installed_apps(icon_size: u16) -> Result<Vec<AppInfo>> {
    let mut apps = Vec::new();

    // Search the /Applications directory
    let applications_dir = Path::new("/Applications");
    if applications_dir.exists() {
        apps.extend(scan_directory(applications_dir, icon_size)?);
    }

    // Search the /System/Applications directory
    let system_apps_dir = Path::new("/System/Applications");
    if system_apps_dir.exists() {
        apps.extend(scan_directory(system_apps_dir, icon_size)?);
    }

    // Search the user's Applications directory
    if let Some(home_dir) = std::env::var_os("HOME") {
        let user_apps = PathBuf::from(home_dir).join("Applications");
        if user_apps.exists() {
            apps.extend(scan_directory(&user_apps, icon_size)?);
        }
    }

    Ok(apps)
}

/// Scans a directory for .app bundles and parses them.
#[cfg(target_os = "macos")]
fn scan_directory(dir: &Path, icon_size: u16) -> Result<Vec<AppInfo>> {
    let mut apps = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("app") {
            if let Ok(app_info) = parse_app_bundle(&path, icon_size) {
                apps.push(app_info);
            }
        }
    }

    Ok(apps)
}

/// Parses an application bundle (.app) to extract its information.
#[cfg(target_os = "macos")]
fn parse_app_bundle(app_path: &Path, icon_size: u16) -> Result<AppInfo> {
    let info_plist_path = app_path.join("Contents/Info.plist");

    if !info_plist_path.exists() {
        return Err(AppInfoError::BundleParseError {
            path: app_path.display().to_string(),
        });
    }

    // Read Info.plist
    let plist_data = fs::read(&info_plist_path)?;
    let plist: plist::Value =
        plist::from_bytes(&plist_data).map_err(|e| AppInfoError::PlistError(e.to_string()))?;

    let dict = plist
        .as_dictionary()
        .ok_or_else(|| AppInfoError::PlistError("Invalid plist format".to_string()))?;

    // Extract application information
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            app_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let identifier = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    // Get the icon
    let icon = if icon_size > 0 {
        get_file_icon(app_path, icon_size).ok()
    } else {
        None
    };

    Ok(AppInfo {
        name,
        version,
        path: app_path.to_path_buf(),
        icon,
        identifier,
        publisher: None,    // Publisher info is not typically stored in Info.plist on macOS
        install_date: None, // Can be obtained from the file system, but requires extra implementation
    })
}

/// Gets the icon for a given file path on macOS.
#[cfg(target_os = "macos")]
pub fn get_file_icon(path: &Path, size: u16) -> Result<Icon> {
    let canonical_path = path
        .canonicalize()
        .map_err(|_| AppInfoError::FileIconError(crate::error::FileIconError::PathDoesNotExist))?;

    let file_path = NSString::from_str(
        canonical_path
            .to_str()
            .ok_or_else(|| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?,
    );

    unsafe {
        // Get the shared workspace and the file icon
        let shared_workspace = NSWorkspace::sharedWorkspace();
        let image: Id<NSImage> = shared_workspace.iconForFile(&file_path);

        // Set the target size
        let desired_size = CGSize {
            width: size as CGFloat,
            height: size as CGFloat,
        };

        // Create a bitmap representation
        let bitmap_representation: Id<NSBitmapImageRep> = {
            let allocated: Allocated<NSBitmapImageRep> = msg_send_id![class!(NSBitmapImageRep), alloc];
            let rep: Id<NSBitmapImageRep> = msg_send_id![
                allocated,
                initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>(),
                pixelsWide: size as isize,
                pixelsHigh: size as isize,
                bitsPerSample: 8 as isize,
                samplesPerPixel: 4 as isize,
                hasAlpha: true,
                isPlanar: false,
                colorSpaceName: &*NSString::from_str("NSDeviceRGBColorSpace"),
                bytesPerRow: size as isize * 4,
                bitsPerPixel: 32 as isize
            ];
            rep
        };

        // Set up the graphics context
        let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_representation)
            .ok_or_else(|| AppInfoError::FileIconError(crate::error::FileIconError::Failed))?;
        context.saveGraphicsState();
        NSGraphicsContext::setCurrentContext(Some(&context));

        // Draw the icon
        image.setSize(desired_size);
        image.drawAtPoint_fromRect_operation_fraction(
            CGPoint::ZERO,
            CGRect::new(CGPoint::ZERO, desired_size),
            NSCompositingOperation::Copy,
            1.0,
        );

        // Finalize drawing
        context.flushGraphics();
        context.restoreGraphicsState();

        // Get the pixel data
        let pixels = std::slice::from_raw_parts(
            bitmap_representation.bitmapData(),
            bitmap_representation.bytesPerPlane() as usize,
        )
        .to_vec();

        Ok(Icon {
            width: size as u32,
            height: size as u32,
            pixels,
        })
    }
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn get_installed_apps(_icon_size: u16) -> Result<Vec<AppInfo>> {
    Err(AppInfoError::UnsupportedPlatform)
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn get_file_icon(_path: &std::path::Path, _size: u16) -> Result<Icon> {
    Err(AppInfoError::UnsupportedPlatform)
}
