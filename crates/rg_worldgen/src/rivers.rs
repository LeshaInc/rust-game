#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use std::collections::BinaryHeap;

use bevy::prelude::*;
use rand::Rng;
use raqote::{
    AntialiasMode, DrawOptions, DrawTarget, LineCap, LineJoin, Path, PathBuilder, SolidSource,
    Source, StrokeStyle,
};
use rg_core::{Grid, PoissonDiscSampling};
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct RiversSettings {
    pub point_radius: f32,
    pub inertia: f32,
    pub evaporation: f32,
    pub erosion: f32,
}

pub fn generate_river_map<R: Rng>(
    rng: &mut R,
    progress: &WorldgenProgress,
    settings: &RiversSettings,
    height_map: &mut Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_river_map").entered();

    progress.set(WorldgenStage::Rivers, 0);

    let size = height_map.size();

    let points = generate_points(rng, height_map, settings);
    progress.set(WorldgenStage::Rivers, 20);

    let mut queue = BinaryHeap::new();
    initialize_queue(&mut queue, &points);
    progress.set(WorldgenStage::Rivers, 30);

    let downstream = generate_downstream_map(&mut queue, &points, settings);
    progress.set(WorldgenStage::Rivers, 40);

    let upstream = generate_upstream_map(&points, &downstream);
    progress.set(WorldgenStage::Rivers, 50);

    let volume = compute_volume(&points, &upstream, settings);
    progress.set(WorldgenStage::Rivers, 60);

    let erosion_map = generate_erosion_map(&points, height_map, &downstream, &volume);
    progress.set(WorldgenStage::Rivers, 70);

    apply_erosion(&erosion_map, height_map, settings);
    progress.set(WorldgenStage::Rivers, 80);

    let strahler = compute_strahler(&points, &upstream);
    progress.set(WorldgenStage::Rivers, 90);

    let river_map = draw_rivers(&points, size, &downstream, &upstream, &strahler);
    progress.set(WorldgenStage::Rivers, 100);

    river_map
}

#[derive(Default)]
struct Points {
    count: usize,
    positions: Vec<Vec2>,
    heights: Vec<f32>,
    neighbors: Vec<Vec<usize>>,
}

fn generate_points<R: Rng>(
    rng: &mut R,
    height_map: &Grid<f32>,
    settings: &RiversSettings,
) -> Points {
    let _scope = info_span!("generate_points").entered();

    let mut points = Points::default();

    points.positions =
        PoissonDiscSampling::new(rng, height_map.size().as_vec2(), settings.point_radius, 8).points;
    points.count = points.positions.len();

    let iter = points.positions.iter();
    let points_f64 = iter
        .map(|pt| delaunator::Point {
            x: pt.x as f64,
            y: pt.y as f64,
        })
        .collect::<Vec<_>>();

    let triangulation = {
        let _scope = info_span!("triangulation").entered();
        delaunator::triangulate(&points_f64)
    };

    let it = points.positions.iter();
    points.heights = it.map(|pt| height_map[pt.as_ivec2()]).collect::<Vec<_>>();

    points.neighbors = vec![vec![]; points.count];

    let mut point_to_halfedge = vec![0; points.count];

    for edge in 0..triangulation.triangles.len() {
        let endpoint = triangulation.triangles[delaunator::next_halfedge(edge)];
        if triangulation.halfedges[edge] != delaunator::EMPTY {
            point_to_halfedge[endpoint] = edge;
        }
    }

    for (point, neighbors) in points.neighbors.iter_mut().enumerate() {
        let start = point_to_halfedge[point];
        let mut incoming = start;
        loop {
            neighbors.push(triangulation.triangles[incoming]);
            let outgoing = delaunator::next_halfedge(incoming);
            incoming = triangulation.halfedges[outgoing];
            if incoming == delaunator::EMPTY || incoming == start {
                break;
            }
        }
    }

    points
}

struct QueueItem {
    priority: f32,
    start_i: usize,
    end_i: usize,
    dir: Vec2,
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for QueueItem {}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(f32::total_cmp(&self.priority, &other.priority))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        f32::total_cmp(&self.priority, &other.priority)
    }
}

fn initialize_queue(queue: &mut BinaryHeap<QueueItem>, points: &Points) {
    let _scope = info_span!("initialize_queue").entered();

    for start_i in 0..points.count {
        if points.heights[start_i] > 0.0 {
            continue;
        }

        for &end_i in &points.neighbors[start_i] {
            if points.heights[end_i] < 0.0 {
                continue;
            }

            let start = points.positions[start_i];
            let end = points.positions[end_i];
            let dir = (end - start).normalize();

            queue.push(QueueItem {
                priority: 1.0,
                start_i,
                end_i,
                dir,
            });
        }
    }
}

