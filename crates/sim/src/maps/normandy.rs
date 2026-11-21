//! Stage 6 Map: **Normandy Bocage** (`normandy_bocage`, `384 × 384 m`, `24 × 24` chunks).
//!
//! Lush French coastal hedgerow countryside featuring:
//! - Patchwork Norman pastures (`GRASS` / `GRASS_DARK`) separated by 1.5 m bocage hedgerows
//!   (`HEDGE`) and low cobblestone farm walls with wide 16–22 m tank lane gaps
//! - Gently rolling sunken country lanes (`DIRT`) and coastal sand dunes along the far east edge
//! - Zone A (`-108, 6`): Walled West Apple Orchard & Timber/Stone Norman Farmstead
//! - Zone B (`0, 0`): Central Norman Crossroads Village with an open cobblestone roundabout
//! - Zone C (`108, -6`): East Atlantic Wall Bunker Hill fortified with concrete casemates

use voxel::glam::IVec3;
use voxel::{WORLD_VOXEL_SIZE, VoxelId, VoxelWorld};

use crate::map_gen::{MapAtmosphere, MapMetadata, TacticalPositionKind};
use crate::maps::{
    MAP_MAX_Y, MAP_MIN_Y, MapId, build_pine_tree, build_rect_building, build_rock,
    build_wall_segment, fill_terrain_columns, finalize_map, gaussian, hash2d, init_standard_world,
};

// Stage 6 MaterialRegistry voxel IDs (registered in `MaterialRegistry::new()` at indices 43..=53).
const VOXEL_SAND: VoxelId = VoxelId(43);
const VOXEL_SAND_DARK: VoxelId = VoxelId(44);
const VOXEL_PINE_LEAVES: VoxelId = VoxelId(51);
const VOXEL_BUNKER_CONCRETE: VoxelId = VoxelId(53);

const ALLIES_SPAWNS: [(f32, f32); 6] = [
    (0.0, 148.0),
    (-18.0, 152.0),
    (18.0, 152.0),
    (-38.0, 148.0),
    (38.0, 148.0),
    (0.0, 162.0),
];

const AXIS_SPAWNS: [(f32, f32); 6] = [
    (0.0, -148.0),
    (-18.0, -152.0),
    (18.0, -152.0),
    (-38.0, -148.0),
    (38.0, -148.0),
    (0.0, -162.0),
];

const CAPTURE_ZONES: [(char, f32, f32, f32); 3] = [
    ('A', -108.0, 6.0, 16.0),
    ('B', 0.0, 0.0, 16.0),
    ('C', 108.0, -6.0, 16.0),
];

/// Rectangular building or Atlantic Wall bunker specification.
#[derive(Copy, Clone)]
struct StructureSpec {
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    wall_height_vox: i32,
    wall_mat: VoxelId,
    trim_mat: VoxelId,
    roof_mat: VoxelId,
    pitched_roof: bool,
}

impl StructureSpec {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        cx: f32,
        cz: f32,
        width_m: f32,
        depth_m: f32,
        wall_height_vox: i32,
        wall_mat: VoxelId,
        trim_mat: VoxelId,
        roof_mat: VoxelId,
        pitched_roof: bool,
    ) -> Self {
        Self {
            cx,
            cz,
            width_m,
            depth_m,
            wall_height_vox,
            wall_mat,
            trim_mat,
            roof_mat,
            pitched_roof,
        }
    }
}

