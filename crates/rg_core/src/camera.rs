use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::NoFrustumCulling;
use bevy::sprite::Anchor;

use crate::chunk::{WorldOrigin, WorldOriginChanged, CHUNK_SIZE};
use crate::material::GlobalDitherOffset;
use crate::scale::GameScale;
use crate::CoreSystems;

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                create_blit_target,
                handle_input,
                update_transform,
                update_camera,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            handle_updated_origin.after(CoreSystems::UpdateOrigin),
        );
    }
}

#[derive(Debug, Component)]
pub struct CameraController {
    pub translation: Vec3,
    pub rotation: Quat,
    pub zoom: f32,

    pub target_translation: Vec3,
    pub target_rotation: Quat,
    pub target_zoom: f32,

    pub translation_smoothing: Vec3,
    pub rotation_smoothing: f32,
    pub zoom_smoothing: f32,

    pub translation_snap: f32,
    pub rotation_snap: f32,
    pub zoom_snap: f32,

    pub translation_max_diff_squared: f32,

    /// Camera meters per pixel
    pub camera_scale: f32,
    pub camera_pitch: f32,
    pub camera_near: f32,
    pub camera_far: f32,
    pub camera_distance: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        CameraController {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            zoom: 1.0,
            target_translation: Vec3::ZERO,
            target_rotation: Quat::IDENTITY,
            target_zoom: 1.0,
            translation_smoothing: Vec3::new(0.1, 0.1, 0.3),
            rotation_smoothing: 0.0001,
            zoom_smoothing: 0.001,
            translation_snap: 0.0001,
            rotation_snap: 0.003,
            zoom_snap: 0.001,
            translation_max_diff_squared: 2500.0,
            camera_scale: 1.0 / 48.0,
            camera_pitch: 30f32.to_radians(),
            camera_near: 0.1,
            camera_far: 100.0,
            camera_distance: 50.0,
        }
    }
}

#[derive(Debug, Component)]
pub struct BlitTarget {
    image: Handle<Image>,
    sprite: Entity,
}

fn create_blit_target(
    mut commands: Commands,
    q_controller: Query<Entity, (With<CameraController>, Without<BlitTarget>)>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(controller) = q_controller.get_single() else {
        return;
    };

    let image = images.add(Image {
        texture_descriptor: TextureDescriptor {
            label: Some("camera blit target"),
            size: Extent3d::default(),
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
    });

    let sprite = commands
        .spawn((
            Name::new("Camera Blit Target"),
            SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::TopLeft,
                    ..default()
                },
                texture: image.clone(),
                ..default()
            },
            // apparently it's broken
            NoFrustumCulling,
        ))
        .id();

    commands
        .entity(controller)
        .insert(BlitTarget { image, sprite });
}

fn update_transform(mut q_controller: Query<&mut CameraController>, time: Res<Time>) {
    let Ok(mut controller) = q_controller.get_single_mut() else {
        return;
    };

    let dist = controller
        .translation
        .distance_squared(controller.target_translation);
    if dist < controller.translation_snap || dist > controller.translation_max_diff_squared {
        controller.translation = controller.target_translation;
    } else {
        let alpha = 1.0 - controller.translation_smoothing.powf(time.delta_seconds());
        let target = controller.target_translation;
        let trans = &mut controller.translation;
        trans.x = lerp(trans.x, target.x, alpha.x);
        trans.y = lerp(trans.y, target.y, alpha.y);
        trans.z = lerp(trans.z, target.z, alpha.z);
    }

    if controller
        .rotation
        .angle_between(controller.target_rotation)
        < controller.rotation_snap
    {
        controller.rotation = controller.target_rotation;
    } else {
        let alpha = 1.0 - controller.rotation_smoothing.powf(time.delta_seconds());
        controller.rotation = controller.rotation.slerp(controller.target_rotation, alpha);
    }

    if (controller.zoom - controller.target_zoom).abs() < controller.zoom_snap {
        controller.zoom = controller.target_zoom;
    } else {
        let alpha = 1.0 - controller.zoom_smoothing.powf(time.delta_seconds());
        controller.zoom = controller.zoom * (1.0 - alpha) + controller.target_zoom * alpha;
    }
}

