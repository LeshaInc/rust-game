use bevy::prelude::*;

pub fn get_barycentric(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> Vec3 {
    let area_abc = ((b - a).cross(c - a)).z;
    let area_pbc = ((b - p).cross(c - p)).z;
    let area_pca = ((c - p).cross(a - p)).z;
    let bary_x = area_pbc / area_abc;
    let bary_y = area_pca / area_abc;
    Vec3::new(bary_x, bary_y, 1.0 - bary_x - bary_y)
}

pub fn is_inside_barycentric(bary: Vec3) -> bool {
    (0.0 <= bary.x && bary.x <= 1.0)
        && (0.0 <= bary.y && bary.y <= 1.0)
        && (0.0 <= bary.z && bary.z <= 1.0)
}
