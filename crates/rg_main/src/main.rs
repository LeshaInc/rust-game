mod camera_controller;

use std::time::Duration;

use bevy::asset::ChangeWatcher;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use rg_pixel_material::{PixelMaterial, PixelMaterialPlugin};
use rg_terrain::TerrainPlugin;

use crate::camera_controller::{CameraController, CameraControllerPlugin};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(800., 600.)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(100)),
                    ..default()
                }),
        )
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(PixelMaterialPlugin)
        .add_plugins(CameraControllerPlugin)
        .add_plugins(TerrainPlugin)
        .insert_resource(Msaa::Off)
        .add_systems(Startup, setup)
        .add_systems(Update, handle_input)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PixelMaterial>>,
) {
    // plane
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(PixelMaterial {
            color: Color::rgb(0.3, 0.7, 0.3),
            dither_enabled: false,
            ..default()
        }),
        ..default()
    });
    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(PixelMaterial {
            color: Color::rgb(0.3, 0.3, 0.7),
            ..default()
        }),
        ..default()
    });
    // sphere
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 0.5,
            sectors: 32,
            stacks: 16,
        })),
        transform: Transform::from_xyz(-1.2, 0.5, 1.2),
        material: materials.add(PixelMaterial {
            color: Color::rgb(0.7, 0.3, 0.3),
            ..default()
        }),
        ..default()
    });
    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 4800.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-0.8) * Quat::from_rotation_y(0.3),
            ..default()
        },
        ..default()
    });

    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                order: -1,
                ..default()
            },
            ..default()
        },
        CameraController::default(),
        DepthPrepass,
        NormalPrepass,
    ));

    debug!("Spawned everything");
}

fn handle_input(
    mut q_controller: Query<&mut CameraController>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut controller = q_controller.single_mut();
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::A) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        direction.x += 1.0;
    }
    if keyboard_input.pressed(KeyCode::W) {
        direction.z -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        direction.z += 1.0;
    }

    direction = controller.rotation * direction.normalize_or_zero();
    controller.target_translation += direction * 6.0 * time.delta_seconds();

    if keyboard_input.just_pressed(KeyCode::Q) {
        controller.target_rotation *= Quat::from_rotation_y(45f32.to_radians());
    }

    if keyboard_input.just_pressed(KeyCode::E) {
        controller.target_rotation *= Quat::from_rotation_y(-45f32.to_radians());
    }
}