/// Generates the Stage 6 **"Normandy Bocage"** (`384 × 384 m`) tactical battlefield.
pub fn generate_normandy_map() -> (VoxelWorld, MapMetadata) {
    let mut world = init_standard_world();

    // 1. Gently rolling Norman countryside heightfield (3.0..=9.5 m) & patchwork surface materials
    fill_terrain_columns(
        &mut world,
        VoxelId::ROCK,
        VoxelId::DIRT,
        None,
        VoxelId::WATER,
        normandy_height_m,
        select_surface_material,
    );

    // 2. Central Crossroads Village (Zone B), West Orchard Farmstead (Zone A),
    //    East Atlantic Wall Bunkers (Zone C), and two outlying bocage farm cottages.
    let structures = [
        // --- Central Norman Crossroads Village (10 farmhouses framing the open Zone B roundabout) ---
        StructureSpec::new(
            -25.0,
            -24.0,
            11.0,
            9.0,
            9,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            -43.0,
            -22.0,
            10.0,
            11.0,
            8,
            VoxelId::COBBLESTONE,
            VoxelId::PLASTER_WALL,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            -26.0,
            -44.0,
            10.0,
            9.0,
            7,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            25.0,
            -24.0,
            11.0,
            9.0,
            9,
            VoxelId::COBBLESTONE,
            VoxelId::PLASTER_WALL,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            43.0,
            -24.0,
            10.0,
            10.0,
            7,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            26.0,
            -44.0,
            11.0,
            9.0,
            8,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            -25.0,
            24.0,
            11.0,
            9.0,
            8,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            -43.0,
            26.0,
            10.0,
            10.0,
            7,
            VoxelId::COBBLESTONE,
            VoxelId::PLASTER_WALL,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            25.0,
            24.0,
            11.0,
            9.0,
            9,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            43.0,
            25.0,
            10.0,
            10.0,
            8,
            VoxelId::COBBLESTONE,
            VoxelId::PLASTER_WALL,
            VoxelId::ROOF_TILE,
            true,
        ),
        // --- West Orchard & Farmstead (Zone A, timber & stone barns) ---
        StructureSpec::new(
            -130.0,
            -18.0,
            12.0,
            9.0,
            8,
            VoxelId::COBBLESTONE,
            VoxelId::WOOD,
            VoxelId::ROOF_SLATE,
            true,
        ),
        StructureSpec::new(
            -132.0,
            28.0,
            11.0,
            14.0,
            8,
            VoxelId::WOOD,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            -84.0,
            -18.0,
            11.0,
            9.0,
            7,
            VoxelId::WOOD,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            -84.0,
            29.0,
            10.0,
            9.0,
            7,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_SLATE,
            true,
        ),
        // --- East Atlantic Wall Bunker Hill (Zone C, reinforced concrete casemates) ---
        StructureSpec::new(
            136.0,
            -6.0,
            10.0,
            12.0,
            7,
            VOXEL_BUNKER_CONCRETE,
            VoxelId::CONCRETE_DARK,
            VOXEL_BUNKER_CONCRETE,
            false,
        ),
        StructureSpec::new(
            130.0,
            -28.0,
            12.0,
            12.0,
            7,
            VOXEL_BUNKER_CONCRETE,
            VoxelId::CONCRETE_DARK,
            VOXEL_BUNKER_CONCRETE,
            false,
        ),
        StructureSpec::new(
            131.0,
            17.0,
            12.0,
            13.0,
            7,
            VOXEL_BUNKER_CONCRETE,
            VoxelId::CONCRETE_DARK,
            VOXEL_BUNKER_CONCRETE,
            false,
        ),
        StructureSpec::new(
            84.0,
            -28.0,
            11.0,
            9.0,
            6,
            VOXEL_BUNKER_CONCRETE,
            VoxelId::CONCRETE_DARK,
            VOXEL_BUNKER_CONCRETE,
            false,
        ),
        StructureSpec::new(
            84.0,
            17.0,
            11.0,
            9.0,
            6,
            VOXEL_BUNKER_CONCRETE,
            VoxelId::CONCRETE_DARK,
            VOXEL_BUNKER_CONCRETE,
            false,
        ),
        // --- Outlying bocage field cottages ---
        StructureSpec::new(
            -84.0,
            106.0,
            10.0,
            9.0,
            7,
            VoxelId::PLASTER_WALL,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_TILE,
            true,
        ),
        StructureSpec::new(
            84.0,
            -106.0,
            11.0,
            9.0,
            7,
            VoxelId::WOOD,
            VoxelId::COBBLESTONE,
            VoxelId::ROOF_SLATE,
            true,
        ),
    ];

    for s in &structures {
        build_rect_building(
            &mut world,
            &normandy_height_m,
            s.cx,
            s.cz,
            s.width_m,
            s.depth_m,
            s.wall_height_vox,
            s.wall_mat,
            s.trim_mat,
            s.roof_mat,
            s.pitched_roof,
        );
    }

    // 3. Signature Normandy Bocage Hedgerows (1.5 m = 3 vox tall, 1.5 m thick)
    //    Divides patchwork pastures into ambush lanes while guaranteeing 18–24 m tank gaps
    //    along x = -108, -56, 0, +56, +108 and z = -74, 0, +74.
    let hedgerows: &[(f32, f32, f32, f32)] = &[
        // E-W hedgerow rows (z = ±58, ±92, ±120)
        // North inner row (z = -58)
        (-164.0, -58.0, -120.0, -58.0),
        (-96.0, -58.0, -66.0, -58.0),
        (-46.0, -58.0, -12.0, -58.0),
        (12.0, -58.0, 46.0, -58.0),
        (66.0, -58.0, 96.0, -58.0),
        (120.0, -58.0, 138.0, -58.0),
        // South inner row (z = +58)
        (-164.0, 58.0, -120.0, 58.0),
        (-96.0, 58.0, -66.0, 58.0),
        (-46.0, 58.0, -12.0, 58.0),
        (12.0, 58.0, 46.0, 58.0),
        (66.0, 58.0, 96.0, 58.0),
        (120.0, 58.0, 138.0, 58.0),
        // North mid-field row (z = -92)
        (-164.0, -92.0, -120.0, -92.0),
        (-96.0, -92.0, -66.0, -92.0),
        (-46.0, -92.0, -12.0, -92.0),
        (12.0, -92.0, 46.0, -92.0),
        (66.0, -92.0, 96.0, -92.0),
        (120.0, -92.0, 138.0, -92.0),
        // South mid-field row (z = +92)
        (-164.0, 92.0, -120.0, 92.0),
        (-96.0, 92.0, -66.0, 92.0),
        (-46.0, 92.0, -12.0, 92.0),
        (12.0, 92.0, 46.0, 92.0),
        (66.0, 92.0, 96.0, 92.0),
        (120.0, 92.0, 138.0, 92.0),
        // Outer flank hedgerows (z = ±120, keeping central spawn staging aprons wide open)
        (-164.0, -120.0, -120.0, -120.0),
        (-96.0, -120.0, -66.0, -120.0),
        (66.0, -120.0, 96.0, -120.0),
        (120.0, -120.0, 138.0, -120.0),
        (-164.0, 120.0, -120.0, 120.0),
        (-96.0, 120.0, -66.0, 120.0),
        (66.0, 120.0, 96.0, 120.0),
        (120.0, 120.0, 138.0, 120.0),
        // N-S hedgerow dividers (x = -148, -74, -36, +36, +74) with wide 18–24 m E-W lane gaps
        // Far-West spine (x = -148)
        (-148.0, -150.0, -148.0, -126.0),
        (-148.0, -114.0, -148.0, -86.0),
        (-148.0, -62.0, -148.0, -28.0),
        (-148.0, 38.0, -148.0, 62.0),
        (-148.0, 86.0, -148.0, 114.0),
        (-148.0, 126.0, -148.0, 150.0),
        // Mid-West divider (x = -74)
        (-74.0, -150.0, -74.0, -126.0),
        (-74.0, -114.0, -74.0, -86.0),
        (-74.0, -62.0, -74.0, -34.0),
        (-74.0, 36.0, -74.0, 62.0),
        (-74.0, 86.0, -74.0, 114.0),
        (-74.0, 126.0, -74.0, 150.0),
        // Inner-West field divider (x = -36, outside central village)
        (-36.0, -114.0, -36.0, -86.0),
        (-36.0, 86.0, -36.0, 114.0),
        // Inner-East field divider (x = +36, outside central village)
        (36.0, -114.0, 36.0, -86.0),
        (36.0, 86.0, 36.0, 114.0),
        // Mid-East divider (x = +74)
        (74.0, -150.0, 74.0, -126.0),
        (74.0, -114.0, 74.0, -86.0),
        (74.0, -62.0, 74.0, -38.0),
        (74.0, 32.0, 74.0, 62.0),
        (74.0, 86.0, 74.0, 114.0),
        (74.0, 126.0, 74.0, 150.0),
    ];

    for &(x0, z0, x1, z1) in hedgerows {
        build_wall_segment(
            &mut world,
            &normandy_height_m,
            x0,
            z0,
            x1,
            z1,
            3, // 1.5 m bocage hedge
            1, // 1.5 m thick earth-and-hedge berm
            VoxelId::HEDGE,
        );
    }

    // 4. Low cobblestone courtyard & orchard walls (1.0–1.5 m tall, single-voxel crisp masonry)
    let stone_walls: &[(f32, f32, f32, f32, i32)] = &[
        // Central village courtyard walls framing Zone B roundabout corners (all r >= 24 m)
        (-18.0, -34.0, -18.0, -21.0, 3),
        (18.0, -34.0, 18.0, -21.0, 3),
        (-18.0, 21.0, -18.0, 34.0, 3),
        (18.0, 21.0, 18.0, 34.0, 3),
        (-34.0, -18.0, -22.0, -18.0, 2),
        (22.0, -18.0, 34.0, -18.0, 2),
        (-34.0, 18.0, -22.0, 18.0, 2),
        (22.0, 18.0, 34.0, 18.0, 2),
        // West Apple Orchard stone enclosure (Zone A, wide 20 m gaps on N/S/E/W approaches)
        (-136.0, -11.0, -136.0, -4.0, 2),
        (-136.0, 15.0, -136.0, 21.0, 2),
        (-123.0, -20.0, -117.0, -20.0, 2),
        (-98.0, -20.0, -91.0, -20.0, 2),
        (-124.0, 31.0, -117.0, 31.0, 2),
        (-98.0, 31.0, -91.0, 31.0, 2),
        (-78.0, -12.0, -78.0, -5.0, 2),
        (-78.0, 16.0, -78.0, 23.0, 2),
    ];

    for &(x0, z0, x1, z1, h_vox) in stone_walls {
        build_wall_segment(
            &mut world,
            &normandy_height_m,
            x0,
            z0,
            x1,
            z1,
            h_vox,
            0,
            VoxelId::COBBLESTONE,
        );
    }

    // 5. Sandbag arcs & Atlantic Wall trench lines (1.0 m = 2 vox tall hull-down cover)
    let sandbags: &[(f32, f32, f32, f32)] = &[
        // West Orchard (Zone A) hull-down sandbag arcs (outside r >= 20 m)
        (-125.0, -6.0, -125.0, 0.0),
        (-125.0, 12.0, -125.0, 18.0),
        (-91.0, -6.0, -91.0, 0.0),
        (-91.0, 12.0, -91.0, 18.0),
        // East Atlantic Wall Bunker Hill (Zone C) sandbag trench parapets (outside r >= 20 m)
        (92.0, -19.0, 99.0, -22.0),
        (117.0, -23.0, 123.0, -19.0),
        (92.0, 8.0, 99.0, 12.0),
        (117.0, 12.0, 123.0, 8.0),
        (88.0, -14.0, 88.0, -10.0),
        (88.0, 0.0, 88.0, 4.0),
        (128.0, -18.0, 128.0, -13.0),
        (128.0, 2.0, 128.0, 7.0),
        // Mid-map crossroads ambush sandbag nests
        (-62.0, -13.0, -58.0, -13.0),
        (-62.0, 15.0, -58.0, 15.0),
        (58.0, -15.0, 62.0, -15.0),
        (58.0, 13.0, 62.0, 13.0),
    ];

    for &(x0, z0, x1, z1) in sandbags {
        build_wall_segment(
            &mut world,
            &normandy_height_m,
            x0,
            z0,
            x1,
            z1,
            2,
            0,
            VoxelId::SANDBAG,
        );
    }

    // 6. Coastal bluff rocks & countryside boulders (outside all lanes & clearance zones)
    let rocks: &[(f32, f32, f32, f32)] = &[
        // Coastal bluff outcrops overlooking the eastern Normandy sand strip
        (146.0, -34.0, 3.2, 2.4),
        (148.0, 26.0, 3.4, 2.5),
        (146.0, -76.0, 3.0, 2.2),
        (146.0, 76.0, 3.2, 2.3),
        (150.0, -112.0, 3.5, 2.4),
        (150.0, 112.0, 3.5, 2.4),
        // Subtle Norman fieldstone outcrops on flanks
        (-156.0, -34.0, 2.8, 2.0),
        (-156.0, 42.0, 2.8, 2.0),
        (-68.0, 68.0, 2.4, 1.8),
        (68.0, -68.0, 2.4, 1.8),
    ];

    for &(rx, rz, radius_m, height_m) in rocks {
        if !near_any_spawn_or_zone(rx, rz, 19.0) {
            build_rock(
                &mut world,
                &normandy_height_m,
                rx,
                rz,
                radius_m,
                height_m,
                VoxelId::ROCK,
            );
        }
    }

    // 7. West Apple Orchard trees (tidy Norman orchard rows outside Zone A clearance)
    let orchard_spots: &[(f32, f32)] = &[
        (-142.0, -4.0),
        (-142.0, 16.0),
        (-124.0, -34.0),
        (-100.0, -34.0),
        (-90.0, -34.0),
        (-124.0, 44.0),
        (-100.0, 44.0),
        (-90.0, 44.0),
        (-140.0, -32.0),
        (-140.0, 44.0),
    ];
    for &(tx, tz) in orchard_spots {
        if is_tree_spot_clear(tx, tz, &structures, hedgerows) {
            build_orchard_tree(&mut world, tx, tz);
        }
    }

    // 8. Rural Norman pines & bocage field trees (deterministic sparse placement)
    place_countryside_trees(&mut world, &structures, hedgerows);

    let positions_xz = [
        (
            "West Apple Orchard",
            -108.0,
            6.0,
            TacticalPositionKind::HullDownRidge,
        ),
        (
            "Crossroads Roundabout",
            0.0,
            0.0,
            TacticalPositionKind::VillageAmbush,
        ),
        (
            "Atlantic Wall Bunker Hill",
            108.0,
            -6.0,
            TacticalPositionKind::FlankOverwatch,
        ),
        (
            "South-West Bocage Lane",
            -56.0,
            74.0,
            TacticalPositionKind::HullDownRidge,
        ),
        (
            "North-East Coastal Approach",
            56.0,
            -74.0,
            TacticalPositionKind::FlankOverwatch,
        ),
        (
            "Sunken Lane Crossroads",
            -56.0,
            0.0,
            TacticalPositionKind::VillageAmbush,
        ),
    ];

    finalize_map(
        world,
        MapId::NormandyBocage,
        MapAtmosphere::normandy_bocage(),
        &ALLIES_SPAWNS,
        &AXIS_SPAWNS,
        &CAPTURE_ZONES,
        &positions_xz,
    )
}

