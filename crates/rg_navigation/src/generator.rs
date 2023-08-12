use std::collections::VecDeque;

use bevy::math::{ivec2, vec2};
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use rg_core::{Grid, VecToBits};
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::collider_set::ColliderSet;
use crate::{cell_pos_to_world, NavMeshChunk, NavMeshSettings, CHUNK_CELLS, CHUNK_SIZE};

pub fn generate_chunk(
    settings: &NavMeshSettings,
    colliders: &ColliderSet,
    chunk_pos: IVec2,
) -> NavMeshChunk {
    let _span = info_span!("generate_chunk").entered();

    let height_map = generate_height_map(settings, colliders, chunk_pos);
    let connections = generate_connections(settings, &height_map);
    let mut edges = generate_edges(&connections);
    sort_edges(&mut edges);
    let triangulation_edges = triangulate(&edges);

    NavMeshChunk {
        height_map,
        connections,
        edges,
        triangulation_edges,
    }
}

fn generate_height_map(
    settings: &NavMeshSettings,
    collider_set: &ColliderSet,
    chunk_pos: IVec2,
) -> Grid<f32> {
    let _span = info_span!("generate_height_map").entered();

    let size = UVec2::splat(CHUNK_CELLS);
    Grid::par_from_fn(size, |cell| {
        let pos = cell_pos_to_world(chunk_pos, cell.as_vec2() + 0.5);
        collider_set
            .check_walkability(settings, pos)
            .unwrap_or(f32::NAN)
    })
}

fn generate_connections(settings: &NavMeshSettings, height_map: &Grid<f32>) -> Grid<u8> {
    let _span = info_span!("generate_connections").entered();

    Grid::from_fn(height_map.size(), |cell| {
        let cell_height = height_map[cell];
        if cell_height.is_nan() {
            return 0;
        }

        let mut connections = 0;

        for (i, neighbor) in height_map.neighborhood_4(cell) {
            let neighbor_height = height_map[neighbor];
            if neighbor_height.is_nan() {
                continue;
            }

            if (cell_height - neighbor_height).abs() <= settings.climb_height {
                connections |= (1 << i) as u8;
            }
        }

        connections
    })
}

fn generate_edges(connections: &Grid<u8>) -> Vec<(Vec2, Vec2)> {
    let _span = info_span!("generate_edges").entered();

    let mut edges = Vec::new();

    let min_cell = ivec2(-1, -1);
    let max_cell = connections.size().as_ivec2() - 1;
    let cells = (min_cell.x..=max_cell.x)
        .flat_map(move |y| (min_cell.y..=max_cell.y).map(move |x| ivec2(x, y)));

    for cell in cells {
        let mut add_edge = |x1, y1, x2, y2| {
            edges.push((
                (cell.as_vec2() + vec2(x1, y1) + 0.5) / (CHUNK_CELLS as f32) * CHUNK_SIZE,
                (cell.as_vec2() + vec2(x2, y2) + 0.5) / (CHUNK_CELLS as f32) * CHUNK_SIZE,
            ));
        };

        let get = |sx, sy| u8::from(connections.get(cell + ivec2(sx, sy)).unwrap_or(&0) > &0);
        let case = get(0, 0) | get(1, 0) << 1 | get(1, 1) << 2 | get(0, 1) << 3;

        let is_corner = cell == min_cell
            || cell == max_cell
            || cell == ivec2(min_cell.x, max_cell.y)
            || cell == ivec2(max_cell.x, min_cell.y);

        match case {
            1 if is_corner => {
                add_edge(0.0, 0.5, 0.5, 0.5);
                add_edge(0.5, 0.5, 0.5, 0.0);
            }
            1 => {
                add_edge(0.0, 0.5, 0.5, 0.0);
            }
            2 if is_corner => {
                add_edge(0.5, 0.0, 0.5, 0.5);
                add_edge(0.5, 0.5, 1.0, 0.5);
            }
            2 => {
                add_edge(0.5, 0.0, 1.0, 0.5);
            }
            3 => {
                add_edge(0.0, 0.5, 1.0, 0.5);
            }
            4 if is_corner => {
                add_edge(1.0, 0.5, 0.5, 0.5);
                add_edge(0.5, 0.5, 0.5, 1.0);
            }
            4 => {
                add_edge(1.0, 0.5, 0.5, 1.0);
            }
            5 => {
                add_edge(0.0, 0.5, 0.5, 0.0);
                add_edge(1.0, 0.5, 0.5, 1.0);
            }
            6 => {
                add_edge(0.5, 0.0, 0.5, 1.0);
            }
            7 => {
                add_edge(0.0, 0.5, 0.5, 1.0);
            }
            8 if is_corner => {
                add_edge(0.5, 1.0, 0.5, 0.5);
                add_edge(0.5, 0.5, 0.0, 0.5);
            }
            8 => {
                add_edge(0.5, 1.0, 0.0, 0.5);
            }
            9 => {
                add_edge(0.5, 1.0, 0.5, 0.0);
            }
            10 => {
                add_edge(0.5, 0.0, 1.0, 0.5);
                add_edge(0.5, 1.0, 0.0, 0.5);
            }
            11 => {
                add_edge(0.5, 1.0, 1.0, 0.5);
            }
            12 => {
                add_edge(1.0, 0.5, 0.0, 0.5);
            }
            13 => {
                add_edge(1.0, 0.5, 0.5, 0.0);
            }
            14 => {
                add_edge(0.5, 0.0, 0.0, 0.5);
            }
            _ => {}
        }
    }

    edges
}

