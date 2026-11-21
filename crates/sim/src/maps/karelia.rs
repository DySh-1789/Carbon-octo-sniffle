//! Karelia Ridge (`karelia_ridge`) — `384 × 384 m` Nordic Pine Canyon & Granite Highlands.
//!
//! Features:
//! - West Sniper Granite Ridge (Zone A, `~9.5 m`) with granite boulder clusters and timber bunker pillboxes.
//! - Central Rocky Bowl & Logging Camp (Zone B, `~4.5 m`) framed by slate-roofed log cabins and sandbag arcs.
//! - East Highland Pass (Zone C, `~8.5 m`) twin-ridge corridor with granite outcrops, log barricades, and dense flanking spruce.
//! - Winding northern/central cold brook (`vy = 4`, surface `2.5 m`) crossed by a wide central timber-and-stone ford
//!   plus secondary west/east flank fords.

use voxel::glam::IVec3;
use voxel::{WORLD_VOXEL_SIZE, VoxelId, VoxelWorld};

use crate::map_gen::{MapAtmosphere, MapMetadata, TacticalPositionKind};

use super::{
    MAP_MAX_Y, MAP_MIN_Y, MapId, build_pine_tree, build_rect_building, build_rock,
    build_wall_segment, fill_terrain_columns, finalize_map, gaussian, hash2d, init_standard_world,
};

/// Water voxel layer for the winding cold Nordic brook (`top = (4 + 1) * 0.5 = 2.5 m`).
const BROOK_WATER_VY: i32 = 4;

/// Minimum obstacle clearance around spawns, road centerlines, and capture zone centers (meters).
const CLEARANCE_M: f32 = 15.0;

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

const ZONES: [(char, f32, f32, f32); 3] = [
    ('A', -108.0, -4.0, 16.0),
    ('B', 0.0, 4.0, 16.0),
    ('C', 108.0, -4.0, 16.0),
];

struct BuildingSpec {
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    wall_h: i32,
    wall_mat: VoxelId,
    trim_mat: VoxelId,
    roof_mat: VoxelId,
    pitched: bool,
}

