use bevy::prelude::*;

pub struct VersionOverlayPlugin {
    pub git_describe: &'static str,
    pub git_commit_date: &'static str,
}

impl Plugin for VersionOverlayPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let asset_server = app.world.resource::<AssetServer>();
        let font = asset_server.load("fonts/m5x7.ttf");

        let mut msg = format!("rg {}", self.git_describe);
        if self.git_commit_date == "1980-01-01" {
            msg.push('*');
        } else {
            msg += &format!(" ({})", self.git_commit_date);
        }

        app.world
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
                    },
                ));
            });
    }
}
