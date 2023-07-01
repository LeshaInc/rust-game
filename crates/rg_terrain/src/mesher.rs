use bevy::math::{ivec2, vec2, vec3, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashMap;
use bevy_rapier3d::prelude::*;
use futures_lite::future;
use rg_billboard::{MultiBillboard, MultiBillboardBundle};
use rg_core::{SharedGrid, NEIGHBORHOOD_8};

use crate::grass::{self, GeneratedGrass};
use crate::{
    Chunk, ChunkHeightmap, ChunkPos, Chunks, Seed, TerrainGrassMaterial, CHUNK_RESOLUTION,
    CHUNK_SIZE, MAX_UPDATES_PER_FRAME,
};

const VERTICES_CAP: usize = 128 * 1024;
const INDICES_CAP: usize = 128 * 1024;

struct MeshGenerator {
    seed: u64,
    chunk_pos: IVec2,
    heightmaps: Heightmaps,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
    height_step: f32,
    cell_first_vertex_idx: usize,
    height: f32,
    up_height: f32,
    mask: u8,
    up_mask: u8,
    down_mask: u8,
    flip_x: bool,
    flip_y: bool,
    rotate: bool,
}

#[derive(Debug)]
struct MeshResult {
    mesh: Mesh,
    grass: GeneratedGrass,
    collider: Collider,
}

impl MeshGenerator {
    fn new(seed: u64, chunk_pos: IVec2, heightmaps: Heightmaps) -> MeshGenerator {
        MeshGenerator {
            seed,
            chunk_pos,
            heightmaps,
            positions: Vec::with_capacity(VERTICES_CAP),
            normals: Vec::with_capacity(VERTICES_CAP),
            indices: Vec::with_capacity(INDICES_CAP),
            height_step: 0.2,
            cell_first_vertex_idx: 0,
            height: 0.0,
            up_height: 0.0,
            mask: 0,
            up_mask: 0,
            down_mask: 0,
            flip_x: false,
            flip_y: false,
            rotate: false,
        }
    }

    fn generate(mut self) -> MeshResult {
        let _span = info_span!("chunk mesh generator").entered();

        self.generate_cells();
        self.snap_vertices();
        self.cleanup_triangles();
        self.compute_normals();
        self.snap_normals();
        self.deduplicate();
        self.apply_scale();

        let collider = self.create_collider();
        let grass = grass::generate(self.seed, self.chunk_pos, &self.positions, &self.indices);
        let mesh = self.create_mesh();

        MeshResult {
            mesh,
            grass,
            collider,
        }
    }

    fn generate_cells(&mut self) {
        let _span = info_span!("generate cells").entered();

        for y in 0..CHUNK_RESOLUTION.y as i32 {
            for x in 0..CHUNK_RESOLUTION.x as i32 {
                let first_vertex_idx = self.positions.len();
                self.cell_first_vertex_idx = first_vertex_idx;

                self.generate_cell(ivec2(x, y));

                for pos in &mut self.positions[first_vertex_idx..] {
                    pos.x += x as f32;
                    pos.z += y as f32;
                }
            }
        }
    }

    fn generate_cell(&mut self, pos: IVec2) {
        let height_tl = self.get_quantized_height(pos + ivec2(0, 0));
        let height_tr = self.get_quantized_height(pos + ivec2(1, 0));
        let height_br = self.get_quantized_height(pos + ivec2(1, 1));
        let height_bl = self.get_quantized_height(pos + ivec2(0, 1));

        let mut heights = [height_tl, height_tr, height_br, height_bl];
        heights.sort_unstable_by(|a, b| f32::total_cmp(b, a));

        let mut height_i = 0;
        let mut up_height = heights[0] + 100.0;

        while height_i < heights.len() {
            let height = heights[height_i];
            height_i += 1;
            while height_i < heights.len() && heights[height_i] == height {
                height_i += 1;
            }

            self.mask = u8::from(height_tl == height)
                | u8::from(height_tr == height) << 1
                | u8::from(height_br == height) << 2
                | u8::from(height_bl == height) << 3;

            self.up_mask = u8::from(height_tl > height)
                | u8::from(height_tr > height) << 1
                | u8::from(height_br > height) << 2
                | u8::from(height_bl > height) << 3;

            self.down_mask = u8::from(height_tl < height)
                | u8::from(height_tr < height) << 1
                | u8::from(height_br < height) << 2
                | u8::from(height_bl < height) << 3;

            self.height = height;
            self.up_height = up_height;
            self.marching_squares();

            up_height = height;
        }
    }

    fn snap_vertices(&mut self) {
        let _span = info_span!("snap vertices").entered();

        for pos in &mut self.positions {
            let (height, grad) = self.heightmaps.sample_height_and_grad(pos.xz());
            if (pos.y - height).abs().powi(2) < 0.0025 / grad.length_squared() {
                pos.y = height;
            }
        }
    }

    fn cleanup_triangles(&mut self) {
        let _span = info_span!("cleanup triangles").entered();

        let mut idx = 0;
        while idx < self.indices.len() {
            let a = self.positions[self.indices[idx] as usize];
            let b = self.positions[self.indices[idx + 1] as usize];
            let c = self.positions[self.indices[idx + 2] as usize];

            if (a - b).cross(a - c).length_squared() < 1e-10 {
                self.indices.swap_remove(idx + 2);
                self.indices.swap_remove(idx + 1);
                self.indices.swap_remove(idx);
            } else {
                idx += 3;
            }
        }
    }

    fn compute_normals(&mut self) {
        let _span = info_span!("compute normals").entered();

        for indices in self.indices.chunks_exact(3) {
            let pos_a = self.positions[indices[0] as usize];
            let pos_b = self.positions[indices[1] as usize];
            let pos_c = self.positions[indices[2] as usize];
            let normal = (pos_b - pos_a).cross(pos_c - pos_a).normalize();
            self.normals[indices[0] as usize] = normal;
            self.normals[indices[1] as usize] = normal;
            self.normals[indices[2] as usize] = normal;
        }
    }

    fn snap_normals(&mut self) {
        let _span = info_span!("snap normals").entered();

        for (pos, normal) in self.positions.iter_mut().zip(&mut self.normals) {
            let (_, grad) = self.heightmaps.sample_height_and_grad(pos.xz());
            let target_normal = vec3(-grad.x, 2.0, -grad.y).normalize();
            if normal.y.abs() > 0.1 && normal.dot(target_normal) > 0.9 {
                *normal = target_normal;
            }
        }
    }

    fn deduplicate(&mut self) {
        let _span = info_span!("deduplicate").entered();

        let mut map = HashMap::with_capacity(self.positions.len());

        let mut new_positions = Vec::with_capacity(self.positions.len());
        let mut new_normals = Vec::with_capacity(self.positions.len());

        for index in &mut self.indices {
            let pos = self.positions[*index as usize];
            let normal = self.normals[*index as usize];

            let bit_pos = UVec3::new(pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits());
            let bit_normal = UVec3::new(normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits());

            *index = *map.entry((bit_pos, bit_normal)).or_insert_with(|| {
                let new_index = new_positions.len() as u32;
                new_positions.push(pos);
                new_normals.push(normal);
                new_index
            });
        }

        self.positions = new_positions;
        self.normals = new_normals;
    }

    fn apply_scale(&mut self) {
        let _span = info_span!("apply scale").entered();

        let scale = CHUNK_SIZE / CHUNK_RESOLUTION.as_vec2();
        for pos in &mut self.positions {
            pos.x *= scale.x;
            pos.z *= scale.y;
        }
    }

    fn create_collider(&self) -> Collider {
        let _span = info_span!("create collider").entered();

        let mut indices = Vec::with_capacity(self.indices.len() / 3);
        for triangle in self.indices.chunks_exact(3) {
            indices.push([triangle[0], triangle[1], triangle[2]]);
        }
        Collider::trimesh_with_flags(
            self.positions.clone(),
            indices,
            TriMeshFlags::HALF_EDGE_TOPOLOGY | TriMeshFlags::CONNECTED_COMPONENTS,
        )
    }

    fn create_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U32(self.indices)));

        mesh
    }

    fn marching_squares(&mut self) {
        self.flip_x = false;
        self.flip_y = false;
        self.rotate = false;

        let start_vertex = self.positions.len();
        let start_index = self.indices.len();

        if self.mask == 1 {
            self.ms_case_1();
        } else if self.mask == 2 {
            self.flip_x = true;
            self.ms_transform_masks();
            self.ms_case_1();
        } else if self.mask == 3 {
            self.ms_case_3();
        } else if self.mask == 4 {
            self.flip_x = true;
            self.flip_y = true;
            self.ms_transform_masks();
            self.ms_case_1();
        } else if self.mask == 5 {
            self.ms_case_5();
        } else if self.mask == 6 {
            self.rotate = true;
            self.ms_transform_masks();
            self.ms_case_3();
        } else if self.mask == 7 {
            self.ms_case_7();
        } else if self.mask == 8 {
            self.flip_y = true;
            self.ms_transform_masks();
            self.ms_case_1();
        } else if self.mask == 9 {
            self.flip_y = true;
            self.rotate = true;
            self.ms_transform_masks();
            self.ms_case_3();
        } else if self.mask == 10 {
            self.flip_y = true;
            self.ms_transform_masks();
            self.ms_case_5();
        } else if self.mask == 11 {
            self.flip_x = true;
            self.ms_transform_masks();
            self.ms_case_7();
        } else if self.mask == 12 {
            self.flip_y = true;
            self.ms_transform_masks();
            self.ms_case_3();
        } else if self.mask == 13 {
            self.flip_y = true;
            self.rotate = true;
            self.ms_transform_masks();
            self.ms_case_7();
        } else if self.mask == 14 {
            self.rotate = true;
            self.ms_transform_masks();
            self.ms_case_7();
        } else if self.mask == 15 {
            self.ms_case_15();
        }

        self.ms_transform_points(start_vertex, start_index);
    }

    #[allow(clippy::identity_op)]
    fn ms_transform_masks(&mut self) {
        for mask in [&mut self.mask, &mut self.up_mask, &mut self.down_mask] {
            if self.rotate {
                *mask = (*mask >> 1 & 1) << 0
                    | (*mask >> 2 & 1) << 1
                    | (*mask >> 3 & 1) << 2
                    | (*mask >> 0 & 1) << 3;
            }

            if self.flip_x {
                *mask = (*mask >> 1 & 1) << 0
                    | (*mask >> 0 & 1) << 1
                    | (*mask >> 3 & 1) << 2
                    | (*mask >> 2 & 1) << 3;
            }

            if self.flip_y {
                *mask = (*mask >> 3 & 1) << 0
                    | (*mask >> 2 & 1) << 1
                    | (*mask >> 1 & 1) << 2
                    | (*mask >> 0 & 1) << 3;
            }
        }
    }

    fn ms_transform_points(&mut self, start_vertex: usize, start_index: usize) {
        let positions = &mut self.positions[start_vertex..];

        if self.flip_x {
            for pos in &mut positions[..] {
                *pos = vec3(1.0 - pos.x, pos.y, pos.z);
            }
        }

        if self.flip_y {
            for pos in &mut positions[..] {
                *pos = vec3(pos.x, pos.y, 1.0 - pos.z);
            }
        }

        if self.rotate {
            for pos in &mut positions[..] {
                *pos = vec3(1.0 - pos.z, pos.y, pos.x);
            }
        }

        if !(self.flip_x ^ self.flip_y) {
            for indices in self.indices[start_index..].chunks_exact_mut(3) {
                indices.swap(1, 2);
            }
        }
    }

    fn ms_transform_point(&self, mut pos: Vec3) -> Vec3 {
        if self.flip_x {
            pos = vec3(1.0 - pos.x, pos.y, pos.z);
        }

        if self.flip_y {
            pos = vec3(pos.x, pos.y, 1.0 - pos.z);
        }

        if self.rotate {
            pos = vec3(1.0 - pos.z, pos.y, pos.x);
        }

        pos
    }

    fn ms_triangle_3d(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let index = self.positions.len() as u32;
        self.positions.extend([a, b, c]);
        self.normals.extend([Vec3::ZERO; 3]);
        self.indices.extend([index, index + 1, index + 2]);
    }

    fn ms_quad_3d(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let index = self.positions.len() as u32;
        self.positions.extend([a, b, c, d]);
        self.normals.extend([Vec3::ZERO; 4]);
        self.indices.extend([index, index + 1, index + 2]);
        self.indices.extend([index, index + 2, index + 3]);
    }

    fn ms_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2) {
        self.ms_triangle_3d(
            vec3(a.x, self.height, a.y),
            vec3(b.x, self.height, b.y),
            vec3(c.x, self.height, c.y),
        );
    }

    fn ms_quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2) {
        self.ms_quad_3d(
            vec3(a.x, self.height, a.y),
            vec3(b.x, self.height, b.y),
            vec3(c.x, self.height, c.y),
            vec3(d.x, self.height, d.y),
        );
    }

    fn ms_wall(&mut self, a: Vec2, b: Vec2) {
        if self.up_mask == 0 {
            return;
        }

        let a_tr = self.ms_transform_point(vec3(a.x, self.height, a.y));

        let mut up_height = 1000.0;
        for pos in &self.positions[self.cell_first_vertex_idx..] {
            if pos.xz() == a_tr.xz() && pos.y > self.height && pos.y < up_height {
                up_height = pos.y;
            }
        }

        self.ms_quad_3d(
            vec3(b.x, self.height, b.y),
            vec3(a.x, self.height, a.y),
            vec3(a.x, up_height, a.y),
            vec3(b.x, up_height, b.y),
        );
    }

    fn ms_case_1(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.5, 0.0), vec2(0.0, 0.5));

        if self.up_mask == 2 && self.down_mask == 12 || self.up_mask == 12 && self.down_mask == 2 {
            self.ms_triangle(vec2(0.0, 0.5), vec2(0.5, 0.0), vec2(1.0, 0.5));

            if self.up_mask == 2 {
                self.ms_wall(vec2(0.5, 0.0), vec2(1.0, 0.5));
            } else {
                self.ms_wall(vec2(1.0, 0.5), vec2(0.0, 0.5));
            }
        } else if self.up_mask == 8 && self.down_mask == 6
            || self.up_mask == 6 && self.down_mask == 8
        {
            self.ms_triangle(vec2(0.0, 0.5), vec2(0.5, 0.0), vec2(0.5, 1.0));

            if self.up_mask == 8 {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.0, 0.5));
            } else {
                self.ms_wall(vec2(0.5, 0.0), vec2(0.5, 1.0));
            }
        } else if self.up_mask == 4 && self.down_mask == 10 {
            self.ms_quad(
                vec2(0.5, 0.0),
                vec2(1.0, 0.5),
                vec2(0.5, 1.0),
                vec2(0.0, 0.5),
            );

            self.ms_wall(vec2(1.0, 0.5), vec2(0.5, 1.0));
        } else {
            self.ms_wall(vec2(0.5, 0.0), vec2(0.0, 0.5));
        }
    }

    fn ms_case_3(&mut self) {
        self.ms_quad(
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(1.0, 0.5),
            vec2(0.0, 0.5),
        );

        if self.up_mask == 4 && self.down_mask == 8 || self.up_mask == 8 && self.down_mask == 4 {
            self.ms_triangle(vec2(0.0, 0.5), vec2(1.0, 0.5), vec2(0.5, 1.0));

            if self.up_mask == 4 {
                self.ms_wall(vec2(1.0, 0.5), vec2(0.5, 1.0));
            } else {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.0, 0.5));
            }
        } else {
            self.ms_wall(vec2(1.0, 0.5), vec2(0.0, 0.5));
        }
    }

    fn ms_case_5(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.5, 0.0), vec2(0.0, 0.5));
        self.ms_triangle(vec2(1.0, 1.0), vec2(0.5, 1.0), vec2(1.0, 0.5));

        if self.up_mask != 10 {
            self.ms_quad(
                vec2(0.5, 0.0),
                vec2(1.0, 0.5),
                vec2(0.5, 1.0),
                vec2(0.0, 0.5),
            );

            if (self.up_mask & 8) != 0 {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.0, 0.5));
            }

            if (self.up_mask & 2) != 0 {
                self.ms_wall(vec2(0.5, 0.0), vec2(1.0, 0.5));
            }
        } else {
            self.ms_wall(vec2(0.5, 0.0), vec2(0.0, 0.5));
            self.ms_wall(vec2(0.5, 1.0), vec2(1.0, 0.5));
        }
    }

    fn ms_case_7(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
        self.ms_quad(
            vec2(0.0, 0.0),
            vec2(1.0, 1.0),
            vec2(0.5, 1.0),
            vec2(0.0, 0.5),
        );

        self.ms_wall(vec2(0.5, 1.0), vec2(0.0, 0.5));
    }

    fn ms_case_15(&mut self) {
        self.ms_quad(
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(1.0, 1.0),
            vec2(0.0, 1.0),
        );
    }

    fn get_quantized_height(&self, pos: IVec2) -> f32 {
        (self.heightmaps.get_height(pos) / self.height_step).floor() * self.height_step
    }
}

