use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::NoFrustumCulling;
use bevy::sprite::Anchor;
use rg_pixel_material::GlobalDitherOffset;

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (create_blit_target, update_transform, update_camera).chain(),
        );
    }
}

#[derive(Debug, Component)]
pub struct CameraController {
    pub translation: Vec3,
    pub rotation: Quat,
    pub target_translation: Vec3,
    pub target_rotation: Quat,
    pub translation_smoothing: f32,
    pub translation_snap: f32,
    pub rotation_smoothing: f32,
    pub rotation_snap: f32,
    /// Screen pixels per camera pixels
    pub pixel_scale: f32,
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
            target_translation: Vec3::ZERO,
            target_rotation: Quat::IDENTITY,
            translation_smoothing: 0.01,
            translation_snap: 0.0001,
            rotation_smoothing: 0.0001,
            rotation_snap: 0.003,
            pixel_scale: 2.0,
            camera_scale: 1.0 / 48.0,
            camera_pitch: 30f32.to_radians(),
            camera_near: 0.1,
            camera_far: 50.0,
            camera_distance: 25.0,
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
    let Ok(controller) = q_controller.get_single() else { return };

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
    let Ok(mut controller) = q_controller.get_single_mut() else { return };

    if controller
        .translation
        .distance_squared(controller.target_translation)
        < controller.translation_snap
    {
        controller.translation = controller.target_translation;
    } else {
        let alpha = 1.0 - controller.translation_smoothing.powf(time.delta_seconds());
        controller.translation = controller
            .translation
            .lerp(controller.target_translation, alpha);
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
) {
    let Ok(window) = q_window.get_single() else { return };

    let Ok((controller, mut camera_transform, mut camera_projection, mut camera, blit_target)) =
        q_controller.get_single_mut() else
    {
        return;
    };

    let scale = Vec3::new(
        controller.camera_scale.recip(),
        controller.camera_scale.recip() * controller.camera_pitch.cos(),
        controller.camera_scale.recip() * controller.camera_pitch.sin(),
    );

    let pos = controller.rotation.inverse() * controller.translation;
    let snapped_pos = (pos * scale).round() / scale;
    let offset = snapped_pos - pos;

    dither_offset.0 = UVec2::new(
        ((pos.x * scale.x).round() as i32).rem_euclid(4) as u32,
        ((pos.z * scale.z).round() as i32 - (pos.y * scale.y).round() as i32).rem_euclid(4) as u32,
    );

    camera_transform.rotation =
        controller.rotation * Quat::from_rotation_x(-controller.camera_pitch);
    camera_transform.translation = controller.translation
        + controller.rotation * offset
        + camera_transform.rotation * Vec3::Z * controller.camera_distance;

    *camera_projection = Projection::Orthographic(OrthographicProjection {
        near: controller.camera_near,
        far: controller.camera_far,
        scale: controller.camera_scale,
        ..default()
    });

    camera.target = RenderTarget::Image(blit_target.image.clone());

    let width = (window.physical_width() as f32 / controller.pixel_scale).ceil() as u32 + 2;
    let height = (window.physical_height() as f32 / controller.pixel_scale).ceil() as u32 + 2;
    let extent = Extent3d {
        width,
        height,
        ..default()
    };

    let Some(image) = images.get_mut(&blit_target.image) else {
        return;
    };

    image.resize(extent);

    let Ok(mut sprite_transform) = q_sprite.get_mut(blit_target.sprite) else {
        return;
    };

    sprite_transform.scale = Vec3::splat(controller.pixel_scale);
    sprite_transform.translation = Vec3::new(
        -window.width() / 2.0 + ((offset.x * scale.x - 0.5) * controller.pixel_scale).round(),
        window.height() / 2.0
            - ((offset.z * scale.z - offset.y * scale.y - 0.5) * controller.pixel_scale).round(),
        0.0,
    );
}
