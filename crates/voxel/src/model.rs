use glam::{IVec3, UVec3, Vec3};

use crate::chunk::TANK_VOXEL_SIZE;
use crate::material::VoxelId;

/// Bounded fine-scale voxel grid used for tank assemblies (hull, turret, gun barrel).
#[derive(Clone, Debug)]
pub struct VoxelModel {
    pub size: UVec3,
    /// Pivot offset in voxel coordinates; vertex positions are shifted by `-pivot * voxel_size`.
    pub pivot: Vec3,
    pub voxel_size: f32,
    voxels: Vec<VoxelId>,
}

impl VoxelModel {
    pub fn new(size: UVec3, pivot: Vec3, voxel_size: f32) -> Self {
        let len = (size.x * size.y * size.z) as usize;
        Self {
            size,
            pivot,
            voxel_size,
            voxels: vec![VoxelId::AIR; len],
        }
    }

    #[inline]
    fn idx(&self, x: u32, y: u32, z: u32) -> usize {
        (x + z * self.size.x + y * self.size.x * self.size.z) as usize
    }

    #[inline]
    pub fn in_bounds(&self, pos: IVec3) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && pos.z >= 0
            && (pos.x as u32) < self.size.x
            && (pos.y as u32) < self.size.y
            && (pos.z as u32) < self.size.z
    }

    #[inline]
    pub fn get(&self, pos: IVec3) -> VoxelId {
        if !self.in_bounds(pos) {
            return VoxelId::AIR;
        }
        self.voxels[self.idx(pos.x as u32, pos.y as u32, pos.z as u32)]
    }

    #[inline]
    pub fn set(&mut self, pos: IVec3, id: VoxelId) {
        if !self.in_bounds(pos) {
            return;
        }
        let idx = self.idx(pos.x as u32, pos.y as u32, pos.z as u32);
        self.voxels[idx] = id;
    }

    /// Inclusive integer box fill.
    pub fn fill_box(&mut self, min: IVec3, max: IVec3, id: VoxelId) {
        let min = min.max(IVec3::ZERO);
        let max = max.min(self.size.as_ivec3() - IVec3::ONE);
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                for x in min.x..=max.x {
                    self.set(IVec3::new(x, y, z), id);
                }
            }
        }
    }

    pub fn non_air_count(&self) -> usize {
        self.voxels.iter().filter(|v| !v.is_air()).count()
    }
}

/// Complete three-part voxel assembly for a tank (local forward is `-Z`, up is `+Y`).
#[derive(Clone, Debug)]
pub struct TankVoxelModels {
    pub hull: VoxelModel,
    pub turret: VoxelModel,
    pub barrel: VoxelModel,
    /// Offset from hull root (ground contact center) to turret ring pivot in meters.
    pub turret_mount_offset: Vec3,
    /// Offset from turret pivot to gun trunnion pivot in meters.
    pub gun_mount_offset: Vec3,
    /// Offset from gun trunnion pivot to barrel muzzle tip in meters (`-Z` direction).
    pub muzzle_offset: Vec3,
}

