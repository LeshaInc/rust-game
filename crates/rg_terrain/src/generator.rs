use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use crate::{CHUNK_RESOLUTION, CHUNK_SIZE};

pub fn generate(_seed: u64, chunk_pos: IVec2) -> Mesh {
    let mut builder = MeshBuilder::default();
    for sx in 0..CHUNK_RESOLUTION {
        for sz in 0..CHUNK_RESOLUTION {
            let fx = CHUNK_SIZE * (sx as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.x as f32;
            let fz = CHUNK_SIZE * (sz as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.y as f32;

            let y = fx.sin() * fz.cos() * 0.2;

            let a = Vec3::new(sx as f32, y, sz as f32);
            let b = a + Vec3::new(0.0, 0.0, 1.0);
            let c = a + Vec3::new(1.0, 0.0, 1.0);
            let d = a + Vec3::new(1.0, 0.0, 0.0);
            builder.quad(a, b, c, d);
        }
    }

    let scale = CHUNK_SIZE / (CHUNK_RESOLUTION as f32);
    builder.apply_scale(Vec3::new(scale, 1.0, scale));
    builder.build()
}

#[derive(Default)]
pub struct MeshBuilder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    pub fn vertex(&mut self, pos: Vec3) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(pos);
        self.normals.push(Vec3::Y);
        self.uvs.push(Vec2::ZERO);
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

    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let ai = self.vertex(a);
        let bi = self.vertex(b);
        let ci = self.vertex(c);
        let di = self.vertex(d);
        self.triangle_indices(ai, bi, ci);
        self.triangle_indices(ai, ci, di);
    }

    pub fn apply_translation(&mut self, translation: Vec3) {
        for pos in &mut self.positions {
            *pos += translation;
        }
    }

    pub fn apply_scale(&mut self, scale: Vec3) {
        for pos in &mut self.positions {
            *pos *= scale;
        }
    }

    pub fn build(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U32(self.indices)));

        mesh
    }
}
