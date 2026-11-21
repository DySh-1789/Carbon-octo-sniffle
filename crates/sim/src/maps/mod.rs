//! Stage 6 Tactical Map Catalog: 5 distinct `384 × 384 m` WWII battlefields, each staying
//! strictly inside a single `24 × 24` vertical chunk layer (`vy in 0..=31`, `<= 576` chunks,
//! `< 12 MB` sparse voxel RAM) with verified `NavGrid` walkability and atmospheric presets.

pub mod el_alamein;
pub mod karelia;
pub mod normandy;
pub mod stalingrad;

use serde::{Deserialize, Serialize};
use voxel::glam::{IVec3, Vec2, Vec3};
use voxel::{WORLD_VOXEL_SIZE, VoxelId, VoxelWorld};

use crate::map_gen::{
    CaptureZoneDef, MapAtmosphere, MapMetadata, TacticalPosition, generate_european_valley_map,
    generate_minimap_data,
};

/// Standard playable half-extent (`192 m` -> `384 × 384 m` battlefield = `24 × 24` chunks).
pub const MAP_HALF_EXTENT_M: i32 = 192;
pub const MAP_HALF_VOX: i32 = 384;
pub const MAP_MIN_Y: i32 = 0;
pub const MAP_MAX_Y: i32 = 31;

/// Playable Stage 6 map catalog (5 distinct biomes & tactical layouts).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MapId {
    #[default]
    EuropeanValley,
    ElAlamein,
    StalingradWinter,
    KareliaRidge,
    NormandyBocage,
}

impl MapId {
    pub const ALL: [MapId; 5] = [
        MapId::EuropeanValley,
        MapId::ElAlamein,
        MapId::StalingradWinter,
        MapId::KareliaRidge,
        MapId::NormandyBocage,
    ];

    pub fn id(self) -> &'static str {
        match self {
            MapId::EuropeanValley => "european_valley",
            MapId::ElAlamein => "el_alamein",
            MapId::StalingradWinter => "stalingrad_winter",
            MapId::KareliaRidge => "karelia_ridge",
            MapId::NormandyBocage => "normandy_bocage",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MapId::EuropeanValley => "European Valley",
            MapId::ElAlamein => "El Alamein Dunes",
            MapId::StalingradWinter => "Winter Stalingrad",
            MapId::KareliaRidge => "Karelia Ridge",
            MapId::NormandyBocage => "Normandy Bocage",
        }
    }

    pub fn biome_tag(self) -> &'static str {
        match self {
            MapId::EuropeanValley => "TEMPERATE VILLAGE",
            MapId::ElAlamein => "NORTH AFRICAN DESERT",
            MapId::StalingradWinter => "WINTER RUINED CITY",
            MapId::KareliaRidge => "NORDIC PINE CANYON",
            MapId::NormandyBocage => "COASTAL HEDGEROWS",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Generates any of the 5 catalog battlefields `(VoxelWorld, MapMetadata)`.
pub fn generate_map(id: MapId) -> (VoxelWorld, MapMetadata) {
    match id {
        MapId::EuropeanValley => generate_european_valley_map(),
        MapId::ElAlamein => el_alamein::generate_el_alamein_map(),
        MapId::StalingradWinter => stalingrad::generate_stalingrad_map(),
        MapId::KareliaRidge => karelia::generate_karelia_map(),
        MapId::NormandyBocage => normandy::generate_normandy_map(),
    }
}

/// Allocates a standard `384 × 384 m` single-vertical-layer (`vy in 0..=31`) `VoxelWorld`.
pub fn init_standard_world() -> VoxelWorld {
    let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);
    world.bounds_min = IVec3::new(-MAP_HALF_VOX, MAP_MIN_Y, -MAP_HALF_VOX);
    world.bounds_max = IVec3::new(MAP_HALF_VOX - 1, MAP_MAX_Y, MAP_HALF_VOX - 1);
    world
}

/// Smooth Gaussian hill/basin helper reused across procedural heightfields.
#[inline]
pub fn gaussian(x: f32, z: f32, cx: f32, cz: f32, rx: f32, rz: f32) -> f32 {
    let dx = (x - cx) / rx.max(1.0);
    let dz = (z - cz) / rz.max(1.0);
    (-(dx * dx + dz * dz)).exp()
}

