use bevy::prelude::{App, Plugin, Resource};
use utils::PersistentPaths;

/// A wrapper to implement the Bevy [`Resource`] trait for [`PersistentPaths`].
#[derive(Resource, Clone, Debug, derive_more::Deref, derive_more::DerefMut)]
pub struct PersistentPathsResource(pub PersistentPaths);

/// A plugin that initializes and inserts the [`PersistentPathsResource`].
pub struct PathsPlugin;

impl Plugin for PathsPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<PersistentPathsResource>() {
            app.insert_resource(PersistentPathsResource(PersistentPaths::resolve_client()));
        }
    }
}
