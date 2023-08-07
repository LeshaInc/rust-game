use bevy::prelude::*;
use contour::ContourBuilder;
use raqote::{
    AntialiasMode, DrawOptions, DrawTarget, PathBuilder, SolidSource, Source, StrokeStyle,
};
use rayon::prelude::*;
use rg_core::progress::ProgressStage;
use rg_core::Grid;
use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct TopographySettings {
    pub max_height: f32,
    pub iso_step: f32,
}

pub fn generate_topographic_map(
    progress: &mut ProgressStage,
    settings: &TopographySettings,
    height_map: &Grid<f32>,
) -> Grid<[u8; 3]> {
    let _scope = info_span!("generate_topographic_map").entered();

    let size = height_map.size();
    let height_data = progress.task(|| height_map.values().map(|&v| v as f64).collect::<Vec<_>>());

    let thresholds = progress.task(|| {
        (0..=(settings.max_height / settings.iso_step) as i32)
            .map(|v| (v as f64) * (settings.iso_step as f64))
            .collect::<Vec<_>>()
    });

    let lines = progress.multi_task(thresholds.len(), |task| {
        thresholds
            .par_iter()
            .flat_map(|&threshold| {
                let builder = ContourBuilder::new(size.x, size.y, true);
                let lines = builder
                    .lines(&height_data, &[threshold])
                    .expect("contouring failed")
                    .remove(0);
                task.subtask_completed();
                (lines.into_inner().0).0.into_par_iter().map(move |line| {
                    let points = line
                        .points()
                        .map(|p| Vec2::new(p.x() as f32, p.y() as f32))
                        .collect::<Vec<_>>();
                    (threshold, points)
                })
            })
            .collect::<Vec<_>>()
    });

    let mut target = progress.task(|| {
        let mut target = DrawTarget::new(size.x as i32, size.y as i32);
        target.clear(SolidSource {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        });
        target
    });

    progress.task(|| {
        for &(threshold, ref line) in &lines {
            let mut path = PathBuilder::new();

            path.move_to(line[0].x, line[0].y);

            for &pos in &line[1..] {
                path.line_to(pos.x, pos.y);
            }

            path.close();

            target.stroke(
                &path.finish(),
                &Source::Solid(SolidSource {
                    r: 100,
                    g: 100,
                    b: 100,
                    a: 255,
                }),
                &StrokeStyle {
                    width: if threshold == 0.0 { 2.0 } else { 1.0 },
                    ..default()
                },
                &DrawOptions {
                    antialias: AntialiasMode::None,
                    ..default()
                },
            );
        }
    });

    progress.task(|| {
        let data = target
            .get_data()
            .iter()
            .map(|&v| [(v >> 16) as u8, (v >> 8) as u8, v as u8])
            .collect::<Vec<_>>();
        Grid::from_data(size, data)
    })
}
