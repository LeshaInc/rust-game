use bevy::prelude::*;
use rg_core::progress::ProgressReader;

use crate::WorldgenState;

rg_core::progress_stages! {
    pub enum WorldgenStage {
        Init => "Initializing world generator...",
        Island => "Generating the island...",
        Height => "Raising mountains...",
        Rivers => "Forming rivers...",
        Shores => "Generating shores...",
        Biomes => "Generating biomes...",
        Topography => "Mapping the world...",
        Saving => "Saving the world...",
    }
}

#[derive(Resource, Deref)]
pub struct WorldgenProgress(pub ProgressReader<WorldgenStage>);

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
    let stage = progress.stage();
    let percentage = progress.percentage();

    let mut stage_text = q_stage_text.single_mut();
    stage_text.sections[0].value = stage.message().into();

    let mut percentage_text = q_percentage_text.single_mut();
    percentage_text.sections[0].value = format!("{:.0}%", percentage);
}

fn destroy_ui(root: Res<UiRoot>, mut commands: Commands) {
    commands.entity(root.0).despawn_recursive();
    commands.remove_resource::<UiRoot>();
}
