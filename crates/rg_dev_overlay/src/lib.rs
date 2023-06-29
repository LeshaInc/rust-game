use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::plot::{Line, Plot};
use bevy_egui::egui::{self, pos2, Color32, Frame, Rounding};
use bevy_egui::EguiContext;

pub struct DevOverlayPlugin;

impl Plugin for DevOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DevOverlaySettings::default())
            .insert_resource(GizmoConfig {
                enabled: false,
                aabb: AabbGizmoConfig {
                    draw_all: true,
                    ..default()
                },
                ..default()
            })
            .insert_resource(FrameTimePoints::default())
            .add_systems(
                Update,
                (
                    handle_input,
                    record_frame_time.before(ui_left_side),
                    ui_left_side.run_if(is_enabled),
                ),
            );
    }
}

#[derive(Default, Resource)]
pub struct DevOverlaySettings {
    pub enabled: bool,
}

pub fn is_enabled(settings: Res<DevOverlaySettings>) -> bool {
    settings.enabled
}

fn handle_input(
    input: Res<Input<KeyCode>>,
    mut settings: ResMut<DevOverlaySettings>,
    mut gizmo_config: ResMut<GizmoConfig>,
) {
    if input.just_pressed(KeyCode::F3) {
        settings.enabled = !settings.enabled;
    }

    gizmo_config.enabled = settings.enabled;
}

#[derive(Default, Resource)]
struct FrameTimePoints(Vec<[f64; 2]>);

impl FrameTimePoints {
    fn avg_frame_time(&self) -> f64 {
        if self.0.is_empty() {
            return 0.0;
        }

        let sum = self.0.iter().map(|v| v[1]).sum::<f64>();
        sum / (self.0.len() as f64)
    }
}

fn record_frame_time(time: Res<Time>, mut points: ResMut<FrameTimePoints>) {
    let instant = time.raw_elapsed_seconds_f64();
    let frame_time = time.raw_delta_seconds_f64();
    points.0.push([instant, frame_time]);
    while points.0.len() > 100 {
        points.0.remove(0);
    }
}

fn ui_left_side(
    mut ctx: Query<&mut EguiContext, With<PrimaryWindow>>,
    frame_time_points: Res<FrameTimePoints>,
) {
    let mut ctx = ctx.single_mut();

    let window = egui::Window::new("dev_overlay_left")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(pos2(0.0, 0.0))
        .frame(Frame {
            rounding: Rounding::none(),
            fill: Color32::from_black_alpha(220),
            ..default()
        });

    window.show(ctx.get_mut(), |ui| {
        let avg_frame_time = frame_time_points.avg_frame_time();

        ui.label(format!(
            "FPS: {:.1} ({:.1} ms)",
            1.0 / avg_frame_time,
            avg_frame_time * 1000.0
        ));

        let line = Line::new(frame_time_points.0.clone()).fill(0.0);
        Plot::new("fps_plot")
            .width(200.0)
            .height(40.0)
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_y(0.0)
            .include_y(1.0 / 40.0)
            .set_margin_fraction(egui::vec2(0.0, 0.0))
            .show_background(false)
            .show(ui, |plot| plot.line(line));
    });
}