fn update_camera(
    q_window: Query<&Window>,
    mut q_controller: Query<(
        &CameraController,
        &mut Transform,
        &mut Projection,
        &mut Camera,
        &BlitTarget,
    )>,
    mut q_sprite: Query<&mut Transform, Without<CameraController>>,
    mut dither_offset: ResMut<GlobalDitherOffset>,
    mut images: ResMut<Assets<Image>>,
    game_scale: Res<GameScale>,
    world_origin: Res<WorldOrigin>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    let Ok((controller, mut camera_transform, mut camera_projection, mut camera, blit_target)) =
        q_controller.get_single_mut()
    else {
        return;
    };

    let camera_rot_inv = controller.rotation.inverse();
    let camera_scale = controller.camera_scale / controller.zoom;
    let camera_distance = controller.camera_distance / controller.zoom;
    let pixel_scale = game_scale.pixels as f32;

    let basis = Vec3::new(
        1.0,
        controller.camera_pitch.sin().abs(),
        controller.camera_pitch.cos().abs(),
    ) / camera_scale;

    let origin = camera_rot_inv * (world_origin.0.as_vec2() * CHUNK_SIZE).extend(0.0);
    let pos = origin + camera_rot_inv * controller.translation;
    let snapped_pos = (pos * basis).round() / basis;
    let offset = snapped_pos - pos;

    let dither = ((pos * basis % 4.0).round()).as_ivec3();
    dither_offset.0 = UVec2::new(
        dither.x.rem_euclid(4) as u32,
        (-dither.y - dither.z).rem_euclid(4) as u32,
    );

    camera_transform.rotation =
        controller.rotation * Quat::from_rotation_x(-controller.camera_pitch);
    camera_transform.translation = controller.translation
        + controller.rotation * offset
        + camera_transform.rotation * Vec3::Y * -camera_distance;

    camera_transform.rotate_local_x(90f32.to_radians());

    *camera_projection = Projection::Orthographic(OrthographicProjection {
        near: controller.camera_near,
        far: controller.camera_far / controller.zoom,
        scale: camera_scale,
        ..default()
    });

    camera.target = RenderTarget::Image(blit_target.image.clone());

    let width = (window.physical_width() as f32 / pixel_scale).ceil() as u32 + 2;
    let height = (window.physical_height() as f32 / pixel_scale).ceil() as u32 + 2;
    let extent = Extent3d {
        width,
        height,
        ..default()
    };

    let scale_factor = window.scale_factor() as f32;

    let Some(image) = images.get_mut(&blit_target.image) else {
        return;
    };

    image.resize(extent);

    let Ok(mut sprite_transform) = q_sprite.get_mut(blit_target.sprite) else {
        return;
    };

    let sprite_scale = pixel_scale / scale_factor;

    sprite_transform.scale = Vec3::splat(sprite_scale);
    sprite_transform.translation = Vec3::new(
        -window.width() / 2.0 + ((offset.x * basis.x - 0.5) * sprite_scale).round(),
        window.height() / 2.0
            + ((offset.z * basis.z + offset.y * basis.y + 1.0) * sprite_scale).round(),
        0.0,
    );
}

fn handle_input(
    mut q_camera: Query<&mut CameraController>,
    keyboard_input: Res<Input<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
) {
    let Ok(mut camera) = q_camera.get_single_mut() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Q) {
        camera.target_rotation *= Quat::from_rotation_z(45f32.to_radians());
    }

    if keyboard_input.just_pressed(KeyCode::E) {
        camera.target_rotation *= Quat::from_rotation_z(-45f32.to_radians());
    }

    for scroll_event in scroll_events.read() {
        let delta = match scroll_event.unit {
            MouseScrollUnit::Line => scroll_event.y,
            MouseScrollUnit::Pixel => scroll_event.y / 16.0,
        };

        let sensitivity = 0.1;
        let min_zoom = 0.125;
        let max_zoom = 2.0;

        if delta > 0.0 {
            camera.target_zoom *= 1.0 + delta * sensitivity;
        } else {
            camera.target_zoom /= 1.0 - delta * sensitivity;
        }

        camera.target_zoom = camera.target_zoom.clamp(min_zoom, max_zoom);
    }

    if keyboard_input.just_pressed(KeyCode::F1) {
        camera.target_zoom = 1.0;
    }
}

fn handle_updated_origin(
    mut ev_origin_changed: EventReader<WorldOriginChanged>,
    mut q_camera: Query<&mut CameraController>,
) {
    let Ok(mut camera) = q_camera.get_single_mut() else {
        return;
    };

    for event in ev_origin_changed.read() {
        camera.translation += event.translation;
        camera.target_translation += event.translation;
    }
}

// TODO: move this somewhere else
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}