fn generate_downstream_map(
    queue: &mut BinaryHeap<QueueItem>,
    points: &Points,
    settings: &RiversSettings,
) -> Vec<Option<usize>> {
    let _scope = info_span!("generate_downstream_map").entered();

    let mut downstream = vec![None; points.count];

    while let Some(edge) = queue.pop() {
        if downstream[edge.end_i].is_some() {
            continue;
        }

        downstream[edge.end_i] = Some(edge.start_i);

        for &neighbor_i in &points.neighbors[edge.end_i] {
            if neighbor_i == edge.start_i
                || downstream[neighbor_i].is_some()
                || points.heights[neighbor_i] < points.heights[edge.end_i]
            {
                continue;
            }

            let start = points.positions[edge.end_i];
            let end = points.positions[neighbor_i];
            let neighbor_dir = (end - start).normalize();

            let priority = neighbor_dir.dot(edge.dir);
            let weighted_dir = neighbor_dir.lerp(edge.dir, settings.inertia).normalize();

            queue.push(QueueItem {
                priority,
                start_i: edge.end_i,
                end_i: neighbor_i,
                dir: weighted_dir,
            });
        }
    }

    downstream
}

fn generate_upstream_map(points: &Points, downstream: &[Option<usize>]) -> Vec<Vec<usize>> {
    let _scope = info_span!("generate_upstream_map").entered();

    let mut upstream = vec![vec![]; points.count];

    for (i, &j) in downstream.iter().enumerate() {
        if let Some(j) = j {
            upstream[j].push(i);
        }
    }

    upstream
}

fn compute_volume(points: &Points, upstream: &[Vec<usize>], settings: &RiversSettings) -> Vec<f32> {
    let _scope = info_span!("compute_volume").entered();

    let mut volume = vec![f32::NAN; points.count];

    for i in 0..points.count {
        compute_volume_at_point(&mut volume, upstream, i, settings);
    }

    volume
}

fn compute_volume_at_point(
    volume: &mut [f32],
    upstream: &[Vec<usize>],
    i: usize,
    settings: &RiversSettings,
) {
    if !volume[i].is_nan() {
        return;
    }

    let mut v = 1.0;

    for &up in &upstream[i] {
        compute_volume_at_point(volume, upstream, up, settings);
        v += volume[up];
    }

    volume[i] = v * (1.0 - settings.evaporation);
}

fn generate_erosion_map(
    points: &Points,
    height_map: &Grid<f32>,
    downstream: &[Option<usize>],
    volume: &[f32],
) -> Grid<f32> {
    let _scope = info_span!("generate_erosion_map").entered();

    let max_height = height_map.max_value();

    let mut erosion_map = Grid::new(height_map.size(), 0.0);

    for start_i in 0..points.count {
        let Some(end_i) = downstream[start_i] else {
            continue;
        };

        let start = points.positions[start_i];
        let end = points.positions[end_i];
        let len = (end - start).length();
        let dir = (end - start) / len;

        let start_volume = volume[start_i];
        let end_volume = volume[end_i];

        aa_line(start, end, |cell, alpha| {
            if !erosion_map.contains_cell(cell) {
                return;
            }

            let height = (height_map[cell] / max_height).max(0.0);
            let alpha = alpha * (height.powi(2) * 0.95 + 0.05);

            let proj = cell.as_vec2() + Vec2::splat(0.5) - start;
            let dist = proj.dot(dir).max(0.0);
            let t = dist / len;
            let volume = start_volume * (1.0 - t) + end_volume * t;
            erosion_map[cell] += volume * alpha;
        });
    }

    erosion_map.blur(3);
    erosion_map.blur(3);

    erosion_map
}

fn apply_erosion(erosion_map: &Grid<f32>, height_map: &mut Grid<f32>, settings: &RiversSettings) {
    let _scope = info_span!("apply_erosion").entered();

    for (cell, height_map) in height_map.entries_mut() {
        let unscaled = 1.0 / (1.0 + erosion_map[cell].powf(1.1));
        let fac = unscaled * settings.erosion + (1.0 - settings.erosion);
        *height_map *= fac;
    }
}

fn compute_strahler(points: &Points, upstream: &[Vec<usize>]) -> Vec<u8> {
    let _scope = info_span!("compute_strahler").entered();

    let mut volume = vec![0; points.count];

    for i in 0..points.count {
        compute_strahler_at_point(&mut volume, upstream, i);
    }

    volume
}

