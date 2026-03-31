use crate::prelude::{get_resource_path, get_user_config_path};
use bevy::ecs::prelude::Resource;
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Resource, Clone)]
pub struct AppConfig {
    pub texture_pack: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            texture_pack: "rhinestone".to_string(),
        }
    }
}

pub fn load_config() -> AppConfig {
    let mut config = AppConfig::default();

    // try to load bundled "factory" config from assets
    let bundled_path = get_resource_path("assets/config.ron");
    if let Ok(content) = std::fs::read_to_string(&bundled_path) {
        match ron::from_str::<AppConfig>(&content) {
            Ok(bundled_config) => {
                config = bundled_config;
                info!("Loaded bundled config from {:?}", bundled_path);
            }
            Err(e) => warn!(
                "Failed to parse bundled config at {:?}: {}",
                bundled_path, e
            ),
        }
    }

    // try to load user-specific config from the platform's config directory
    if let Some(user_path) = get_user_config_path() {
        if let Ok(content) = std::fs::read_to_string(&user_path) {
            match ron::from_str::<AppConfig>(&content) {
                Ok(user_config) => {
                    config = user_config;
                    info!("Loaded user override config from {:?}", user_path);
                }
                Err(e) => warn!("Failed to parse user config at {:?}: {}", user_path, e),
            }
        }
    }

    config
}
