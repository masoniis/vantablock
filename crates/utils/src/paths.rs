use directories::ProjectDirs;
use std::{fs, path::PathBuf};

/// OS-standard locations for persistent application data.
///
/// Uses the `directories` crate to resolve standard platform config and data directories:
/// - macOS: `~/Library/Application Support/com.masoniis.vantablock/`
/// - Windows: `%AppData%\Roaming\masoniis\vantablock\` (Config) and `%AppData%\Local\masoniis\vantablock\` (Data)
/// - Linux: `~/.config/vantablock/` and `~/.local/share/vantablock/`
#[derive(Clone, Debug)]
pub struct PersistentPaths {
    /// Standard location for game assets (textures, models, configs).
    pub assets_dir: PathBuf,
    /// Standard location for the configuration file.
    pub config_dir: PathBuf,
    /// Standard location for large data files (world saves).
    /// Uses local data directory to avoid network roaming issues on Windows.
    pub saves_dir: PathBuf,
    /// Standard location for transient data (mesh caches, processed textures).
    pub cache_dir: PathBuf,
    /// Standard location for logs and session state.
    pub logs_dir: PathBuf,
}

impl PersistentPaths {
    /// Resolves and creates the standard project directories for the current platform.
    ///
    /// This function will attempt to create all necessary subdirectories on disk.
    /// It panics if the OS fails to provide a standard directory structure.
    pub fn resolve_client() -> Self {
        // if we are in a cargo workspace, resolve paths for dev env using "client" target
        if let Some(dev_paths) = Self::try_resolve_dev("client") {
            return dev_paths;
        }

        // otherwise assume a production environment
        Self::resolve_client_paths()
    }

    /// Resolves folders for a dedicated server environment.
    /// Places all data inside the current working directory (portable mode).
    pub fn resolve_server() -> Self {
        // if we are in a cargo workspace, resolve paths for dev env using "server" target
        if let Some(dev_paths) = Self::try_resolve_dev("server") {
            return dev_paths;
        }

        // use the dir that the executable is run from
        let root = std::env::current_dir().expect("Failed to get current working directory");

        let paths = Self {
            assets_dir: root.join("assets"),
            config_dir: root.join("config"),
            saves_dir: root.join("saves"),
            cache_dir: root.join("cache"),
            logs_dir: root.join("logs"),
        };

        paths.ensure_exists();
        paths
    }

    // INFO: ---------------------------
    //         private utilities
    // ---------------------------------

    /// Checks if we are running via Cargo or in debug mode, and returns the constructed dev paths if so.
    fn try_resolve_dev(target: &str) -> Option<Self> {
        let is_dev_env = std::env::var("CARGO").is_ok() || cfg!(debug_assertions);

        if is_dev_env {
            let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            // crate root is two dirs up from utils crate
            if let Some(workspace_root) = crate_root.parent().and_then(|p| p.parent()) {
                // isolate dev data into ./vantadev/client or ./vantadev/server
                let dev_data = workspace_root.join("vantadev").join(target);

                let paths = Self {
                    assets_dir: workspace_root.join("assets"), // assets dir is same for both client/server
                    config_dir: dev_data.join("config"),
                    saves_dir: dev_data.join("saves"),
                    cache_dir: dev_data.join("cache"),
                    logs_dir: dev_data.join("logs"),
                };

                paths.ensure_exists();
                return Some(paths);
            }
        }
        None
    }

    /// Resolves folder for a prod environment based on the host OS conventions
    fn resolve_client_paths() -> Self {
        let exe_path = std::env::current_exe().expect("Failed to get executable path");
        let exe_dir = exe_path
            .parent()
            .expect("Executable has no parent directory");

        let assets_dir = if cfg!(target_os = "macos") && exe_dir.ends_with("MacOS") {
            exe_dir.parent().unwrap().join("Resources").join("assets")
        } else {
            exe_dir.join("assets")
        };

        let (config_dir, saves_dir, cache_dir, logs_dir) = Self::get_os_dirs();

        let paths = Self {
            assets_dir,
            config_dir,
            saves_dir,
            cache_dir,
            logs_dir,
        };
        paths.ensure_exists();
        paths
    }

    /// Resolves the standardized OS directories for config/data/caching
    fn get_os_dirs() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let proj_dirs = ProjectDirs::from("com", "masoniis", "vantablock")
            .expect("The OS failed to provide standard directories. Vantablock can't save any data without this functionality!");

        (
            proj_dirs.config_dir().to_path_buf(),
            proj_dirs.data_local_dir().join("saves"),
            proj_dirs.cache_dir().to_path_buf(),
            proj_dirs
                .state_dir()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| proj_dirs.data_dir().join("logs")),
        )
    }

    fn ensure_exists(&self) {
        fs::create_dir_all(&self.config_dir).ok();
        fs::create_dir_all(&self.saves_dir).ok();
        fs::create_dir_all(&self.cache_dir).ok();
        fs::create_dir_all(&self.logs_dir).ok();
    }
}