const BUILDINGS: [BuildingSpec; 12] = [
    // West Sniper Granite Ridge (Zone A) — flat-roof timber bunker pillboxes with granite trim
    BuildingSpec {
        cx: -134.0,
        cz: -4.0,
        width_m: 8.0,
        depth_m: 10.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::GRANITE,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: false,
    },
    BuildingSpec {
        cx: -134.0,
        cz: -30.0,
        width_m: 8.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::GRANITE,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: false,
    },
    BuildingSpec {
        cx: -134.0,
        cz: 22.0,
        width_m: 8.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::GRANITE,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: false,
    },
    // Central Rocky Bowl & Logging Camp (Zone B) — pitched slate-roof wooden log cabins
    BuildingSpec {
        cx: -28.0,
        cz: 30.0,
        width_m: 11.0,
        depth_m: 9.0,
        wall_h: 7,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: 28.0,
        cz: 28.0,
        width_m: 12.0,
        depth_m: 9.0,
        wall_h: 7,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: -28.0,
        cz: -20.0,
        width_m: 10.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: 28.0,
        cz: -20.0,
        width_m: 10.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: -44.0,
        cz: 24.0,
        width_m: 8.0,
        depth_m: 7.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: 44.0,
        cz: 26.0,
        width_m: 10.0,
        depth_m: 8.0,
        wall_h: 7,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    // North brookside guard cabins overlooking the central timber-stone ford
    BuildingSpec {
        cx: -26.0,
        cz: -68.0,
        width_m: 9.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    BuildingSpec {
        cx: 26.0,
        cz: -70.0,
        width_m: 9.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::TREE_TRUNK,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: true,
    },
    // East Highland Pass (Zone C) — outer ridge timber watch bunker
    BuildingSpec {
        cx: 138.0,
        cz: 22.0,
        width_m: 8.0,
        depth_m: 8.0,
        wall_h: 6,
        wall_mat: VoxelId::WOOD,
        trim_mat: VoxelId::GRANITE,
        roof_mat: VoxelId::ROOF_SLATE,
        pitched: false,
    },
];

const BOULDERS: [(f32, f32, f32, f32); 29] = [
    // West Sniper Granite Ridge (Zone A) clusters
    (-138.0, -18.0, 3.6, 3.0),
    (-140.0, 10.0, 3.8, 3.2),
    (-130.0, 38.0, 3.4, 2.8),
    (-130.0, -44.0, 3.5, 2.8),
    (-82.0, -24.0, 3.2, 2.5),
    (-80.0, 22.0, 3.0, 2.4),
    (-72.0, -34.0, 3.0, 2.4),
    (-72.0, 44.0, 3.2, 2.6),
    (-130.0, 68.0, 3.8, 3.0),
    (-134.0, -76.0, 3.8, 3.0),
    // Central Rocky Bowl rim outcrops (Zone B)
    (-44.0, -20.0, 3.2, 2.6),
    (44.0, -22.0, 3.2, 2.6),
    (-46.0, 46.0, 3.5, 2.7),
    (46.0, 46.0, 3.5, 2.7),
    (-48.0, -82.0, 3.4, 2.6),
    (48.0, -82.0, 3.4, 2.6),
    (-30.0, 96.0, 3.4, 2.6),
    (30.0, 96.0, 3.4, 2.6),
    // East Highland Pass (Zone C) twin-ridge granite clusters
    (134.0, -16.0, 3.8, 3.2),
    (136.0, 10.0, 3.6, 3.0),
    (132.0, -34.0, 3.6, 3.0),
    (134.0, 34.0, 3.6, 2.8),
    (146.0, -4.0, 4.2, 3.5),
    (82.0, -22.0, 3.2, 2.6),
    (80.0, 22.0, 3.0, 2.4),
    (70.0, -34.0, 3.2, 2.6),
    (72.0, 42.0, 3.2, 2.6),
    (130.0, 66.0, 3.8, 3.0),
    (132.0, -78.0, 3.8, 3.0),
];

/// Generates the `384 × 384 m` Karelia Ridge battlefield (`VoxelWorld`, `MapMetadata`).
pub fn generate_karelia_map() -> (VoxelWorld, MapMetadata) {
    let mut world = init_standard_world();

    // 1. Continuous Nordic canyon & granite highland terrain + shallow cold brook at vy = 4
    fill_terrain_columns(
        &mut world,
        VoxelId::ROCK,
        VoxelId::DIRT,
        Some(BROOK_WATER_VY),
        VoxelId::WATER,
        karelia_height_m,
        select_surface_material,
    );

    // 2. Wide central timber-and-stone fordable crossing + secondary west & east flank fords
    build_timber_stone_ford(&mut world, 0.0, 7.0, true);
    build_timber_stone_ford(&mut world, -90.0, 5.5, false);
    build_timber_stone_ford(&mut world, 88.5, 5.5, false);

    // 3. Timber bunker pillboxes (West Ridge A), wooden log cabins (Logging Camp B), and East watch outpost
    for b in &BUILDINGS {
        build_rect_building(
            &mut world,
            &karelia_height_m,
            b.cx,
            b.cz,
            b.width_m,
            b.depth_m,
            b.wall_h,
            b.wall_mat,
            b.trim_mat,
            b.roof_mat,
            b.pitched,
        );
    }

    // 4. Granite boulder clusters framing West Sniper Ridge (A), Central Bowl rim (B), and East Pass (C)
    for &(bx, bz, rad, height) in &BOULDERS {
        build_rock(
            &mut world,
            &karelia_height_m,
            bx,
            bz,
            rad,
            height,
            VoxelId::GRANITE,
        );
    }

    // 5. Sandbag emplacements & wooden log barricades (low hull-down cover, 1.0 m tall = 2 voxels)
    // Logging Camp (Zone B) sandbag perimeter arcs
    let sandbags: &[(f32, f32, f32, f32)] = &[
        (-21.0, 22.0, -17.0, 26.0),
        (17.0, 26.0, 21.0, 22.0),
        (-21.0, -14.0, -17.0, -18.0),
        (17.0, -18.0, 21.0, -14.0),
        // Overwatch sandbags framing central ford approaches
        (-21.0, -28.0, -16.5, -28.0),
        (16.5, -28.0, 21.0, -28.0),
        (-21.0, -58.0, -16.5, -58.0),
        (16.5, -58.0, 21.0, -58.0),
        // West Sniper Ridge (Zone A) overlook sandbag firing arcs
        (-86.0, -21.0, -86.0, -16.0),
        (-86.0, 16.0, -86.0, 21.0),
    ];
    for &(x0, z0, x1, z1) in sandbags {
        build_wall_segment(
            &mut world,
            &karelia_height_m,
            x0,
            z0,
            x1,
            z1,
            2,
            0,
            VoxelId::SANDBAG,
        );
    }

    // East Highland Pass (Zone C) wooden log barricades + Logging Camp stacked timber piles
    let log_barricades: &[(f32, f32, f32, f32)] = &[
        // East Highland Pass corridor barricades
        (127.0, -24.0, 127.0, -15.0),
        (127.0, 14.0, 127.0, 22.0),
        (86.0, -24.0, 86.0, -16.0),
        (86.0, 16.0, 86.0, 24.0),
        (126.0, 44.0, 134.0, 48.0),
        (126.0, -44.0, 134.0, -48.0),
        // Stacked timber piles beside Logging Camp cabins
        (-36.0, 19.0, -30.0, 19.0),
        (30.0, 19.0, 36.0, 19.0),
        (-36.0, -14.0, -30.0, -14.0),
        (30.0, -14.0, 36.0, -14.0),
    ];
    for &(x0, z0, x1, z1) in log_barricades {
        build_wall_segment(
            &mut world,
            &karelia_height_m,
            x0,
            z0,
            x1,
            z1,
            2,
            0,
            VoxelId::WOOD,
        );
    }

    // 6. Dense northern spruce / pine forest along flanks, canyon rim, and East Pass shoulders
    place_nordic_pines(&mut world);

    // 7. Tactical metadata, capture rings, chunk compaction, and 192×192 minimap
    let positions = [
        (
            "West Sniper Granite Ridge",
            -108.0,
            -4.0,
            TacticalPositionKind::HullDownRidge,
        ),
        (
            "South-West Granite Crest",
            -104.0,
            56.0,
            TacticalPositionKind::HullDownRidge,
        ),
        (
            "Central Logging Camp",
            0.0,
            4.0,
            TacticalPositionKind::VillageAmbush,
        ),
        (
            "Nordic Timber Ford",
            0.0,
            -42.0,
            TacticalPositionKind::BridgeChokepoint,
        ),
        (
            "East Highland Pass",
            108.0,
            -4.0,
            TacticalPositionKind::FlankOverwatch,
        ),
        (
            "North-East Ridge Overlook",
            106.0,
            -104.0,
            TacticalPositionKind::HullDownRidge,
        ),
    ];

    finalize_map(
        world,
        MapId::KareliaRidge,
        MapAtmosphere::karelia_ridge(),
        &ALLIES_SPAWNS,
        &AXIS_SPAWNS,
        &ZONES,
        &positions,
    )
}

/// Smoothstep helper in `[0, 1]`.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Winding Z centerline of the cold northern/central Nordic brook.
/// Dips through the central logging canyon (`z ≈ -42 m` at `x = 0`) and swings north around
/// the elevated West (`x = -108`) and East (`x = +108`) granite highlands so every approach
/// stays gently graded.
#[inline]
fn brook_center_z(wx: f32) -> f32 {
    -56.0 + 14.0 * (wx * 0.019).cos() + 3.5 * (wx * 0.041).sin()
}

/// Central north-south muddy logging road X centerline (`z ∈ [-168, 168]`).
#[inline]
fn central_road_x(wz: f32) -> f32 {
    2.4 * (wz * 0.022).sin() * smoothstep(54.0, 108.0, wz.abs())
}

/// Cross-canyon east-west forest trail Z centerline connecting Zone A (-108,-4) <-> B (0,4) <-> C (108,-4).
#[inline]
fn cross_road_z(wx: f32) -> f32 {
    let t = (wx.abs() / 108.0).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    4.0 * (1.0 - s) + (-4.0) * s
}

/// West flank highland trail X centerline connecting South Spawn -> Zone A -> West Ford -> North Spawn.
#[inline]
fn west_road_x(wz: f32) -> f32 {
    let u = ((wz + 4.0).abs() / 156.0).clamp(0.0, 1.0);
    let s = u * u * (3.0 - 2.0 * u);
    -108.0 * (1.0 - s) + (-38.0) * s
}

/// East flank pass trail X centerline (symmetric counterpart of the West flank trail).
#[inline]
fn east_road_x(wz: f32) -> f32 {
    -west_road_x(wz)
}

/// Continuous deterministic Nordic canyon & granite highlands heightfield (`2.5..=11.5 m` surface).
///
/// The submerged brook bed dips to `2.15 m` (`top_vy = 3 < BROOK_WATER_VY`) so `fill_terrain_columns`
/// covers it with shallow `WATER` up to `vy = 4` (`2.5 m`), yielding zero step at the water's edge
/// and strictly `2.5..=11.5 m` surface elevation across the entire `384 × 384 m` map.
fn karelia_height_m(wx: f32, wz: f32) -> f32 {
    // 1. Base taiga elevation + gentle broad organic relief
    let mut h = 4.6
        + 0.55 * (wx * 0.017).sin() * (wz * 0.015).cos()
        + 0.30 * (wx * 0.031 + 0.9).cos() * (wz * 0.027 - 0.5).sin();

    // 2. West Sniper Granite Ridge (Zone A ~9.5 m) & north/south hull-down shoulders
    h += 5.0 * gaussian(wx, wz, -112.0, -4.0, 56.0, 58.0);
    h += 3.0 * gaussian(wx, wz, -116.0, 62.0, 44.0, 42.0);
    h += 2.2 * gaussian(wx, wz, -120.0, -104.0, 48.0, 44.0);
    h += 2.2 * gaussian(wx, wz, -164.0, -4.0, 38.0, 74.0);

    // 3. East Highland Pass (Zone C ~8.5 m) — elevated twin-ridge corridor
    h += 4.2 * gaussian(wx, wz, 108.0, -4.0, 56.0, 58.0);
    // Outer east ridge wall & inner east twin-ridge crests framing the corridor
    h += 3.0 * gaussian(wx, wz, 160.0, -4.0, 36.0, 70.0);
    h += 2.2 * gaussian(wx, wz, 66.0, -24.0, 28.0, 34.0);
    h += 2.0 * gaussian(wx, wz, 68.0, 36.0, 28.0, 34.0);
    h += 2.6 * gaussian(wx, wz, 114.0, 68.0, 46.0, 42.0);
    h += 1.9 * gaussian(wx, wz, 122.0, -112.0, 50.0, 46.0);

    // 4. South Allies & North Axis commanding spawn staging plateaus
    h += 2.1 * gaussian(wx, wz, 0.0, 154.0, 64.0, 42.0);
    h += 2.1 * gaussian(wx, wz, 0.0, -154.0, 64.0, 42.0);

    // 5. Smoothly carve Central Rocky Bowl & Logging Camp around (0, 4) to ~4.5 m
    let bowl_dist = (wx * wx + (wz - 4.0).powi(2)).sqrt();
    let bowl_blend = 1.0 - smoothstep(24.0, 60.0, bowl_dist);
    h = h * (1.0 - bowl_blend) + 4.5 * bowl_blend;

    // 6. Smooth tactical combat plateaus at Zone A (~9.5 m) and Zone C (~8.5 m)
    let za_dist = ((wx + 108.0).powi(2) + (wz + 4.0).powi(2)).sqrt();
    let za_blend = 1.0 - smoothstep(16.0, 52.0, za_dist);
    h = h * (1.0 - za_blend) + 9.5 * za_blend;

    let zc_dist = ((wx - 108.0).powi(2) + (wz + 4.0).powi(2)).sqrt();
    let zc_blend = 1.0 - smoothstep(16.0, 52.0, zc_dist);
    h = h * (1.0 - zc_blend) + 8.5 * zc_blend;

    // 7. Gentle spawn staging pads so all 12 tanks spawn on clean, level ground
    let allies_pad =
        1.0 - smoothstep(32.0, 56.0, ((wx * 0.72).powi(2) + (wz - 153.0).powi(2)).sqrt());
    h = h * (1.0 - allies_pad) + 6.5 * allies_pad;
    let axis_pad =
        1.0 - smoothstep(32.0, 56.0, ((wx * 0.72).powi(2) + (wz + 153.0).powi(2)).sqrt());
    h = h * (1.0 - axis_pad) + 6.5 * axis_pad;

    // 8. Carve the shallow winding northern/central cold brook with extra-gentle ford banks
    let d_brook = (wz - brook_center_z(wx)).abs();
    let ford_w = gaussian(wx, 0.0, 0.0, 0.0, 32.0, 1.0)
        .max(gaussian(wx, 0.0, -90.0, 0.0, 48.0, 1.0))
        .max(gaussian(wx, 0.0, 88.5, 0.0, 48.0, 1.0));
    let stream_half_w = 4.6 + 0.6 * ford_w;
    let bank_outer = 36.0 + 30.0 * ford_w;
    let stream_cut = 1.0 - smoothstep(stream_half_w, bank_outer, d_brook);
    let bed_y = 2.15; // top_vy = 3 < BROOK_WATER_VY (4) -> covered cleanly by WATER at 2.5 m
    h = h * (1.0 - stream_cut) + bed_y * stream_cut;

    // 9. Subtle outer Nordic canyon rim near map borders
    let edge = wx.abs().max(wz.abs());
    let rim = smoothstep(168.0, 192.0, edge);
    h += rim * 2.0 * (1.0 - stream_cut);

    // Clamp non-brook land strictly inside 2.5..=11.5 m; brook channel down to 2.15 m (water at 2.5 m)
    let min_h = if stream_cut > 0.5 { 2.15 } else { 2.5 };
    h.clamp(min_h, 11.5)
}

/// Checks if `(wx, wz)` falls on a muddy forest logging road, flank trail, or logging camp yard.
fn road_material_at(wx: f32, wz: f32) -> Option<VoxelId> {
    // Central logging camp clearing around Zone B (0, 4)
    let d_camp = (wx * wx + (wz - 4.0).powi(2)).sqrt();
    if d_camp <= 15.5 {
        return Some(VoxelId::DIRT);
    }

    // Central N-S logging road
    if wz.abs() <= 168.0 && (wx - central_road_x(wz)).abs() <= 4.6 {
        let d_brook = (wz - brook_center_z(wx)).abs();
        if d_brook <= 14.0 {
            return Some(VoxelId::COBBLESTONE);
        }
        return Some(VoxelId::DIRT);
    }

    // Cross-canyon E-W forest trail (Zone A <-> Zone B <-> Zone C)
    if wx.abs() <= 112.0 && (wz - cross_road_z(wx)).abs() <= 3.8 {
        return Some(VoxelId::DIRT);
    }

    // West & East flank highland trails (accounting for diagonal slope so apparent width stays ~7.5 m)
    if wz.abs() <= 154.0 {
        let wx_c = west_road_x(wz);
        let d_west = (wx - wx_c).abs() * 0.88;
        if d_west <= 3.6 {
            return Some(VoxelId::DIRT);
        }
        let ex_c = east_road_x(wz);
        let d_east = (wx - ex_c).abs() * 0.88;
        if d_east <= 3.6 {
            return Some(VoxelId::DIRT);
        }
    }

    None
}

/// Selects Karelia Ridge ground surface material: muddy trails, granite highlands, brook banks, taiga moss.
fn select_surface_material(wx: f32, wz: f32, height_m: f32) -> VoxelId {
    if let Some(road) = road_material_at(wx, wz) {
        return road;
    }

    // Cold brook gravel/mud banks with scattered granite riverside stone
    let d_brook = (wz - brook_center_z(wx)).abs();
    if d_brook < 10.5 && height_m < 3.65 {
        let n = hash2d(wx.floor() as i32 / 2, wz.floor() as i32 / 2, 101);
        if height_m < 3.05 && n > 0.72 {
            return VoxelId::GRANITE;
        }
        return VoxelId::DIRT;
    }

    // Exposed Nordic granite highland slopes & ridge crests
    let patch = (wx * 0.048).sin() * (wz * 0.044).cos();
    if height_m > 8.1 || (height_m > 7.25 && patch > 0.12) {
        return VoxelId::GRANITE;
    }

    // Mottled boreal pine-forest floor (`GRASS_DARK` dominant with sparse heather/dirt needle beds)
    if patch < -0.46 {
        VoxelId::DIRT
    } else if patch > 0.52 && height_m < 6.8 {
        VoxelId::GRASS
    } else {
        VoxelId::GRASS_DARK
    }
}

/// Builds a flush, wide timber-and-stone fordable crossing across the cold brook.
///
/// The deck sits flush at `vy = BROOK_WATER_VY` (`2.5 m`) with `COBBLESTONE` paving and transverse
/// `WOOD` corduroy planks so tanks and `NavGrid` cross with zero step penalties, flanked by granite
/// shoulder riprap outside the driving lane.
fn build_timber_stone_ford(
    world: &mut VoxelWorld,
    center_x: f32,
    half_width_m: f32,
    main_crossing: bool,
) {
    let center_z = brook_center_z(center_x);
    let half_len_m = if main_crossing { 11.5 } else { 9.5 };
    let vx_min = ((center_x - half_width_m - 1.5) / WORLD_VOXEL_SIZE).floor() as i32;
    let vx_max = ((center_x + half_width_m + 1.5) / WORLD_VOXEL_SIZE).ceil() as i32;
    let vz_min = ((center_z - half_len_m) / WORLD_VOXEL_SIZE).floor() as i32;
    let vz_max = ((center_z + half_len_m) / WORLD_VOXEL_SIZE).ceil() as i32;

    for vz in vz_min..=vz_max {
        let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
        let d_stream = (wz - brook_center_z(center_x)).abs();
        for vx in vx_min..=vx_max {
            let wx = (vx as f32 + 0.5) * WORLD_VOXEL_SIZE;
            let dx = (wx - center_x).abs();
            let h_m = karelia_height_m(wx, wz);
            let terrain_vy = ((h_m / WORLD_VOXEL_SIZE).round() as i32 - 1)
                .clamp(MAP_MIN_Y + 1, MAP_MAX_Y - 2);

            if dx <= half_width_m {
                // Flush ford causeway over water and immediate shore (`vy = 4` = 2.5 m)
                if terrain_vy <= BROOK_WATER_VY {
                    world.fill_column_raw(
                        vx,
                        vz,
                        (BROOK_WATER_VY - 2).max(MAP_MIN_Y),
                        BROOK_WATER_VY - 1,
                        VoxelId::COBBLESTONE,
                    );
                    let mat = if main_crossing && vz.rem_euclid(3) == 0 && d_stream <= 6.5 {
                        VoxelId::WOOD
                    } else {
                        VoxelId::COBBLESTONE
                    };
                    world.set_voxel_raw(IVec3::new(vx, BROOK_WATER_VY, vz), mat);
                    for vy in (BROOK_WATER_VY + 1)..=(BROOK_WATER_VY + 6) {
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
                    }
                } else if terrain_vy == BROOK_WATER_VY + 1 && d_stream <= half_len_m {
                    // Cobblestone transition apron on immediate dry approach
                    world.set_voxel_raw(IVec3::new(vx, terrain_vy, vz), VoxelId::COBBLESTONE);
                }
            } else if d_stream <= 7.0 && terrain_vy <= BROOK_WATER_VY {
                // Flush granite riprap shoulder stones flanking the ford (0-step flush edge)
                world.fill_column_raw(
                    vx,
                    vz,
                    (BROOK_WATER_VY - 1).max(MAP_MIN_Y),
                    BROOK_WATER_VY,
                    VoxelId::GRANITE,
                );
            }
        }
    }

    // Low rustic timber & granite ford marker posts well outside the 14 m central driving lane
    if main_crossing {
        for &sx in &[center_x - half_width_m - 2.5, center_x + half_width_m + 2.5] {
            for &sz in &[center_z - 8.5, center_z + 8.5] {
                let vx = (sx / WORLD_VOXEL_SIZE).round() as i32;
                let vz = (sz / WORLD_VOXEL_SIZE).round() as i32;
                let base_vy = ((karelia_height_m(sx, sz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
                    .clamp(BROOK_WATER_VY, MAP_MAX_Y - 4);
                world.set_voxel_raw(IVec3::new(vx, base_vy + 1, vz), VoxelId::GRANITE);
                world.set_voxel_raw(IVec3::new(vx, base_vy + 2, vz), VoxelId::TREE_TRUNK);
            }
        }
    }
}

/// Minimum Euclidean distance from `(wx, wz)` to any of the 4 Karelia Ridge road centerlines.
fn min_dist_to_road_centerlines(wx: f32, wz: f32) -> f32 {
    let mut best = f32::INFINITY;

    let seg_dist = |x0: f32, z0: f32, x1: f32, z1: f32| -> f32 {
        let dx = x1 - x0;
        let dz = z1 - z0;
        let l2 = dx * dx + dz * dz;
        let t = if l2 < 1e-6 {
            0.0
        } else {
            (((wx - x0) * dx + (wz - z0) * dz) / l2).clamp(0.0, 1.0)
        };
        let px = x0 + t * dx;
        let pz = z0 + t * dz;
        ((wx - px).powi(2) + (wz - pz).powi(2)).sqrt()
    };

    const STEPS: usize = 32;

    // 1. Central N-S road (`z in -168..=168`)
    for i in 0..STEPS {
        let z0 = -168.0 + 336.0 * (i as f32 / STEPS as f32);
        let z1 = -168.0 + 336.0 * ((i + 1) as f32 / STEPS as f32);
        best = best.min(seg_dist(central_road_x(z0), z0, central_road_x(z1), z1));
    }

    // 2. Cross-canyon E-W road (`x in -108..=108`)
    for i in 0..STEPS {
        let x0 = -108.0 + 216.0 * (i as f32 / STEPS as f32);
        let x1 = -108.0 + 216.0 * ((i + 1) as f32 / STEPS as f32);
        best = best.min(seg_dist(x0, cross_road_z(x0), x1, cross_road_z(x1)));
    }

    // 3. West & East flank highland trails (`z in -152..=152`)
    for i in 0..STEPS {
        let z0 = -152.0 + 304.0 * (i as f32 / STEPS as f32);
        let z1 = -152.0 + 304.0 * ((i + 1) as f32 / STEPS as f32);
        best = best.min(seg_dist(west_road_x(z0), z0, west_road_x(z1), z1));
        best = best.min(seg_dist(east_road_x(z0), z0, east_road_x(z1), z1));
    }

    best
}

/// Checks whether `(wx, wz)` satisfies the `>= 15 m` clearance from all spawns, road centerlines,
/// and capture zone centers (with optional extra padding for obstacle footprints).
fn is_clear_of_core_corridors(wx: f32, wz: f32, extra_pad: f32) -> bool {
    let min_d = CLEARANCE_M + extra_pad;
    for &(sx, sz) in ALLIES_SPAWNS.iter().chain(AXIS_SPAWNS.iter()) {
        if ((wx - sx).powi(2) + (wz - sz).powi(2)).sqrt() < min_d {
            return false;
        }
    }
    for &(_, zx, zz, _) in &ZONES {
        if ((wx - zx).powi(2) + (wz - zz).powi(2)).sqrt() < min_d {
            return false;
        }
    }
    min_dist_to_road_centerlines(wx, wz) >= min_d
}

/// Populates dense Nordic pine/spruce stands (`VoxelId::PINE_LEAVES`) along the canyon flanks,
/// East Highland Pass shoulders, and taiga slopes while respecting all 15 m clearances.
fn place_nordic_pines(world: &mut VoxelWorld) {
    let spacing = 10.5f32;
    let mut gx = -176.0f32;
    let mut ix = 0i32;
    while gx <= 176.0 {
        let mut gz = -176.0f32;
        let mut iz = 0i32;
        while gz <= 176.0 {
            let h = hash2d(ix, iz, 773);
            let jx = (hash2d(ix, iz, 1301) - 0.5) * 5.2;
            let jz = (hash2d(ix, iz, 2909) - 0.5) * 5.2;
            let tx = gx + jx;
            let tz = gz + jz;

            // Higher forest density along flanks, East Pass shoulders, and outer taiga rim
            let flank_density = if tx > 58.0 || tx < -124.0 || tz.abs() > 86.0 {
                0.56
            } else {
                0.28
            };

            if h < flank_density && is_pine_spot_valid(tx, tz) {
                let trunk_h = 8 + ((hash2d(ix, iz, 4111) * 3.0) as i32);
                build_pine_tree(
                    world,
                    &karelia_height_m,
                    tx,
                    tz,
                    trunk_h,
                    VoxelId::PINE_LEAVES,
                );
            }

            gz += spacing;
            iz += 1;
        }
        gx += spacing;
        ix += 1;
    }
}

fn is_pine_spot_valid(wx: f32, wz: f32) -> bool {
    // Keep strictly inside map bounds so ±3-voxel pine crowns never spill outside 24×24 chunks
    if wx.abs() > 178.0 || wz.abs() > 178.0 {
        return false;
    }
    // Keep clear of the cold brook channel & soggy banks
    if (wz - brook_center_z(wx)).abs() < 11.5 {
        return false;
    }
    // Keep exposed high granite peaks & sniper sightlines clear; avoid clipping MAP_MAX_Y
    let h_m = karelia_height_m(wx, wz);
    if !(3.0..=9.4).contains(&h_m) {
        return false;
    }
    // Strict >= 15 m clearance (with +1.5 m crown radius pad) from spawns, roads, and zone centers
    if !is_clear_of_core_corridors(wx, wz, 1.5) {
        return false;
    }
    // Avoid overlapping cabins/bunkers
    for b in &BUILDINGS {
        if (wx - b.cx).abs() < b.width_m * 0.5 + 5.5 && (wz - b.cz).abs() < b.depth_m * 0.5 + 5.5 {
            return false;
        }
    }
    // Avoid overlapping granite boulders
    for &(bx, bz, rad, _) in &BOULDERS {
        if ((wx - bx).powi(2) + (wz - bz).powi(2)).sqrt() < rad + 4.5 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use voxel::build_basic_tank_models;

    use crate::ai::NavGrid;
    use crate::sim_world::SimWorld;
    use crate::tank::TankConfig;

    use super::*;

    #[test]
    fn karelia_ridge_meets_budget_nav_slope_and_spawn_guarantees() {
        let (world, meta) = generate_karelia_map();

        assert_eq!(meta.id, "karelia_ridge");
        assert_eq!(meta.spawn_allies.len(), 6);
        assert_eq!(meta.spawn_axis.len(), 6);
        assert_eq!(meta.capture_zones.len(), 3);

        // 1. Strict chunk count & sparse voxel RAM budgets
        assert!(
            world.chunk_count() <= 576,
            "chunk_count {} exceeded 576",
            world.chunk_count()
        );
        assert!(
            world.memory_bytes() < 12 * 1024 * 1024,
            "memory_bytes {} exceeded 12 MiB",
            world.memory_bytes()
        );

        // Verify all hand-placed buildings & boulders obey >= 15 m clearance from spawns, roads, and zones
        for b in &BUILDINGS {
            assert!(
                is_clear_of_core_corridors(b.cx, b.cz, 0.0),
                "building at ({}, {}) violates 15 m clearance",
                b.cx,
                b.cz
            );
        }
        for &(bx, bz, _, _) in &BOULDERS {
            assert!(
                is_clear_of_core_corridors(bx, bz, 0.0),
                "boulder at ({bx}, {bz}) violates 15 m clearance"
            );
        }

        // Verify key landmark elevations match design spec
        let h_a = karelia_height_m(-108.0, -4.0);
        let h_b = karelia_height_m(0.0, 4.0);
        let h_c = karelia_height_m(108.0, -4.0);
        assert!((h_a - 9.5).abs() < 0.6, "Zone A height expected ~9.5 m, got {h_a}");
        assert!((h_b - 4.5).abs() < 0.6, "Zone B height expected ~4.5 m, got {h_b}");
        assert!((h_c - 8.5).abs() < 0.6, "Zone C height expected ~8.5 m, got {h_c}");

        // Verify gentle climbable slopes (<= 12 deg) along all 4 road centerlines
        let max_tan = 12.0_f32.to_radians().tan();
        for i in -150..=150 {
            let z = i as f32;
            let cx = central_road_x(z);
            let dz_slope =
                (karelia_height_m(cx, z + 1.0) - karelia_height_m(cx, z - 1.0)).abs() * 0.5;
            assert!(
                dz_slope <= max_tan,
                "central road slope too steep at z={z}: tan={dz_slope:.3} > {max_tan:.3}"
            );

            let wx = west_road_x(z);
            let wx1 = west_road_x(z + 1.0);
            let w_ds = ((wx1 - wx).powi(2) + 1.0).sqrt();
            let w_slope = (karelia_height_m(wx1, z + 1.0) - karelia_height_m(wx, z)).abs() / w_ds;
            assert!(
                w_slope <= max_tan,
                "west flank road slope too steep at ({wx:.1},{z}): tan={w_slope:.3} > {max_tan:.3}"
            );

            let ex = east_road_x(z);
            let ex1 = east_road_x(z + 1.0);
            let e_ds = ((ex1 - ex).powi(2) + 1.0).sqrt();
            let e_slope = (karelia_height_m(ex1, z + 1.0) - karelia_height_m(ex, z)).abs() / e_ds;
            assert!(
                e_slope <= max_tan,
                "east flank road slope too steep at ({ex:.1},{z}): tan={e_slope:.3} > {max_tan:.3}"
            );
        }
        for i in -107..=107 {
            let x = i as f32;
            let cz0 = cross_road_z(x);
            let cz1 = cross_road_z(x + 1.0);
            let ds = (1.0 + (cz1 - cz0).powi(2)).sqrt();
            let c_slope = (karelia_height_m(x + 1.0, cz1) - karelia_height_m(x, cz0)).abs() / ds;
            assert!(
                c_slope <= max_tan,
                "cross road slope too steep at ({x},{cz0:.1}): tan={c_slope:.3} > {max_tan:.3}"
            );
        }

        // 2. NavGrid walkability from spawn_allies[0] to all 3 capture zones & spawn_axis[0]
        let nav = NavGrid::build(&world);
        let start = meta.spawn_allies[0];

        for zone in &meta.capture_zones {
            let path = nav.find_path(start, zone.center);
            assert!(
                !path.is_empty(),
                "NavGrid failed to find path from Allies spawn to Zone {}",
                zone.label
            );
            let end = *path.last().unwrap();
            let dist = ((end.x - zone.center.x).powi(2) + (end.z - zone.center.z).powi(2)).sqrt();
            assert!(
                dist <= zone.radius,
                "Path to Zone {} ended {:.1} m away (radius {:.1} m)",
                zone.label,
                dist,
                zone.radius
            );
        }

        let axis_goal = meta.spawn_axis[0];
        let cross_path = nav.find_path(start, axis_goal);
        assert!(
            !cross_path.is_empty(),
            "NavGrid failed to find path from Allies spawn to Axis spawn"
        );
        let end = *cross_path.last().unwrap();
        let spawn_dist = ((end.x - axis_goal.x).powi(2) + (end.z - axis_goal.z).powi(2)).sqrt();
        assert!(
            spawn_dist <= 12.0,
            "Path to Axis spawn ended {spawn_dist:.1} m away (limit 12.0 m)"
        );

        // 3. Zero hull collisions for 12 tanks spawned at Allies/Axis spawn points
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
                "tank {} had {colls} hull collisions at spawn {:?}",
                unit.name, unit.state.position
            );
        }
    }
}
