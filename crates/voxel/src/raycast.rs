use glam::{IVec3, Vec3};

use crate::chunk::VoxelWorld;
use crate::material::VoxelId;

/// Result of a 3D DDA voxel raycast against `VoxelWorld`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VoxelHit {
    /// Grid coordinates of the hit solid voxel.
    pub voxel: IVec3,
    /// Outward surface normal of the entered voxel face (or `IVec3::ZERO` if ray started inside solid).
    pub normal: IVec3,
    /// Distance in meters from ray `origin` to `point`.
    pub distance: f32,
    /// World-space impact point in meters.
    pub point: Vec3,
    /// Material ID of the hit voxel.
    pub material: VoxelId,
}

/// Exact 3D Amanatides & Woo DDA raymarch through `VoxelWorld`.
///
/// `dir` does not need to be normalized (returns `None` if its length is near zero).
pub fn raycast_world(
    world: &VoxelWorld,
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
) -> Option<VoxelHit> {
    let dir_len = dir.length();
    if dir_len <= 1e-6 || max_distance <= 0.0 {
        return None;
    }
    let dir = dir / dir_len;
    let vs = world.voxel_size;
    let inv_vs = 1.0 / vs;

    // Convert ray origin into voxel-grid units where each voxel spans `[i, i+1)`
    let grid_origin = origin * inv_vs;
    let max_t_grid = max_distance * inv_vs;

    let mut voxel = IVec3::new(
        grid_origin.x.floor() as i32,
        grid_origin.y.floor() as i32,
        grid_origin.z.floor() as i32,
    );

    // Handle starting inside a solid voxel
    let start_mat = world.get_voxel(voxel);
    if world.materials.is_solid(start_mat) {
        return Some(VoxelHit {
            voxel,
            normal: IVec3::ZERO,
            distance: 0.0,
            point: origin,
            material: start_mat,
        });
    }

    let step = IVec3::new(
        if dir.x > 0.0 { 1 } else if dir.x < 0.0 { -1 } else { 0 },
        if dir.y > 0.0 { 1 } else if dir.y < 0.0 { -1 } else { 0 },
        if dir.z > 0.0 { 1 } else if dir.z < 0.0 { -1 } else { 0 },
    );

    let t_delta = Vec3::new(
        if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::INFINITY },
        if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::INFINITY },
        if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::INFINITY },
    );

    let mut t_max = Vec3::new(
        if dir.x > 0.0 {
            ((voxel.x + 1) as f32 - grid_origin.x) / dir.x
        } else if dir.x < 0.0 {
            (voxel.x as f32 - grid_origin.x) / dir.x
        } else {
            f32::INFINITY
        },
        if dir.y > 0.0 {
            ((voxel.y + 1) as f32 - grid_origin.y) / dir.y
        } else if dir.y < 0.0 {
            (voxel.y as f32 - grid_origin.y) / dir.y
        } else {
            f32::INFINITY
        },
        if dir.z > 0.0 {
            ((voxel.z + 1) as f32 - grid_origin.z) / dir.z
        } else if dir.z < 0.0 {
            (voxel.z as f32 - grid_origin.z) / dir.z
        } else {
            f32::INFINITY
        },
    );

    // Upper iteration cap proportional to max distance to prevent infinite loops on degenerate inputs
    let max_steps = (max_t_grid.ceil() as usize) * 3 + 16;
    for _ in 0..max_steps {
        let (t_grid, normal) = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            let t = t_max.x;
            t_max.x += t_delta.x;
            voxel.x += step.x;
            (t, IVec3::new(-step.x, 0, 0))
        } else if t_max.y <= t_max.z {
            let t = t_max.y;
            t_max.y += t_delta.y;
            voxel.y += step.y;
            (t, IVec3::new(0, -step.y, 0))
        } else {
            let t = t_max.z;
            t_max.z += t_delta.z;
            voxel.z += step.z;
            (t, IVec3::new(0, 0, -step.z))
        };

        if t_grid > max_t_grid {
            return None;
        }

        let mat = world.get_voxel(voxel);
        if world.materials.is_solid(mat) {
            let distance = t_grid * vs;
            return Some(VoxelHit {
                voxel,
                normal,
                distance,
                point: origin + dir * distance,
                material: mat,
            });
        }
    }

    None
}
