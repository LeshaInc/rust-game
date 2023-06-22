use std::time::Duration;

use bevy::asset::ChangeWatcher;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::prelude::*;
use bevy::render::camera::{RenderTarget, ScalingMode};
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::sprite::Anchor;
use bevy::window::{WindowResized, WindowResolution};
use rg_pixel_material::{PixelMaterial, PixelMaterialPlugin};

const PIXEL_SCALE: f32 = 2.0;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
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
        .add_plugin(PixelMaterialPlugin)
        .insert_resource(Msaa::Off)
        .add_systems(Startup, setup)
        .add_systems(Update, on_resize_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PixelMaterial>>,
    mut images: ResMut<Assets<Image>>,
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

    let size = Extent3d {
        width: 128,
        height: 128,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    image.resize(size);

    let image_handle = images.add(image);

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(
                Quat::from_rotation_y(45f32.to_radians())
                    * Quat::from_rotation_x(-30f32.to_radians()),
            ),
            camera: Camera {
                order: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            projection: OrthographicProjection {
                scale: 1.0,
                near: -10.0,
                far: 10.0,
                scaling_mode: ScalingMode::Fixed {
                    width: 800.0,
                    height: 600.0,
                },
                ..default()
            }
            .into(),
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
    ));

    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::TopLeft,
                ..default()
            },
            texture: image_handle.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(PIXEL_SCALE)),
            ..default()
        },
        BlitTargetSprite,
    ));
}

#[derive(Component)]
struct BlitTargetSprite;

fn on_resize_system(
    q_window: Query<&Window>,
    mut q_blit_target: Query<&mut Handle<Image>, With<BlitTargetSprite>>,
    mut q_camera_3d: Query<&mut Projection, With<Camera3d>>,
    mut q_camera_2d: Query<&mut Transform, With<Camera2d>>,
    mut resize_events: EventReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
) {
    if resize_events.iter().last().is_none() {
        return;
    }

    let window = q_window.single();
    let width = (window.physical_width() as f32 / PIXEL_SCALE).ceil() as u32;
    let height = (window.physical_height() as f32 / PIXEL_SCALE).ceil() as u32;

    let new_extent = Extent3d {
        width,
        height,
        ..default()
    };

    let Ok(sprite_texture) = q_blit_target.get_single_mut() else { return };

    let Some(image) = images.get_mut(&sprite_texture) else { return };
    image.resize(new_extent);

    let Ok(mut camera_projection) = q_camera_3d.get_single_mut() else { return };

    *camera_projection = OrthographicProjection {
        scale: 1.0,
        near: -100.0,
        far: 100.0,
        scaling_mode: ScalingMode::Fixed {
            width: width as f32 / 64.0,
            height: height as f32 / 64.0,
        },
        ..default()
    }
    .into();

    let Ok(mut camera_2d_transform) = q_camera_2d.get_single_mut() else { return };
    *camera_2d_transform = Transform::from_xyz(window.width() / 2.0, -window.height() / 2.0, 999.0);
}
