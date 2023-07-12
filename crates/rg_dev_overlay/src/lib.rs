use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::plot::{Line, Plot};
use bevy_egui::egui::{self, pos2, Color32, Frame, Rounding};
use bevy_egui::EguiContext;
use bevy_rapier3d::render::DebugRenderContext as RapierDebugRenderContext;

pub struct DevOverlayPlugin;

impl Plugin for DevOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DevOverlaySettings {
            show_frame_statistics: true,
            ..default()
        })
        .insert_resource(GizmoConfig {
            enabled: false,
            aabb: AabbGizmoConfig {
                draw_all: false,
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
                ui_settings
                    .run_if(|s: Res<DevOverlaySettings>| s.show_settings)
                    .after(handle_input),
                ui_left_side
                    .run_if(|s: Res<DevOverlaySettings>| s.enabled)
                    .after(ui_settings),
            ),
        );
    }
}

#[derive(Default, Resource)]
pub struct DevOverlaySettings {
    pub enabled: bool,
    pub show_settings: bool,
    pub show_frame_statistics: bool,
    pub show_navmesh: bool,
    pub show_colliders: bool,
}

fn handle_input(
    input: Res<Input<KeyCode>>,
    mut settings: ResMut<DevOverlaySettings>,
    mut gizmo_config: ResMut<GizmoConfig>,
    mut rapier_config: ResMut<RapierDebugRenderContext>,
) {
    if input.just_pressed(KeyCode::F3) {
        settings.enabled = !settings.enabled;
    }

    if input.just_pressed(KeyCode::F4) {
        settings.show_settings = !settings.show_settings;
    }

    gizmo_config.enabled = settings.enabled;
    rapier_config.enabled = settings.enabled && settings.show_colliders;
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
    settings: Res<DevOverlaySettings>,
    frame_time_points: Res<FrameTimePoints>,
) {
    let mut ctx = ctx.single_mut();

    let window = egui::Window::new("Dev Overlay Left")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(pos2(0.0, 0.0))
        .frame(Frame {
            rounding: Rounding::none(),
            fill: Color32::from_black_alpha(220),
            ..default()
        });

    window.show(ctx.get_mut(), |ui| {
        if settings.show_frame_statistics {
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
        }
    });
}

fn ui_settings(
    mut ctx: Query<&mut EguiContext, With<PrimaryWindow>>,
    mut settings: ResMut<DevOverlaySettings>,
    mut gizmo_config: ResMut<GizmoConfig>,
) {
    let mut ctx = ctx.single_mut();

    let window = egui::Window::new("Dev Overlay Settings");

    window.show(ctx.get_mut(), |ui| {
        ui.checkbox(&mut settings.enabled, "Enabled");
        ui.set_enabled(settings.enabled);
        ui.checkbox(&mut settings.show_frame_statistics, "Show frame statistics");
        ui.checkbox(&mut gizmo_config.aabb.draw_all, "Show bounding boxes");
        ui.checkbox(&mut settings.show_navmesh, "Show navigation mesh");
        ui.checkbox(&mut settings.show_colliders, "Show colliders");
    });
}
