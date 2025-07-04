use app_info::{get_installed_apps, AppInfo, Icon};
use image::{ImageBuffer, RgbaImage};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching installed applications...");

    // Get all installed applications, icon size is 64x64
    let apps = get_installed_apps(64)?;

    println!("Found {} applications", apps.len());

    // Create output directory
    let output_dir = "app_icons";
    if !Path::new(output_dir).exists() {
        fs::create_dir(output_dir)?;
        println!("Created output directory: {}", output_dir);
    }

    let mut saved_count = 0;

    for app in apps {
        if let Some(ref icon) = app.icon {
            match save_icon_as_png(&app, &icon, output_dir) {
                Ok(filename) => {
                    saved_count += 1;
                    println!("Saved: {}", filename);
                }
                Err(e) => {
                    eprintln!("Error saving icon for {}: {}", app.name, e);
                }
            }
        } else {
            println!("Skipped {} (no icon)", app.name);
        }
    }

    println!(
        "Done! Saved {} icons to the {} directory",
        saved_count, output_dir
    );
    Ok(())
}

fn save_icon_as_png(
    app: &AppInfo,
    icon: &Icon,
    output_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create an image buffer
    let img: RgbaImage = ImageBuffer::from_raw(icon.width, icon.height, icon.pixels.clone())
        .ok_or("Failed to create image buffer")?;

    // Sanitize application name by removing invalid filename characters
    let safe_name = sanitize_filename(&app.name);
    let filename = format!("{}/{}.png", output_dir, safe_name);

    // Save as a PNG file
    img.save(&filename)?;

    Ok(filename)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            // Replace invalid filename characters
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
