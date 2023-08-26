use std::marker::PhantomData;

use bevy::prelude::{UiScale as BevyUiScale, *};
use bevy_egui::EguiSettings;

pub struct ScalePlugin;

impl Plugin for ScalePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiScale>()
            .register_type::<UiScaleSetting>()
            .insert_resource(UiScale::new(1))
            .insert_resource(UiScaleSetting::auto(0.5)) // for now
            .register_type::<GameScale>()
            .register_type::<GameScaleSetting>()
            .insert_resource(GameScale::new(1))
            .insert_resource(GameScaleSetting::auto(1.5))
            .add_systems(
                PreUpdate,
                (
                    update_scale::<Game>,
                    (update_scale::<Ui>, update_ui_scale).chain(),
                ),
            );
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
#[doc(hidden)]
pub struct Ui;

pub type UiScale = Scale<Ui>;
pub type UiScaleSetting = ScaleSetting<Ui>;

#[derive(Debug, Clone, Copy, Reflect)]
#[doc(hidden)]
pub struct Game;

pub type GameScale = Scale<Game>;
pub type GameScaleSetting = ScaleSetting<Game>;

#[derive(Debug, Clone, Copy, Resource, Reflect)]
#[reflect(Resource)]
pub struct Scale<T> {
    pub pixels: u8,
    #[reflect(ignore)]
    marker: PhantomData<T>,
}

impl<T> Scale<T> {
    pub fn new(pixels: u8) -> Scale<T> {
        Scale {
            pixels,
            marker: PhantomData,
        }
    }
}

impl<T> Default for Scale<T> {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Copy, Resource, Reflect)]
#[reflect(Resource)]
pub struct ScaleSetting<T> {
    pub pixels: Option<u8>,
    pub auto_scale: f64,
    #[reflect(ignore)]
    marker: PhantomData<T>,
}

impl<T> ScaleSetting<T> {
    pub fn auto(auto_scale: f64) -> ScaleSetting<T> {
        ScaleSetting {
            pixels: None,
            auto_scale,
            marker: PhantomData,
        }
    }

    pub fn manual(pixels: u8) -> ScaleSetting<T> {
        ScaleSetting {
            pixels: Some(pixels),
            auto_scale: 1.0,
            marker: PhantomData,
        }
    }
}

impl<T> Default for ScaleSetting<T> {
    fn default() -> Self {
        Self::auto(1.0)
    }
}

fn update_scale<T: Send + Sync + 'static>(
    window: Query<&Window>,
    setting: Res<ScaleSetting<T>>,
    mut scale: ResMut<Scale<T>>,
) {
    if let Some(new_scale) = setting.pixels {
        scale.pixels = new_scale.max(1);
        return;
    }

    let Ok(window) = window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * setting.auto_scale;
    scale.pixels = scale_factor.round().max(1.0).min(255.0) as u8;
}

fn update_ui_scale(
    window: Query<&Window>,
    ui_scale: Res<UiScale>,
    mut bevy_ui_scale: ResMut<BevyUiScale>,
    mut egui_settings: ResMut<EguiSettings>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };

    let scale_factor = (ui_scale.pixels as f64) / window.scale_factor();
    egui_settings.scale_factor = scale_factor;
    bevy_ui_scale.scale = scale_factor;
}
