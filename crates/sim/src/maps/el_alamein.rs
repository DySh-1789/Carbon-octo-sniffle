//! Stage 6 Tactical Battlefield: **El Alamein Dunes** (`384 × 384 m`, North African Desert).
//!
//! Layout overview:
//! - **West Sand Dune Ridge (Zone A, `(-108, -6)`)**: Rolling hull-down desert dunes (`~8.5 m`)
//!   with sandbag gun pits and sandstone outcrops.
//! - **Central Desert Oasis (Zone B, `(0, 0)`)**: Open convoy crossroads flanked by flat-roofed
//!   mudbrick desert outpost buildings (with a wide `>= 16 m` central avenue) and a shallow
//!   turquoise oasis pool just east (`x = 22..44, z = -12..12`, water at `vy = 4`) ringed by
//!   grass fringe and date palms.
//! - **East Sandstone Wadi & Outpost (Zone C, `(108, 6)`)**: Flat-topped rocky mesa plateau
//!   (`~7.5 m`) with smooth climbing ramps, mudbrick outpost buildings, low mudbrick walls, and
//!   sandstone rock outcrops overlooking a dry wadi.

use voxel::{VoxelId, VoxelWorld};

use crate::map_gen::{MapAtmosphere, MapMetadata, TacticalPositionKind};
use crate::maps::{
    MapId, build_palm_tree, build_rect_building, build_rock, build_wall_segment,
    fill_terrain_columns, finalize_map, gaussian, hash2d, init_standard_world,
};

/// Stage 6 desert voxel material IDs matching `MaterialRegistry` indices 43..=46.
const SAND: VoxelId = VoxelId(43);
const SAND_DARK: VoxelId = VoxelId(44);
const SANDSTONE: VoxelId = VoxelId(45);
const MUDBRICK: VoxelId = VoxelId(46);

/// Oasis shallow water surface voxel level (`vy = 4` -> `y = 2.5 m`).
const OASIS_WATER_VY: i32 = 4;

/// Oasis pool ellipse center and half-extents (`x = 22..44, z = -12..12`).
const OASIS_CX: f32 = 33.0;
const OASIS_CZ: f32 = 0.0;
const OASIS_RX: f32 = 11.0;
const OASIS_RZ: f32 = 12.0;

/// Allies (South) 6-tank formation spawns (`z = 148..162`, `x in [-38, -18, 0, 18, 38]`).
const ALLIES_SPAWNS: [(f32, f32); 6] = [
    (0.0, 148.0),
    (-18.0, 152.0),
    (18.0, 152.0),
    (-38.0, 148.0),
    (38.0, 148.0),
    (0.0, 162.0),
];

/// Axis (North) 6-tank formation spawns (`z = -148..-162`, `x in [-38, -18, 0, 18, 38]`).
const AXIS_SPAWNS: [(f32, f32); 6] = [
    (0.0, -148.0),
    (-18.0, -152.0),
    (18.0, -152.0),
    (-38.0, -148.0),
    (38.0, -148.0),
    (0.0, -162.0),
];

/// Three tactical capture zones (`A`, `B`, `C`), each with radius `16.0 m`.
const CAPTURE_ZONES: [(char, f32, f32, f32); 3] = [
    ('A', -108.0, -6.0, 16.0),
    ('B', 0.0, 0.0, 16.0),
    ('C', 108.0, 6.0, 16.0),
];

/// Minimum clear radius around every spawn point and capture zone center.
const PROTECTED_CLEAR_RADIUS_M: f32 = 14.0;

struct OutpostBuilding {
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    wall_height_vox: i32,
}

