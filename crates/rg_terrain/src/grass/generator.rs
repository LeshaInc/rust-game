use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
use rg_core::billboard::{BillboardInstance, MultiBillboard};
use rg_core::chunk::{CHUNK_SIZE, CHUNK_TILES};
use rg_core::grid::Grid;
use rg_core::PoissonDiscSampling;

use crate::utils::{get_barycentric, is_inside_barycentric};

pub const MIN_RADIUS: f32 = 0.14;

#[derive(Debug)]
pub struct GrassResult {
    pub multi_billboard: MultiBillboard,
}

pub fn generate(seed: u64, chunk_pos: IVec2, mesh: &Mesh, density_map: &Grid<f32>) -> GrassResult {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("bad mesh positions")
    };

    let Some(Indices::U32(indices)) = mesh.indices() else {
        panic!("bad mesh indices")
    };

    let _span = info_span!("chunk grass generator").entered();

    let mut rng = Pcg32::seed_from_u64(seed ^ (chunk_pos.x as u64) ^ (chunk_pos.y as u64) << 32);
    let sampling = PoissonDiscSampling::new(&mut rng, Vec2::splat(CHUNK_SIZE), MIN_RADIUS, 8);
    let grid = sampling.grid;

    let mut instances = Vec::with_capacity(32 * 1024);

    for indices in indices.chunks_exact(3) {
        let pos_a = Vec3::from(positions[indices[0] as usize]);
        let pos_b = Vec3::from(positions[indices[1] as usize]);
        let pos_c = Vec3::from(positions[indices[2] as usize]);

        let cell_a = pos_a.xy() / sampling.cell_size;
        let cell_b = pos_b.xy() / sampling.cell_size;
        let cell_c = pos_c.xy() / sampling.cell_size;

        let min_cell = cell_a.min(cell_b).min(cell_c).floor().as_ivec2();
        let max_cell = cell_a.max(cell_b).max(cell_c).ceil().as_ivec2();

        let cells = (min_cell.x..=max_cell.x)
            .flat_map(|x| (min_cell.y..=max_cell.y).map(move |y| IVec2::new(x, y)));

        let points = cells.flat_map(|cell| grid.get(cell));
        for &pos in points {
            let mut pos = pos.extend(0.0);
            let bary = get_barycentric(pos_a, pos_b, pos_c, pos);
            if !is_inside_barycentric(bary) {
                continue;
            }

            let density = density_map.sample(pos.xy() / CHUNK_SIZE * (CHUNK_TILES as f32) - 0.5);
            if density.is_nan() || density <= 0.0 {
                continue;
            }

            if density < 1.0 && !rng.gen_bool(density as f64) {
                continue;
            }

            pos.z = bary.dot(Vec3::new(pos_a.z, pos_b.z, pos_c.z));

            instances.push(BillboardInstance {
                pos,
                normal: Vec3::Z,
                size: Vec2::new(8.0 / 48.0, 16.0 / 48.0),
                color: Vec3::new(1.0, 1.0, 1.0),
                random: rng.gen_range(0..u32::MAX),
            });
        }
    }

    GrassResult {
        multi_billboard: MultiBillboard {
            instances: instances.into(),
            anchor: Vec2::new(0.5, 1.0),
        },
    }
}
