use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// Returns the platform-specific path where user configuration should be stored.
///
/// Uses the `directories` crate to resolve standard platform config directories:
/// - macOS: `~/Library/Application Support/com.masoniis.vantablock/config.ron`
/// - Windows: `%AppData%\masoniis\vantablock\config\config.ron`
/// - Linux: `~/.config/vantablock/config.ron`
pub fn get_user_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "masoniis", "vantablock")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.ron"))
}

/// Attempts to resolve a path to a resource (file or directory) by checking multiple locations.
///
/// 1. Checks relative to the Current Working Directory (CWD).
/// 2. Checks relative to the executable's directory (Windows/Linux bundles).
/// 3. Checks the macOS `.app` bundle `Resources` directory.
///
/// This ensures assets are found whether running via `cargo run` or as a distributed bundle.
pub fn get_resource_path(relative_path: impl AsRef<Path>) -> PathBuf {
    let relative_path = relative_path.as_ref();

    // try relative to CWD (usually works in development)
    if relative_path.exists() {
        return relative_path.to_path_buf();
    }

    // try relative to executable (useful for distributed bundles)
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        // check in same directory as exe (Windows, Linux, some macOS setups)
        let same_dir_path = exe_dir.join(relative_path);
        if same_dir_path.exists() {
            return same_dir_path;
        }

        // check in macOS .app Bundle Resources (../Resources/)
        let macos_resource_path = exe_dir
            .parent()
            .map(|p| p.join("Resources"))
            .map(|p| p.join(relative_path));
        if let Some(path) = macos_resource_path
            && path.exists()
        {
            return path;
        }
    }

    // Fallback to original path
    relative_path.to_path_buf()
}
