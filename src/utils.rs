use bevy::{ecs::system::SystemParam, hierarchy::DespawnRecursiveExt};

use crate::{player::PlayerIdx, prelude::*};

/// System parameter that can be used to reset the game world.
///
/// Currently this just means de-spawning all of the entities other than the camera and resetting
/// the camera position, but in the future this process might be more involved so we centralize the
/// logic here so it can be re-used.
#[derive(SystemParam)]
pub struct ResetController<'w, 's> {
    commands: Commands<'w, 's>,
    camera: Query<
        'w,
        's,
        (
            &'static mut Camera,
            &'static mut Transform,
            &'static mut OrthographicProjection,
        ),
        With<Camera>,
    >,
    entities_to_despawn: Query<'w, 's, Entity, (Without<Camera>, Without<PlayerIdx>)>,
}

impl<'w, 's> ResetController<'w, 's> {
    /// Clean up the game world, despawning all the gameplay entities, but leaving necessary
    /// entities like camera.
    pub fn reset_world(mut self) {
        // Clean up all entities other than the camera and the player entities
        for entity in self.entities_to_despawn.iter() {
            self.commands.entity(entity).despawn_recursive();
        }

        // Reset camera position
        if let Some((mut camera, mut transform, mut projection)) = self.camera.iter_mut().next() {
            camera.viewport = default();
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
            projection.scale = 1.0;
        }
    }
}