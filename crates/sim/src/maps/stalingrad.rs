//! **Winter Stalingrad** (`stalingrad_winter`) — Frozen Ruined Industrial City (`384 × 384 m`).
//!
//! Layout overview:
//! - Wide tank-friendly urban grid with 3 north-south avenues (`x = -108, 0, +108`, `>= 20 m` wide)
//!   crossed by 3 east-west streets (`z = -70, 0, +70`, `>= 18 m` wide) plus wide staging boulevards.
//! - **Zone A (`(-108, 0)`, r = 16 m)**: West Tractor Factory Complex — bombed brick & concrete
//!   industrial shells framing an open paved factory yard.
//! - **Zone B (`(0, 0)`, r = 16 m)**: Central Red Square / Fountain Plaza — broad cobblestone square
//!   with a flush frozen fountain mosaic framed by multi-story brick ruins and corner sandbags.
//! - **Zone C (`(108, 0)`, r = 16 m)**: East Frozen Volga Riverbank & Rail Yard — flush steel rail
//!   lines, brick depot warehouses, embankment barricades, winter pines, and a driveable frozen
//!   `ICE` Volga river flank along `x > 136 m` at `Y = 2.5 m`.

use voxel::{VoxelId, VoxelWorld};

use crate::map_gen::{MapAtmosphere, MapMetadata, TacticalPositionKind};
use crate::maps::{
    MapId, build_pine_tree, build_rect_building, build_rock, build_ruined_building,
    build_wall_segment, fill_terrain_columns, finalize_map, gaussian, hash2d, init_standard_world,
};

const VOX_SNOW: VoxelId = VoxelId::SNOW;
const VOX_SNOW_DARK: VoxelId = VoxelId::SNOW_DARK;
const VOX_ICE: VoxelId = VoxelId::ICE;
const VOX_RUBBLE: VoxelId = VoxelId::RUBBLE;
const VOX_PINE_LEAVES: VoxelId = VoxelId::PINE_LEAVES;

/// 6 Allies spawn points along the southern staging boulevard (`z in 148..=162`).
const ALLIES_SPAWNS: [(f32, f32); 6] = [
    (0.0, 148.0),
    (-18.0, 150.0),
    (18.0, 150.0),
    (-38.0, 156.0),
    (38.0, 156.0),
    (0.0, 162.0),
];

/// 6 Axis spawn points along the northern staging boulevard (`z in -162..=-148`).
const AXIS_SPAWNS: [(f32, f32); 6] = [
    (0.0, -148.0),
    (18.0, -150.0),
    (-18.0, -150.0),
    (38.0, -156.0),
    (-38.0, -156.0),
    (0.0, -162.0),
];

/// Capture zones A (West Tractor Factory), B (Red Square), C (Volga Rail Yard).
const CAPTURE_ZONES: [(char, f32, f32, f32); 3] = [
    ('A', -108.0, 0.0, 16.0),
    ('B', 0.0, 0.0, 16.0),
    ('C', 108.0, 0.0, 16.0),
];

