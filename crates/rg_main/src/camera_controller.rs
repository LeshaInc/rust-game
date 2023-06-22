use bevy::audio::CpalSample;
use bevy::prelude::*;
use rg_pixel_material::GlobalDitherOffset;

const SPEED: f32 = 0.1;
const SMOOTHING: f32 = 0.001;

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_input, update_transform));
    }
}

#[derive(Debug, Component)]
pub struct CameraController {
    pub target_translation: Vec3,
    pub target_rotation: Quat,
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Default for CameraController {
    fn default() -> Self {
        CameraController {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            target_translation: Vec3::ZERO,
            target_rotation: Quat::IDENTITY,
        }
    }
}

fn handle_input(
    mut q_controller: Query<&mut CameraController>,
    keyboard_input: Res<Input<KeyCode>>,
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
    controller.target_translation += direction * 0.1;

    if keyboard_input.just_pressed(KeyCode::Q) {
        controller.target_rotation *= Quat::from_rotation_y(45f32.to_radians());
    }

    if keyboard_input.just_pressed(KeyCode::E) {
        controller.target_rotation *= Quat::from_rotation_y(-45f32.to_radians());
    }
}

fn update_transform(
    mut q_controller: Query<(&mut CameraController, &mut Transform)>,
    time: Res<Time>,
    mut dither_offset: ResMut<GlobalDitherOffset>,
) {
    let (mut controller, mut transform) = q_controller.single_mut();

    // TODO
    let pixel_scale = Vec3::new(64.0, 64.0, 64.0 * 0.5);

    let alpha = 1.0 - SMOOTHING.powf(time.delta_seconds());
    controller.translation = controller
        .translation
        .lerp(controller.target_translation, alpha);
    controller.rotation = controller.rotation.slerp(controller.target_rotation, alpha);

    let pos = controller.rotation.inverse() * controller.translation;
    let snapped_pos = (pos * pixel_scale).round() / pixel_scale;
    let offset = snapped_pos - pos;

    transform.translation = controller.translation + controller.rotation * offset;

    transform.rotation = controller.rotation * Quat::from_rotation_x(-30f32.to_radians());

    dither_offset.0 = UVec2::new(
        ((pos.x * pixel_scale.x).round() as i32).rem_euclid(4) as u32,
        ((pos.z * pixel_scale.z).round() as i32).rem_euclid(4) as u32,
    );
}
