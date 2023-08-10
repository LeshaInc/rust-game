use bevy::prelude::*;

pub struct VersionOverlayPlugin;

impl Plugin for VersionOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_version_overlay);
    }
}

fn add_version_overlay(asset_server: Res<AssetServer>, mut commands: Commands) {
    let font = asset_server.load("fonts/m5x7.ttf");

    let mut msg = format!("rg {}", env!("VERGEN_GIT_DESCRIBE"));

    let commit_date = env!("VERGEN_GIT_COMMIT_DATE");
    if commit_date == "1980-01-01" {
        msg.push_str("*");
    } else {
        msg += &format!(" ({commit_date})");
    }

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK.with_a(0.4)),
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(TextBundle::from_section(
                msg,
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}