struct Heightmaps {
    center: SharedGrid<f32>,
    neighbors: [SharedGrid<f32>; 8],
}

impl Heightmaps {
    fn get_height(&self, pos: IVec2) -> f32 {
        if let Some(&height) = self.center.get(pos) {
            return height;
        }

        for (i, &dir) in NEIGHBORHOOD_8.iter().enumerate() {
            let pos = pos - dir * CHUNK_RESOLUTION.as_ivec2();
            if let Some(&height) = self.neighbors[i].get(pos) {
                return height;
            }
        }

        0.0
    }

    fn sample_height_and_grad(&self, pos: Vec2) -> (f32, Vec2) {
        let ipos = pos.as_ivec2();
        let fpos = pos - ipos.as_vec2();

        let tl = self.get_height(ipos + ivec2(0, 0));
        let tr = self.get_height(ipos + ivec2(1, 0));
        let bl = self.get_height(ipos + ivec2(0, 1));
        let br = self.get_height(ipos + ivec2(1, 1));

        fn lerp(a: f32, b: f32, t: f32) -> f32 {
            a * (1.0 - t) + b * t
        }

        let height = lerp(lerp(tl, tr, fpos.x), lerp(bl, br, fpos.x), fpos.y);
        let grad_x = lerp(tr - tl, br - bl, fpos.y);
        let grad_y = lerp(bl - tl, br - tr, fpos.x);

        (height, vec2(grad_x, grad_y))
    }
}

