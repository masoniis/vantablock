use bevy::app::{App, Plugin};
use bevy::ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use shared::lifecycle::PersistentPathsResource;
use std::fs;
use tracing::{info, warn};
use utils::PersistentPaths;

#[derive(Debug, Serialize, Deserialize, Resource, Clone)]
pub struct ClientSettings {
    pub texture_pack: String,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            texture_pack: "rhinestone".to_string(),
        }
    }
}

impl ClientSettings {
    const FILE_NAME: &'static str = "client_settings.ron";

    /// Loads the user-specific config from the platform's config directory.
    /// If it does not exist, generates the default config and writes it to disk.
    pub fn load_or_create(paths: &PersistentPaths) -> Self {
        let file_path = paths.config_dir.join(Self::FILE_NAME);

        // try to load user-specific config
        if let Ok(content) = fs::read_to_string(&file_path) {
            match ron::from_str::<Self>(&content) {
                Ok(user_config) => {
                    info!("Loaded user config from {:?}", file_path);
                    return user_config;
                }
                Err(e) => {
                    warn!(
                        "Failed to parse user config at {:?}: {}. Falling back to defaults.",
                        file_path, e
                    );
                }
            }
        }

        // fallback to default
        let default_config = Self::default();
        info!("Using default configuration.");

        // write the default config to the disk
        let pretty = ron::ser::PrettyConfig::default();
        if let Ok(ron_string) = ron::ser::to_string_pretty(&default_config, pretty) {
            if let Err(e) = fs::write(&file_path, ron_string) {
                warn!("Failed to write default config to disk: {}", e);
            } else {
                info!("Generated default config at {:?}", file_path);
            }
        }

        default_config
    }
}

/// A plugin that automatically loads or creates the [`ClientSettings`] resource.
pub struct ClientSettingsPlugin;

impl Plugin for ClientSettingsPlugin {
    fn build(&self, app: &mut App) {
        // if paths isn't already there, this will trigger the resolve (via PathsPlugin being added before this)
        let paths = app
            .world()
            .get_resource::<PersistentPathsResource>()
            .map(|r| r.0.clone())
            .unwrap_or_else(PersistentPaths::resolve);

        app.insert_resource(ClientSettings::load_or_create(&paths));
    }
}