/// Smooth rolling Normandy countryside heightfield strictly within `3.0..=9.5 m`.
///
/// Features the East Atlantic Wall coastal rise (`~8.5 m` around Zone C), a gentle orchard knoll
/// at Zone A (`~6.2 m`), a level central village crossroads (`~4.8 m`), subtle sunken rural lanes,
/// and a gently sloping sandy coastal dune terrace east of `x = 142 m`.
pub fn normandy_height_m(wx: f32, wz: f32) -> f32 {
    let mut h = 4.6
        + 0.42 * (wx * 0.021).sin() * (wz * 0.018).cos()
        + 0.28 * ((wx + wz) * 0.015).cos();

    // East Atlantic Wall Bunker Hill (Zone C overlooking rise ~8.5 m)
    h += 3.9 * gaussian(wx, wz, 110.0, -6.0, 50.0, 56.0);
    h += 0.9 * gaussian(wx, wz, 128.0, -34.0, 34.0, 32.0);
    h += 0.85 * gaussian(wx, wz, 128.0, 28.0, 34.0, 32.0);

    // West Orchard & Farmstead gentle rise (Zone A ~6.4 m)
    h += 1.8 * gaussian(wx, wz, -110.0, 6.0, 52.0, 56.0);

    // Subtle mid-field rolling bocage ridges
    h += 1.15 * gaussian(wx, wz, -68.0, 78.0, 42.0, 36.0);
    h += 1.15 * gaussian(wx, wz, -68.0, -78.0, 42.0, 36.0);
    h += 0.95 * gaussian(wx, wz, 56.0, 82.0, 40.0, 36.0);
    h += 0.95 * gaussian(wx, wz, 56.0, -82.0, 40.0, 36.0);

    // Gently level the Central Norman Crossroads Village around ~4.8 m
    let village_blend = gaussian(wx, wz, 0.0, 0.0, 42.0, 42.0);
    h = h * (1.0 - 0.65 * village_blend) + 4.8 * (0.65 * village_blend);

    // Subtle sunken country lanes (chemins creux) along rural dirt approaches
    if wx.abs() > 54.0 && wx.abs() < 138.0 {
        let ew_z = main_ew_lane_z(wx);
        let d_ew = (wz - ew_z).abs();
        if d_ew < 9.0 {
            let t = 1.0 - d_ew / 9.0;
            h -= 0.35 * t * t;
        }
    }

    // Gentle slope down onto the far-east coastal sand dune strip (x > 142 m)
    if wx > 138.0 {
        let t = ((wx - 138.0) / 40.0).clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t);
        h -= 2.2 * smooth;
        h += 0.28 * smooth * (wz * 0.055 + wx * 0.03).sin();
    }

    h.clamp(3.0, 9.5)
}

