use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppInfoError {
    #[error("Failed to read applications directory: {0}")]
    DirectoryReadError(#[from] std::io::Error),
    
    #[error("Failed to parse application bundle: {path}")]
    BundleParseError { path: String },
    
    #[error("Failed to read Info.plist: {0}")]
    PlistError(String),
    
    #[error("Registry access error: {0}")]
    RegistryError(String),
    
    #[error("Application not found: {name}")]
    AppNotFound { name: String },
    
    #[error("Unsupported platform")]
    UnsupportedPlatform,
    
    #[error("Failed to get file icon: {0}")]
    FileIconError(#[from] FileIconError),
}

#[derive(Error, Debug)]
pub enum FileIconError {
    #[error("Path does not exist")]
    PathDoesNotExist,
    
    #[error("Icon size cannot be zero")]
    NullIconSize,
    
    #[error("Failed to extract icon")]
    Failed,
    
    #[error("Platform not supported")]
    PlatformNotSupported,
}

pub type Result<T> = std::result::Result<T, AppInfoError>;
