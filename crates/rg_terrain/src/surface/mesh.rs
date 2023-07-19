use bevy::math::{ivec2, uvec2, vec2, vec3, Vec3Swizzles};
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
    colors: Vec<Vec4>,
    indices: Vec<u32>,
    height_step: f32,
    cell: IVec2,
    cell_first_vertex: usize,
    cell_first_index: usize,
    cell_indices: Grid<[usize; 2]>,
    cell_vertices: Grid<[usize; 2]>,
    cell_walls: Grid<Vec<usize>>,
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
            colors: Vec::with_capacity(VERTICES_CAP),
            indices: Vec::with_capacity(INDICES_CAP),
            height_step: 0.25,
            cell: IVec2::ZERO,
            cell_first_vertex: 0,
            cell_first_index: 0,
            cell_indices: Grid::new(UVec2::splat(CHUNK_TILES + 1), [0, 0]),
            cell_vertices: Grid::new(UVec2::splat(CHUNK_TILES + 1), [0, 0]),
            cell_walls: Grid::new(UVec2::splat(CHUNK_TILES + 1), Vec::new()),
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
        self.compute_colors();
        self.snap_normals();
        self.merge_quads(16);
        self.merge_quads(8);
        self.merge_quads(4);
        self.merge_quads(2);
        self.remove_rejected_triangles();
        self.cleanup_triangles();
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
                self.cell = ivec2(x, y);
                self.cell_first_vertex = self.positions.len();
                self.cell_first_index = self.indices.len();

                self.generate_cell(ivec2(x, y));

                for pos in &mut self.positions[self.cell_first_vertex..] {
                    pos.x += x as f32;
                    pos.y += y as f32;
                }

                self.compute_cell_normals();
                self.snap_cell_vertices();

                self.cell_indices[self.cell] = [self.cell_first_index, self.indices.len()];
                self.cell_vertices[self.cell] = [self.cell_first_vertex, self.positions.len()];
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

    fn snap_cell_vertices(&mut self) {
        for pos in &mut self.positions[self.cell_first_vertex..] {
            let height = self.heightmap.sample(pos.xy());
            let grad = self.heightmap.sample_grad(pos.xy());

            let alpha = (grad.length() * 3.0).clamp(0.0, 1.0).powf(3.0);
            pos.z = pos.z * alpha + height * (1.0 - alpha);
        }

        for i in self.cell_first_vertex..self.positions.len() {
            let pos = self.positions[i];
            let mut min_diff = f32::INFINITY;

            for j in self.cell_first_vertex..self.positions.len() {
                if i == j {
                    continue;
                }
                let neighbor = self.positions[j];
                if neighbor.xy() == pos.xy() {
                    min_diff = min_diff.min(neighbor.z - pos.z);
                }
            }

            if -0.09 < min_diff && min_diff < 0.0 {
                self.positions[i].z += min_diff;
            }
        }
    }

    fn compute_colors(&mut self) {
        let _span = info_span!("compute colors").entered();

        let positions = self.positions.iter();
        let normals = self.normals.iter();
        let colors = self.colors.iter_mut();

        for ((&pos, &normal), color) in positions.zip(normals).zip(colors) {
            if normal.z.abs() < 0.1 {
                continue;
            }

            let mut min_dist: f32 = 0.5;

            let cell = pos.xy().as_ivec2();
            let neighbors = self.cell_walls.neighborhood_8(cell).map(|v| v.1);
            let cells = neighbors.chain(std::iter::once(cell));
            let walls = cells.flat_map(|cell| &self.cell_walls[cell]);

            for &idx in walls {
                if (self.positions[idx].z - self.positions[idx + 3].z).abs() < 0.01
                    && (self.positions[idx + 1].z - self.positions[idx + 2].z).abs() < 0.01
                {
                    continue;
                }

                for i in 0..2 {
                    let a = self.positions[i * 2 + idx];
                    let b = self.positions[i * 2 + idx + 1];

                    let norm = (b - a).normalize();
                    let fac = ((pos - a).dot(norm)).clamp(0.0, 1.0);
                    let vec = pos - a - fac * (b - a);
                    let dist = vec.x.abs() + vec.y.abs();

                    min_dist = min_dist.min(dist);
                }
            }

            color.x = (1.0 - min_dist / 0.5).clamp(0.0, 1.0);
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

    fn compute_cell_normals(&mut self) {
        for indices in self.indices[self.cell_first_index..].chunks_exact(3) {
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
            if normal.z.abs() < 0.1 {
                continue;
            }

            let grad = self.heightmap.sample_grad(pos.xy());
            let target_normal = vec3(-grad.x, -grad.y, 1.0 * TILE_SIZE).normalize();
            *normal = (*normal * 0.7 + target_normal * 0.3).normalize();
        }
    }

    fn merge_quads(&mut self, size: usize) {
        let _span = info_span!("merge quads {size}").entered();

        let origins = (0..CHUNK_TILES).step_by(size).flat_map(move |x| {
            (0..CHUNK_TILES)
                .step_by(size)
                .map(move |y| uvec2(x, y).as_ivec2())
        });

        'next_group: for origin in origins {
            let cells = (origin.x..origin.x + size as i32)
                .flat_map(move |x| (origin.y..origin.y + size as i32).map(move |y| ivec2(x, y)));

            let mut group_z = None;

            for cell in cells.clone() {
                let [start_vertex, end_vertex] = self.cell_vertices[cell];
                if end_vertex - start_vertex != 4 {
                    continue 'next_group;
                }

                let z = self.positions[start_vertex].z;
                if let Some(group_z) = group_z {
                    if z != group_z {
                        continue 'next_group;
                    }
                } else {
                    group_z = Some(z);
                }

                if self.positions[start_vertex + 1..end_vertex]
                    .iter()
                    .any(|pos| pos.z != z)
                {
                    continue 'next_group;
                }
            }

            for cell in cells {
                let [start_idx, end_idx] = self.cell_indices[cell];
                for idx in &mut self.indices[start_idx..end_idx] {
                    *idx = u32::MAX;
                }
                self.cell_indices[cell] = [0, 0];
                self.cell_vertices[cell] = [0, 0];
            }

            let Some(group_z) = group_z else {
                continue;
            };

            let idx = self.positions.len() as u32;

            for pos in [
                ivec2(0, 0),
                ivec2(size as i32, 0),
                ivec2(size as i32, size as i32),
                ivec2(0, size as i32),
            ] {
                self.positions
                    .push((origin + pos).as_vec2().extend(group_z));
                self.normals.push(Vec3::Z);
                self.colors.push(Vec4::ZERO);
            }

            self.indices
                .extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
        }
    }

    fn remove_rejected_triangles(&mut self) {
        self.indices.retain(|&v| v != u32::MAX);
    }

    fn deduplicate(&mut self) {
        let _span = info_span!("deduplicate").entered();

        let mut map = HashMap::<(UVec3, UVec3, UVec4), u32>::with_capacity(self.positions.len());

        let mut new_positions = Vec::with_capacity(self.positions.len());
        let mut new_normals = Vec::with_capacity(self.positions.len());
        let mut new_colors = Vec::with_capacity(self.positions.len());

        for index in &mut self.indices {
            let pos = self.positions[*index as usize];
            let normal = self.normals[*index as usize];
            let color = self.colors[*index as usize];

            let bit_pos = UVec3::new(pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits());
            let bit_normal = UVec3::new(normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits());
            let bit_color = UVec4::new(
                color.x.to_bits(),
                color.y.to_bits(),
                color.z.to_bits(),
                color.w.to_bits(),
            );

            *index = *map
                .entry((bit_pos, bit_normal, bit_color))
                .or_insert_with(|| {
                    let new_index = new_positions.len() as u32;
                    new_positions.push(pos);
                    new_normals.push(normal);
                    new_colors.push(color);
                    new_index
                });
        }

        self.positions = new_positions;
        self.normals = new_normals;
        self.colors = new_colors;
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
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
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
        self.colors.extend([Vec4::ZERO; 3]);
        self.indices.extend([index, index + 1, index + 2]);
    }

    fn ms_quad_3d(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let index = self.positions.len() as u32;
        self.positions.extend([a, b, c, d]);
        self.normals.extend([Vec3::ZERO; 4]);
        self.colors.extend([Vec4::ZERO; 4]);
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
        for pos in &self.positions[self.cell_first_vertex..] {
            if pos.xy() == a_tr.xy() && pos.z > self.height && pos.z < up_height {
                up_height = pos.z;
            }
        }

        self.cell_walls[self.cell].push(self.positions.len());

        self.ms_quad_3d(
            b.extend(self.height),
            a.extend(self.height),
            a.extend(up_height),
            b.extend(up_height),
        );
    }

    fn ms_case_1(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.0), vec2(0.25, 0.25));
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.25), vec2(0.0, 0.25));
        self.ms_triangle(vec2(0.25, 0.0), vec2(0.5, 0.0), vec2(0.25, 0.25));
        self.ms_triangle(vec2(0.0, 0.25), vec2(0.25, 0.25), vec2(0.0, 0.5));

        if self.up_mask == 2 && self.down_mask == 12 || self.up_mask == 12 && self.down_mask == 2 {
            self.ms_triangle(vec2(0.25, 0.25), vec2(0.5, 0.0), vec2(0.75, 0.25));
            self.ms_quad(
                vec2(0.25, 0.25),
                vec2(0.75, 0.25),
                vec2(1.00, 0.50),
                vec2(0.00, 0.50),
            );

            if self.up_mask == 2 {
                self.ms_wall(vec2(0.5, 0.0), vec2(0.75, 0.25));
                self.ms_wall(vec2(0.75, 0.25), vec2(1.0, 0.5));
            } else {
                self.ms_wall(vec2(1.0, 0.5), vec2(0.0, 0.5));
            }
        } else if self.up_mask == 8 && self.down_mask == 6
            || self.up_mask == 6 && self.down_mask == 8
        {
            self.ms_triangle(vec2(0.25, 0.25), vec2(0.25, 0.75), vec2(0.0, 0.5));
            self.ms_quad(
                vec2(0.25, 0.25),
                vec2(0.50, 0.00),
                vec2(0.50, 1.00),
                vec2(0.25, 0.75),
            );

            if self.up_mask == 8 {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.25, 0.75));
                self.ms_wall(vec2(0.25, 0.75), vec2(0.0, 0.5));
            } else {
                self.ms_wall(vec2(0.5, 0.0), vec2(0.5, 1.0));
            }
        } else if self.up_mask == 4 && self.down_mask == 10 {
            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.25, 0.25),
                vec2(0.50, 0.00),
                vec2(0.75, 0.25),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.25),
                vec2(1.00, 0.50),
                vec2(0.75, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.75),
                vec2(0.50, 1.00),
                vec2(0.25, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.25, 0.75),
                vec2(0.00, 0.50),
                vec2(0.25, 0.25),
            );

            self.ms_wall(vec2(1.0, 0.5), vec2(0.75, 0.75));
            self.ms_wall(vec2(0.75, 0.75), vec2(0.5, 1.0));
        } else {
            self.ms_wall(vec2(0.5, 0.0), vec2(0.25, 0.25));
            self.ms_wall(vec2(0.25, 0.25), vec2(0.0, 0.5));
        }
    }

    fn ms_case_3(&mut self) {
        if self.up_mask == 4 && self.down_mask == 8 || self.up_mask == 8 && self.down_mask == 4 {
            self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.25), vec2(0.0, 0.25));
            self.ms_triangle(vec2(0.0, 0.25), vec2(0.25, 0.25), vec2(0.0, 0.5));
            self.ms_triangle(vec2(1.0, 0.0), vec2(1.0, 0.25), vec2(0.75, 0.25));
            self.ms_triangle(vec2(1.0, 0.25), vec2(1.0, 0.5), vec2(0.75, 0.25));
            self.ms_triangle(vec2(0.25, 0.25), vec2(0.75, 0.25), vec2(0.5, 0.5));

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.25),
                vec2(1.00, 0.50),
                vec2(0.75, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.75),
                vec2(0.50, 1.00),
                vec2(0.25, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.25, 0.75),
                vec2(0.00, 0.50),
                vec2(0.25, 0.25),
            );

            if self.up_mask == 4 {
                self.ms_wall(vec2(1.0, 0.5), vec2(0.75, 0.75));
                self.ms_wall(vec2(0.75, 0.75), vec2(0.5, 1.0));
            } else {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.25, 0.75));
                self.ms_wall(vec2(0.25, 0.75), vec2(0.0, 0.5));
            }
        } else {
            self.ms_quad(
                vec2(0.0, 0.00),
                vec2(1.0, 0.00),
                vec2(1.0, 0.25),
                vec2(0.0, 0.25),
            );

            self.ms_quad(
                vec2(0.0, 0.25),
                vec2(1.0, 0.25),
                vec2(1.0, 0.50),
                vec2(0.0, 0.50),
            );

            self.ms_wall(vec2(1.0, 0.5), vec2(0.0, 0.5));
        }
    }

    fn ms_case_5(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.0), vec2(0.25, 0.25));
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.25), vec2(0.0, 0.25));
        self.ms_triangle(vec2(0.25, 0.0), vec2(0.5, 0.0), vec2(0.25, 0.25));
        self.ms_triangle(vec2(0.0, 0.25), vec2(0.25, 0.25), vec2(0.0, 0.5));

        self.ms_triangle(vec2(0.75, 0.75), vec2(1.0, 0.75), vec2(1.0, 1.0));
        self.ms_triangle(vec2(0.75, 0.75), vec2(1.0, 1.0), vec2(0.75, 1.0));
        self.ms_triangle(vec2(0.75, 0.75), vec2(1.0, 0.5), vec2(1.0, 0.75));
        self.ms_triangle(vec2(0.75, 0.75), vec2(0.75, 1.0), vec2(0.5, 1.0));

        if self.up_mask != 10 {
            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.25, 0.25),
                vec2(0.50, 0.00),
                vec2(0.75, 0.25),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.25),
                vec2(1.00, 0.50),
                vec2(0.75, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.75, 0.75),
                vec2(0.50, 1.00),
                vec2(0.25, 0.75),
            );

            self.ms_quad(
                vec2(0.50, 0.50),
                vec2(0.25, 0.75),
                vec2(0.00, 0.50),
                vec2(0.25, 0.25),
            );

            if (self.up_mask & 8) != 0 {
                self.ms_wall(vec2(0.5, 1.0), vec2(0.25, 0.75));
                self.ms_wall(vec2(0.25, 0.75), vec2(0.0, 0.5));
            }

            if (self.up_mask & 2) != 0 {
                self.ms_wall(vec2(0.5, 0.0), vec2(0.75, 0.25));
                self.ms_wall(vec2(0.75, 0.25), vec2(1.0, 0.5));
            }
        } else {
            self.ms_wall(vec2(0.5, 0.0), vec2(0.25, 0.25));
            self.ms_wall(vec2(0.25, 0.25), vec2(0.0, 0.5));

            self.ms_wall(vec2(0.5, 1.0), vec2(0.75, 0.75));
            self.ms_wall(vec2(0.75, 0.75), vec2(1.0, 0.5));
        }
    }

    fn ms_case_7(&mut self) {
        self.ms_triangle(vec2(0.0, 0.0), vec2(0.25, 0.25), vec2(0.0, 0.25));
        self.ms_triangle(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.25, 0.25));
        self.ms_triangle(vec2(0.0, 0.25), vec2(0.25, 0.25), vec2(0.0, 0.5));
        self.ms_triangle(vec2(0.25, 0.25), vec2(1.0, 0.0), vec2(0.5, 0.5));

        self.ms_triangle(vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.75, 0.75));
        self.ms_triangle(vec2(0.75, 0.75), vec2(1.0, 1.0), vec2(0.75, 1.0));
        self.ms_triangle(vec2(0.75, 0.75), vec2(0.75, 1.0), vec2(0.5, 1.0));
        self.ms_triangle(vec2(0.75, 0.75), vec2(0.5, 0.5), vec2(1.0, 0.0));

        self.ms_quad(
            vec2(0.50, 0.50),
            vec2(0.75, 0.75),
            vec2(0.50, 1.00),
            vec2(0.25, 0.75),
        );

        self.ms_quad(
            vec2(0.50, 0.50),
            vec2(0.25, 0.75),
            vec2(0.00, 0.50),
            vec2(0.25, 0.25),
        );

        self.ms_wall(vec2(0.5, 1.0), vec2(0.25, 0.75));
        self.ms_wall(vec2(0.25, 0.75), vec2(0.0, 0.5));
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
