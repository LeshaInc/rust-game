use rand::Rng;
use rg_core::Grid;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Biome {
    Ocean,
    Plains,
    Forest,
}

pub fn generate_biomes<R: Rng>(rng: &mut R, elevation: &Grid<f32>) -> Grid<Biome> {
    let size = elevation.size();

    let mut biomes = Grid::new(size, Biome::Ocean);

    let mut noise = Grid::new(size, 0.0);
    noise.add_fbm_noise(rng, 0.1, 1.0, 3);

    for cell in biomes.cells() {
        if elevation[cell] < 0.0 {
            continue;
        }

        biomes[cell] = if noise[cell] > 0.5 {
            Biome::Forest
        } else {
            Biome::Plains
        }
    }

    biomes
}
