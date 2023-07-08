use bevy::math::{ivec2, vec2, vec3, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::utils::HashMap;
use bevy_rapier3d::prelude::*;
use rg_core::Grid;

use crate::chunk::{CHUNK_TILES, TILE_SIZE};

const VERTICES_CAP: usize = 128 * 1024;
const INDICES_CAP: usize = 128 * 1024;

pub struct MeshGenerator {
    heightmap: Grid<f32>,
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

pub struct MeshResult {
    pub mesh: Mesh,
    pub collider: Collider,
}

impl MeshGenerator {
    pub fn new(heightmap: Grid<f32>) -> MeshGenerator {
        MeshGenerator {
            heightmap,
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

    pub fn generate(mut self) -> MeshResult {
        let _span = info_span!("chunk mesh generator").entered();

        self.generate_cells();
        self.snap_vertices();
        self.cleanup_triangles();
        self.compute_normals();
        self.snap_normals();
        self.deduplicate();
        self.apply_scale();

        let collider = self.create_collider();
        let mesh = self.create_mesh();

        MeshResult { mesh, collider }
    }

    fn generate_cells(&mut self) {
        let _span = info_span!("generate cells").entered();

        for y in 0..CHUNK_TILES as i32 {
            for x in 0..CHUNK_TILES as i32 {
                let first_vertex_idx = self.positions.len();
                self.cell_first_vertex_idx = first_vertex_idx;

                self.generate_cell(ivec2(x, y));

                for pos in &mut self.positions[first_vertex_idx..] {
                    pos.x += x as f32;
                    pos.y += y as f32;
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
            let height = self.heightmap.sample(pos.xy());
            let grad = self.heightmap.sample_grad(pos.xy());

            if (pos.z - height).abs().powi(2) < 0.0025 / grad.length_squared() {
                pos.z = height;
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
            let grad = self.heightmap.sample_grad(pos.xy());
            let target_normal = vec3(-grad.x, -grad.y, 2.0).normalize();
            if normal.z.abs() > 0.1 && normal.dot(target_normal) > 0.9 {
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

        for pos in &mut self.positions {
            pos.x *= TILE_SIZE;
            pos.y *= TILE_SIZE;
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
                *pos = vec3(pos.x, 1.0 - pos.y, pos.z);
            }
        }

        if self.rotate {
            for pos in &mut positions[..] {
                *pos = vec3(1.0 - pos.y, pos.x, pos.z);
            }
        }

        if self.flip_x ^ self.flip_y {
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
            pos = vec3(pos.x, 1.0 - pos.y, pos.z);
        }

        if self.rotate {
            pos = vec3(1.0 - pos.y, pos.x, pos.z);
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
            a.extend(self.height),
            b.extend(self.height),
            c.extend(self.height),
        );
    }

    fn ms_quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2) {
        self.ms_quad_3d(
            a.extend(self.height),
            b.extend(self.height),
            c.extend(self.height),
            d.extend(self.height),
        );
    }

    fn ms_wall(&mut self, a: Vec2, b: Vec2) {
        if self.up_mask == 0 {
            return;
        }

        let a_tr = self.ms_transform_point(a.extend(self.height));

        let mut up_height = 1000.0;
        for pos in &self.positions[self.cell_first_vertex_idx..] {
            if pos.xy() == a_tr.xy() && pos.z > self.height && pos.z < up_height {
                up_height = pos.z;
            }
        }

        self.ms_quad_3d(
            b.extend(self.height),
            a.extend(self.height),
            a.extend(up_height),
            b.extend(up_height),
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
        (self.heightmap[pos] / self.height_step).floor() * self.height_step
    }
}