fn compute_strahler_at_point(strahler: &mut [u8], upstream: &[Vec<usize>], i: usize) {
    if strahler[i] > 0 {
        return;
    }

    if upstream[i].is_empty() {
        strahler[i] = 1;
        return;
    }

    for &up in &upstream[i] {
        compute_strahler_at_point(strahler, upstream, up);
    }

    let max_idx = upstream[i]
        .iter()
        .copied()
        .max_by_key(|&idx| strahler[idx])
        .unwrap();
    let max_val = strahler[max_idx];

    strahler[i] = if upstream[i]
        .iter()
        .any(|&idx| idx != max_idx && strahler[idx] == max_val)
    {
        max_val + 1
    } else {
        max_val
    };
}

fn draw_rivers(
    points: &Points,
    size: UVec2,
    downstream: &[Option<usize>],
    upstream: &[Vec<usize>],
    strahler: &[u8],
) -> Grid<f32> {
    let _scope = info_span!("draw_rivers").entered();

    let min_strahler = 4;
    let mut target = DrawTarget::new(size.x as i32, size.y as i32);

    target.clear(SolidSource {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });

    let mut spline = Vec::new();

    for start_i in 0..points.count {
        let cur_strahler = strahler[start_i];
        if cur_strahler < min_strahler {
            continue;
        }

        if upstream[start_i]
            .iter()
            .any(|&v| strahler[v] == cur_strahler)
        {
            continue;
        }

        spline.clear();
        spline.push(points.positions[start_i]);

        let mut cur_i = start_i;
        while strahler[cur_i] == cur_strahler {
            let Some(next_i) = downstream[cur_i] else {
                break;
            };

            spline.push(points.positions[next_i]);
            cur_i = next_i;
        }

        if spline.len() < 2 {
            continue;
        }

        let path = points_to_path(&spline);

        target.stroke(
            &path,
            &Source::Solid(SolidSource {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }),
            &StrokeStyle {
                width: (cur_strahler - min_strahler + 1) as f32,
                cap: LineCap::Round,
                join: LineJoin::Round,
                ..default()
            },
            &DrawOptions {
                antialias: AntialiasMode::Gray,
                ..default()
            },
        );
    }

    let data = target
        .get_data()
        .iter()
        .map(|&v| (v as u8) as f32 / 255.0)
        .collect::<Vec<_>>();
    Grid::from_data(size, data)
}

fn points_to_path(points: &[Vec2]) -> Path {
    let segments = points.len() - 1;
    if segments == 1 {
        let mut path = PathBuilder::new();
        path.move_to(points[0].x, points[0].y);
        path.line_to(points[1].x, points[1].y);
        return path.finish();
    }

    let mut ad = Vec::with_capacity(segments);
    let mut d = Vec::with_capacity(segments);
    let mut bd = Vec::with_capacity(segments);
    let mut rhs_array = Vec::with_capacity(segments);

    for i in 0..segments {
        let rhs_x_value;
        let rhs_y_value;

        let p0 = points[i];
        let p3 = points[i + 1];

        if i == 0 {
            bd.push(0.0);
            d.push(2.0);
            ad.push(1.0);

            rhs_x_value = p0.x + 2.0 * p3.x;
            rhs_y_value = p0.y + 2.0 * p3.y;
        } else if i == segments - 1 {
            bd.push(2.0);
            d.push(7.0);
            ad.push(0.0);
            rhs_x_value = 8.0 * p0.x + p3.x;
            rhs_y_value = 8.0 * p0.y + p3.y;
        } else {
            bd.push(1.0);
            d.push(4.0);
            ad.push(1.0);
            rhs_x_value = 4.0 * p0.x + 2.0 * p3.x;
            rhs_y_value = 4.0 * p0.y + 2.0 * p3.y;
        }

        rhs_array.push(Vec2::new(rhs_x_value, rhs_y_value));
    }

    thomas_algorithm(&bd, &d, &mut ad, &mut rhs_array, points)
}