/// Curated tactical positions embedded into `MapMetadata`.
const TACTICAL_POSITIONS: [(&str, f32, f32, TacticalPositionKind); 8] = [
    ("Tractor Plant Yard Overlook", -94.0, -20.0, TacticalPositionKind::VillageAmbush),
    ("Pavlov's House Corner", -20.0, -20.0, TacticalPositionKind::VillageAmbush),
    ("Univermag South Ambush", -20.0, 20.0, TacticalPositionKind::VillageAmbush),
    ("Red Square East Overwatch", 20.0, 0.0, TacticalPositionKind::FlankOverwatch),
    ("Volga Ice Flanking Lane", 156.0, 0.0, TacticalPositionKind::FlankOverwatch),
    ("North Rail Yard Embankment", 120.0, -40.0, TacticalPositionKind::HullDownRidge),
    ("South Rail Yard Embankment", 120.0, 40.0, TacticalPositionKind::HullDownRidge),
    ("Lenin Prospekt Crossroad", 0.0, -70.0, TacticalPositionKind::BridgeChokepoint),
];

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth urban winter heightfield in `2.5..=6.5 m` with gentle ramps only (zero unnavigable steps)
/// and a completely flat frozen Volga river sheet (`Y = 2.5 m`) along `x > 136 m`.
fn stalingrad_height_m(wx: f32, wz: f32) -> f32 {
    if wx >= 136.0 {
        return 2.5;
    }

    // Base urban winter plateau
    let mut h = 4.15;

    // Gentle western industrial terrace rise behind the Tractor Factory
    let west_terrace = 1.0 - smoothstep(-154.0, -122.0, wx);
    h += 1.15 * west_terrace;

    // Subtle winter snow knolls on the far western corners and approach flanks
    h += 0.85 * gaussian(wx, wz, -166.0, -152.0, 34.0, 34.0);
    h += 0.85 * gaussian(wx, wz, -166.0, 152.0, 34.0, 34.0);
    h += 0.65 * gaussian(wx, wz, -62.0, -134.0, 38.0, 28.0);
    h += 0.65 * gaussian(wx, wz, 58.0, -134.0, 38.0, 28.0);
    h += 0.65 * gaussian(wx, wz, -62.0, 134.0, 38.0, 28.0);
    h += 0.65 * gaussian(wx, wz, 58.0, 134.0, 38.0, 28.0);

    // Subtle broad-wavelength snowdrift undulation across city blocks
    let drift = 0.22 * (wx * 0.022).sin() * (wz * 0.020).cos()
        + 0.14 * ((wx - wz) * 0.015).cos();

    // Gently level the 3 capture plazas & spawn aprons so foundations and squares sit cleanly flush
    let flat_b = gaussian(wx, wz, 0.0, 0.0, 28.0, 28.0);
    let flat_a = gaussian(wx, wz, -108.0, 0.0, 26.0, 26.0);
    let flat_c = gaussian(wx, wz, 108.0, 0.0, 22.0, 26.0);
    let flat_south = gaussian(wx, wz, 0.0, 154.0, 48.0, 18.0);
    let flat_north = gaussian(wx, wz, 0.0, -154.0, 48.0, 18.0);
    let flatten = (flat_b + flat_a + flat_c + flat_south + flat_north).clamp(0.0, 1.0);

    h += drift * (1.0 - 0.85 * flatten);

    // Smooth embankment descent from x = 116 m down to the frozen Volga ice sheet at x = 136 m
    let bank_t = smoothstep(116.0, 136.0, wx);
    let final_h = h * (1.0 - bank_t) + 2.5 * bank_t;

    final_h.clamp(2.5, 6.5)
}

/// Returns true if `(wx, wz)` lies inside any wide urban avenue, cross street, or staging boulevard.
#[inline]
fn is_on_avenue_corridor(wx: f32, wz: f32, pad_m: f32) -> bool {
    let on_west_ave = (wx + 108.0).abs() <= 10.0 + pad_m && wz.abs() <= 152.0;
    let on_center_ave = wx.abs() <= 10.5 + pad_m && wz.abs() <= 170.0;
    let on_east_ave = (wx - 108.0).abs() <= 10.0 + pad_m && wz.abs() <= 152.0;

    let on_center_st = wz.abs() <= 10.0 + pad_m && (-144.0..=138.0).contains(&wx);
    let on_north_st = (wz + 70.0).abs() <= 9.5 + pad_m && (-164.0..=138.0).contains(&wx);
    let on_south_st = (wz - 70.0).abs() <= 9.5 + pad_m && (-164.0..=138.0).contains(&wx);

    let on_north_blvd = (wz + 140.0).abs() <= 10.0 + pad_m && wx.abs() <= 120.0;
    let on_south_blvd = (wz - 140.0).abs() <= 10.0 + pad_m && wx.abs() <= 120.0;

    on_west_ave
        || on_center_ave
        || on_east_ave
        || on_center_st
        || on_north_st
        || on_south_st
        || on_north_blvd
        || on_south_blvd
}