/// Centerline Z coordinate of the main East-West lane connecting Zone A -> Zone B -> Zone C.
#[inline]
fn main_ew_lane_z(wx: f32) -> f32 {
    -6.0 * (wx / 108.0).clamp(-1.0, 1.0)
}

/// Minimum distance from `(wx, wz)` to a line segment `(ax, az) -> (bx, bz)`.
#[inline]
fn dist_point_to_segment(wx: f32, wz: f32, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    let abx = bx - ax;
    let abz = bz - az;
    let len2 = abx * abx + abz * abz;
    if len2 < 1e-4 {
        return ((wx - ax).powi(2) + (wz - az).powi(2)).sqrt();
    }
    let t = (((wx - ax) * abx + (wz - az) * abz) / len2).clamp(0.0, 1.0);
    let px = ax + t * abx;
    let pz = az + t * abz;
    ((wx - px).powi(2) + (wz - pz).powi(2)).sqrt()
}

/// Returns the road/plaza surface material if `(wx, wz)` lies on a paved road, sunken dirt lane,
/// or farmyard apron.
fn road_or_plaza_material(wx: f32, wz: f32) -> Option<VoxelId> {
    // 1. Central Norman Crossroads Village cobblestone roundabout (Zone B)
    let dist_b = (wx * wx + wz * wz).sqrt();
    if dist_b <= 19.5 {
        return Some(VoxelId::COBBLESTONE);
    }

    // 2. Main N-S Highway (South Allies Spawn -> Zone B -> North Axis Spawn)
    if wx.abs() <= 5.5 && wz.abs() <= 166.0 {
        return Some(if wz.abs() <= 54.0 {
            VoxelId::COBBLESTONE
        } else {
            VoxelId::DIRT
        });
    }

    // 3. Main E-W Avenue (Zone A -> Zone B -> Zone C)
    if wx.abs() <= 134.0 && (wz - main_ew_lane_z(wx)).abs() <= 5.0 {
        return Some(if wx.abs() <= 52.0 {
            VoxelId::COBBLESTONE
        } else {
            VoxelId::DIRT
        });
    }

    // 4. Zone A West Orchard farmyard pad
    let dist_a = ((wx + 108.0).powi(2) + (wz - 6.0).powi(2)).sqrt();
    if dist_a <= 13.5 {
        return Some(VoxelId::COBBLESTONE);
    }
    if dist_a <= 18.0 {
        return Some(VoxelId::DIRT);
    }

    // 5. Zone C East Atlantic Wall bunker plateau earthwork
    let dist_c = ((wx - 108.0).powi(2) + (wz + 6.0).powi(2)).sqrt();
    if dist_c <= 17.5 {
        return Some(VoxelId::DIRT);
    }

    // 6. West & East flank N-S lanes connecting spawns to Zone A & Zone C
    if (wx + 108.0).abs() <= 4.5 && wz.abs() <= 132.0 {
        return Some(VoxelId::DIRT);
    }
    if (wx - 108.0).abs() <= 4.5 && wz.abs() <= 132.0 {
        return Some(VoxelId::DIRT);
    }

    // Diagonal spawn-to-flank connector lanes
    let connectors: &[(f32, f32, f32, f32)] = &[
        (-38.0, 148.0, -108.0, 130.0),
        (38.0, 148.0, 108.0, 130.0),
        (-38.0, -148.0, -108.0, -130.0),
        (38.0, -148.0, 108.0, -130.0),
        (-42.0, 150.0, 42.0, 150.0),
        (-42.0, -150.0, 42.0, -150.0),
    ];
    for &(ax, az, bx, bz) in connectors {
        if dist_point_to_segment(wx, wz, ax, az, bx, bz) <= 4.2 {
            return Some(VoxelId::DIRT);
        }
    }

    // 7. Mid-field sunken farm tracks threading through bocage tank lane gaps
    if ((wx + 56.0).abs() <= 3.5 || (wx - 56.0).abs() <= 3.5) && wz.abs() <= 128.0 {
        return Some(VoxelId::DIRT);
    }
    if ((wz + 74.0).abs() <= 3.5 || (wz - 74.0).abs() <= 3.5) && (-150.0..=132.0).contains(&wx) {
        return Some(VoxelId::DIRT);
    }

    None
}

