use std::hash::{Hash, Hasher};
use std::ops::Range;

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::utils::HashMap;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

use super::{BillboardInstance, MultiBillboard};
use crate::VecToBits;

pub struct ScatterPlugin;

impl Plugin for ScatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, scatter);
    }
}

#[derive(Debug, Clone, Component)]
pub struct ScatterMultiBillboard {
    pub seed: u64,
    pub count: usize,
    pub move_along_normal: Range<f32>,
    pub instance_color: Vec3,
    pub instance_size: Vec2,
    pub anchor: Vec2,
    pub mesh: Handle<Mesh>,
}

impl PartialEq for ScatterMultiBillboard {
    fn eq(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.count == other.count
            && self.instance_color.to_bits() == other.instance_color.to_bits()
            && self.instance_size.to_bits() == other.instance_size.to_bits()
            && self.anchor.to_bits() == other.anchor.to_bits()
            && self.mesh == other.mesh
    }
}

impl Eq for ScatterMultiBillboard {}

impl Hash for ScatterMultiBillboard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.seed.hash(state);
        self.count.hash(state);
        self.instance_color.to_bits().hash(state);
        self.instance_size.to_bits().hash(state);
        self.anchor.to_bits().hash(state);
        self.mesh.hash(state);
    }
}

#[derive(Default)]
struct Cache {
    // TODO: trimming
    map: HashMap<ScatterMultiBillboard, Handle<MultiBillboard>>,
}

fn scatter(
    q_sources: Query<(Entity, &ScatterMultiBillboard)>,
    meshes: Res<Assets<Mesh>>,
    mut multi_billboards: ResMut<Assets<MultiBillboard>>,
    mut commands: Commands,
    mut cache: Local<Cache>,
) {
    for (entity, source) in q_sources.iter() {
        if let Some(multi_billboard) = cache.map.get(source) {
            commands
                .entity(entity)
                .remove::<ScatterMultiBillboard>()
                .insert(multi_billboard.clone());
            continue;
        };

        let Some(mesh) = meshes.get(&source.mesh) else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            warn!("bad mesh for multi billboard scattering (missing position attribute)");
            continue;
        };

        let triangles: Vec<[Vec3; 3]> = match mesh.indices() {
            Some(Indices::U16(indices)) => indices
                .chunks_exact(3)
                .map(|triangle| [0, 1, 2].map(|i| Vec3::from(positions[triangle[i] as usize])))
                .collect(),
            Some(Indices::U32(indices)) => indices
                .chunks_exact(3)
                .map(|triangle| [0, 1, 2].map(|i| Vec3::from(positions[triangle[i] as usize])))
                .collect(),
            None => positions
                .chunks_exact(3)
                .map(|triangle| [0, 1, 2].map(|i| Vec3::from(triangle[i])))
                .collect(),
        };

        let mut rng = Pcg32::seed_from_u64(source.seed);
        let mut instances = Vec::with_capacity(source.count);

        let total_area: f32 = triangles.iter().map(|&v| triangle_area(v)).sum();

        for triangle in triangles {
            let area = triangle_area(triangle);
            let count = (area / total_area * (source.count) as f32).round() as u32;

            for _ in 0..count {
                let mut pos = sample_triangle(&mut rng, triangle);
                let normal = triangle_normal(triangle);
                pos += normal * rng.gen_range(source.move_along_normal.clone());
                instances.push(BillboardInstance {
                    pos,
                    normal,
                    size: source.instance_size,
                    color: source.instance_color,
                    random: rng.gen(),
                })
            }
        }

        let multi_billboard = multi_billboards.add(MultiBillboard {
            instances: instances.into(),
            anchor: source.anchor,
        });

        cache.map.insert(source.clone(), multi_billboard.clone());

        commands
            .entity(entity)
            .remove::<ScatterMultiBillboard>()
            .insert(multi_billboard);
    }
}

fn triangle_area([a, b, c]: [Vec3; 3]) -> f32 {
    (a - b).cross(a - c).length() * 0.5
}

fn sample_triangle<R: Rng>(rng: &mut R, [a, b, c]: [Vec3; 3]) -> Vec3 {
    let mut u = rng.gen_range(0.0..1.0);
    let mut v = rng.gen_range(0.0..1.0);
    if u + v >= 1.0 {
        u = 1.0 - u;
        v = 1.0 - v;
    }

    a + u * (b - a) + v * (c - a)
}

fn triangle_normal([a, b, c]: [Vec3; 3]) -> Vec3 {
    (a - b).cross(a - c).normalize()
}