/// Paints surface materials for snow-blanketed Stalingrad boulevards, squares, factory yards,
/// rail tracks, rubble fringes, and the frozen Volga River.
fn stalingrad_surface_mat(wx: f32, wz: f32, _h_m: f32) -> VoxelId {
    // 1. Frozen Volga River along far eastern border (x > 136 m, smooth driveable ice at Y = 2.5 m)
    if wx > 136.0 {
        let snow_streak = (wx * 0.08 + wz * 0.035).sin() * (wz * 0.065).cos();
        if wx > 178.0 || snow_streak > 0.74 {
            return VOX_SNOW;
        }
        return VOX_ICE;
    }
    if wx > 133.5 {
        return VOX_SNOW_DARK;
    }

    // 2. Central Red Square / Fountain Plaza (Zone B at (0, 0))
    if wx.abs() <= 24.0 && wz.abs() <= 24.0 {
        let r = (wx * wx + wz * wz).sqrt();
        if r < 2.2 {
            return VOX_ICE; // Flush frozen central fountain basin
        }
        if r < 3.4 {
            return VoxelId::COBBLESTONE; // Flush stone fountain rim
        }
        if r < 4.8 {
            return VOX_SNOW; // Snow-dusted inner apron
        }
        if wx.abs() <= 5.5 || wz.abs() <= 5.5 {
            return VoxelId::ROAD_ASPHALT;
        }
        if wx.abs() <= 22.0 && wz.abs() <= 22.0 {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }

    // 3. West Tractor Factory Yard (Zone A at (-108, 0))
    let dx_a = (wx + 108.0).abs();
    if dx_a <= 22.0 && wz.abs() <= 22.0 {
        if dx_a <= 6.0 || wz.abs() <= 5.5 {
            return VoxelId::ROAD_ASPHALT;
        }
        let grid_line = ((wx + 108.0).rem_euclid(6.0) < 0.6) || (wz.rem_euclid(6.0) < 0.6);
        if grid_line {
            return VoxelId::CONCRETE_DARK;
        }
        if dx_a <= 19.5 && wz.abs() <= 19.5 {
            return VoxelId::CONCRETE;
        }
        return VOX_SNOW_DARK;
    }

    // 4. East Volga Rail Yard & Embankment Avenue (Zone C at (108, 0))
    if (wx - 108.0).abs() <= 11.5 && wz.abs() <= 148.0 {
        // Flush twin steel rail tracks along the eastern side of the rail yard (x ≈ 113 & 115.5)
        if ((wx - 113.0).abs() <= 0.35 || (wx - 115.5).abs() <= 0.35) && wz.abs() <= 136.0 {
            return VoxelId::STEEL_TRACK;
        }
        if (111.5..=117.0).contains(&wx) && wz.abs() <= 138.0 {
            // Timber sleepers every 2.0 m on dark gravel/rubble ballast
            if wz.rem_euclid(2.0) < 0.55 {
                return VoxelId::WOOD;
            }
            return VoxelId::CONCRETE_DARK;
        }
        // Main cleared rail-yard cobblestone & asphalt haul road (x in [99, 111.5])
        if (101.0..=109.5).contains(&wx) {
            return VoxelId::ROAD_ASPHALT;
        }
        if (98.5..=111.5).contains(&wx) {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }

    // 5. Primary N-S Avenues & E-W Streets
    // Central Lenin Prospekt (x = 0)
    if wx.abs() <= 12.0 && wz.abs() <= 168.0 {
        if wx.abs() <= 6.5 {
            return VoxelId::ROAD_ASPHALT;
        }
        if wx.abs() <= 9.5 {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }
    // West Tractor Factory Avenue (x = -108)
    if (wx + 108.0).abs() <= 11.5 && wz.abs() <= 148.0 {
        if (wx + 108.0).abs() <= 6.0 {
            return VoxelId::ROAD_ASPHALT;
        }
        if (wx + 108.0).abs() <= 9.0 {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }
    // Central Victory Street (z = 0)
    if wz.abs() <= 11.0 && (-144.0..=134.0).contains(&wx) {
        if wz.abs() <= 6.0 {
            return VoxelId::ROAD_ASPHALT;
        }
        if wz.abs() <= 8.5 {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }
    // North & South Cross Streets (z = -70, +70)
    let d_cross = (wz + 70.0).abs().min((wz - 70.0).abs());
    if d_cross <= 10.5 && (-164.0..=134.0).contains(&wx) {
        if d_cross <= 5.5 {
            return VoxelId::COBBLESTONE;
        }
        if d_cross <= 8.0 {
            return VoxelId::ROAD_ASPHALT;
        }
        return VOX_SNOW_DARK;
    }

    // 6. North & South Tank Deployment Boulevards & Spawn Aprons
    let d_stage = (wz + 140.0).abs().min((wz - 140.0).abs());
    if d_stage <= 9.5 && wx.abs() <= 118.0 {
        if d_stage <= 5.5 {
            return VoxelId::COBBLESTONE;
        }
        return VOX_SNOW_DARK;
    }
    if wx.abs() <= 46.0 && (136.0..=168.0).contains(&wz.abs()) {
        if wx.abs() <= 40.0 && (144.0..=164.0).contains(&wz.abs()) {
            return VOX_SNOW_DARK;
        }
        return VoxelId::COBBLESTONE;
    }

    // 7. Urban rubble fringes hugging city block perimeters & mottled winter snowpack
    let in_city_core = (-162.0..=120.0).contains(&wx) && wz.abs() <= 118.0;
    if in_city_core {
        let h_noise = hash2d((wx * 0.5).floor() as i32, (wz * 0.5).floor() as i32, 701);
        let near_street = is_on_avenue_corridor(wx, wz, 5.5);
        if near_street && h_noise > 0.62 {
            return VOX_RUBBLE;
        }
        if near_street && h_noise > 0.34 {
            return VOX_SNOW_DARK;
        }
    }

    // Tank-churned dark snow drifts vs fresh winter snow
    let drift_patch = (wx * 0.048).sin() * (wz * 0.052).cos();
    if drift_patch > 0.30 {
        VOX_SNOW_DARK
    } else {
        VOX_SNOW
    }
}

/// Checks that a footprint centered at `(cx, cz)` with half-extents `(hw, hd)` stays outside
/// every spawn point's 16 m keep-out radius and every capture zone's 18 m keep-out radius.
fn is_clear_of_spawns_and_zones(cx: f32, cz: f32, hw: f32, hd: f32) -> bool {
    let min_x = cx - hw;
    let max_x = cx + hw;
    let min_z = cz - hd;
    let max_z = cz + hd;

    let dist_to_box = |px: f32, pz: f32| -> f32 {
        let qx = px.clamp(min_x, max_x) - px;
        let qz = pz.clamp(min_z, max_z) - pz;
        (qx * qx + qz * qz).sqrt()
    };

    for &(sx, sz) in ALLIES_SPAWNS.iter().chain(AXIS_SPAWNS.iter()) {
        if dist_to_box(sx, sz) < 16.5 {
            return false;
        }
    }
    for &(_, zx, zz, radius) in &CAPTURE_ZONES {
        if dist_to_box(zx, zz) < radius + 2.0 {
            return false;
        }
    }
    true
}

/// Spec for a bombed-out multi-story Stalingrad ruin.
struct RuinSpec {
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    max_wall_vox: i32,
    wall_mat: VoxelId,
    seed: u32,
}

/// Spec for an intact or semi-intact brick/concrete industrial structure.
struct BuildingSpec {
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    wall_h: i32,
    wall_mat: VoxelId,
    trim_mat: VoxelId,
    roof_mat: VoxelId,
    pitched: bool,
}

/// Generates the complete `384 × 384 m` **Winter Stalingrad** battlefield `(VoxelWorld, MapMetadata)`.
pub fn generate_stalingrad_map() -> (VoxelWorld, MapMetadata) {
    let mut world = init_standard_world();

    // 1. Fill snow-blanketed urban plateau & frozen Volga ice sheet (single chunk layer vy in 0..=31)
    fill_terrain_columns(
        &mut world,
        VoxelId::ROCK,
        VoxelId::DIRT,
        None,
        VoxelId::WATER,
        stalingrad_height_m,
        stalingrad_surface_mat,
    );

    // 2. Bombed-out multi-story urban ruins framing Red Square, city blocks, Tractor Factory, and Rail Yard
    let ruins = [
        // --- Central Red Square (Zone B) landmark ruins (all >= 23 m from x=0 / z=0) ---
        RuinSpec { cx: -32.0, cz: -30.0, w: 18.0, d: 14.0, max_wall_vox: 17, wall_mat: VoxelId::BRICK, seed: 101 },
        RuinSpec { cx: 32.0, cz: -30.0, w: 18.0, d: 14.0, max_wall_vox: 16, wall_mat: VoxelId::BRICK, seed: 102 },
        RuinSpec { cx: -32.0, cz: 30.0, w: 18.0, d: 14.0, max_wall_vox: 17, wall_mat: VoxelId::PLASTER_WALL, seed: 103 },
        RuinSpec { cx: 32.0, cz: 30.0, w: 18.0, d: 14.0, max_wall_vox: 16, wall_mat: VoxelId::BRICK, seed: 104 },
        // --- Mid-North Inner-West block (x in [-92, -46], z in [-56, -16]) ---
        RuinSpec { cx: -68.0, cz: -28.0, w: 20.0, d: 14.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 201 },
        RuinSpec { cx: -54.0, cz: -50.0, w: 18.0, d: 12.0, max_wall_vox: 14, wall_mat: VoxelId::CONCRETE_DARK, seed: 202 },
        // --- Mid-South Inner-West block (x in [-92, -46], z in [16, 56]) ---
        RuinSpec { cx: -68.0, cz: 28.0, w: 20.0, d: 14.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 203 },
        RuinSpec { cx: -54.0, cz: 50.0, w: 18.0, d: 12.0, max_wall_vox: 14, wall_mat: VoxelId::PLASTER_WALL, seed: 204 },
        RuinSpec { cx: -82.0, cz: 49.0, w: 14.0, d: 14.0, max_wall_vox: 13, wall_mat: VoxelId::CONCRETE_DARK, seed: 205 },
        // --- Mid-North Inner-East block (x in [46, 92], z in [-56, -16]) ---
        RuinSpec { cx: 68.0, cz: -28.0, w: 20.0, d: 14.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 301 },
        RuinSpec { cx: 84.0, cz: -48.0, w: 14.0, d: 16.0, max_wall_vox: 14, wall_mat: VoxelId::BRICK, seed: 302 },
        // --- Mid-South Inner-East block (x in [46, 92], z in [16, 56]) ---
        RuinSpec { cx: 68.0, cz: 28.0, w: 20.0, d: 14.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 303 },
        RuinSpec { cx: 54.0, cz: 50.0, w: 18.0, d: 12.0, max_wall_vox: 14, wall_mat: VoxelId::PLASTER_WALL, seed: 304 },
        // --- North Outer City Blocks (z in [-116, -84]) ---
        RuinSpec { cx: -34.0, cz: -98.0, w: 22.0, d: 16.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 401 },
        RuinSpec { cx: -74.0, cz: -100.0, w: 22.0, d: 16.0, max_wall_vox: 14, wall_mat: VoxelId::CONCRETE_DARK, seed: 402 },
        RuinSpec { cx: 34.0, cz: -98.0, w: 22.0, d: 16.0, max_wall_vox: 15, wall_mat: VoxelId::PLASTER_WALL, seed: 403 },
        RuinSpec { cx: 76.0, cz: -100.0, w: 20.0, d: 18.0, max_wall_vox: 14, wall_mat: VoxelId::BRICK, seed: 404 },
        // --- South Outer City Blocks (z in [84, 116]) ---
        RuinSpec { cx: -34.0, cz: 98.0, w: 22.0, d: 16.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 501 },
        RuinSpec { cx: -74.0, cz: 100.0, w: 22.0, d: 16.0, max_wall_vox: 14, wall_mat: VoxelId::CONCRETE_DARK, seed: 502 },
        RuinSpec { cx: 34.0, cz: 98.0, w: 22.0, d: 16.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 503 },
        // --- West Tractor Factory Complex (Zone A, x in [-168, -130]) ---
        RuinSpec { cx: -148.0, cz: -28.0, w: 26.0, d: 20.0, max_wall_vox: 17, wall_mat: VoxelId::CONCRETE_DARK, seed: 601 },
        RuinSpec { cx: -148.0, cz: 28.0, w: 26.0, d: 20.0, max_wall_vox: 17, wall_mat: VoxelId::BRICK, seed: 602 },
        RuinSpec { cx: -156.0, cz: 0.0, w: 18.0, d: 16.0, max_wall_vox: 16, wall_mat: VoxelId::CONCRETE_DARK, seed: 603 },
        RuinSpec { cx: -146.0, cz: 98.0, w: 26.0, d: 18.0, max_wall_vox: 15, wall_mat: VoxelId::BRICK, seed: 604 },
        RuinSpec { cx: -148.0, cz: -142.0, w: 20.0, d: 14.0, max_wall_vox: 13, wall_mat: VoxelId::CONCRETE_DARK, seed: 605 },
        RuinSpec { cx: -148.0, cz: 142.0, w: 20.0, d: 14.0, max_wall_vox: 13, wall_mat: VoxelId::BRICK, seed: 606 },
    ];

    for r in &ruins {
        debug_assert!(is_clear_of_spawns_and_zones(r.cx, r.cz, r.w * 0.5, r.d * 0.5));
        build_ruined_building(
            &mut world,
            &stalingrad_height_m,
            r.cx,
            r.cz,
            r.w,
            r.d,
            r.max_wall_vox,
            r.wall_mat,
            VOX_RUBBLE,
            r.seed,
        );
    }

    // 3. Semi-intact brick warehouses & concrete Tractor Factory halls for silhouette contrast
    let buildings = [
        // Tractor Factory North Reinforced Hall
        BuildingSpec {
            cx: -146.0,
            cz: -98.0,
            w: 26.0,
            d: 18.0,
            wall_h: 13,
            wall_mat: VoxelId::CONCRETE_DARK,
            trim_mat: VoxelId::CONCRETE,
            roof_mat: VoxelId::ROOF_SLATE,
            pitched: false,
        },
        // Inner-West Workers' Blockhouse
        BuildingSpec {
            cx: -82.0,
            cz: -49.0,
            w: 14.0,
            d: 14.0,
            wall_h: 11,
            wall_mat: VoxelId::BRICK,
            trim_mat: VoxelId::CONCRETE_DARK,
            roof_mat: VoxelId::ROOF_SLATE,
            pitched: true,
        },
        // Inner-East Brick Supply House
        BuildingSpec {
            cx: 54.0,
            cz: -50.0,
            w: 18.0,
            d: 12.0,
            wall_h: 11,
            wall_mat: VoxelId::BRICK,
            trim_mat: VoxelId::CONCRETE,
            roof_mat: VoxelId::ROOF_SLATE,
            pitched: true,
        },
        // East Rail Yard Brick Freight Warehouse (South of Zone C)
        BuildingSpec {
            cx: 84.0,
            cz: 48.0,
            w: 14.0,
            d: 16.0,
            wall_h: 12,
            wall_mat: VoxelId::BRICK,
            trim_mat: VoxelId::CONCRETE_DARK,
            roof_mat: VoxelId::ROOF_SLATE,
            pitched: false,
        },
        // South-East Rail Depot Shed
        BuildingSpec {
            cx: 76.0,
            cz: 100.0,
            w: 20.0,
            d: 18.0,
            wall_h: 12,
            wall_mat: VoxelId::BRICK,
            trim_mat: VoxelId::CONCRETE_DARK,
            roof_mat: VoxelId::ROOF_SLATE,
            pitched: false,
        },
    ];

    for b in &buildings {
        debug_assert!(is_clear_of_spawns_and_zones(b.cx, b.cz, b.w * 0.5, b.d * 0.5));
        build_rect_building(
            &mut world,
            &stalingrad_height_m,
            b.cx,
            b.cz,
            b.w,
            b.d,
            b.wall_h,
            b.wall_mat,
            b.trim_mat,
            b.roof_mat,
            b.pitched,
        );
    }

    // 4. Sandbag barricades & concrete anti-tank parapets (strictly outside avenues & 18m zone radii)
    let walls: [(f32, f32, f32, f32, i32, i32, VoxelId); 14] = [
        // Red Square corner sandbag emplacements (dist >= 23.4 m from (0, 0))
        (-20.0, -18.0, -15.0, -18.0, 2, 0, VoxelId::SANDBAG),
        (15.0, -18.0, 20.0, -18.0, 2, 0, VoxelId::SANDBAG),
        (-20.0, 18.0, -15.0, 18.0, 2, 0, VoxelId::SANDBAG),
        (15.0, 18.0, 20.0, 18.0, 2, 0, VoxelId::SANDBAG),
        // Tractor Factory perimeter yard walls (dist >= 22 m from (-108, 0))
        (-129.0, -16.0, -123.0, -16.0, 2, 0, VoxelId::CONCRETE_DARK),
        (-129.0, 16.0, -123.0, 16.0, 2, 0, VoxelId::CONCRETE_DARK),
        (-92.0, -18.0, -86.0, -18.0, 2, 0, VoxelId::SANDBAG),
        (-92.0, 18.0, -86.0, 18.0, 2, 0, VoxelId::SANDBAG),
        // East Rail Yard & Volga Embankment concrete/sandbag barricades (leaving wide access onto ice)
        (92.0, -18.0, 92.0, -13.0, 2, 0, VoxelId::SANDBAG),
        (92.0, 13.0, 92.0, 18.0, 2, 0, VoxelId::SANDBAG),
        (123.0, -46.0, 123.0, -32.0, 2, 0, VoxelId::CONCRETE_DARK),
        (123.0, 32.0, 123.0, 46.0, 2, 0, VoxelId::CONCRETE_DARK),
        (123.0, -108.0, 123.0, -94.0, 2, 0, VoxelId::SANDBAG),
        (123.0, 94.0, 123.0, 108.0, 2, 0, VoxelId::SANDBAG),
    ];

    for &(x0, z0, x1, z1, h_vox, half_thick, mat) in &walls {
        build_wall_segment(
            &mut world,
            &stalingrad_height_m,
            x0,
            z0,
            x1,
            z1,
            h_vox,
            half_thick,
            mat,
        );
    }

    // 5. Urban masonry rubble mounds tucked beside ruined blocks & river boulders along Volga slope
    let rocks: [(f32, f32, f32, f32, VoxelId); 14] = [
        // Rubble mounds adjacent to ruined facades (inside city blocks, clear of avenues)
        (-44.0, -24.0, 2.0, 1.2, VOX_RUBBLE),
        (44.0, -24.0, 2.0, 1.2, VOX_RUBBLE),
        (-44.0, 24.0, 2.0, 1.2, VOX_RUBBLE),
        (44.0, 24.0, 2.0, 1.2, VOX_RUBBLE),
        (-132.0, -36.0, 2.2, 1.3, VOX_RUBBLE),
        (-132.0, 36.0, 2.2, 1.3, VOX_RUBBLE),
        (-86.0, -26.0, 1.8, 1.1, VOX_RUBBLE),
        (86.0, 26.0, 1.8, 1.1, VOX_RUBBLE),
        // Frost-dusted Volga embankment boulders (outside street exits & ice sheet)
        (129.0, -40.0, 2.2, 1.3, VoxelId::ROCK),
        (130.0, 40.0, 2.2, 1.3, VoxelId::ROCK),
        (128.0, -102.0, 2.4, 1.4, VoxelId::ROCK),
        (128.0, 102.0, 2.4, 1.4, VoxelId::ROCK),
        (131.0, -148.0, 2.0, 1.2, VoxelId::ROCK),
        (131.0, 148.0, 2.0, 1.2, VoxelId::ROCK),
    ];

    for &(rx, rz, rad, h_m, mat) in &rocks {
        if is_clear_of_spawns_and_zones(rx, rz, rad + 1.0, rad + 1.0) {
            build_rock(&mut world, &stalingrad_height_m, rx, rz, rad, h_m, mat);
        }
    }

    // 6. Winter pine trees along Volga embankments, western factory perimeter, and outer snow parks
    let embankment_trees: [(f32, f32); 20] = [
        (127.0, -158.0),
        (131.0, -138.0),
        (127.0, -112.0),
        (132.0, -90.0),
        (127.0, -50.0),
        (132.0, -30.0),
        (127.0, 30.0),
        (132.0, 50.0),
        (127.0, 90.0),
        (132.0, 112.0),
        (127.0, 138.0),
        (131.0, 158.0),
        // Sparse frost-bitten courtyard & park pines along north/south outskirts
        (-86.0, -162.0),
        (-66.0, -168.0),
        (72.0, -164.0),
        (92.0, -168.0),
        (-86.0, 162.0),
        (-66.0, 168.0),
        (72.0, 164.0),
        (92.0, 168.0),
    ];

    for (idx, &(tx, tz)) in embankment_trees.iter().enumerate() {
        let trunk_h = 7 + (hash2d(tx as i32, tz as i32, 900 + idx as u32) * 3.0) as i32;
        if is_clear_of_spawns_and_zones(tx, tz, 3.0, 3.0)
            && !is_on_avenue_corridor(tx, tz, 3.0)
        {
            build_pine_tree(
                &mut world,
                &stalingrad_height_m,
                tx,
                tz,
                trunk_h,
                VOX_PINE_LEAVES,
            );
        }
    }

    // Sparse perimeter winter pines on the far western ridge behind the Tractor Factory
    let mut gz = -168.0f32;
    while gz <= 168.0 {
        let h = hash2d(-172, gz as i32, 1337);
        if h > 0.35 {
            let tx = -172.0 + (h - 0.5) * 6.0;
            let tz = gz + (hash2d(-170, gz as i32, 1338) - 0.5) * 5.0;
            if tz.abs() <= 174.0
                && !is_on_avenue_corridor(tx, tz, 4.0)
                && is_clear_of_spawns_and_zones(tx, tz, 3.5, 3.5)
            {
                let trunk_h = 7 + ((h * 3.0) as i32);
                build_pine_tree(
                    &mut world,
                    &stalingrad_height_m,
                    tx,
                    tz,
                    trunk_h,
                    VOX_PINE_LEAVES,
                );
            }
        }
        gz += 16.0;
    }

    finalize_map(
        world,
        MapId::StalingradWinter,
        MapAtmosphere::stalingrad_winter(),
        &ALLIES_SPAWNS,
        &AXIS_SPAWNS,
        &CAPTURE_ZONES,
        &TACTICAL_POSITIONS,
    )
}

#[cfg(test)]
mod tests {
    use voxel::build_basic_tank_models;

    use super::*;
    use crate::ai::NavGrid;
    use crate::battle::Team;
    use crate::sim_world::SimWorld;
    use crate::tank::TankConfig;

    #[test]
    fn stalingrad_winter_meets_memory_navgrid_and_spawn_collision_guarantees() {
        let (world, meta) = generate_stalingrad_map();

        // 1. Metadata & biome verification
        assert_eq!(meta.id, "stalingrad_winter");
        assert_eq!(meta.name, "Winter Stalingrad");
        assert_eq!(meta.spawn_allies.len(), 6);
        assert_eq!(meta.spawn_axis.len(), 6);
        assert_eq!(meta.capture_zones.len(), 3);

        // Verify terrain heightfield stays strictly within 2.5..=6.5 m everywhere
        for gz in (-184..=184).step_by(8) {
            for gx in (-184..=184).step_by(8) {
                let h = stalingrad_height_m(gx as f32, gz as f32);
                assert!(
                    (2.5..=6.5).contains(&h),
                    "terrain height at ({gx}, {gz}) out of 2.5..=6.5 m: {h}"
                );
            }
        }

        // Frozen Volga River eastern flank must be smooth driveable ICE at Y = 2.5 m
        let ice_h = world.surface_height_at(156.0, 0.0).expect("Volga ice surface");
        assert!(
            (ice_h - 2.5).abs() < 0.01,
            "frozen Volga river ice should sit at Y = 2.5 m, got {ice_h}"
        );
        let vx = (156.0 / world.voxel_size).floor() as i32;
        let vz = 0;
        let (_, ice_mat) = world.top_solid_voxel_at(vx, vz).expect("ice voxel");
        assert_eq!(ice_mat, VOX_ICE);

        // 2. Strict chunk & RAM budgets
        assert!(
            world.chunk_count() <= 576,
            "chunk_count {} exceeded 576",
            world.chunk_count()
        );
        assert!(
            world.memory_bytes() < 12 * 1024 * 1024,
            "memory_bytes {} exceeded 12 MB",
            world.memory_bytes()
        );

        // 3. NavGrid connectivity: spawn_allies[0] -> Zones A, B, C and -> spawn_axis[0]
        let nav = NavGrid::build(&world);
        assert!(nav.walkable_count() > 3000, "urban grid should be broadly walkable");

        for &sp in meta.spawn_allies.iter().chain(meta.spawn_axis.iter()) {
            assert!(
                nav.is_walkable_at(sp),
                "spawn point {sp:?} must lie on a walkable NavGrid cell"
            );
        }

        let start = meta.spawn_allies[0];
        for zone in &meta.capture_zones {
            let path = nav.find_path(start, zone.center);
            assert!(
                !path.is_empty(),
                "NavGrid failed to find path from {:?} to Zone {}",
                start,
                zone.label
            );
            let end = *path.last().unwrap();
            let dist_xz =
                ((end.x - zone.center.x).powi(2) + (end.z - zone.center.z).powi(2)).sqrt();
            assert!(
                dist_xz < zone.radius,
                "path to Zone {} ended {:.1} m away (must be < {:.1} m)",
                zone.label,
                dist_xz,
                zone.radius
            );
        }

        let enemy_spawn = meta.spawn_axis[0];
        let cross_path = nav.find_path(start, enemy_spawn);
        assert!(
            !cross_path.is_empty(),
            "NavGrid failed to find path from Allies spawn to Axis spawn"
        );
        let cross_end = *cross_path.last().unwrap();
        let cross_dist =
            ((cross_end.x - enemy_spawn.x).powi(2) + (cross_end.z - enemy_spawn.z).powi(2)).sqrt();
        assert!(
            cross_dist < 12.0,
            "cross-map path ended {:.1} m from Axis spawn (must be < 12.0 m)",
            cross_dist
        );

        // Frozen Volga flank must also be walkable
        assert!(
            nav.is_walkable_at(voxel::glam::Vec3::new(156.0, 2.5, 0.0)),
            "frozen Volga river ice must be walkable on NavGrid"
        );

        // 4. Zero hull collisions for all 12 spawned tanks (6 Allies + 6 Axis)
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let sim = SimWorld::new_team_battle(world, meta, cfg, 6);
        assert_eq!(sim.tanks.len(), 12);
        assert_eq!(sim.alive_count(Team::Allies), 6);
        assert_eq!(sim.alive_count(Team::Axis), 6);

        for unit in &sim.tanks {
            let colls = unit.state.count_hull_collisions(
                &sim.voxels,
                unit.state.position.x,
                unit.state.position.z,
                unit.state.hull_yaw,
            );
            assert_eq!(
                colls, 0,
                "tank {} spawned with {} hull collisions at {:?}",
                unit.name, colls, unit.state.position
            );
        }
    }
}
