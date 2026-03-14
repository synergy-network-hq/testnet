use std::env;
use std::path::{Path, PathBuf};

/// Gets the project root directory by looking for Cargo.toml
/// or by using the binary's location to infer the project root
pub fn get_project_root() -> Option<PathBuf> {
    // First, try to find Cargo.toml in current directory or parents
    let mut current = env::current_dir().ok()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Some(current);
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback: try to infer from binary location
    if let Ok(exe_path) = env::current_exe() {
        // If binary is in target/release/ or target/debug/, go up 2 levels
        if let Some(parent) = exe_path.parent() {
            if parent.ends_with("release") || parent.ends_with("debug") {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.ends_with("target") {
                        if let Some(project_root) = grandparent.parent() {
                            let cargo_toml = project_root.join("Cargo.toml");
                            if cargo_toml.exists() {
                                return Some(project_root.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Resolves a path relative to the project root, or returns absolute path as-is
pub fn resolve_data_path(relative_path: &str) -> PathBuf {
    // If it's already absolute, use it as-is
    if Path::new(relative_path).is_absolute() {
        return PathBuf::from(relative_path);
    }

    // Try to get project root
    if let Some(project_root) = get_project_root() {
        project_root.join(relative_path)
    } else {
        // Fallback to current directory (original behavior)
        PathBuf::from(relative_path)
    }
}

/// Validates that we're running from the correct project root
pub fn validate_project_root() -> Result<PathBuf, String> {
    if let Some(project_root) = get_project_root() {
        // Check for required directories
        let config_dir = project_root.join("config");
        let data_dir = project_root.join("data");

        if !config_dir.exists() {
            return Err(format!(
                "Invalid project root: config/ directory not found in {}",
                project_root.display()
            ));
        }

        Ok(project_root)
    } else {
        Err(
            "Could not determine project root. Please run from the synergy-testbeta directory."
                .to_string(),
        )
    }
}
