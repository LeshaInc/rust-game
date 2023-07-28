#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use std::collections::BinaryHeap;

use bevy::prelude::*;
use rand::Rng;
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

pub fn generate_rivers<R: Rng>(
    rng: &mut R,
    progress: &WorldgenProgress,
    settings: &RiversSettings,
    elevation: &mut Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_rivers").entered();

    progress.set(WorldgenStage::Rivers, 0);

    let size = elevation.size();

    let points = generate_points(rng, elevation, settings);
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

    let volume_map = generate_volume_map(&points, size, &downstream, &volume);
    progress.set(WorldgenStage::Rivers, 70);

    apply_erosion(&volume_map, elevation, settings);
    progress.set(WorldgenStage::Rivers, 80);

    let strahler = compute_strahler(&points, &upstream);
    progress.set(WorldgenStage::Rivers, 90);

    let river_map = generate_river_map(&points, size, &downstream, &strahler);
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
    elevation: &Grid<f32>,
    settings: &RiversSettings,
) -> Points {
    let _scope = info_span!("generate_points").entered();

    let mut points = Points::default();

    points.positions =
        PoissonDiscSampling::new(rng, elevation.size().as_vec2(), settings.point_radius, 8).points;
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
    points.heights = it.map(|pt| elevation[pt.as_ivec2()]).collect::<Vec<_>>();

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

fn generate_volume_map(
    points: &Points,
    size: UVec2,
    downstream: &[Option<usize>],
    volume: &[f32],
) -> Grid<f32> {
    let _scope = info_span!("generate_volume_map").entered();

    let mut volume_map = Grid::new(size, 0.0);

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
            if !volume_map.contains_cell(cell) {
                return;
            }

            let proj = cell.as_vec2() + Vec2::splat(0.5) - start;
            let dist = proj.dot(dir).max(0.0);
            let t = dist / len;
            let volume = start_volume * (1.0 - t) + end_volume * t;
            volume_map[cell] += volume * alpha;
        });
    }

    volume_map
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

fn generate_river_map(
    points: &Points,
    size: UVec2,
    downstream: &[Option<usize>],
    strahler: &[u8],
) -> Grid<f32> {
    let _scope = info_span!("generate_river_map").entered();

    let mut river_map = Grid::new(size * 2, false);

    for start_i in 0..points.count {
        let Some(end_i) = downstream[start_i] else {
            continue;
        };

        let start = points.positions[start_i] * 2.0;
        let end = points.positions[end_i] * 2.0;

        if strahler[start_i] <= 2 {
            continue;
        }

        line(start, end, |cell| {
            if !river_map.contains_cell(cell) {
                return;
            }

            river_map[cell] = true;
        });
    }

    let mut blurred = river_map.to_f32();
    blurred.blur(3);
    blurred.blur(3);
    blurred.map_range_inplace(0.0, 1.0);

    for cell in blurred.cells() {
        if river_map[cell] {
            blurred[cell] = 1.0;
        }
    }

    blurred
}

fn line(start: Vec2, end: Vec2, mut callback: impl FnMut(IVec2)) {
    let mut plot = |x, y| callback(IVec2::new(x, y));

    let start_x = start.x as i32;
    let start_y = start.y as i32;
    let end_x = end.x as i32;
    let end_y = end.y as i32;

    if start_x == end_x && start_y == end_y {
        plot(start_x, end_x);
        return;
    }

    let min_x = start_x.min(end_x);
    let (max_x, min_y, max_y) = if min_x == start_x {
        (end_x, start_y, end_y)
    } else {
        (start_x, end_y, start_y)
    };

    let diff_x = max_x - min_x;
    let diff_y = max_y - min_y;

    if diff_x > diff_y.abs() {
        let mut y = min_y as f32;
        let dy = (diff_y as f32) / (diff_x as f32);
        for x in min_x..=max_x {
            plot(x, y.round() as i32);
            y += dy;
        }
    } else {
        let mut x = min_x as f32;
        let dx = (diff_x as f32) / (diff_y as f32);
        if max_y >= min_y {
            for y in min_y..=max_y {
                plot(x.round() as i32, y);
                x += dx;
            }
        } else {
            for y in (max_y..=min_y).rev() {
                plot(x.round() as i32, y);
                x -= dx;
            }
        }
    }
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

fn apply_erosion(volume_map: &Grid<f32>, elevation: &mut Grid<f32>, settings: &RiversSettings) {
    let _scope = info_span!("apply_erosion").entered();

    let mut erosion_map = volume_map.clone();

    erosion_map.blur(2);
    erosion_map.blur(2);

    for (cell, elevation) in elevation.entries_mut() {
        let unscaled = 1.0 / (1.0 + erosion_map[cell].powf(1.1));
        let fac = unscaled * settings.erosion + (1.0 - settings.erosion);
        *elevation *= fac;
    }
}