/// Selects the top terrain voxel material across Normandy pastures, roads, and coastal dunes.
fn select_surface_material(wx: f32, wz: f32, _h_m: f32) -> VoxelId {
    // Far-east coastal dune strip (x > 142 m with organic wavy dune verge)
    let dune_edge = 142.0 + 3.5 * (wz * 0.045).sin();
    if wx > dune_edge {
        let ripple = ((wx * 0.14 + wz * 0.08).sin() + (wz * 0.11).cos()) * 0.5;
        return if ripple < -0.22 {
            VOXEL_SAND_DARK
        } else {
            VOXEL_SAND
        };
    }

    // Sandy coastal verge blending between bunker hill grass and dune strip
    if wx > dune_edge - 4.5 {
        let n = hash2d((wx * 0.5).floor() as i32, (wz * 0.5).floor() as i32, 901);
        if n > 0.48 {
            return VOXEL_SAND;
        }
    }

    // Roads, town cobblestone square, and sunken country lanes
    if let Some(mat) = road_or_plaza_material(wx, wz) {
        return mat;
    }

    // Patchwork Norman bocage fields aligned loosely with hedgerow divisions
    let plot_x = ((wx + 186.0) / 37.0).floor() as i32;
    let plot_z = ((wz + 186.0) / 35.0).floor() as i32;
    let checker = (plot_x + plot_z).rem_euclid(2) == 0;
    let plot_hash = hash2d(plot_x, plot_z, 404);

    // Occasional subtle plowed pasture bands inside darker fields
    if !checker && plot_hash > 0.55 {
        let furrow = ((wx * 0.65).floor() as i32).rem_euclid(4) == 0;
        if furrow {
            return VoxelId::GRASS;
        }
        return VoxelId::GRASS_DARK;
    }

    if checker {
        VoxelId::GRASS
    } else {
        VoxelId::GRASS_DARK
    }
}