/// Generates the Stage 1 base medium tank at `0.2 m` voxel resolution.
///
/// Coordinate orientation: `-Z` is forward, `+X` is right, `+Y` is up.
pub fn build_basic_tank_models() -> TankVoxelModels {
    let vs = TANK_VOXEL_SIZE; // 0.2 m

    // ------------------------------------------------------------------
    // 1. HULL: 16 wide (3.2m) x 8 tall (1.6m) x 30 long (6.0m)
    //    Pivot at X=8.0, Y=0.0 (bottom of tracks touches ground Y=0), Z=15.0
    //    Forward = low Z (0..15), Rear = high Z (15..30)
    // ------------------------------------------------------------------
    let mut hull = VoxelModel::new(UVec3::new(16, 8, 30), Vec3::new(8.0, 0.0, 15.0), vs);

    // Left & right tracks (x = 0..=2 and x = 13..=15)
    for &(tx0, tx1) in &[(0, 2), (13, 15)] {
        // Bottom track run
        hull.fill_box(IVec3::new(tx0, 0, 3), IVec3::new(tx1, 0, 26), VoxelId::STEEL_TRACK);
        // Sloped front and rear track ascents
        hull.fill_box(IVec3::new(tx0, 1, 1), IVec3::new(tx1, 2, 2), VoxelId::STEEL_TRACK);
        hull.fill_box(IVec3::new(tx0, 2, 0), IVec3::new(tx1, 3, 1), VoxelId::STEEL_TRACK);
        hull.fill_box(IVec3::new(tx0, 1, 27), IVec3::new(tx1, 2, 28), VoxelId::STEEL_TRACK);
        hull.fill_box(IVec3::new(tx0, 2, 28), IVec3::new(tx1, 3, 29), VoxelId::STEEL_TRACK);
        // Top track run
        hull.fill_box(IVec3::new(tx0, 3, 1), IVec3::new(tx1, 3, 28), VoxelId::STEEL_TRACK);
        // Side track walls + road wheels
        hull.fill_box(IVec3::new(tx0, 1, 3), IVec3::new(tx1, 2, 26), VoxelId::STEEL_OLIVE_DARK);
        for wz in [5, 9, 13, 17, 21, 25] {
            hull.fill_box(
                IVec3::new(tx0, 1, wz - 1),
                IVec3::new(tx1, 2, wz),
                VoxelId::RUBBER_WHEEL,
            );
            // Hubcap accent on outer side
            let outer_x = if tx0 == 0 { 0 } else { 15 };
            hull.set(IVec3::new(outer_x, 1, wz), VoxelId::STEEL_ACCENT);
        }
        // Armored track fenders / mudguards
        hull.fill_box(IVec3::new(tx0, 4, 1), IVec3::new(tx1, 4, 28), VoxelId::STEEL_OLIVE);
    }

    // Lower hull tub between tracks (x = 3..=12)
    hull.fill_box(IVec3::new(3, 1, 4), IVec3::new(12, 3, 27), VoxelId::STEEL_OLIVE_DARK);
    // Lower front glacis step
    hull.fill_box(IVec3::new(3, 2, 2), IVec3::new(12, 3, 3), VoxelId::STEEL_OLIVE_DARK);
    // Main upper hull fighting compartment & engine deck
    hull.fill_box(IVec3::new(3, 4, 4), IVec3::new(12, 6, 27), VoxelId::STEEL_OLIVE);
    // Sloped upper front glacis (iconic stepped voxel slope toward -Z)
    hull.fill_box(IVec3::new(3, 4, 1), IVec3::new(12, 4, 3), VoxelId::STEEL_OLIVE);
    hull.fill_box(IVec3::new(3, 5, 2), IVec3::new(12, 5, 4), VoxelId::STEEL_OLIVE);
    hull.fill_box(IVec3::new(3, 6, 5), IVec3::new(12, 6, 26), VoxelId::STEEL_OLIVE);
    // Slight sloped upper hull sides (chamfered top edges)
    hull.fill_box(IVec3::new(2, 5, 5), IVec3::new(2, 5, 26), VoxelId::STEEL_OLIVE);
    hull.fill_box(IVec3::new(13, 5, 5), IVec3::new(13, 5, 26), VoxelId::STEEL_OLIVE);

    // Driver vision slit on upper glacis (left-front)
    hull.fill_box(IVec3::new(4, 5, 2), IVec3::new(6, 5, 2), VoxelId::GLASS_OPTIC);
    // Headlights on front fenders
    hull.set(IVec3::new(1, 5, 2), VoxelId::STEEL_ACCENT);
    hull.set(IVec3::new(14, 5, 2), VoxelId::STEEL_ACCENT);

    // Rear engine deck cooling grilles & exhaust pipes
    for gx in [4, 6, 9, 11] {
        hull.fill_box(IVec3::new(gx, 6, 20), IVec3::new(gx, 6, 25), VoxelId::STEEL_TRACK);
    }
    hull.fill_box(IVec3::new(5, 5, 28), IVec3::new(6, 5, 28), VoxelId::STEEL_TRACK);
    hull.fill_box(IVec3::new(9, 5, 28), IVec3::new(10, 5, 28), VoxelId::STEEL_TRACK);

    // Turret ring collar on hull roof (centered around Z=13)
    hull.fill_box(IVec3::new(5, 7, 10), IVec3::new(10, 7, 16), VoxelId::STEEL_OLIVE_DARK);

    // ------------------------------------------------------------------
    // 2. TURRET: 10 wide (2.0m) x 6 tall (1.2m) x 14 long (2.8m)
    //    Pivot at X=5.0, Y=0.0, Z=7.0 (mounts right onto turret ring)
    // ------------------------------------------------------------------
    let mut turret = VoxelModel::new(UVec3::new(10, 6, 14), Vec3::new(5.0, 0.0, 7.0), vs);

    // Main armored turret body
    turret.fill_box(IVec3::new(1, 0, 2), IVec3::new(8, 3, 11), VoxelId::STEEL_OLIVE);
    // Cheek bevels & wider rear bustle
    turret.fill_box(IVec3::new(0, 0, 4), IVec3::new(9, 2, 10), VoxelId::STEEL_OLIVE);
    turret.fill_box(IVec3::new(2, 0, 1), IVec3::new(7, 3, 2), VoxelId::STEEL_OLIVE);
    // Rear turret bustle stowage box
    turret.fill_box(IVec3::new(2, 1, 12), IVec3::new(7, 3, 13), VoxelId::STEEL_OLIVE_DARK);
    // Sloped turret roof cap
    turret.fill_box(IVec3::new(2, 4, 3), IVec3::new(7, 4, 10), VoxelId::STEEL_OLIVE);
    // Gun mantlet armor block at front (z = 0..=1)
    turret.fill_box(IVec3::new(3, 1, 0), IVec3::new(6, 3, 1), VoxelId::STEEL_OLIVE_DARK);
    // Commander's cupola (offset slightly left-rear on roof)
    turret.fill_box(IVec3::new(2, 5, 7), IVec3::new(4, 5, 9), VoxelId::STEEL_OLIVE_DARK);
    turret.set(IVec3::new(3, 5, 7), VoxelId::GLASS_OPTIC);
    // Loader hatch accent on right
    turret.fill_box(IVec3::new(6, 4, 7), IVec3::new(7, 4, 8), VoxelId::STEEL_OLIVE_DARK);
    // Side turret accent stripe
    turret.fill_box(IVec3::new(0, 1, 6), IVec3::new(0, 1, 8), VoxelId::STEEL_ACCENT);
    turret.fill_box(IVec3::new(9, 1, 6), IVec3::new(9, 1, 8), VoxelId::STEEL_ACCENT);

    // ------------------------------------------------------------------
    // 3. GUN BARREL: 3 wide x 3 tall x 20 long (4.0m)
    //    Pivot at X=1.5, Y=1.5, Z=19.0 (trunnion at rear of barrel; extends along -Z)
    // ------------------------------------------------------------------
    let mut barrel = VoxelModel::new(UVec3::new(3, 3, 20), Vec3::new(1.5, 1.5, 19.0), vs);

    // Thick mantlet collar near trunnion (z = 16..=19)
    barrel.fill_box(IVec3::new(0, 0, 16), IVec3::new(2, 2, 19), VoxelId::STEEL_OLIVE_DARK);
    // Tapered reinforced breech tube (z = 12..=15)
    barrel.fill_box(IVec3::new(1, 1, 1), IVec3::new(1, 1, 15), VoxelId::STEEL_GUN);
    // Slightly thicker sleeve at base
    barrel.fill_box(IVec3::new(0, 1, 12), IVec3::new(2, 1, 15), VoxelId::STEEL_GUN);
    barrel.fill_box(IVec3::new(1, 0, 12), IVec3::new(1, 2, 15), VoxelId::STEEL_GUN);
    // Muzzle brake at barrel tip (z = 0..=1)
    barrel.fill_box(IVec3::new(0, 1, 0), IVec3::new(2, 1, 1), VoxelId::STEEL_TRACK);
    barrel.set(IVec3::new(1, 0, 0), VoxelId::STEEL_TRACK);
    barrel.set(IVec3::new(1, 2, 0), VoxelId::STEEL_TRACK);

    // Mount offsets in meters
    // Turret sits on hull top at y = 7.0 * 0.2 = 1.4m, slightly forward of hull center (z = 13 - 15 = -2 voxels = -0.4m)
    let turret_mount_offset = Vec3::new(0.0, 7.0 * vs, -2.0 * vs);
    // Gun trunnion sits in turret mantlet center: y = 2.0 * 0.2 = 0.4m, z = -6.5 * 0.2 = -1.3m
    let gun_mount_offset = Vec3::new(0.0, 2.0 * vs, -6.5 * vs);
    // Barrel extends 19 voxels forward along -Z => 3.8m
    let muzzle_offset = Vec3::new(0.0, 0.0, -19.5 * vs);

    TankVoxelModels {
        hull,
        turret,
        barrel,
        turret_mount_offset,
        gun_mount_offset,
        muzzle_offset,
    }
}
