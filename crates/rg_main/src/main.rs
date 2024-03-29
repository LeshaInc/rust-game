use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::math::Vec3Swizzles;
use bevy::pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::window::{PresentMode, WindowResolution};
use bevy_egui::EguiPlugin;
use bevy_rapier3d::prelude::*;
use rg_agent::{AgentPlugin, SpawnCharacter};
use rg_ai::AiPlugin;
use rg_core::chunk::{ChunkSpawnCenter, FloatingOrigin, WorldOrigin, CHUNK_SIZE};
use rg_core::material::PixelMaterial;
use rg_core::{CameraController, CollisionLayers, CorePlugins};
use rg_dev_overlay::{DevOverlayPlugin, VersionOverlayPlugin};
use rg_navigation::NavigationPlugin;
use rg_terrain::TerrainPlugin;
use rg_worldgen::WorldgenPlugin;
use rg_worldgen_api::WorldgenState;

fn main() {
    App::new()
        .edit_schedule(Main, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoVsync,
                        resolution: WindowResolution::new(800., 600.),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin::default()),
        )
        .add_plugins(EguiPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default().disabled())
        .add_plugins(CorePlugins)
        .add_plugins(WorldgenPlugin)
        .add_plugins(TerrainPlugin)
        .add_plugins(NavigationPlugin)
        .add_plugins(AiPlugin)
        .add_plugins(AgentPlugin)
        .add_plugins(DevOverlayPlugin)
        .add_plugins(VersionOverlayPlugin {
            git_describe: env!("VERGEN_GIT_DESCRIBE"),
            git_commit_date: env!("VERGEN_GIT_COMMIT_DATE"),
        })
        .insert_resource(ClearColor(Color::rgb_linear(0.5, 0.5, 1.0)))
        .insert_resource(RapierConfiguration {
            gravity: Vec3::Z * -30.0,
            ..default()
        })
        .insert_resource(Msaa::Off)
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(AmbientLight {
            color: Color::rgb(0.8, 0.85, 1.0),
            brightness: 0.5,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                spawn_character
                    .run_if(in_state(WorldgenState::Done))
                    .run_if(not(resource_exists::<CharacterSpawned>())),
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 4800.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.1,
            shadow_normal_bias: 0.5,
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 1,
            minimum_distance: 20.0,
            maximum_distance: 100.0,
            ..default()
        }
        .build(),
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::from_rotation_x(-0.8) * Quat::from_rotation_z(0.3),
            ..default()
        },
        ..default()
    });

    commands.spawn(Camera2dBundle {
        camera: Camera::default(),
        deband_dither: DebandDither::Disabled,
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                order: -1,
                ..default()
            },
            dither: DebandDither::Disabled,
            ..default()
        },
        UiCameraConfig { show_ui: false },
        CameraController::default(),
        DepthPrepass,
        NormalPrepass,
        FloatingOrigin,
        RenderLayers::from_layers(&[0, 1]),
    ));
}

#[derive(Resource)]
struct CharacterSpawned;

fn spawn_character(
    origin: Res<WorldOrigin>,
    physics_context: Res<RapierContext>,
    mut commands: Commands,
) {
    let pos = Vec3::new(1024.0, 2048.0, 100.0) - (origin.0.as_vec2() * CHUNK_SIZE).extend(0.0);
    commands.insert_resource(ChunkSpawnCenter(pos.xy()));
    if let Some((_, toi)) =
        physics_context.cast_ray(pos, -Vec3::Z, 1000.0, false, QueryFilter::new())
    {
        let pos = pos - Vec3::Z * (toi - 2.0);
        commands.spawn((SpawnCharacter, Transform::from_translation(pos)));
        commands.insert_resource(CharacterSpawned);
    }
}

fn handle_input(
    q_camera: Query<&CameraController>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PixelMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let Ok(camera) = q_camera.get_single() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::R) {
        commands.spawn((
            MaterialMeshBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                transform: Transform::from_translation(
                    camera.translation + Vec3::new(0.0, 0.0, 5.0),
                ),
                material: materials.add(PixelMaterial {
                    color: Color::rgb(0.3, 0.3, 0.7),
                    dither_enabled: true,
                    bands: 10,
                    ..default()
                }),
                ..default()
            },
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            CollisionLayers::DYNAMIC_GROUP,
            FloatingOrigin,
        ));
    }
}