fn thomas_algorithm(
    bd: &[f32],
    d: &[f32],
    ad: &mut [f32],
    rhs_array: &mut [Vec2],
    points: &[Vec2],
) -> Path {
    let segments = points.len() - 1;
    let mut solution_set = vec![Vec2::NAN; segments];

    ad[0] = ad[0] / d[0];
    rhs_array[0].x = rhs_array[0].x / d[0];
    rhs_array[0].y = rhs_array[0].y / d[0];

    if segments > 2 {
        for i in 1..=segments - 2 {
            let rhs_value_x = rhs_array[i].x;
            let prev_rhs_value_x = rhs_array[i - 1].x;

            let rhs_value_y = rhs_array[i].y;
            let prev_rhs_value_y = rhs_array[i - 1].y;

            ad[i] = ad[i] / (d[i] - bd[i] * ad[i - 1]);

            let exp1x = rhs_value_x - (bd[i] * prev_rhs_value_x);
            let exp1y = rhs_value_y - (bd[i] * prev_rhs_value_y);
            let exp2 = d[i] - bd[i] * ad[i - 1];

            rhs_array[i].x = exp1x / exp2;
            rhs_array[i].y = exp1y / exp2;
        }
    }

    let last_idx = segments - 1;
    let exp1 = rhs_array[last_idx].x - bd[last_idx] * rhs_array[last_idx - 1].x;
    let exp1y = rhs_array[last_idx].y - bd[last_idx] * rhs_array[last_idx - 1].y;
    let exp2 = d[last_idx] - bd[last_idx] * ad[last_idx - 1];
    rhs_array[last_idx].x = exp1 / exp2;
    rhs_array[last_idx].y = exp1y / exp2;

    solution_set[last_idx] = rhs_array[last_idx];

    for i in (0..last_idx).rev() {
        let control_point_x = rhs_array[i].x - (ad[i] * solution_set[i + 1].x);
        let control_point_y = rhs_array[i].y - (ad[i] * solution_set[i + 1].y);
        solution_set[i] = Vec2::new(control_point_x, control_point_y);
    }

    let mut path = PathBuilder::new();
    path.move_to(points[0].x, points[0].y);

    for i in 0..segments {
        let p1 = points[i + 1];
        if i == segments - 1 {
            let c1 = solution_set[i];
            let c2 = 0.5 * (p1 + c1);
            path.cubic_to(c1.x, c1.y, c2.x, c2.y, p1.x, p1.y)
        } else {
            let c1 = solution_set[i + 1];
            let c2 = 2.0 * p1 - c1;
            path.cubic_to(c1.x, c1.y, c2.x, c2.y, p1.x, p1.y)
        }
    }

    path.finish()
}

fn aa_line(start: Vec2, end: Vec2, mut callback: impl FnMut(IVec2, f32)) {
    let mut plot = |x, y, f| callback(IVec2::new(x, y), f);

    let mut x0 = start.x;
    let mut y0 = start.y;
    let mut x1 = end.x;
    let mut y1 = end.y;

    let steep = (y1 - y0).abs() > (x1 - x0).abs();

    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }

    let dx = x1 - x0;
    let dy = y1 - y0;

    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

    // handle first endpoint
    let xend = x0.round();
    let yend = y0 + gradient * (xend - x0);
    let xgap = 1.0 - (x0 + 0.5).fract();
    let xpxl1 = xend;
    let ypxl1 = yend.trunc();
    if steep {
        plot(ypxl1 as i32, xpxl1 as i32, (1.0 - yend.fract()) * xgap);
        plot(ypxl1 as i32 + 1, xpxl1 as i32, yend.fract() * xgap);
    } else {
        plot(xpxl1 as i32, ypxl1 as i32, (1.0 - (yend).fract()) * xgap);
        plot(xpxl1 as i32, ypxl1 as i32 + 1, yend.fract() * xgap);
    }

    let mut intery = yend + gradient;

    // handle second endpoint
    let xend = x1.round();
    let yend = y1 + gradient * (xend - x1);
    let xgap = (x1 + 0.5).fract();
    let xpxl2 = xend;
    let ypxl2 = (yend).trunc();
    if steep {
        plot(ypxl2 as i32, xpxl2 as i32, (1.0 - yend.fract()) * xgap);
        plot(ypxl2 as i32 + 1, xpxl2 as i32, yend.fract() * xgap);
    } else {
        plot(xpxl2 as i32, ypxl2 as i32, (1.0 - yend.fract()) * xgap);
        plot(xpxl2 as i32, ypxl2 as i32, yend.fract() * xgap);
    }

    // main loop
    if steep {
        for x in (xpxl1 as i32 + 1)..=(xpxl2 as i32 - 1) {
            plot(intery as i32, x, 1.0 - intery.fract());
            plot(intery as i32 + 1, x, intery.fract());
            intery += gradient;
        }
    } else {
        for x in (xpxl1 as i32 + 1)..=(xpxl2 as i32 - 1) {
            plot(x, intery as i32, 1.0 - intery.fract());
            plot(x, intery as i32 + 1, intery.fract());
            intery += gradient
        }
    }
}