#[derive(Debug, Component)]
pub struct ChunkMeshTask(Task<MeshResult>);

pub fn schedule_system(
    q_chunks: Query<
        (Entity, &ChunkPos, &ChunkHeightmap),
        (With<Chunk>, Without<Handle<Mesh>>, Without<ChunkMeshTask>),
    >,
    q_chunk_heightmaps: Query<&ChunkHeightmap, With<Chunk>>,
    chunks: Res<Chunks>,
    seed: Res<Seed>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    let mut count = 0;

    for (chunk_id, &ChunkPos(chunk_pos), heightmap) in q_chunks.iter() {
        let neighbors = chunks
            .get_neighbors(chunk_pos)
            .map(|neighbor_id| neighbor_id.and_then(|id| q_chunk_heightmaps.get(id).ok()));

        if neighbors.iter().any(|v| v.is_none()) {
            continue;
        }

        count += 1;
        if count > MAX_UPDATES_PER_FRAME {
            break;
        }

        let center = heightmap.0.clone();
        let neighbors = neighbors.map(|v| v.unwrap().0.clone());

        let task = task_pool.spawn(async move {
            let generator = MeshGenerator::new(seed, chunk_pos, Heightmaps { center, neighbors });
            generator.generate()
        });

        commands.entity(chunk_id).insert(ChunkMeshTask(task));
    }
}

pub fn update_system(
    mut q_chunks: Query<(Entity, &mut ChunkMeshTask), With<Chunk>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut multi_billboards: ResMut<Assets<MultiBillboard>>,
    grass_material: Res<TerrainGrassMaterial>,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut().take(MAX_UPDATES_PER_FRAME) {
        let Some(res) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        let grass_id = commands
            .spawn((
                grass_material.0.clone(),
                MultiBillboardBundle {
                    multi_billboard: multi_billboards.add(res.grass.multi_billboard),
                    ..default()
                },
            ))
            .id();

        commands
            .entity(chunk_id)
            .add_child(grass_id)
            .remove::<ChunkMeshTask>()
            .insert((meshes.add(res.mesh), res.collider));
    }
}
