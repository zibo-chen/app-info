# app-info

`app-info` is a Rust library for retrieving information about installed applications on macOS and Windows. It provides details such as application name, version, path, icon, and more. The library also supports fetching icons for specific files.

## Features

- Retrieve a list of installed applications.
- Fetch application details such as name, version, path, and icon.
- Get icons for specific files.
- Cross-platform support for macOS and Windows.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
app-info = "0.1"
```

## Usage

### Get Installed Applications

```rust
use app_info::get_installed_apps;

fn main() {
    let apps = get_installed_apps(64).expect("Failed to get installed apps");
    for app in apps {
        println!("App Name: {}", app.name);
        if let Some(icon) = app.icon {
            println!("Icon Size: {}x{}", icon.width, icon.height);
        }
    }
}
```

### Find an Application by Name

```rust
use app_info::find_app_by_name;

fn main() {
    let app_name = "Calculator";
    match find_app_by_name(app_name, 64) {
        Ok(app) => println!("Found app: {}", app.name),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Get File Icon

```rust
use app_info::get_file_icon;

fn main() {
    let path = "/path/to/file";
    match get_file_icon(path, 64) {
        Ok(icon) => println!("Icon Size: {}x{}", icon.width, icon.height),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Example: Save Application Icons

The `examples/save_icon.rs` script demonstrates how to save icons of installed applications to a directory.

```bash
cargo run --example save_icon
```

## Supported Platforms

- **macOS**: Retrieves application information using system APIs.
- **Windows**: Retrieves application information from the registry and system APIs.

## Limitations

- Not supported on Linux or other platforms.
- Some applications may not have icons available.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
