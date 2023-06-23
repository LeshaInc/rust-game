use bevy::math::{ivec2, uvec2, vec2, vec3, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use crate::{
    Chunk, ChunkHeightmap, ChunkMap, ChunkPos, Chunks, CHUNK_RESOLUTION, CHUNK_SIZE, NEIGHBOR_DIRS,
};

pub struct MeshGenerator {
    builder: MeshBuilder,
    height_step: f32,
    neighbor_heightmaps: [ChunkMap<f32>; 8],
    heightmap: ChunkMap<f32>,

    cell_builder: MeshBuilder,
    height: f32,
    up_height: f32,
    mask: u8,
    up_mask: u8,
    down_mask: u8,
    flip_x: bool,
    flip_y: bool,
    rotate: bool,
}

impl MeshGenerator {
    pub fn new(neighbor_heightmaps: [ChunkMap<f32>; 8], heightmap: ChunkMap<f32>) -> MeshGenerator {
        MeshGenerator {
            builder: MeshBuilder::default(),
            height_step: 0.1,
            neighbor_heightmaps,
            heightmap,

            cell_builder: MeshBuilder::default(),
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

    pub fn generate(mut self) -> Mesh {
        let _span = info_span!("chunk mesh generator").entered();

        for x in 0..CHUNK_RESOLUTION {
            for y in 0..CHUNK_RESOLUTION {
                let pos = uvec2(x, y).as_ivec2();
                self.generate_cell(pos);
                self.cell_builder
                    .apply_translation(vec3(x as f32, 0.0, y as f32));
                self.builder.append(&mut self.cell_builder);
            }
        }

        self.compute_normals();

        let scale = CHUNK_SIZE / (CHUNK_RESOLUTION as f32);
        self.builder.apply_scale(vec3(scale, 1.0, scale));
        self.builder.build()
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

            self.mask = u8::from(height_tl == height) << 0
                | u8::from(height_tr == height) << 1
                | u8::from(height_br == height) << 2
                | u8::from(height_bl == height) << 3;

            self.up_mask = u8::from(height_tl > height) << 0
                | u8::from(height_tr > height) << 1
                | u8::from(height_br > height) << 2
                | u8::from(height_bl > height) << 3;

            self.down_mask = u8::from(height_tl < height) << 0
                | u8::from(height_tr < height) << 1
                | u8::from(height_br < height) << 2
                | u8::from(height_bl < height) << 3;

            self.height = height;
            self.up_height = up_height;
            self.marching_squares();

            up_height = height;
        }
    }

    fn marching_squares(&mut self) {
        self.flip_x = false;
        self.flip_y = false;
        self.rotate = false;

        let case = if self.mask == 1 {
            Self::ms_case_1
        } else if self.mask == 2 {
            self.flip_x = true;
            Self::ms_case_1
        } else if self.mask == 3 {
            Self::ms_case_3
        } else if self.mask == 4 {
            self.flip_x = true;
            self.flip_y = true;
            Self::ms_case_1
        } else if self.mask == 5 {
            Self::ms_case_5
        } else if self.mask == 6 {
            self.rotate = true;
            Self::ms_case_3
        } else if self.mask == 7 {
            Self::ms_case_7
        } else if self.mask == 8 {
            self.flip_y = true;
            Self::ms_case_1
        } else if self.mask == 9 {
            self.flip_y = true;
            self.rotate = true;
            Self::ms_case_3
        } else if self.mask == 10 {
            self.flip_y = true;
            Self::ms_case_5
        } else if self.mask == 11 {
            self.flip_x = true;
            Self::ms_case_7
        } else if self.mask == 12 {
            self.flip_y = true;
            Self::ms_case_3
        } else if self.mask == 13 {
            self.flip_y = true;
            self.rotate = true;
            Self::ms_case_7
        } else if self.mask == 14 {
            self.rotate = true;
            Self::ms_case_7
        } else if self.mask == 15 {
            Self::ms_case_15
        } else {
            return;
        };

        self.ms_transform_masks();
        case(self);
    }

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

    fn ms_triangle_3d(&mut self, mut a: Vec3, mut b: Vec3, mut c: Vec3) {
        a = self.ms_transform_point(a);
        b = self.ms_transform_point(b);
        c = self.ms_transform_point(c);

        if !(self.flip_x ^ self.flip_y) {
            std::mem::swap(&mut b, &mut c);
        }

        self.cell_builder.triangle(a, b, c);
    }

    fn ms_quad_3d(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        self.ms_triangle_3d(a, b, c);
        self.ms_triangle_3d(a, c, d);
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
        for pos in &self.cell_builder.positions {
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

    fn compute_normals(&mut self) {
        for index in self.builder.indices.iter().step_by(3) {
            let index = *index as usize;
            let pos_a = self.builder.positions[index + 0];
            let pos_b = self.builder.positions[index + 1];
            let pos_c = self.builder.positions[index + 2];
            let normal = (pos_b - pos_a).cross(pos_c - pos_a).normalize();
            self.builder.normals[index + 0] = normal;
            self.builder.normals[index + 1] = normal;
            self.builder.normals[index + 2] = normal;
        }
    }

    fn get_height(&self, pos: IVec2) -> f32 {
        if inside_chunk(pos) {
            return self.heightmap.get(pos.as_uvec2());
        }

        for (i, &dir) in NEIGHBOR_DIRS.iter().enumerate() {
            let pos = pos - dir * UVec2::splat(CHUNK_RESOLUTION).as_ivec2();
            if inside_chunk(pos) {
                return self.neighbor_heightmaps[i].get(pos.as_uvec2());
            }
        }

        return 0.0;
    }

    fn get_quantized_height(&self, pos: IVec2) -> f32 {
        (self.get_height(pos) / self.height_step).floor() * self.height_step
    }
}

fn inside_chunk(pos: IVec2) -> bool {
    pos.x >= 0 && pos.y >= 0 && pos.x < CHUNK_RESOLUTION as i32 && pos.y < CHUNK_RESOLUTION as i32
}

#[derive(Debug, Component)]
pub struct ChunkMeshTask(Task<Mesh>);

pub fn schedule_system(
    q_chunks: Query<
        (Entity, &ChunkPos, &ChunkHeightmap),
        (With<Chunk>, Without<Handle<Mesh>>, Without<ChunkMeshTask>),
    >,
    q_chunk_heightmaps: Query<&ChunkHeightmap, With<Chunk>>,
    chunks: Res<Chunks>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();

    for (chunk_id, &ChunkPos(chunk_pos), heightmap) in &q_chunks {
        let neighbor_heightmaps = chunks
            .get_neighbors(chunk_pos)
            .map(|neighbor_id| neighbor_id.and_then(|id| q_chunk_heightmaps.get(id).ok()));

        if !neighbor_heightmaps.iter().all(|v| v.is_some()) {
            continue;
        }

        let neighbor_heightmaps = neighbor_heightmaps.map(|v| v.unwrap().0.clone());
        let heightmap = heightmap.0.clone();

        let task = task_pool.spawn(async move {
            let generator = MeshGenerator::new(neighbor_heightmaps, heightmap);
            generator.generate()
        });

        commands.entity(chunk_id).insert(ChunkMeshTask(task));
    }
}

pub fn update_system(
    mut q_chunks: Query<(Entity, &mut ChunkMeshTask), With<Chunk>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (chunk_id, mut task) in &mut q_chunks {
        let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) else  {
            continue;
        };

        let mesh_handle = meshes.add(mesh);

        commands
            .entity(chunk_id)
            .remove::<ChunkMeshTask>()
            .insert(mesh_handle);
    }
}

#[derive(Default)]
pub struct MeshBuilder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    pub fn vertex(&mut self, pos: Vec3) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(pos);
        self.normals.push(Vec3::Y);
        index
    }

    pub fn triangle_indices(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }

    pub fn triangle(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let ai = self.vertex(a);
        let bi = self.vertex(b);
        let ci = self.vertex(c);
        self.triangle_indices(ai, bi, ci);
    }

    pub fn map_positions(&mut self, mut mapper: impl FnMut(Vec3) -> Vec3) {
        for pos in &mut self.positions {
            *pos = mapper(*pos)
        }
    }

    pub fn apply_translation(&mut self, translation: Vec3) {
        self.map_positions(|pos| pos + translation);
    }

    pub fn apply_scale(&mut self, scale: Vec3) {
        self.map_positions(|pos| pos * scale);
    }

    pub fn append(&mut self, other: &mut MeshBuilder) {
        let base_index = self.positions.len() as u32;
        self.positions.append(&mut other.positions);
        self.normals.append(&mut other.normals);
        self.indices
            .extend(other.indices.drain(..).map(|idx| idx + base_index));
    }

    pub fn build(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U32(self.indices)));

        mesh
    }
}
