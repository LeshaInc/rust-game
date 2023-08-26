#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod collider_set;
mod generator;
mod listener;
mod navmesh;

use bevy::prelude::*;
use rg_dev_overlay::DevOverlaySettings;
use rg_navigation_api::NavigationApiPlugin;

use crate::listener::ListenerPlugin;
use crate::navmesh::{draw_navmesh_gizmos, draw_navmesh_heightmap_gizmos};
pub use crate::navmesh::{Link, LinkKind, NavMesh, NavMeshChunk, Triangle};

pub const NAVMESH_QUALITY: u32 = 2;
pub const CHUNK_OVERSCAN: f32 = 1.0;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NavigationApiPlugin)
            .init_resource::<NavMeshSettings>()
            .add_plugins(ListenerPlugin)
            .add_systems(
                Update,
                (
                    draw_navmesh_gizmos
                        .run_if(|s: Res<DevOverlaySettings>| s.enabled && s.show_navmesh),
                    draw_navmesh_heightmap_gizmos
                        .run_if(|s: Res<DevOverlaySettings>| s.enabled && s.show_navmesh_heightmap),
                ),
            );
    }
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct NavMeshSettings {
    pub max_tasks_in_flight: usize,
    pub change_delay: u32,
    pub min_world_z: f32,
    pub max_world_z: f32,
    pub climb_height: f32,
    pub agent_height: f32,
    pub agent_radius: f32,
    pub agent_offset: f32,
}

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            max_tasks_in_flight: 4,
            change_delay: 5,
            min_world_z: -200.0,
            max_world_z: 200.0,
            climb_height: 1.0,
            agent_height: 1.8,
            agent_radius: 0.3,
            agent_offset: 0.05,
        }
    }
}