/// True if `(wx, wz)` lies within `radius_m` of any Allies/Axis spawn or capture zone center.
fn near_any_spawn_or_zone(wx: f32, wz: f32, radius_m: f32) -> bool {
    let r2 = radius_m * radius_m;
    for &(sx, sz) in ALLIES_SPAWNS.iter().chain(AXIS_SPAWNS.iter()) {
        let dx = wx - sx;
        let dz = wz - sz;
        if dx * dx + dz * dz < r2 {
            return true;
        }
    }
    for &(_, zx, zz, _) in &CAPTURE_ZONES {
        let dx = wx - zx;
        let dz = wz - zz;
        if dx * dx + dz * dz < r2 {
            return true;
        }
    }
    false
}

/// Checks whether a tree trunk at `(wx, wz)` stays strictly clear of spawns, capture zones,
/// roads, buildings, bunkers, and hedgerows.
fn is_tree_spot_clear(
    wx: f32,
    wz: f32,
    structures: &[StructureSpec],
    hedgerows: &[(f32, f32, f32, f32)],
) -> bool {
    // Stay inside playable borders and west of the sandy coastal dune strip
    if !(-174.0..=138.0).contains(&wx) || wz.abs() > 174.0 {
        return false;
    }

    // Strict >= 19 m clearance around all 12 spawns and all 3 capture zone centers
    if near_any_spawn_or_zone(wx, wz, 19.0) {
        return false;
    }

    // Keep central village streets tidy and unobstructed
    if wx.abs() < 52.0 && wz.abs() < 52.0 {
        return false;
    }

    // Keep clear of all roads & sunken lanes (with 5.5 m buffer)
    for &(ox, oz) in &[
        (0.0, 0.0),
        (5.5, 0.0),
        (-5.5, 0.0),
        (0.0, 5.5),
        (0.0, -5.5),
    ] {
        if road_or_plaza_material(wx + ox, wz + oz).is_some() {
            return false;
        }
    }

    // Keep clear of buildings & Atlantic Wall bunkers
    for s in structures {
        if (wx - s.cx).abs() < s.width_m * 0.5 + 6.5 && (wz - s.cz).abs() < s.depth_m * 0.5 + 6.5 {
            return false;
        }
    }

    // Keep clear of hedgerow lines so canopies don't block 16 m tank gaps
    for &(x0, z0, x1, z1) in hedgerows {
        if dist_point_to_segment(wx, wz, x0, z0, x1, z1) < 4.5 {
            return false;
        }
    }

    true
}

