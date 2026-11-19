use voxel::glam::IVec3;
use voxel::{WORLD_VOXEL_SIZE, VoxelId, VoxelWorld};

/// Generates the Stage 1 flat proving ground (`256m × 256m` by default when `half_extent_m = 128`).
///
/// Ground surface sits flush at `Y = 0.0` (solid ground layers occupy `vy = -3..=-1`).
/// Flat painted concrete distance markers at `50 m`, `100 m`, and `150 m` along `-Z` help the
/// player visually judge driving speed and ballistic drop while keeping the arena completely flat.
pub fn generate_flat_proving_ground(half_extent_m: i32) -> VoxelWorld {
    let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);
    let half_vox = (half_extent_m as f32 / WORLD_VOXEL_SIZE).round() as i32;

    let min_x = -half_vox;
    let max_x = half_vox - 1;
    let min_z = -half_vox;
    let max_z = half_vox - 1;

    world.bounds_min = IVec3::new(min_x, -3, min_z);
    world.bounds_max = IVec3::new(max_x, 0, max_z);

    // Subsoil dirt foundation (`vy = -3..=-2`)
    world.fill_box(
        IVec3::new(min_x, -3, min_z),
        IVec3::new(max_x, -2, max_z),
        VoxelId::DIRT,
    );

    // Flat top grass layer (`vy = -1`, top face at Y = 0.0)
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(max_x, -1, max_z),
        VoxelId::GRASS,
    );

    // Subtle mowed proving-ground grass lanes every 16m for speed perception
    let stripe_width = 8; // 4m wide stripes
    for x in min_x..=max_x {
        if x.rem_euclid(32) < stripe_width {
            world.fill_box(
                IVec3::new(x, -1, min_z),
                IVec3::new(x, -1, max_z),
                VoxelId::GRASS_DARK,
            );
        }
    }

    // Flat concrete starting pad centered around origin (`16m × 16m`)
    world.fill_box(IVec3::new(-16, -1, -16), IVec3::new(15, -1, 15), VoxelId::CONCRETE);
    world.fill_box(IVec3::new(-16, -1, -16), IVec3::new(15, -1, -15), VoxelId::CONCRETE_DARK);
    world.fill_box(IVec3::new(-16, -1, 14), IVec3::new(15, -1, 15), VoxelId::CONCRETE_DARK);
    world.fill_box(IVec3::new(-16, -1, -16), IVec3::new(-15, -1, 15), VoxelId::CONCRETE_DARK);
    world.fill_box(IVec3::new(14, -1, -16), IVec3::new(15, -1, 15), VoxelId::CONCRETE_DARK);

    // Flat concrete runway lane heading forward (-Z) for firing-range orientation
    let lane_end_z = (min_z + 12).min(-40);
    world.fill_box(
        IVec3::new(-6, -1, lane_end_z),
        IVec3::new(5, -1, -17),
        VoxelId::CONCRETE_DARK,
    );

    // Flush range marker strips at 50m, 100m, and 150m along -Z (all flat at vy = -1)
    for dist_m in [50, 100, 150] {
        if dist_m < half_extent_m - 6 {
            let vz = -(dist_m as f32 / WORLD_VOXEL_SIZE).round() as i32;
            world.fill_box(
                IVec3::new(-24, -1, vz),
                IVec3::new(23, -1, vz + 1),
                VoxelId::RANGE_MARKER,
            );
        }
    }

    // Flat arena boundary strip (flush yellow/dark perimeter so player sees the map edge)
    let border = 3;
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(max_x, -1, min_z + border),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(min_x, -1, max_z - border),
        IVec3::new(max_x, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(min_x + border, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(max_x - border, -1, min_z),
        IVec3::new(max_x, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );

    world.mark_all_dirty();
    world
}