fn sort_edges(edges: &mut Vec<(Vec2, Vec2)>) {
    let _span = info_span!("sort_edges").entered();

    let mut chains = Vec::new();
    let mut chain_starts = HashMap::new();
    let mut chain_ends = HashMap::new();

    for &edge in edges.iter() {
        let (start, end) = edge;
        let start_key = start.to_bits();
        let end_key = end.to_bits();

        let start_chain = chain_ends.get(&start_key).copied();
        let end_chain = chain_starts.get(&end_key).copied();

        match (start_chain, end_chain) {
            (None, None) => {
                let chain_idx = chains.len();
                let mut chain = VecDeque::with_capacity(256);
                chain.push_back(edge);
                chains.push(chain);
                chain_starts.insert(start_key, chain_idx);
                chain_ends.insert(end_key, chain_idx);
            }
            (Some(start_chain_idx), None) => {
                chains[start_chain_idx].push_back(edge);
                chain_ends.remove(&start_key);
                chain_ends.insert(end_key, start_chain_idx);
            }
            (None, Some(end_chain_idx)) => {
                chains[end_chain_idx].push_front(edge);
                chain_starts.remove(&end_key);
                chain_starts.insert(start_key, end_chain_idx);
            }
            (Some(start_chain_idx), Some(end_chain_idx)) if start_chain_idx == end_chain_idx => {
                chains[start_chain_idx].push_back(edge);
            }
            (Some(start_chain_idx), Some(end_chain_idx)) => {
                let end_chain = std::mem::take(&mut chains[end_chain_idx]);
                chain_starts.remove(&end_key);
                let end_key = end_chain.back().unwrap().1.to_bits();
                chain_ends.remove(&end_key);
                chains[start_chain_idx].push_back(edge);
                chains[start_chain_idx].extend(end_chain);
                chain_ends.remove(&start_key);
                chain_ends.insert(end_key, start_chain_idx);
            }
        }
    }

    edges.clear();
    for chain in chains {
        if !chain.is_empty() {
            edges.extend(join_edges(chain.into_iter()));
        }
    }
}

fn join_edges(edges: impl Iterator<Item = (Vec2, Vec2)>) -> Vec<(Vec2, Vec2)> {
    let mut edges = edges.peekable();
    let mut res_edges = Vec::new();

    while let Some((a_start, mut a_end)) = edges.next() {
        while let Some(&(b_start, b_end)) = edges.peek() {
            if b_start == a_end && (b_end - b_start).perp_dot(a_end - a_start) == 0.0 {
                a_end = b_end;
                edges.next();
            } else {
                break;
            }
        }

        res_edges.push((a_start, a_end));
    }

    let first_edge = res_edges[0];
    let last_edge = res_edges[res_edges.len() - 1];

    if first_edge.0 == last_edge.1
        && (first_edge.0 - first_edge.1).perp_dot(last_edge.0 - last_edge.1) == 0.0
    {
        res_edges[0].0 = last_edge.0;
        res_edges.pop();
    }

    res_edges
}

fn triangulate(edges: &[(Vec2, Vec2)]) -> Vec<(Vec2, Vec2)> {
    fn point2(v: Vec2) -> Point2<f32> {
        Point2::new(v.x, v.y)
    }

    fn point2_to_vec2(p: Point2<f32>) -> Vec2 {
        vec2(p.x, p.y)
    }

    let _span = info_span!("triangulate").entered();

    let mut triangulation = ConstrainedDelaunayTriangulation::<Point2<_>>::new();
    let mut constraint_edges = HashSet::new();

    for &(start, end) in edges {
        let v1 = triangulation.insert(point2(start)).unwrap();
        let v2 = triangulation.insert(point2(end)).unwrap();
        triangulation.add_constraint(v1, v2);
        let edge = triangulation.get_edge_from_neighbors(v1, v2).unwrap().fix();
        constraint_edges.insert(edge);
    }

    let mut faces = HashSet::new();
    for face in triangulation.inner_faces() {
        faces.insert(face.fix());
    }

    'face_loop: for face in triangulation.inner_faces() {
        let start = face.adjacent_edge();
        let mut edge = face.adjacent_edge().ccw();
        while edge != start {
            let outgoing_obstacle = constraint_edges.contains(&edge.fix());
            let incoming_obstacle = constraint_edges.contains(&edge.rev().fix());
            if outgoing_obstacle != incoming_obstacle {
                if incoming_obstacle {
                    faces.remove(&face.fix());
                }
                continue 'face_loop;
            }
            edge = edge.ccw();
        }
    }

    faces
        .iter()
        .flat_map(|&v| triangulation.face(v).adjacent_edges())
        .map(|edge| {
            (
                point2_to_vec2(edge.vertices()[0].position()),
                point2_to_vec2(edge.vertices()[1].position()),
            )
        })
        .collect()
}