/// Builds a compact broadleaf Norman apple orchard tree with 3.0 m clearance beneath its canopy.
fn build_orchard_tree(world: &mut VoxelWorld, wx: f32, wz: f32) {
    let cvx = (wx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (wz / WORLD_VOXEL_SIZE).round() as i32;
    let base_vy = ((normandy_height_m(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
        .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 10);
    let trunk_h = 6; // 3.0 m trunk
    world.fill_column_raw(cvx, cvz, base_vy + 1, base_vy + trunk_h, VoxelId::TREE_TRUNK);

    for &(dy0, dy1, r) in &[(5, 7, 2), (8, 9, 1)] {
        for dy in dy0..=dy1 {
            let vy = base_vy + dy;
            if vy > MAP_MAX_Y {
                continue;
            }
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx != 0 || dz != 0 || dy > trunk_h {
                        world.set_voxel_raw(IVec3::new(cvx + dx, vy, cvz + dz), VoxelId::LEAVES);
                    }
                }
            }
        }
    }
}

/// Places deterministic Norman pine & broadleaf trees along field edges and hills without
/// intruding on tank lanes, hedgerow gaps, buildings, capture zones, or spawns.
fn place_countryside_trees(
    world: &mut VoxelWorld,
    structures: &[StructureSpec],
    hedgerows: &[(f32, f32, f32, f32)],
) {
    let spacing = 18.0f32;
    let mut gx = -168.0f32;
    while gx <= 136.0 {
        let mut gz = -168.0f32;
        while gz <= 168.0 {
            let ix = gx as i32;
            let iz = gz as i32;
            let chance = hash2d(ix, iz, 701);
            if chance < 0.34 {
                let jx = (hash2d(ix, iz, 709) - 0.5) * 8.0;
                let jz = (hash2d(ix, iz, 719) - 0.5) * 8.0;
                let tx = gx + jx;
                let tz = gz + jz;
                if is_tree_spot_clear(tx, tz, structures, hedgerows) {
                    let style = hash2d(ix, iz, 727);
                    if style < 0.55 {
                        let foliage = if style < 0.28 {
                            VOXEL_PINE_LEAVES
                        } else {
                            VoxelId::LEAVES
                        };
                        build_pine_tree(world, &normandy_height_m, tx, tz, 7, foliage);
                    } else {
                        build_orchard_tree(world, tx, tz);
                    }
                }
            }
            gz += spacing;
        }
        gx += spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::NavGrid;
    use crate::sim_world::SimWorld;
    use crate::tank::TankConfig;
    use voxel::build_basic_tank_models;

    #[test]
    fn normandy_bocage_meets_memory_nav_clearance_and_spawn_guarantees() {
        let (world, meta) = generate_normandy_map();

        assert_eq!(meta.id, "normandy_bocage");
        assert_eq!(meta.spawn_allies.len(), 6);
        assert_eq!(meta.spawn_axis.len(), 6);
        assert_eq!(meta.capture_zones.len(), 3);

        // Verify Stage 6 material IDs resolve to the expected registry entries
        assert_eq!(world.materials.get(VOXEL_SAND).name, "Sand");
        assert_eq!(
            world.materials.get(VOXEL_BUNKER_CONCRETE).name,
            "BunkerConcrete"
        );

        // 1. Strict chunk count & memory budget
        assert!(
            world.chunk_count() <= 576,
            "expected <= 576 chunks, got {}",
            world.chunk_count()
        );
        assert!(
            world.memory_bytes() < 12 * 1024 * 1024,
            "expected < 12 MiB voxel RAM, got {} bytes",
            world.memory_bytes()
        );

        // Verify terrain height bounds across the map and coastal rise at Zone C
        let h_c = normandy_height_m(108.0, -6.0);
        assert!(
            (7.5..=9.5).contains(&h_c),
            "Zone C bunker hill should be ~8.5 m coastal rise, got {h_c:.2} m"
        );
        for gz in (-184..=184).step_by(8) {
            for gx in (-184..=184).step_by(8) {
                let h = normandy_height_m(gx as f32, gz as f32);
                assert!(
                    (3.0..=9.5).contains(&h),
                    "height out of 3.0..=9.5 m at ({gx}, {gz}): {h}"
                );
            }
        }

        // Verify 15 m clearance around every spawn point and every capture zone center
        for (cx, cz) in ALLIES_SPAWNS
            .iter()
            .copied()
            .chain(AXIS_SPAWNS.iter().copied())
            .chain(CAPTURE_ZONES.iter().map(|&(_, x, z, _)| (x, z)))
        {
            for dz in -15..=15 {
                for dx in -15..=15 {
                    if dx * dx + dz * dz > 15 * 15 {
                        continue;
                    }
                    let wx = cx + dx as f32;
                    let wz = cz + dz as f32;
                    let expected_h = normandy_height_m(wx, wz);
                    let actual_h = world.surface_height_at(wx, wz).unwrap_or(0.0);
                    assert!(
                        actual_h <= expected_h + 0.6,
                        "obstacle found within 15 m of ({cx}, {cz}) at ({wx}, {wz}): actual={actual_h:.2} vs terrain={expected_h:.2}"
                    );
                }
            }
        }

        // 2. NavGrid walkability: spawn_allies[0] -> all 3 zones (A, B, C) and -> spawn_axis[0]
        let nav = NavGrid::build(&world);
        assert!(
            nav.walkable_count() > 2500,
            "expected generous walkable area, got {}",
            nav.walkable_count()
        );

        let allies0 = meta.spawn_allies[0];
        for zone in &meta.capture_zones {
            let path = nav.find_path(allies0, zone.center);
            assert!(
                !path.is_empty(),
                "NavGrid failed to find path from spawn_allies[0] to Zone {}",
                zone.label
            );
            let end = *path.last().unwrap();
            let dist_xz =
                ((end.x - zone.center.x).powi(2) + (end.z - zone.center.z).powi(2)).sqrt();
            assert!(
                dist_xz < zone.radius,
                "path to Zone {} ended too far away ({dist_xz:.1} m >= {:.1} m)",
                zone.label,
                zone.radius
            );
        }

        let axis0 = meta.spawn_axis[0];
        let cross_path = nav.find_path(allies0, axis0);
        assert!(
            !cross_path.is_empty(),
            "NavGrid failed to find cross-map path from spawn_allies[0] to spawn_axis[0]"
        );
        let cross_end = *cross_path.last().unwrap();
        let cross_dist =
            ((cross_end.x - axis0.x).powi(2) + (cross_end.z - axis0.z).powi(2)).sqrt();
        assert!(
            cross_dist < 12.0,
            "cross-map path ended too far from spawn_axis[0] ({cross_dist:.1} m)"
        );

        // 3. Zero hull collisions for all 12 tanks spawned at Allies & Axis spawn points
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let sim = SimWorld::new_team_battle(world, meta, cfg, 6);
        assert_eq!(sim.tanks.len(), 12);
        for unit in &sim.tanks {
            let colls = unit.state.count_hull_collisions(
                &sim.voxels,
                unit.state.position.x,
                unit.state.position.z,
                unit.state.hull_yaw,
            );
            assert_eq!(
                colls, 0,
                "tank {} spawned with {colls} hull collisions at {:?}",
                unit.name, unit.state.position
            );
        }
    }
}