/// Deterministic spatial hash in `[0, 1)` for natural tree/debris placement.
#[inline]
pub fn hash2d(ix: i32, iz: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(374_761_393)
        ^ (iz as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

/// Fills all `768 × 768` terrain columns using uniform bedrock beneath 1 subsoil + 1 surface layer
/// so `compact_chunks()` collapses lower slices cleanly (keeping total voxel RAM `< 12 MB`).
pub fn fill_terrain_columns<H, M>(
    world: &mut VoxelWorld,
    bedrock: VoxelId,
    subsoil: VoxelId,
    water_vy: Option<i32>,
    water_mat: VoxelId,
    height_fn: H,
    surface_fn: M,
) where
    H: Fn(f32, f32) -> f32,
    M: Fn(f32, f32, f32) -> VoxelId,
{
    let min_x = world.bounds_min.x;
    let max_x = world.bounds_max.x;
    let min_z = world.bounds_min.z;
    let max_z = world.bounds_max.z;

    for vz in min_z..=max_z {
        let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
        for vx in min_x..=max_x {
            let wx = (vx as f32 + 0.5) * WORLD_VOXEL_SIZE;
            let h_m = height_fn(wx, wz);
            let top_vy = ((h_m / WORLD_VOXEL_SIZE).round() as i32 - 1)
                .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 2);

            if let Some(wvy) = water_vy
                && top_vy < wvy
            {
                world.fill_column_raw(vx, vz, MAP_MIN_Y, (top_vy - 1).max(MAP_MIN_Y), bedrock);
                if top_vy > MAP_MIN_Y {
                    world.set_voxel_raw(IVec3::new(vx, top_vy, vz), subsoil);
                }
                world.fill_column_raw(vx, vz, top_vy + 1, wvy, water_mat);
            } else {
                let subsoil_top = (top_vy - 1).max(MAP_MIN_Y);
                let rock_top = (top_vy - 2).max(MAP_MIN_Y);
                if rock_top >= MAP_MIN_Y {
                    world.fill_column_raw(vx, vz, MAP_MIN_Y, rock_top, bedrock);
                }
                if subsoil_top > rock_top {
                    world.fill_column_raw(vx, vz, rock_top + 1, subsoil_top, subsoil);
                }
                let mat = surface_fn(wx, wz, h_m);
                world.set_voxel_raw(IVec3::new(vx, top_vy, vz), mat);
            }
        }
    }
}

/// Builds a hollow rectangular building with foundation, walls, windows, and optional pitched/flat roof.
/// Stays strictly within `vy <= MAP_MAX_Y` (`31`).
#[allow(clippy::too_many_arguments)]
pub fn build_rect_building<H>(
    world: &mut VoxelWorld,
    height_fn: &H,
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    wall_height_vox: i32,
    wall_mat: VoxelId,
    trim_mat: VoxelId,
    roof_mat: VoxelId,
    pitched_roof: bool,
) where
    H: Fn(f32, f32) -> f32,
{
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let hw = ((width_m / WORLD_VOXEL_SIZE).round() as i32 / 2).max(3);
    let hd = ((depth_m / WORLD_VOXEL_SIZE).round() as i32 / 2).max(3);
    let min_x = (cvx - hw).max(world.bounds_min.x + 2);
    let max_x = (cvx + hw).min(world.bounds_max.x - 2);
    let min_z = (cvz - hd).max(world.bounds_min.z + 2);
    let max_z = (cvz + hd).min(world.bounds_max.z - 2);

    let base_vy = ((height_fn(cx, cz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 6);
    let top_wall = (base_vy + wall_height_vox).min(MAP_MAX_Y - if pitched_roof { 4 } else { 1 });
    let found_bot = (base_vy - 4).max(MAP_MIN_Y);

    for vz in min_z..=max_z {
        for vx in min_x..=max_x {
            world.fill_column_raw(vx, vz, found_bot, base_vy, trim_mat);
            let on_x = vx == min_x || vx == max_x;
            let on_z = vz == min_z || vz == max_z;
            let is_wall = on_x || on_z;
            let is_corner = on_x && on_z;

            for vy in (base_vy + 1)..=top_wall {
                if !is_wall {
                    world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
                    continue;
                }
                let rel_y = vy - base_vy;
                let along = if on_z { vx - min_x } else { vz - min_z };
                let span = if on_z { max_x - min_x } else { max_z - min_z };
                let is_win = (rel_y == 3 || rel_y == 4 || rel_y == 8 || rel_y == 9)
                    && !is_corner
                    && along >= 3
                    && along <= span - 3
                    && (along % 5 == 3 || along % 5 == 4);
                let mat = if is_corner || rel_y == wall_height_vox {
                    trim_mat
                } else if is_win {
                    VoxelId::WINDOW_DARK
                } else {
                    wall_mat
                };
                world.set_voxel_raw(IVec3::new(vx, vy, vz), mat);
            }
        }
    }

    if pitched_roof {
        let gable_x = (max_x - min_x) <= (max_z - min_z);
        let layers = ((if gable_x { hw } else { hd }) / 2 + 1).clamp(3, 5);
        for step in 0..layers {
            let vy = top_wall + 1 + step;
            if vy > MAP_MAX_Y {
                break;
            }
            if gable_x {
                let rx0 = min_x + step;
                let rx1 = max_x - step;
                if rx0 > rx1 {
                    break;
                }
                for vz in min_z..=max_z {
                    for vx in rx0..=rx1 {
                        if vx == rx0 || vx == rx1 || step == layers - 1 {
                            world.set_voxel_raw(IVec3::new(vx, vy, vz), roof_mat);
                        } else if vz == min_z || vz == max_z {
                            world.set_voxel_raw(IVec3::new(vx, vy, vz), wall_mat);
                        }
                    }
                }
            } else {
                let rz0 = min_z + step;
                let rz1 = max_z - step;
                if rz0 > rz1 {
                    break;
                }
                for vx in min_x..=max_x {
                    for vz in rz0..=rz1 {
                        if vz == rz0 || vz == rz1 || step == layers - 1 {
                            world.set_voxel_raw(IVec3::new(vx, vy, vz), roof_mat);
                        } else if vx == min_x || vx == max_x {
                            world.set_voxel_raw(IVec3::new(vx, vy, vz), wall_mat);
                        }
                    }
                }
            }
        }
    } else {
        // Flat roof with parapet lip (ideal for desert adobe houses & concrete bunkers)
        let roof_vy = (top_wall + 1).min(MAP_MAX_Y);
        for vz in min_z..=max_z {
            for vx in min_x..=max_x {
                world.set_voxel_raw(IVec3::new(vx, roof_vy, vz), roof_mat);
                if roof_vy < MAP_MAX_Y
                    && (vx == min_x || vx == max_x || vz == min_z || vz == max_z)
                {
                    world.set_voxel_raw(IVec3::new(vx, roof_vy + 1, vz), wall_mat);
                }
            }
        }
    }
}

/// Builds a bombed-out hollow ruin with jagged broken walls and interior rubble piles (Stalingrad / urban cover).
#[allow(clippy::too_many_arguments)]
pub fn build_ruined_building<H>(
    world: &mut VoxelWorld,
    height_fn: &H,
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    max_wall_vox: i32,
    wall_mat: VoxelId,
    rubble_mat: VoxelId,
    seed: u32,
) where
    H: Fn(f32, f32) -> f32,
{
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let hw = ((width_m / WORLD_VOXEL_SIZE).round() as i32 / 2).max(4);
    let hd = ((depth_m / WORLD_VOXEL_SIZE).round() as i32 / 2).max(4);
    let min_x = (cvx - hw).max(world.bounds_min.x + 2);
    let max_x = (cvx + hw).min(world.bounds_max.x - 2);
    let min_z = (cvz - hd).max(world.bounds_min.z + 2);
    let max_z = (cvz + hd).min(world.bounds_max.z - 2);

    let base_vy = ((height_fn(cx, cz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 6);

    for vz in min_z..=max_z {
        for vx in min_x..=max_x {
            world.fill_column_raw(vx, vz, (base_vy - 3).max(MAP_MIN_Y), base_vy, rubble_mat);
            let on_x = vx == min_x || vx == max_x;
            let on_z = vz == min_z || vz == max_z;
            if on_x || on_z {
                // Jagged bomb-damaged wall heights with blown-out breach gaps
                let n = hash2d(vx / 2, vz / 2, seed);
                let breach = ((vx - cvx).abs() <= 2 && on_z && n < 0.55)
                    || ((vz - cvz).abs() <= 2 && on_x && n > 0.65);
                if breach {
                    // Low rubble sill across blown-out wall breach
                    world.set_voxel_raw(IVec3::new(vx, base_vy + 1, vz), rubble_mat);
                    continue;
                }
                let local_h = ((max_wall_vox as f32) * (0.45 + 0.55 * n)).round() as i32;
                let top = (base_vy + local_h).min(MAP_MAX_Y);
                for vy in (base_vy + 1)..=top {
                    let rel = vy - base_vy;
                    let is_win = (rel == 3 || rel == 4 || rel == 8 || rel == 9)
                        && ((vx + vz).rem_euclid(5) == 2 || (vx + vz).rem_euclid(5) == 3);
                    if !is_win {
                        let mat = if vy == top && n < 0.35 {
                            rubble_mat
                        } else {
                            wall_mat
                        };
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), mat);
                    }
                }
            } else {
                // Sparse interior masonry debris mounds
                let r = hash2d(vx, vz, seed + 17);
                if r > 0.84 {
                    world.set_voxel_raw(IVec3::new(vx, base_vy + 1, vz), rubble_mat);
                }
            }
        }
    }
}

/// Builds a terrain-conforming wall, hedgerow, sandbag parapet, or anti-tank berm.
#[allow(clippy::too_many_arguments)]
pub fn build_wall_segment<H>(
    world: &mut VoxelWorld,
    height_fn: &H,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    height_vox: i32,
    half_thick_vox: i32,
    mat: VoxelId,
) where
    H: Fn(f32, f32) -> f32,
{
    let dist = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt();
    let steps = (dist / (WORLD_VOXEL_SIZE * 0.7)).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let wx = x0 + (x1 - x0) * t;
        let wz = z0 + (z1 - z0) * t;
        let cvx = (wx / WORLD_VOXEL_SIZE).floor() as i32;
        let cvz = (wz / WORLD_VOXEL_SIZE).floor() as i32;
        let base_vy = ((height_fn(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
            .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 1);
        let top_vy = (base_vy + height_vox).min(MAP_MAX_Y);
        for dz in -half_thick_vox..=half_thick_vox {
            for dx in -half_thick_vox..=half_thick_vox {
                world.fill_column_raw(cvx + dx, cvz + dz, base_vy + 1, top_vy, mat);
            }
        }
    }
}

/// Builds a sculpted voxel boulder / rocky outcrop on any heightfield.
pub fn build_rock<H>(
    world: &mut VoxelWorld,
    height_fn: &H,
    cx: f32,
    cz: f32,
    radius_m: f32,
    height_m: f32,
    mat: VoxelId,
) where
    H: Fn(f32, f32) -> f32,
{
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let rv = (radius_m / WORLD_VOXEL_SIZE).ceil() as i32;
    let hv = (height_m / WORLD_VOXEL_SIZE).ceil() as i32;
    let base_vy = ((height_fn(cx, cz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 2);

    for dz in -rv..=rv {
        for dx in -rv..=rv {
            let nx = dx as f32 / rv.max(1) as f32;
            let nz = dz as f32 / rv.max(1) as f32;
            let d2 = nx * nx + nz * nz;
            if d2 > 1.0 {
                continue;
            }
            let bump = 0.85 + 0.15 * ((dx * 3 + dz * 5) as f32).sin();
            let col_h = (((1.0 - d2).sqrt() * hv as f32) * bump).round() as i32;
            let top = (base_vy + col_h).min(MAP_MAX_Y);
            if top > base_vy {
                world.fill_column_raw(cvx + dx, cvz + dz, base_vy, top, mat);
            }
        }
    }
}

/// Builds a tall conical Nordic pine / spruce tree (Karelia / Normandy / Winter).
pub fn build_pine_tree<H>(
    world: &mut VoxelWorld,
    height_fn: &H,
    wx: f32,
    wz: f32,
    trunk_h: i32,
    foliage: VoxelId,
) where
    H: Fn(f32, f32) -> f32,
{
    let vx = (wx / WORLD_VOXEL_SIZE).round() as i32;
    let vz = (wz / WORLD_VOXEL_SIZE).round() as i32;
    let base_vy = ((height_fn(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 6);
    let top_trunk = (base_vy + trunk_h).min(MAP_MAX_Y - 2);
    world.fill_column_raw(vx, vz, base_vy + 1, top_trunk, VoxelId::TREE_TRUNK);

    for tier in 0..5 {
        let vy = base_vy + 3 + tier * 2;
        if vy > MAP_MAX_Y {
            break;
        }
        let r = (3 - tier / 2).max(1);
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() + dz.abs() <= r + 1 {
                    world.set_voxel_raw(IVec3::new(vx + dx, vy, vz + dz), foliage);
                    if vy < MAP_MAX_Y && dx.abs() + dz.abs() <= r {
                        world.set_voxel_raw(IVec3::new(vx + dx, vy + 1, vz + dz), foliage);
                    }
                }
            }
        }
    }
    if base_vy + 13 <= MAP_MAX_Y {
        world.set_voxel_raw(IVec3::new(vx, base_vy + 13, vz), foliage);
    }
}

/// Builds a North African oasis palm tree (slender trunk + wide crown fronds).
pub fn build_palm_tree<H>(world: &mut VoxelWorld, height_fn: &H, wx: f32, wz: f32)
where
    H: Fn(f32, f32) -> f32,
{
    let vx = (wx / WORLD_VOXEL_SIZE).round() as i32;
    let vz = (wz / WORLD_VOXEL_SIZE).round() as i32;
    let base_vy = ((height_fn(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 12);
    let crown_y = (base_vy + 10).min(MAP_MAX_Y - 2);
    world.fill_column_raw(vx, vz, base_vy + 1, crown_y, VoxelId::TREE_TRUNK);

    for &(dx, dz) in &[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
        for step in 1..=4 {
            let fy = crown_y + if step <= 2 { 1 } else { 0 } - if step == 4 { 1 } else { 0 };
            if fy <= MAP_MAX_Y {
                world.set_voxel_raw(
                    IVec3::new(vx + dx * step, fy, vz + dz * step),
                    VoxelId::LEAVES,
                );
            }
        }
    }
    if crown_y < MAP_MAX_Y {
        world.set_voxel_raw(IVec3::new(vx, crown_y + 1, vz), VoxelId::LEAVES);
    }
}

/// Paints a flush capture-zone perimeter ring on the ground surface.
pub fn paint_zone_circle(world: &mut VoxelWorld, cx: f32, cz: f32, radius_m: f32) {
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let rv = (radius_m / WORLD_VOXEL_SIZE).ceil() as i32;

    for dz in -rv..=rv {
        for dx in -rv..=rv {
            let dist = ((dx * dx + dz * dz) as f32).sqrt() * WORLD_VOXEL_SIZE;
            if (dist - radius_m).abs() <= 0.45 {
                let vx = cvx + dx;
                let vz = cvz + dz;
                if let Some((vy, id)) = world.top_solid_voxel_at(vx, vz)
                    && id != VoxelId::WATER
                {
                    world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::RANGE_MARKER);
                }
            }
        }
    }
}

/// Finalizes chunk compaction, dirty flags, accurate spawn/zone heights, and 192×192 minimap.
pub fn finalize_map(
    mut world: VoxelWorld,
    id: MapId,
    atmosphere: MapAtmosphere,
    allies_xz: &[(f32, f32)],
    axis_xz: &[(f32, f32)],
    zones_xz: &[(char, f32, f32, f32)],
    positions_xz: &[(&str, f32, f32, crate::map_gen::TacticalPositionKind)],
) -> (VoxelWorld, MapMetadata) {
    for &(_, zx, zz, r) in zones_xz {
        paint_zone_circle(&mut world, zx, zz, r);
    }
    world.compact_chunks();
    world.mark_all_dirty();

    let sample = |x: f32, z: f32| -> Vec3 {
        let y = world.surface_height_at(x, z).unwrap_or(0.0);
        Vec3::new(x, y, z)
    };

    let spawn_allies = allies_xz.iter().map(|&(x, z)| sample(x, z)).collect();
    let spawn_axis = axis_xz.iter().map(|&(x, z)| sample(x, z)).collect();
    let capture_zones = zones_xz
        .iter()
        .map(|&(label, x, z, radius)| CaptureZoneDef {
            label,
            center: sample(x, z),
            radius,
        })
        .collect();
    let tactical_positions = positions_xz
        .iter()
        .map(|&(name, x, z, kind)| TacticalPosition {
            name: name.to_string(),
            position: sample(x, z),
            kind,
        })
        .collect();

    let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
    let minimap = generate_minimap_data(&world, 192);
    let meta = MapMetadata {
        id: id.id().to_string(),
        name: id.name().to_string(),
        size_m: max_x - min_x,
        bounds_min_xz: Vec2::new(min_x, min_z),
        bounds_max_xz: Vec2::new(max_x, max_z),
        spawn_allies,
        spawn_axis,
        capture_zones,
        tactical_positions,
        minimap,
        atmosphere,
    };
    (world, meta)
}