/// Generates the `384 × 384 m` **El Alamein Dunes** North African desert battlefield.
pub fn generate_el_alamein_map() -> (VoxelWorld, MapMetadata) {
    let mut world = init_standard_world();

    // 1. Sweeping desert dune heightfield, eastern sandstone mesa, dry wadi, and oasis water basin.
    fill_terrain_columns(
        &mut world,
        SANDSTONE,
        SAND_DARK,
        Some(OASIS_WATER_VY),
        VoxelId::WATER,
        el_alamein_height_m,
        select_surface_material,
    );

    // 2. Flat-roofed mudbrick desert outpost buildings around Zone B (keeping >= 24 m central road)
    //    and perched on the East Sandstone Mesa flanks around Zone C.
    let buildings = [
        // Central Oasis Outpost — West side of central convoy road (inner edges at x = -17)
        OutpostBuilding {
            cx: -22.0,
            cz: -24.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 8,
        },
        OutpostBuilding {
            cx: -22.0,
            cz: 24.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: -39.0,
            cz: -18.0,
            width_m: 9.0,
            depth_m: 10.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: -39.0,
            cz: 18.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 8,
        },
        OutpostBuilding {
            cx: -25.0,
            cz: -48.0,
            width_m: 9.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: -25.0,
            cz: 48.0,
            width_m: 9.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        // Central Oasis Outpost — East side framing north/south approaches around the oasis pool
        OutpostBuilding {
            cx: 22.0,
            cz: -34.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 8,
        },
        OutpostBuilding {
            cx: 22.0,
            cz: 34.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: 60.0,
            cz: -18.0,
            width_m: 9.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: 60.0,
            cz: 18.0,
            width_m: 9.0,
            depth_m: 9.0,
            wall_height_vox: 8,
        },
        // East Sandstone Mesa Outpost (flanking Zone C at (108, 6), all well beyond 14 m radius)
        OutpostBuilding {
            cx: 131.0,
            cz: -14.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 8,
        },
        OutpostBuilding {
            cx: 131.0,
            cz: 25.0,
            width_m: 10.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: 89.0,
            cz: -23.0,
            width_m: 9.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
        OutpostBuilding {
            cx: 89.0,
            cz: 35.0,
            width_m: 9.0,
            depth_m: 8.0,
            wall_height_vox: 7,
        },
    ];

    for b in &buildings {
        build_rect_building(
            &mut world,
            &el_alamein_height_m,
            b.cx,
            b.cz,
            b.width_m,
            b.depth_m,
            b.wall_height_vox,
            MUDBRICK,
            SANDSTONE,
            MUDBRICK,
            false,
        );
    }

    // 3. Tactical low walls & sandbag gun pits (strictly outside 14 m protected spawn/zone radii).
    // Central Oasis Outpost low mudbrick courtyard walls (leaving wide 16 m+ central avenue)
    build_wall_segment(&mut world, &el_alamein_height_m, -16.5, -40.0, -16.5, -31.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, -16.5, 31.0, -16.5, 40.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 16.5, -42.0, 16.5, -36.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 16.5, 36.0, 16.5, 42.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, -33.0, -24.0, -28.0, -24.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, -33.0, 24.0, -28.0, 24.0, 2, 0, MUDBRICK);

    // West Sand Dune Ridge (Zone A at (-108, -6)): hull-down sandbag gun pits around ridge crest
    build_wall_segment(&mut world, &el_alamein_height_m, -95.0, -24.0, -87.0, -24.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -87.0, -24.0, -87.0, -18.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -95.0, 12.0, -87.0, 12.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -87.0, 12.0, -87.0, 6.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -128.0, -16.0, -128.0, -9.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -128.0, -1.0, -128.0, 6.0, 2, 0, VoxelId::SANDBAG);
    // Flanking dune ridge sandbag overwatch nests
    build_wall_segment(&mut world, &el_alamein_height_m, -118.0, -56.0, -108.0, -56.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, -114.0, 48.0, -104.0, 48.0, 2, 0, VoxelId::SANDBAG);

    // East Sandstone Mesa Outpost (Zone C at (108, 6)): low mudbrick perimeter walls & sandbags
    build_wall_segment(&mut world, &el_alamein_height_m, 125.0, -4.0, 131.0, -4.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 125.0, 15.0, 131.0, 15.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 99.0, -14.0, 107.0, -14.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 99.0, 26.0, 107.0, 26.0, 2, 0, MUDBRICK);
    build_wall_segment(&mut world, &el_alamein_height_m, 90.0, -6.0, 90.0, 0.0, 2, 0, VoxelId::SANDBAG);
    build_wall_segment(&mut world, &el_alamein_height_m, 90.0, 13.0, 90.0, 19.0, 2, 0, VoxelId::SANDBAG);

    // 4. Sculpted sandstone rock outcrops on dune ridges, wadi rim, and mesa perimeter.
    let key_boulders: &[(f32, f32, f32, f32)] = &[
        // West Dune Ridge (Zone A flanks)
        (-135.0, -26.0, 3.6, 2.6),
        (-137.0, 18.0, 3.8, 2.8),
        (-86.0, -40.0, 3.0, 2.2),
        (-84.0, 32.0, 3.2, 2.4),
        (-146.0, -56.0, 4.2, 3.0),
        (-142.0, 52.0, 4.0, 2.8),
        (-70.0, -20.0, 2.6, 2.0),
        (-72.0, 20.0, 2.8, 2.1),
        // East Sandstone Mesa & Wadi outcrops (Zone C flanks)
        (140.0, 6.0, 4.0, 3.0),
        (143.0, -34.0, 4.2, 3.2),
        (141.0, 46.0, 4.0, 3.0),
        (78.0, -42.0, 3.2, 2.4),
        (76.0, 46.0, 3.2, 2.4),
        (82.0, -6.0, 2.5, 1.9),
        (82.0, 22.0, 2.5, 1.9),
        // Mid-field desert flank boulders (approach cover between spawns & capture zones)
        (-58.0, 96.0, 3.4, 2.5),
        (58.0, 98.0, 3.4, 2.5),
        (-60.0, -98.0, 3.4, 2.6),
        (60.0, -96.0, 3.4, 2.5),
        (-152.0, -108.0, 4.0, 2.8),
        (150.0, 106.0, 4.0, 2.8),
        (-150.0, 108.0, 3.8, 2.6),
        (152.0, -108.0, 3.8, 2.6),
    ];

    for &(bx, bz, rad, height) in key_boulders {
        if is_clear_of_protected(bx, bz, rad + 1.5) {
            build_rock(&mut world, &el_alamein_height_m, bx, bz, rad, height, SANDSTONE);
        }
    }

    // Natural desert sandstone boulder scatter on outer dunes/wadi flanks (deterministic via hash2d)
    place_desert_scatter_rocks(&mut world, &buildings);

    // 5. Oasis date palms ringing the turquoise pool grass fringe & oasis outpost edges.
    let oasis_palms: &[(f32, f32)] = &[
        // West grass fringe (all >= 18.5 m from Zone B center (0, 0) so fronds stay outside 14 m)
        (19.0, -8.5),
        (19.5, 2.5),
        (20.5, 11.0),
        // North oasis bank
        (26.0, -14.5),
        (33.5, -15.5),
        (41.0, -14.0),
        // East oasis bank
        (46.5, -8.5),
        (48.0, 0.5),
        (46.5, 9.0),
        // South oasis bank
        (25.5, 14.5),
        (34.5, 15.5),
        (42.0, 13.5),
        // Accent shade palms flanking oasis mudbrick compounds
        (-17.5, -17.0),
        (-17.5, 17.0),
        (52.0, -6.5),
        (51.5, 11.5),
    ];

    for &(px, pz) in oasis_palms {
        if is_clear_of_protected(px, pz, 2.5) && oasis_pool_radius(px, pz) >= 1.08 {
            build_palm_tree(&mut world, &el_alamein_height_m, px, pz);
        }
    }

    // 6. Finalize chunks, minimap, spawns, capture zones, and tactical anchors.
    let tactical_positions = [
        ("West Dune Hull-Down Ridge", -108.0, -6.0, TacticalPositionKind::HullDownRidge),
        ("North-West Sand Crest", -118.0, -62.0, TacticalPositionKind::HullDownRidge),
        ("South-West Dune Overlook", -114.0, 56.0, TacticalPositionKind::HullDownRidge),
        ("Central Oasis Outpost", 0.0, 0.0, TacticalPositionKind::VillageAmbush),
        ("Oasis Southern Palm Bypass", 33.0, 22.0, TacticalPositionKind::BridgeChokepoint),
        ("East Sandstone Mesa", 108.0, 6.0, TacticalPositionKind::FlankOverwatch),
        ("Eastern Wadi Overwatch", 72.0, -34.0, TacticalPositionKind::FlankOverwatch),
    ];

    finalize_map(
        world,
        MapId::ElAlamein,
        MapAtmosphere::el_alamein(),
        &ALLIES_SPAWNS,
        &AXIS_SPAWNS,
        &CAPTURE_ZONES,
        &tactical_positions,
    )
}

/// Smooth C1 Hermite interpolation in `[0, 1]`.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Normalized elliptical distance from the central oasis pool (`<= 1.0` lies inside `x=22..44, z=-12..12`).
#[inline]
fn oasis_pool_radius(wx: f32, wz: f32) -> f32 {
    let nx = (wx - OASIS_CX) / OASIS_RX;
    let nz = (wz - OASIS_CZ) / OASIS_RZ;
    (nx * nx + nz * nz).sqrt()
}

/// Elliptical distance from the East Sandstone Mesa center `(108, 6)`.
#[inline]
fn mesa_distance(wx: f32, wz: f32) -> f32 {
    let dx = wx - 108.0;
    let dz = (wz - 6.0) / 1.15;
    (dx * dx + dz * dz).sqrt()
}

/// Centerline X coordinate of the dry eastern desert wadi running north-south between the oasis and mesa.
#[inline]
fn wadi_center_x(wz: f32) -> f32 {
    68.0 + 7.5 * (wz * 0.022).sin()
}

/// Sweeping North African desert heightfield (`2.5..=10.5 m` dry terrain; dips smoothly below
/// `2.25 m` solely inside the shallow oasis pool so water fills at `vy = 4`, surface `2.5 m`).
fn el_alamein_height_m(wx: f32, wz: f32) -> f32 {
    // 1. Base desert elevation with gentle wind-carved diagonal sand dune ripples
    let dune_u = wx * 0.022 + wz * 0.013;
    let dune_v = -wx * 0.011 + wz * 0.024;
    let ripples = 0.60 * dune_u.sin()
        + 0.40 * dune_v.cos()
        + 0.22 * (wx * 0.034 - wz * 0.026).sin();

    let mut h = 4.3 + ripples;

    // 2. West Sand Dune Ridge (Zone A at (-108, -6)): sweeping hull-down crests (~8.5 m)
    h += 4.2 * gaussian(wx, wz, -108.0, -6.0, 44.0, 56.0);
    h += 3.1 * gaussian(wx, wz, -122.0, -68.0, 38.0, 44.0);
    h += 2.9 * gaussian(wx, wz, -118.0, 62.0, 38.0, 44.0);
    h += 3.5 * gaussian(wx, wz, -162.0, 0.0, 32.0, 126.0);

    // 3. Mid-field flanking dunes & far-east rocky wadi rim
    h += 2.1 * gaussian(wx, wz, -56.0, 86.0, 30.0, 26.0);
    h += 2.1 * gaussian(wx, wz, 56.0, 88.0, 30.0, 26.0);
    h += 2.1 * gaussian(wx, wz, -56.0, -88.0, 30.0, 26.0);
    h += 2.1 * gaussian(wx, wz, 56.0, -86.0, 30.0, 26.0);
    h += 4.1 * gaussian(wx, wz, 166.0, 0.0, 30.0, 132.0);

    // 4. Dry sandstone wadi depression between oasis and East Mesa (stays dry >= 3.0 m)
    let wadi_dx = (wx - wadi_center_x(wz)) / 18.0;
    let wadi_z_fade = smoothstep(145.0, 95.0, wz.abs());
    h -= 0.95 * (-wadi_dx * wadi_dx).exp() * wadi_z_fade;

    // 5. East Sandstone Mesa plateau (Zone C at (108, 6)): flat-topped ~7.5 m with 38 m smooth ramp
    let d_mesa = mesa_distance(wx, wz);
    let mesa_blend = 1.0 - smoothstep(20.0, 58.0, d_mesa);
    let mesa_cap_h = 7.5 + 0.14 * (wx * 0.05 + wz * 0.04).sin();
    h = h * (1.0 - mesa_blend) + mesa_cap_h * mesa_blend;

    // 6. Gentle staging plateaus at South (Allies) and North (Axis) spawns
    let south_spawn_w = gaussian(wx, wz, 0.0, 154.0, 58.0, 26.0);
    let north_spawn_w = gaussian(wx, wz, 0.0, -154.0, 58.0, 26.0);
    let spawn_w = south_spawn_w.max(north_spawn_w);
    h = h * (1.0 - spawn_w * 0.65) + 4.8 * (spawn_w * 0.65);

    // 7. Central Desert Oasis valley around Zone B (0, 0) & shallow turquoise pool at (33, 0)
    let oasis_valley_w = gaussian(wx, wz, 14.0, 0.0, 44.0, 38.0);
    h = h * (1.0 - oasis_valley_w * 0.72) + 3.45 * (oasis_valley_w * 0.72);

    // Clamp dry desert terrain strictly inside [2.6, 10.5] m before carving the oasis pool
    h = h.clamp(2.6, 10.5);

    // Carve shallow oasis pool exclusively inside x = 22..44, z = -12..12 (r <= 1.0 -> h < 2.25 m)
    let r_pool = oasis_pool_radius(wx, wz);
    if r_pool < 2.3 {
        let pool_w = 1.0 - smoothstep(0.95, 2.3, r_pool);
        let r_clamped = r_pool.min(1.5);
        let pool_profile = 1.50 + 0.74 * r_clamped * r_clamped;
        h = h * (1.0 - pool_w) + pool_profile * pool_w;
    }

    h.clamp(1.5, 10.5)
}

/// Returns true if `(wx, wz)` lies on one of the compacted `SAND_DARK` desert convoy tracks.
fn is_on_convoy_track(wx: f32, wz: f32) -> bool {
    // 1. Central North-South convoy road connecting South Spawn -> Zone B -> North Spawn
    if wz.abs() <= 162.0 {
        let center_x = 1.8 * (wz * 0.025).sin();
        if (wx - center_x).abs() <= 4.6 {
            return true;
        }
    }

    // 2. West-East track from Zone A (-108, -6) into Zone B (0, 0)
    if (-112.0..=4.0).contains(&wx) {
        let t = (wx + 108.0) / 108.0;
        let track_z = -6.0 * (1.0 - t) + 3.0 * (t * std::f32::consts::PI).sin();
        if (wz - track_z).abs() <= 3.6 {
            return true;
        }
    }

    // 3. Eastbound oasis bypass tracks (south and north arcs skirting the pool to Zone C (108, 6))
    if (2.0..=112.0).contains(&wx) {
        let t = wx / 108.0;
        let south_arc_z = 6.0 * t + 21.0 * (t * std::f32::consts::PI).sin().powi(2);
        let north_arc_z = 6.0 * t - 22.0 * (t * std::f32::consts::PI).sin().powi(2);
        if (wz - south_arc_z).abs() <= 3.4 || (wz - north_arc_z).abs() <= 3.0 {
            return true;
        }
    }

    // 4. West flanking convoy track: South-West spawn (-38, 148) -> Zone A -> North-West spawn (-38, -148)
    if (-152.0..=152.0).contains(&wz) {
        let u = (wz + 6.0) / 154.0;
        let west_x = -38.0 - 70.0 * (1.0 - u * u).max(0.0);
        if (wx - west_x).abs() <= 3.6 {
            return true;
        }
    }

    // 5. East flanking convoy track: South-East spawn (38, 148) -> Zone C -> North-East spawn (38, -148)
    if (-152.0..=152.0).contains(&wz) {
        let u = (wz - 6.0) / 154.0;
        let east_x = 38.0 + 70.0 * (1.0 - u * u).max(0.0);
        if (wx - east_x).abs() <= 3.6 {
            return true;
        }
    }

    false
}

/// Selects the surface voxel material for each dry desert column (`SAND`, `SAND_DARK`,
/// `SANDSTONE`, or oasis `GRASS` fringe).
fn select_surface_material(wx: f32, wz: f32, h_m: f32) -> VoxelId {
    // 1. Lush oasis grass fringe directly surrounding the shallow turquoise pool
    let r_pool = oasis_pool_radius(wx, wz);
    if r_pool < 1.65 {
        let angle = (wz - OASIS_CZ).atan2(wx - OASIS_CX);
        let organic = 0.09 * (angle * 5.0).sin() + 0.05 * (angle * 9.0 + 1.2).cos();
        let r_mod = r_pool + organic;
        if r_mod <= 1.38 {
            return if ((wx * 0.25).sin() * (wz * 0.25).cos()) > 0.25 {
                VoxelId::GRASS_DARK
            } else {
                VoxelId::GRASS
            };
        }
        if r_mod <= 1.56 {
            return SAND_DARK;
        }
    }

    // 2. Compacted desert convoy tracks (clearly legible on ground and minimap)
    if is_on_convoy_track(wx, wz) {
        return SAND_DARK;
    }

    // 3. Rocky sandstone cap on the East Mesa (Zone C), dry wadi banks, and high outer ridges
    let d_mesa = mesa_distance(wx, wz);
    let rocky_noise = 0.5 + 0.5 * (wx * 0.09 + wz * 0.07).sin() * (wx * 0.05 - wz * 0.11).cos();
    if d_mesa < 31.0 && (h_m > 6.4 || rocky_noise > 0.42) {
        return SANDSTONE;
    }
    if (d_mesa < 46.0 && rocky_noise > 0.62) || (wx > 142.0 && h_m > 7.4) {
        return SANDSTONE;
    }
    let wadi_edge = (wx - wadi_center_x(wz)).abs();
    if (10.0..=21.0).contains(&wadi_edge) && wz.abs() < 120.0 && rocky_noise > 0.56 {
        return SANDSTONE;
    }
    if wx < -150.0 && h_m > 8.8 && rocky_noise > 0.55 {
        return SANDSTONE;
    }

    // 4. Subtle wind-blown darker sand troughs across sweeping dunes
    let dune_trough = (wx * 0.022 + wz * 0.013).sin();
    if dune_trough < -0.68 && h_m < 6.5 {
        return SAND_DARK;
    }

    SAND
}

/// Returns true if `(wx, wz)` is at least `PROTECTED_CLEAR_RADIUS_M + extra_margin` away from
/// every Allies/Axis spawn point and every capture zone center.
fn is_clear_of_protected(wx: f32, wz: f32, extra_margin: f32) -> bool {
    let min_dist = PROTECTED_CLEAR_RADIUS_M + extra_margin;
    let min_d2 = min_dist * min_dist;

    for &(sx, sz) in ALLIES_SPAWNS.iter().chain(AXIS_SPAWNS.iter()) {
        let dx = wx - sx;
        let dz = wz - sz;
        if dx * dx + dz * dz < min_d2 {
            return false;
        }
    }
    for &(_, zx, zz, _) in &CAPTURE_ZONES {
        let dx = wx - zx;
        let dz = wz - zz;
        if dx * dx + dz * dz < min_d2 {
            return false;
        }
    }
    true
}

/// Deterministic scatter of smaller sandstone outcrops across open outer dunes & wadi flanks,
/// respecting all convoy tracks, buildings, oasis, spawns, and capture zone clear radii.
fn place_desert_scatter_rocks(world: &mut VoxelWorld, buildings: &[OutpostBuilding]) {
    let spacing = 22.0f32;
    let mut gx = -165.0f32;
    while gx <= 165.0 {
        let mut gz = -165.0f32;
        while gz <= 165.0 {
            let r = hash2d(gx as i32, gz as i32, 0xA1A4_E117);
            if r < 0.24 {
                let jx = (hash2d(gx as i32, gz as i32, 101) - 0.5) * 10.0;
                let jz = (hash2d(gx as i32, gz as i32, 202) - 0.5) * 10.0;
                let rx = gx + jx;
                let rz = gz + jz;

                if is_scatter_rock_spot_clear(rx, rz, buildings) {
                    let rad = 1.5 + hash2d(gx as i32, gz as i32, 303) * 1.1;
                    let height = 1.1 + hash2d(gx as i32, gz as i32, 404) * 0.9;
                    build_rock(world, &el_alamein_height_m, rx, rz, rad, height, SANDSTONE);
                }
            }
            gz += spacing;
        }
        gx += spacing;
    }
}

fn is_scatter_rock_spot_clear(wx: f32, wz: f32, buildings: &[OutpostBuilding]) -> bool {
    if wx.abs() > 172.0 || wz.abs() > 172.0 {
        return false;
    }
    // Keep central village, oasis pool, and central 16 m avenue completely uncluttered
    if wx.abs() < 66.0 && wz.abs() < 56.0 {
        return false;
    }
    if wx.abs() < 14.0 {
        return false;
    }
    if !is_clear_of_protected(wx, wz, 5.0) {
        return false;
    }
    // Keep clear of convoy tracks with a 5 m buffer
    for &(ox, oz) in &[
        (0.0, 0.0),
        (5.0, 0.0),
        (-5.0, 0.0),
        (0.0, 5.0),
        (0.0, -5.0),
    ] {
        if is_on_convoy_track(wx + ox, wz + oz) {
            return false;
        }
    }
    // Keep clear of all outpost buildings
    for b in buildings {
        if (wx - b.cx).abs() < b.width_m * 0.5 + 7.0 && (wz - b.cz).abs() < b.depth_m * 0.5 + 7.0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    use voxel::WORLD_VOXEL_SIZE;
    use voxel::glam::{IVec3, Vec3};

    use crate::ai::NavGrid;
    use crate::tank::{TankConfig, TankState};

    #[test]
    fn el_alamein_memory_navgrid_spawns_and_desert_features_verified() {
        let (world, meta) = generate_el_alamein_map();

        // Verify Stage 6 desert material registry slots match our VoxelId constants
        assert_eq!(world.materials.get(SAND).name, "Sand");
        assert_eq!(world.materials.get(SAND_DARK).name, "SandDark");
        assert_eq!(world.materials.get(SANDSTONE).name, "Sandstone");
        assert_eq!(world.materials.get(MUDBRICK).name, "Mudbrick");

        // 1. Strict chunk & sparse voxel memory budget (single 24×24 layer, < 12 MB)
        let chunks = world.chunk_count();
        let mem = world.memory_bytes();
        assert!(
            chunks <= 576,
            "El Alamein chunk count must be <= 576, got {chunks}"
        );
        assert!(
            mem < 12 * 1024 * 1024,
            "El Alamein sparse voxel RAM must be < 12 MB, got {:.2} MB",
            mem as f64 / (1024.0 * 1024.0)
        );
        assert!(
            world.chunk_coords().all(|c| c.y == 0),
            "All El Alamein chunks must reside in vertical layer cy = 0"
        );

        // Verify metadata basics & oasis water presence
        assert_eq!(meta.id, "el_alamein");
        assert_eq!(meta.spawn_allies.len(), 6);
        assert_eq!(meta.spawn_axis.len(), 6);
        assert_eq!(meta.capture_zones.len(), 3);
        let oasis_center_vx = (OASIS_CX / WORLD_VOXEL_SIZE).round() as i32;
        let oasis_center_vz = (OASIS_CZ / WORLD_VOXEL_SIZE).round() as i32;
        assert_eq!(
            world.get_voxel(IVec3::new(oasis_center_vx, OASIS_WATER_VY, oasis_center_vz)),
            VoxelId::WATER,
            "Oasis pool center must contain shallow water at vy = {OASIS_WATER_VY}"
        );

        // 2. NavGrid pathfinding from Allies spawn[0] to all 3 zones (A, B, C) & to Axis spawn[0]
        let nav = NavGrid::build(&world);
        assert!(
            nav.walkable_count() > 3500,
            "Open desert map should have high NavGrid walkability, got {}",
            nav.walkable_count()
        );

        let start = meta.spawn_allies[0];
        for zone in &meta.capture_zones {
            let path = nav.find_path(start, zone.center);
            assert!(
                !path.is_empty(),
                "NavGrid failed to find path from Allies spawn[0] {start:?} to Zone {}",
                zone.label
            );
            let end = *path.last().unwrap();
            let dist_xz = ((end.x - zone.center.x).powi(2) + (end.z - zone.center.z).powi(2)).sqrt();
            assert!(
                dist_xz < zone.radius,
                "Path to Zone {} ended {:.1} m from center (radius {:.1} m)",
                zone.label,
                dist_xz,
                zone.radius
            );
        }

        let enemy_spawn = meta.spawn_axis[0];
        let cross_path = nav.find_path(start, enemy_spawn);
        assert!(
            !cross_path.is_empty(),
            "NavGrid failed to find cross-map path from Allies spawn[0] to Axis spawn[0]"
        );
        let cross_end = *cross_path.last().unwrap();
        let cross_dist =
            ((cross_end.x - enemy_spawn.x).powi(2) + (cross_end.z - enemy_spawn.z).powi(2)).sqrt();
        assert!(
            cross_dist < 12.0,
            "Cross-map path ended {:.1} m from Axis spawn[0] (must be < 12 m)",
            cross_dist
        );

        // 3. Zero hull collisions for all 12 tanks spawned at Allies & Axis spawn points
        let models = voxel::build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);

        for (i, &pos) in meta.spawn_allies.iter().enumerate() {
            let mut tank = TankState::new(cfg.clone(), pos, 0.0);
            tank.settle_on_terrain(&world);
            let hits = tank.count_hull_collisions(&world, tank.position.x, tank.position.z, 0.0);
            assert_eq!(
                hits, 0,
                "Allies spawn[{i}] at {pos:?} has {hits} hull collisions"
            );
        }
        for (i, &pos) in meta.spawn_axis.iter().enumerate() {
            let mut tank = TankState::new(cfg.clone(), pos, PI);
            tank.settle_on_terrain(&world);
            let hits = tank.count_hull_collisions(&world, tank.position.x, tank.position.z, PI);
            assert_eq!(
                hits, 0,
                "Axis spawn[{i}] at {pos:?} has {hits} hull collisions"
            );
        }

        // 4. Verify 14 m radius around every spawn and capture zone center is clear of obstacles
        let protected_points: Vec<(&str, Vec3)> = meta
            .spawn_allies
            .iter()
            .map(|&p| ("Allies spawn", p))
            .chain(meta.spawn_axis.iter().map(|&p| ("Axis spawn", p)))
            .chain(meta.capture_zones.iter().map(|z| ("Capture zone", z.center)))
            .collect();

        for (label, pt) in protected_points {
            let cvx = (pt.x / WORLD_VOXEL_SIZE).round() as i32;
            let cvz = (pt.z / WORLD_VOXEL_SIZE).round() as i32;
            let rv = (PROTECTED_CLEAR_RADIUS_M / WORLD_VOXEL_SIZE).floor() as i32;
            for dz in -rv..=rv {
                for dx in -rv..=rv {
                    let dist = ((dx * dx + dz * dz) as f32).sqrt() * WORLD_VOXEL_SIZE;
                    if dist > PROTECTED_CLEAR_RADIUS_M {
                        continue;
                    }
                    let vx = cvx + dx;
                    let vz = cvz + dz;
                    let wx = (vx as f32 + 0.5) * WORLD_VOXEL_SIZE;
                    let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
                    let expected_top_vy =
                        ((el_alamein_height_m(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1)
                            .max(OASIS_WATER_VY);
                    if let Some((actual_vy, id)) = world.top_solid_voxel_at(vx, vz) {
                        assert!(
                            actual_vy <= expected_top_vy,
                            "{label} at ({:.1}, {:.1}) has obstacle {:?} at ({wx:.1}, {wz:.1}), vy={actual_vy} > terrain vy={expected_top_vy}",
                            pt.x,
                            pt.z,
                            id
                        );
                    }
                }
            }
        }
    }
}
