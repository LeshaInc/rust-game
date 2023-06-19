mod material;

use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::prelude::*;

use crate::material::PixelMaterial;

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(MaterialPlugin::<PixelMaterial>::default())
        .add_systems(Startup, setup)
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
            color: Color::WHITE,
        }),
        ..default()
    });
    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(PixelMaterial {
            color: Color::rgb(0.3, 0.3, 0.7),
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
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
    ));
}
