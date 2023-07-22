use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use bevy::prelude::*;

use crate::WorldgenState;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WorldgenStage {
    Island = 0,
    Elevation,
    Rivers,
    Biomes,
}

#[derive(Debug, Default, Clone, Resource)]
pub struct WorldgenProgress(Arc<AtomicU16>);

impl WorldgenProgress {
    pub fn set(&self, stage: WorldgenStage, progress: u8) {
        let val = (stage as u16) << 8 | (progress as u16);
        self.0.store(val, Relaxed)
    }

    pub fn get(&self) -> (WorldgenStage, u8, f32) {
        let val = self.0.load(Relaxed);
        let stage = match val >> 8 {
            0 => WorldgenStage::Island,
            1 => WorldgenStage::Elevation,
            _ => WorldgenStage::Rivers,
        };
        let progress = val as u8;

        let frac = progress as f32 / 100.0;
        let total_progress = match stage {
            WorldgenStage::Island => frac * 25.0,
            WorldgenStage::Elevation => frac * 25.0 + 25.0,
            WorldgenStage::Rivers => frac * 25.0 + 50.0,
            WorldgenStage::Biomes => frac * 25.0 + 75.0,
        };

        (stage, progress, total_progress)
    }
}

pub struct WorldgenProgressUiPlugin;

impl Plugin for WorldgenProgressUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldgenState::InProgress), setup_ui)
            .add_systems(
                Update,
                update_ui
                    .run_if(in_state(WorldgenState::InProgress))
                    .run_if(resource_exists::<WorldgenProgress>()),
            )
            .add_systems(OnExit(WorldgenState::InProgress), destroy_ui);
    }
}

#[derive(Resource)]
struct UiRoot(Entity);

#[derive(Component)]
struct StageText;

#[derive(Component)]
struct PercentageText;

fn setup_ui(asset_server: Res<AssetServer>, mut commands: Commands) {
    let font = asset_server.load("fonts/m5x7.ttf");

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..default()
        })
        .with_children(|commands| {
            commands.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: font.clone(),
                        font_size: 48.0,
                        color: Color::WHITE,
                    },
                ),
                StageText,
            ));

            commands.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font,
                        font_size: 48.0,
                        color: Color::WHITE,
                    },
                ),
                PercentageText,
            ));
        })
        .id();

    commands.insert_resource(UiRoot(root));
}

fn update_ui(
    mut q_stage_text: Query<&mut Text, (With<StageText>, Without<PercentageText>)>,
    mut q_percentage_text: Query<&mut Text, (With<PercentageText>, Without<StageText>)>,
    progress: Res<WorldgenProgress>,
) {
    let (stage, _, total_progress) = progress.get();

    let mut stage_text = q_stage_text.single_mut();
    stage_text.sections[0].value = match stage {
        WorldgenStage::Island => "Generating the island...",
        WorldgenStage::Elevation => "Raising mountains...",
        WorldgenStage::Rivers => "Forming rivers...",
        WorldgenStage::Biomes => "Generating biomes...",
    }
    .into();

    let mut percentage_text = q_percentage_text.single_mut();
    percentage_text.sections[0].value = format!("{:.0}%", total_progress);
}

fn destroy_ui(root: Res<UiRoot>, mut commands: Commands) {
    commands.entity(root.0).despawn_recursive();
    commands.remove_resource::<UiRoot>();
}
