use std::collections::BinaryHeap;

use bevy::prelude::Vec2;
use rand::Rng;
use rg_core::{Grid, PoissonDiscSampling};

pub fn generate_rivers<R: Rng>(rng: &mut R, elevation: &Grid<f32>) -> Grid<f32> {
    let size = elevation.size();

    let points = PoissonDiscSampling::new(rng, size.as_vec2(), 4.0).points;
    let points_f64 = points
        .iter()
        .map(|pt| delaunator::Point {
            x: pt.x as f64,
            y: pt.y as f64,
        })
        .collect::<Vec<_>>();

    let triangulation = delaunator::triangulate(&points_f64);

    let point_heights = points
        .iter()
        .map(|pt| elevation[pt.as_ivec2()])
        .collect::<Vec<_>>();

    let mut point_neighbors = vec![vec![]; points.len()];

    for start in 0..triangulation.halfedges.len() {
        let point = triangulation.triangles[start];
        let neighbors = &mut point_neighbors[point];

        let mut incoming = start;
        loop {
            let outgoing = delaunator::next_halfedge(incoming);
            neighbors.push(triangulation.triangles[outgoing]);
            incoming = triangulation.halfedges[outgoing];
            if incoming == delaunator::EMPTY || incoming == start {
                break;
            }
        }
    }

    let mut queue = BinaryHeap::new();

    for (start_i, &start) in points.iter().enumerate() {
        if point_heights[start_i] > 0.0 {
            continue;
        }

        for &end_i in &point_neighbors[start_i] {
            if point_heights[end_i] == 0.0 {
                continue;
            }

            let end = points[end_i];
            let dir = (end - start).normalize();

            queue.push(QueueItem {
                priority: 1.0,
                start_i,
                end_i,
                dir,
            });
        }
    }

    let mut downstream = vec![None; points.len()];

    while let Some(edge) = queue.pop() {
        if downstream[edge.end_i].is_some() {
            continue;
        }

        downstream[edge.end_i] = Some(edge.start_i);

        for &neighbor_i in &point_neighbors[edge.end_i] {
            if neighbor_i == edge.start_i
                || downstream[neighbor_i].is_some()
                || point_heights[neighbor_i] < point_heights[edge.end_i]
            {
                continue;
            }

            let neighbor_dir = (points[neighbor_i] - points[edge.end_i]).normalize();
            let priority = neighbor_dir.dot(edge.dir);
            let weighted_dir = neighbor_dir.lerp(edge.dir, 0.4).normalize();

            queue.push(QueueItem {
                priority,
                start_i: edge.end_i,
                end_i: neighbor_i,
                dir: weighted_dir,
            });
        }
    }

    let mut upstream = vec![vec![]; points.len()];
    for (i, &j) in downstream.iter().enumerate() {
        if let Some(j) = j {
            upstream[j].push(i);
        }
    }

    let mut volume = vec![f32::NAN; points.len()];

    fn compute_volume(volume: &mut [f32], upstream: &[Vec<usize>], i: usize) {
        if !volume[i].is_nan() {
            return;
        }

        let mut v = 1.0;

        for &up in &upstream[i] {
            compute_volume(volume, upstream, up);
            v += volume[up];
        }

        volume[i] = v * (1.0 - 0.2);
    }

    for i in 0..points.len() {
        compute_volume(&mut volume, &upstream, i);
    }

    let max_volume = volume.iter().copied().fold(0.0, f32::max);

    for v in &mut volume {
        *v /= max_volume;
    }

    let mut rivers = Grid::new(size, 0.0);

    for start_i in 0..points.len() {
        let Some(end_i) = downstream[start_i] else {
            continue;
        };

        let start = points[start_i];
        let end = points[end_i];

        let mut prev = None;
        for t in 0..10 {
            let pt = (start + t as f32 / 10.0 * (end - start)).as_ivec2();
            if Some(pt) != prev {
                rivers[pt] += volume[start_i];
                rivers[pt] = rivers[pt].min(1.0);
            }
            prev = Some(pt);
        }
    }

    rivers
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
