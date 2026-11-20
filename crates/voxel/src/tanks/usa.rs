//! Bespoke high-detail `0.10 m` voxel models for all 8 USA armored vehicles (Tiers I–VIII).
//!
//! Coordinate convention (`TankCanvas`):
//! - `x`: `-` left, `+` right, `0` centerline
//! - `y`: `0` ground (hull) or turret ring (turret), `+` up
//! - `z`: `-` FRONT (bow / gun), `+` REAR (engine deck)

use glam::Vec3;

use crate::model::{MuzzleBrakeStyle, TankVoxelModels};
use crate::tanks::{
    TankCanvas, TankPalette, assemble_tank, build_detailed_barrel, build_running_gear, draw_bow_mg,
    draw_cupola, draw_engine_grilles, draw_headlight, draw_white_star_flanks,
};

/// Dispatches a USA vehicle catalog ID (`"us_*"`) to its bespoke `0.10 m` voxel sculptor.
pub fn build_usa_vehicle(vehicle_id: &str) -> Option<TankVoxelModels> {
    match vehicle_id {
        "us_m2a4" => Some(build_m2a4()),
        "us_m3_stuart" => Some(build_m3_stuart()),
        "us_m4a1" => Some(build_m4a1()),
        "us_m10" => Some(build_m10()),
        "us_m4a3e8" => Some(build_m4a3e8()),
        "us_m4a3e2_76" => Some(build_m4a3e2_76()),
        "us_m26" => Some(build_m26()),
        "us_t34" => Some(build_t34()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Shared USA sculpting helpers
// ---------------------------------------------------------------------------

/// Draws symmetric heavy dark steel tow shackles jutting out the front (`-Z`) or rear (`+Z`).
fn draw_tow_shackles(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32, dir_z: i32) {
    c.set_sym(x, y, z, p.trim);
    c.set_sym(x, y + 1, z, p.trim);
    c.set_sym(x, y, z + dir_z, p.trim);
}

/// Draws a US fender headlight with a bright forward lens and dark steel brush guard arch.
fn draw_brush_guard_headlight(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32) {
    draw_headlight(c, p, x, y, z);
    // Brush guard top arch & outer post
    c.set(x, y + 1, z, p.trim);
    let outer = if x >= 0 { x + 1 } else { x - 1 };
    c.set(outer, y, z, p.trim);
}

/// Draws a horizontal front armored driver visor hood with dark optic glass slit facing `-Z`.
fn draw_visor_slit(
    c: &mut TankCanvas,
    p: &TankPalette,
    x0: i32,
    x1: i32,
    y: i32,
    z: i32,
) {
    c.fill(x0, y + 1, z, x1, y + 1, z + 1, p.dark);
    c.fill(x0, y, z, x1, y, z, p.optic);
}

/// Draws a front-facing US white 3x3 cross-star emblem on the glacis/bow at `z`.
fn draw_white_star_front(c: &mut TankCanvas, p: &TankPalette, cx: i32, cy: i32, z: i32) {
    c.set(cx, cy, z, p.white);
    c.set(cx, cy + 1, z, p.white);
    c.set(cx, cy - 1, z, p.white);
    c.set(cx - 1, cy, z, p.white);
    c.set(cx + 1, cy, z, p.white);
}

/// Draws a thin dark steel whip radio aerial rising vertically from a spring base.
fn draw_radio_antenna(c: &mut TankCanvas, p: &TankPalette, x: i32, y0: i32, y1: i32, z: i32) {
    c.set(x, y0, z, p.dark);
    for y in (y0 + 1)..=y1 {
        c.set(x, y, z, p.trim);
    }
}

/// Draws a compact roof-mounted `.50 cal` M2 Browning AA machine gun on a pintle post.
fn draw_50cal_pintle(c: &mut TankCanvas, p: &TankPalette, x: i32, y_base: i32, z: i32) {
    // Pintle post
    c.set(x, y_base, z, p.trim);
    // Receiver box + ammo can
    c.fill(x, y_base + 1, z - 1, x, y_base + 1, z + 1, p.trim);
    c.set(x - 1, y_base + 1, z, p.dark);
    // Forward heavy barrel pointing -Z
    c.set(x, y_base + 1, z - 2, p.trim);
    c.set(x, y_base + 1, z - 3, p.trim);
}

/// Draws a recessed dark steel turret race ring on top of the hull roof.
fn draw_turret_ring(c: &mut TankCanvas, p: &TankPalette, cz: f32, roof_y: i32, r: f32) {
    c.fill_ellipse_y(0.0, cz, r, r, roof_y, roof_y, p.dark);
    c.fill_ellipse_y(0.0, cz, (r - 1.2).max(1.0), (r - 1.2).max(1.0), roof_y, roof_y, p.trim);
}

/// Draws iconic US Sherman / M10 VVSS (Vertical Volute Spring Suspension) bogie housings
/// overlaid on the outer track face at `outer_x` (3 twin-wheel vertical bogie units).
fn draw_vvss_bogies(
    c: &mut TankCanvas,
    p: &TankPalette,
    outer_x: i32,
    z_centers: &[i32],
    track_top_y: i32,
) {
    for &zc in z_centers {
        // Vertical volute spring housing between the twin road wheels
        c.fill_sym(outer_x, 3, zc - 1, outer_x, track_top_y - 2, zc + 1, p.dark);
        // Horizontal suspension cross-beam & return skid on top of bogie
        c.fill_sym(
            outer_x,
            track_top_y - 2,
            zc - 2,
            outer_x,
            track_top_y - 2,
            zc + 2,
            p.trim,
        );
        // Bolt hub on spring cap
        c.set_sym(outer_x, 5, zc, p.trim);
    }
}

/// Draws twin rusty rear exhaust pipes / armored deflectors at the rear hull plate (`+Z`).
fn draw_rear_exhausts(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32) {
    c.fill_sym(x - 1, y, z, x, y + 1, z, p.rust);
    c.set_sym(x, y, z + 1, p.rust);
}

/// Draws spare track links mounted across the front lower nose or glacis.
fn draw_spare_track_bar(c: &mut TankCanvas, p: &TankPalette, x_half: i32, y0: i32, y1: i32, z: i32) {
    for x in -x_half..=x_half {
        let mat = if (x + x_half) % 2 == 0 { p.track } else { p.trim };
        for y in y0..=y1 {
            c.set(x, y, z, mat);
        }
    }
}

/// Draws a low armored oval/circular crew roof hatch at `(cx, cz, y)`.
fn draw_roof_hatch(c: &mut TankCanvas, p: &TankPalette, cx: f32, cz: f32, y: i32, rx: f32, rz: f32) {
    c.fill_ellipse_y(cx, cz, rx, rz, y, y, p.dark);
    c.set(cx.round() as i32, y, (cz - rz * 0.5).round() as i32, p.trim);
}

/// Linearly interpolates a half-width or height smoothly across an integer range.
#[inline]
fn lerp_i32(a: i32, b: i32, t: f32) -> i32 {
    (a as f32 + (b - a) as f32 * t.clamp(0.0, 1.0)).round() as i32
}

// ---------------------------------------------------------------------------
// 1. M2A4 Light Tank (Tier I) — ~4.4m long x 2.4m wide
// ---------------------------------------------------------------------------

fn build_m2a4() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(12, 20, 22);

    // Running gear: 4 VVSS road wheels, 2 return rollers, narrow light tracks
    build_running_gear(&mut hull, p, 8, 11, -19, 19, 7, 4, 2.0, 2, 8);
    // Prominent raised rear idler & twin VVSS bogie housings
    hull.fill_cyl_x_sym(9, 11, 5.2, 17.0, 2.3, p.wheel);
    hull.fill_cyl_x_sym(11, 11, 5.2, 17.0, 1.3, p.dark);
    draw_vvss_bogies(&mut hull, p, 11, &[-9, 6], 7);

    // Lower hull tub & bolted differential nose
    hull.fill(-7, 3, -19, 7, 7, 18, p.dark);
    hull.fill(-7, 4, -20, 7, 6, -19, p.dark);
    hull.fill(-6, 5, -20, 6, 5, -20, p.trim); // bolted transmission flange
    draw_tow_shackles(&mut hull, p, 5, 4, -21, -1);

    // Sloped lower glacis deck leading up to vertical front plate
    for z in -19..=-14 {
        let t = (z + 19) as f32 / 5.0;
        let y_top = lerp_i32(6, 9, t);
        hull.fill(-8, 6, z, 8, y_top, z, p.main);
    }

    // Tall narrow riveted boxy upper hull superstructure
    hull.fill(-9, 8, -13, 9, 13, 17, p.main);
    // Crisp dark riveted edge framing along vertical upper plate & hull roof corners
    hull.fill(-9, 8, -13, 9, 8, -13, p.dark);
    hull.fill(-9, 13, -13, 9, 13, -13, p.dark);
    hull.fill_sym(9, 8, -13, 9, 13, -13, p.dark);
    hull.fill_sym(9, 13, -13, 9, 13, 17, p.dark);

    // Iconic M2A4 dual protruding cheek sponson hull MGs (unmistakable head-on!)
    hull.fill_sym(8, 9, -15, 10, 12, -11, p.dark);
    hull.fill_sym(9, 10, -15, 10, 11, -12, p.main);
    // Twin forward cheek .30 cal MG barrels
    hull.fill_sym(9, 10, -18, 9, 10, -16, p.trim);

    // Central raised driver visor hood + optic slit
    hull.fill(-3, 10, -14, 3, 13, -13, p.dark);
    draw_visor_slit(&mut hull, p, -2, 2, 11, -14);
    // Small bow MG right of driver
    hull.set(5, 10, -14, p.dark);
    hull.fill(5, 10, -16, 5, 10, -15, p.trim);

    // Dual headlights in brush guards on front fenders
    draw_brush_guard_headlight(&mut hull, p, -10, 9, -18);
    draw_brush_guard_headlight(&mut hull, p, 10, 9, -18);

    // Tall rear radial-engine deck with cooling grilles & twin exhausts
    hull.fill(-8, 13, 7, 8, 14, 17, p.main);
    draw_engine_grilles(&mut hull, p, 6, 14, 9, 16);
    hull.fill(-8, 8, 18, 8, 13, 18, p.dark);
    draw_rear_exhausts(&mut hull, p, 5, 9, 19);
    // Wooden pioneer tool strapped to right fender
    hull.fill(10, 9, 4, 10, 9, 13, p.wood);

    // US white stars on upper hull flanks & front lower glacis
    draw_white_star_flanks(&mut hull, p, 9, 10, 0);
    draw_white_star_front(&mut hull, p, 0, 8, -16);

    // Turret ring on hull roof (`y = 13`)
    draw_turret_ring(&mut hull, p, -3.0, 13, 5.2);

    // Compact tall octagonal 37mm turret (`y = 0..=7`) + small octagonal cupola (`y = 8..=11`)
    let mut turret = TankCanvas::new(8, 14, 9);
    for y in 0..=7i32 {
        let shrink = if y >= 6 { 1 } else { 0 };
        let half: i32 = 6 - shrink;
        let oct: i32 = 9 - shrink;
        for z in -half..=half {
            for x in -half..=half {
                if x.abs() + z.abs() <= oct {
                    let is_corner = x.abs() + z.abs() == oct && (y == 0 || y == 7);
                    turret.set(x, y, z, if is_corner { p.dark } else { p.main });
                }
            }
        }
    }
    // Front 37mm M5 armored mantlet & coaxial MG
    turret.fill(-3, 2, -7, 3, 6, -6, p.dark);
    turret.set(2, 4, -8, p.trim); // coax MG
    turret.set(-2, 4, -7, p.optic); // gun sight

    // Small octagonal commander cupola on turret roof
    for y in 8..=10i32 {
        for z in -2..=4i32 {
            for x in -3..=3i32 {
                let dz: i32 = z - 1;
                if x.abs() + dz.abs() <= 4 {
                    turret.set(x, y, z, p.dark);
                }
            }
        }
    }
    // Cupola vision ports & octagonal hatch lid
    turret.set(0, 9, -2, p.optic);
    turret.set(-3, 9, 1, p.black);
    turret.set(3, 9, 1, p.black);
    turret.set(0, 9, 4, p.black);
    turret.fill(-2, 11, -1, 2, 11, 3, p.main);

    // Rear turret radio aerial & white side stars
    draw_radio_antenna(&mut turret, p, -5, 7, 13, 4);
    draw_white_star_flanks(&mut turret, p, 6, 4, 0);

    let barrel = build_detailed_barrel(p, 14, false, 2, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -3.0),
        Vec3::new(0.0, 4.0, -7.0),
        14.0,
    )
}

// ---------------------------------------------------------------------------
// 2. M3 Stuart Light Tank (Tier II) — ~4.5m long x 2.3m wide
// ---------------------------------------------------------------------------

fn build_m3_stuart() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(12, 20, 23);

    // Running gear: 4 bogie wheels + large trailing ground-contact idler wheel!
    build_running_gear(&mut hull, p, 8, 11, -20, 20, 7, 4, 2.0, 3, 8);
    // Iconic M3 Stuart large trailing ground-contact rear idler wheel (`r = 3.0`)
    hull.fill_cyl_x_sym(9, 11, 3.2, 17.5, 3.0, p.wheel);
    hull.fill_cyl_x_sym(11, 11, 3.2, 17.5, 1.8, p.dark);
    hull.set_sym(11, 3, 18, p.trim);
    // Twin VVSS bogie units
    draw_vvss_bogies(&mut hull, p, 11, &[-11, 4], 7);

    // Lower hull belly & sharp angled lower nose
    hull.fill(-7, 3, -19, 7, 7, 19, p.dark);
    for z in -21..=-17 {
        let t = (z + 21) as f32 / 4.0;
        let y_lo = lerp_i32(5, 3, t);
        let y_hi = lerp_i32(6, 8, t);
        hull.fill(-8, y_lo, z, 8, y_hi, z, p.dark);
    }
    hull.fill(-7, 6, -21, 7, 6, -21, p.trim); // sharp riveted nose seam
    draw_tow_shackles(&mut hull, p, 5, 5, -22, -1);

    // Horizontal stepped shelf ahead of vertical driver plate
    hull.fill(-8, 8, -17, 8, 8, -13, p.main);

    // Iconic stepped riveted upper hull with crisp vertical front plate at z = -13
    hull.fill(-9, 8, -13, 9, 13, 19, p.main);
    // Riveted dark edge strips along the stepped front plate & upper hull perimeter
    hull.fill(-9, 13, -13, 9, 13, -13, p.dark);
    hull.fill_sym(9, 8, -13, 9, 13, -13, p.dark);
    hull.fill_sym(9, 13, -13, 9, 13, 19, p.dark);

    // Driver visor hood on left (`-X`), Bow MG on right (`+X`)
    hull.fill(-7, 10, -14, -2, 12, -13, p.dark);
    draw_visor_slit(&mut hull, p, -6, -3, 11, -14);
    draw_bow_mg(&mut hull, p, 4, 10, -14);

    // Dual headlights in brush guards on front fenders
    draw_brush_guard_headlight(&mut hull, p, -10, 9, -19);
    draw_brush_guard_headlight(&mut hull, p, 10, 9, -19);

    // Rear engine deck grilles, radial engine louver box, & twin exhausts
    draw_engine_grilles(&mut hull, p, 7, 13, 8, 18);
    hull.fill(-8, 8, 20, 8, 12, 20, p.dark);
    draw_rear_exhausts(&mut hull, p, 5, 9, 21);
    // Side stowage & pioneer tool
    hull.fill(-11, 9, 6, -10, 10, 14, p.dark);
    hull.fill(10, 9, 5, 10, 9, 15, p.wood);

    // US white stars on flanks & center front plate
    draw_white_star_flanks(&mut hull, p, 9, 10, 1);
    draw_white_star_front(&mut hull, p, 0, 10, -13);

    draw_turret_ring(&mut hull, p, -3.0, 13, 5.4);

    // Crisp octagonal riveted turret topped by a tall circular armored cupola!
    let mut turret = TankCanvas::new(8, 15, 9);
    for y in 0..=6i32 {
        let half: i32 = 6;
        let oct: i32 = 9;
        for z in -half..=half {
            for x in -half..=half {
                if x.abs() + z.abs() <= oct {
                    let on_rivet_seam = (x.abs() + z.abs() == oct) && (y == 0 || y == 6);
                    turret.set(x, y, z, if on_rivet_seam { p.dark } else { p.main });
                }
            }
        }
    }
    // Boxy 37mm M6 armored gun shield / mantlet protruding from turret front
    turret.fill(-3, 1, -8, 3, 5, -6, p.dark);
    turret.fill(-2, 2, -8, 2, 4, -8, p.main);
    turret.set(2, 3, -9, p.trim); // coax MG
    turret.set(-2, 3, -8, p.optic); // gun sight

    // Signature tall circular armored commander cupola on roof (`y = 7..=11`)
    draw_cupola(&mut turret, p, 0.0, 0.5, 7, 4, 3.2);

    // Radio aerial & side white stars
    draw_radio_antenna(&mut turret, p, -5, 6, 14, 4);
    draw_white_star_flanks(&mut turret, p, 6, 3, 0);

    let barrel = build_detailed_barrel(p, 17, false, 3, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -3.0),
        Vec3::new(0.0, 3.0, -8.0),
        17.0,
    )
}

// ---------------------------------------------------------------------------
// 3. M4A1 Sherman Medium Tank (Tier III) — ~5.8m long x 2.6m wide
// ---------------------------------------------------------------------------

fn build_m4a1() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(14, 22, 30);

    // Running gear: 3 VVSS vertical volute spring bogies (6 road wheels) + top return skids
    build_running_gear(&mut hull, p, 9, 12, -26, 26, 8, 6, 2.0, 3, 9);
    draw_vvss_bogies(&mut hull, p, 12, &[-15, 0, 15], 8);

    // Lower hull belly
    hull.fill(-8, 3, -24, 8, 8, 25, p.dark);

    // Iconic rounded 3-piece bolted cast lower differential housing nose (`p.dark`)
    for z in -28..=-23 {
        let dz = (z + 23) as f32; // -5..0
        let y_lo = (3.5 + (dz * dz) * 0.12).round() as i32;
        let y_hi = (8.8 - (dz * dz) * 0.08).round() as i32;
        let hw = if z == -28 { 7 } else { 8 };
        hull.fill(-hw, y_lo, z, hw, y_hi, z, p.dark);
    }
    // 3-piece bolted flange ribs across differential housing
    hull.fill(-7, 6, -28, 7, 6, -28, p.trim);
    hull.fill_sym(4, 4, -27, 4, 8, -27, p.trim);
    draw_tow_shackles(&mut hull, p, 6, 5, -29, -1);

    // UNMISTAKABLE M4A1 CAST UPPER HULL WITH DISTINCTLY ROUNDED SHOULDERS along entire length!
    for z in -15..=25 {
        hull.fill(-12, 9, z, 12, 12, z, p.main);
        hull.fill(-12, 13, z, 12, 13, z, p.main);
        // Smooth cast rounded upper hull shoulders (`y = 14..=16`)
        hull.fill(-11, 14, z, 11, 14, z, p.main);
        hull.fill(-10, 15, z, 10, 15, z, p.main);
        hull.fill(-8, 16, z, 8, 16, z, p.main);
    }

    // Smooth curved cast glacis rising from nose (`z = -24, y = 8`) to roof (`z = -15, y = 16`)
    for z in -24..=-15 {
        let t = (z + 24) as f32 / 9.0;
        let y_top = lerp_i32(8, 16, t);
        for y in 9..=y_top {
            // Round the cast hull outer corners even on the glacis!
            let corner_inset = match y {
                14 => 1,
                15 => 2,
                16 => 4,
                _ => 0,
            };
            let hw = (12 - corner_inset).min(lerp_i32(10, 12, t));
            hull.fill(-hw, y, z, hw, y, z, p.main);
        }
    }

    // Twin protruding cast driver & co-driver vision hoods on the glacis (iconic M4A1 face!)
    hull.fill_sym(3, 13, -19, 7, 16, -15, p.main);
    hull.fill_sym(4, 15, -20, 6, 16, -19, p.dark);
    // Vision slits in driver & co-driver hoods
    hull.fill_sym(4, 14, -20, 6, 14, -20, p.optic);
    // Twin raised cast crew hatches behind hoods
    draw_roof_hatch(&mut hull, p, -5.0, -14.0, 17, 2.2, 2.0);
    draw_roof_hatch(&mut hull, p, 5.0, -14.0, 17, 2.2, 2.0);

    // Bow .30 cal MG on right glacis
    draw_bow_mg(&mut hull, p, 5, 11, -22);

    // Dual headlights in brush guards on front fender corners
    draw_brush_guard_headlight(&mut hull, p, -10, 10, -25);
    draw_brush_guard_headlight(&mut hull, p, 10, 10, -25);

    // Rear engine deck grilles, angled rear plate, & twin rusty exhausts
    draw_engine_grilles(&mut hull, p, 7, 16, 11, 23);
    hull.fill(-11, 9, 26, 11, 14, 26, p.dark);
    hull.fill(-9, 10, 27, 9, 13, 27, p.dark);
    draw_rear_exhausts(&mut hull, p, 6, 9, 28);
    // Pioneer wooden tool bar on right hull side
    hull.fill(12, 10, 7, 12, 10, 19, p.wood);

    // Crisp US white stars on hull flanks & center front glacis
    draw_white_star_flanks(&mut hull, p, 12, 11, 0);
    draw_white_star_front(&mut hull, p, 0, 12, -20);

    draw_turret_ring(&mut hull, p, -2.0, 16, 7.2);

    // Iconic rounded egg-shaped cast 75mm Sherman turret
    let mut turret = TankCanvas::new(10, 14, 12);
    for y in 0..=8 {
        let (rx, rz) = match y {
            0 => (7.8, 8.6),
            1..=4 => (8.4, 9.4),
            5 => (8.2, 9.2),
            6 => (7.8, 8.8),
            7 => (7.2, 8.0),
            _ => (6.2, 7.0),
        };
        turret.fill_ellipse_y(0.0, 0.8, rx, rz, y, y, p.main);
    }

    // Wide rectangular M34A1 armored rotor shield mantlet
    turret.fill(-5, 2, -10, 5, 5, -8, p.dark);
    turret.fill(-5, 1, -9, 5, 6, -8, p.dark);
    turret.fill(-4, 3, -10, 4, 4, -10, p.main); // central curved shield face
    turret.set(3, 4, -11, p.trim); // coax MG barrel
    turret.set(-3, 4, -10, p.optic); // telescopic gun sight

    // Commander low split-hatch cupola (right rear) + loader hatch (left) + ventilator
    draw_cupola(&mut turret, p, 3.0, 2.0, 8, 2, 2.6);
    draw_roof_hatch(&mut turret, p, -3.2, 1.5, 9, 2.0, 1.8);
    turret.set(0, 9, -3, p.dark); // roof ventilator dome

    // Radio whip aerial & white stars on turret flanks
    draw_radio_antenna(&mut turret, p, -6, 7, 13, 6);
    draw_white_star_flanks(&mut turret, p, 8, 4, 1);

    let barrel = build_detailed_barrel(p, 26, false, 4, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 17.0, -2.0),
        Vec3::new(0.0, 3.5, -10.0),
        26.0,
    )
}

// ---------------------------------------------------------------------------
// 4. M10 Wolverine Tank Destroyer (Tier IV) — ~6.0m long x 3.0m wide
// ---------------------------------------------------------------------------

fn build_m10() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(16, 20, 31);

    // Running gear: wide Sherman M4A2 chassis tracks + 3 VVSS bogies
    build_running_gear(&mut hull, p, 10, 14, -27, 27, 8, 6, 2.0, 3, 9);
    draw_vvss_bogies(&mut hull, p, 14, &[-16, 0, 16], 8);

    // Lower hull tub & rounded Sherman 3-piece bolted transmission nose
    hull.fill(-9, 3, -25, 9, 8, 25, p.dark);
    for z in -29..=-24 {
        let dz = (z + 24) as f32;
        let y_lo = (3.5 + dz * dz * 0.12).round() as i32;
        let y_hi = (8.8 - dz * dz * 0.08).round() as i32;
        let hw = if z == -29 { 8 } else { 9 };
        hull.fill(-hw, y_lo, z, hw, y_hi, z, p.dark);
    }
    hull.fill(-8, 6, -29, 8, 6, -29, p.trim);
    draw_tow_shackles(&mut hull, p, 7, 5, -30, -1);

    // UNMISTAKABLE M10 WOLVERINE SLOPED ANGLED WELDED HULL (inward-sloping upper side sponsons!)
    for z in -15..=25 {
        for y in 9..=14 {
            // Slopes cleanly inward from +-14 at y=9 to +-9 at y=14!
            let hw = 14 - (y - 9);
            hull.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Signature M10 bolt-on armor attachment bosses along sloped side plates
    for &bz in &[-10, -2, 6, 14, 21] {
        hull.set_sym(12, 11, bz, p.dark);
        hull.set_sym(10, 13, bz, p.dark);
    }

    // Clean sharp 55-degree sloped glacis (`z = -25..-15`, `y = 8..=14`)
    for z in -25..=-15 {
        let t = (z + 25) as f32 / 10.0;
        let y_top = lerp_i32(8, 14, t);
        for y in 9..=y_top {
            let hw = (14 - (y - 9)).min(lerp_i32(10, 14, t));
            hull.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Bolt-on applique bosses on M10 glacis
    hull.set_sym(7, 10, -22, p.dark);
    hull.set_sym(5, 12, -18, p.dark);

    // Recessed driver & co-driver vision hatches at top of sloped glacis
    hull.fill_sym(3, 14, -15, 6, 15, -12, p.dark);
    hull.fill_sym(4, 14, -16, 5, 14, -16, p.optic);

    // Dual headlights in brush guards on front fender corners
    draw_brush_guard_headlight(&mut hull, p, -12, 10, -25);
    draw_brush_guard_headlight(&mut hull, p, 12, 10, -25);

    // Sloped rear hull plate, engine deck grilles, & twin exhausts
    for z in 26..=28 {
        let y_top = 14 - (z - 25) * 2;
        let hw = 13 - (z - 25);
        hull.fill(-hw, 9, z, hw, y_top.max(9), z, p.dark);
    }
    draw_engine_grilles(&mut hull, p, 7, 14, 11, 24);
    draw_rear_exhausts(&mut hull, p, 6, 9, 29);

    // White US star projected cleanly on sloped flanks & front glacis
    hull.set_sym(12, 11, 2, p.white);
    hull.set_sym(11, 12, 2, p.white);
    hull.set_sym(13, 10, 2, p.white);
    hull.set_sym(12, 11, 1, p.white);
    hull.set_sym(12, 11, 3, p.white);
    draw_white_star_front(&mut hull, p, 0, 11, -20);

    draw_turret_ring(&mut hull, p, -2.0, 14, 7.6);

    // ICONIC OPEN-TOP PENTAGONAL ANGLED TURRET WITH MASSIVE TWIN WEDGE DUCKBILL COUNTERWEIGHTS!
    let mut turret = TankCanvas::new(12, 14, 16);
    for y in 0..=8 {
        // Inward slope as turret rises (`y = 0..=8`)
        let slope = (y as f32 * 0.28).floor() as i32;
        let max_w = 10 - slope;
        for z in -9..=8 {
            // Pentagonal faceted planform: narrows cleanly toward the front mantlet & tapers gently at rear
            let half_x = if z < -2 {
                lerp_i32(5, max_w, (z + 9) as f32 / 7.0)
            } else if z <= 3 {
                max_w
            } else {
                lerp_i32(max_w, (max_w - 2).max(4), (z - 3) as f32 / 5.0)
            };
            for x in -half_x..=half_x {
                // Keep y == 0 solid floor; hollow out interior y = 1..=8 so open-top fighting compartment is unmistakably visible!
                let is_wall = y == 0
                    || x.abs() >= half_x - 1
                    || z <= -8
                    || z >= 7;
                if is_wall {
                    turret.set(x, y, z, if y == 0 { p.dark } else { p.main });
                }
            }
        }
    }

    // Partial front armored overhead brow right behind mantlet (leaving main compartment wide open!)
    turret.fill(-5, 8, -9, 5, 8, -7, p.dark);

    // Detailed interior fighting compartment visible from above: 76.2mm gun breech & recoil guard!
    turret.fill(-2, 2, -7, 2, 5, -1, p.dark);
    turret.fill(-1, 3, -1, 1, 5, 2, p.trim); // breech block
    turret.fill_sym(5, 1, -2, 6, 3, 4, p.dark); // ready-rack shells / crew seats

    // MASSIVE TWIN WEDGE "DUCKBILL" COUNTERWEIGHTS jutting boldly out the rear (`z = 7..=15`)!
    for z in 7..=15 {
        let t = (z - 7) as f32 / 8.0;
        let y_lo = lerp_i32(1, 5, t);
        let y_hi = lerp_i32(8, 6, t);
        let x_outer = lerp_i32(8, 5, t);
        let x_inner = if z >= 11 { 2 } else { 1 };
        turret.fill_sym(x_inner, y_lo, z, x_outer, y_hi, z, p.dark);
        // Main olive top/side skin on counterweight wedges
        if y_hi > y_lo {
            turret.fill_sym(x_inner, y_hi, z, x_outer, y_hi, z, p.main);
        }
    }
    // Center coupling beam between twin duckbill wedges + rear .50 cal pintle MG
    turret.fill(-1, 4, 7, 1, 6, 11, p.trim);
    draw_50cal_pintle(&mut turret, p, 0, 7, 9);
    draw_radio_antenna(&mut turret, p, -7, 7, 13, 8);

    // Tall narrow sloped armored mantlet shield
    turret.fill(-4, 1, -11, 4, 7, -9, p.dark);
    turret.fill(-3, 2, -11, 3, 6, -11, p.main);
    turret.set(-2, 5, -11, p.optic); // telescopic sight

    // White US stars on angled turret side facets
    turret.set_sym(9, 4, 0, p.white);
    turret.set_sym(9, 5, 0, p.white);
    turret.set_sym(10, 3, 0, p.white);
    turret.set_sym(9, 4, -1, p.white);
    turret.set_sym(9, 4, 1, p.white);

    let barrel = build_detailed_barrel(p, 34, false, 4, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 15.0, -2.0),
        Vec3::new(0.0, 4.0, -11.0),
        34.0,
    )
}

// ---------------------------------------------------------------------------
// 5. M4A3E8 "Easy Eight" Sherman (Tier V) — ~6.0m long x 3.0m wide
// ---------------------------------------------------------------------------

fn build_m4a3e8() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(16, 22, 31);

    // Wide HVSS (Horizontal Volute Spring Suspension) tracks (60 cm wide!) + 6 road wheels
    build_running_gear(&mut hull, p, 9, 14, -27, 27, 8, 6, 2.0, 3, 9);
    // Horizontal HVSS twin-bogie suspension beams & wide side sand-shield fenders
    for &zc in &[-16, 0, 16] {
        hull.fill_sym(14, 4, zc - 3, 14, 5, zc + 3, p.dark);
        hull.set_sym(14, 5, zc, p.trim);
    }
    hull.fill_sym(14, 9, -27, 15, 9, 27, p.dark); // crisp wide HVSS fenders

    // Lower hull tub & crisp 1-piece bolted sharp-nosed transmission housing
    hull.fill(-8, 3, -25, 8, 8, 25, p.dark);
    for z in -28..=-25 {
        let t = (z + 28) as f32 / 3.0;
        let y_lo = lerp_i32(5, 3, t);
        hull.fill(-8, y_lo, z, 8, 8, z, p.dark);
    }
    draw_tow_shackles(&mut hull, p, 7, 5, -29, -1);

    // Spare track links strapped across lower nose (iconic Easy Eight combat detail!)
    draw_spare_track_bar(&mut hull, p, 5, 6, 8, -27);

    // UNMISTAKABLE EASY EIGHT SHARP WELDED HULL (crisp flat 90-degree edges — totally distinct from M4A1!)
    hull.fill(-13, 9, -15, 13, 15, 26, p.main);
    // Subtle dark weld-line bevel along upper hull roof edges
    hull.fill_sym(13, 15, -15, 13, 15, 26, p.dark);

    // Wide flat 47-degree single-piece welded glacis (`z = -25..-15`, rising sharply `y = 8..=15`)
    for z in -25..=-15 {
        let t = (z + 25) as f32 / 10.0;
        let y_top = lerp_i32(8, 15, t);
        hull.fill(-13, 9, z, 13, y_top, z, p.main);
        // Sharp welded outer edge bevel
        hull.set_sym(13, y_top, z, p.dark);
    }

    // Large flat welded driver & co-driver roof hatches + periscope hoods (no protruding M4A1 hoods!)
    draw_roof_hatch(&mut hull, p, -5.5, -13.0, 16, 2.6, 2.2);
    draw_roof_hatch(&mut hull, p, 5.5, -13.0, 16, 2.6, 2.2);
    hull.fill_sym(5, 15, -16, 6, 15, -16, p.optic); // flush periscope slits

    // Bow MG on right glacis + travel lock bracket on lower center glacis
    draw_bow_mg(&mut hull, p, 6, 11, -22);
    hull.fill(-1, 9, -24, 1, 10, -24, p.trim);

    // Dual headlights in brush guards on wide HVSS front fenders
    draw_brush_guard_headlight(&mut hull, p, -12, 10, -26);
    draw_brush_guard_headlight(&mut hull, p, 12, 10, -26);

    // Rear M4A3 Ford GAA engine deck grilles, angled rear plate, & twin exhaust deflectors
    draw_engine_grilles(&mut hull, p, 9, 15, 11, 24);
    hull.fill(-12, 9, 27, 12, 14, 27, p.dark);
    draw_rear_exhausts(&mut hull, p, 7, 9, 28);
    // Rear stowage crate & pioneer tool
    hull.fill(-5, 11, 28, 5, 13, 29, p.wood);
    hull.fill(13, 10, 8, 13, 10, 20, p.wood);

    // Crisp US white stars on flat welded flanks & clean wide glacis
    draw_white_star_flanks(&mut hull, p, 13, 12, 1);
    draw_white_star_front(&mut hull, p, 0, 12, -19);

    draw_turret_ring(&mut hull, p, -2.0, 15, 7.6);

    // Large elongated T23 cast turret with pronounced rear radio bustle & raised vision cupola
    let mut turret = TankCanvas::new(11, 16, 15);
    // Main cast body (`y = 0..=8`)
    for y in 0..=8 {
        let (rx, rz) = match y {
            0 => (8.4, 9.2),
            1..=5 => (9.0, 9.8),
            6 => (8.6, 9.4),
            7 => (8.0, 8.8),
            _ => (7.0, 8.0),
        };
        turret.fill_ellipse_y(0.0, -0.5, rx, rz, y, y, p.main);
    }
    // Pronounced elongated rear radio bustle overhang (`z = 4..=13`, undercut cleanly at `y = 0..=1`)
    for z in 4..=13 {
        let t = (z - 4) as f32 / 9.0;
        let hw = lerp_i32(8, 5, t);
        let y_lo = if z <= 6 { 1 } else { 2 };
        let y_hi = if z >= 12 { 7 } else { 8 };
        turret.fill(-hw, y_lo, z, hw, y_hi, z, p.main);
        if z == 13 {
            turret.fill(-hw, y_lo, z, hw, y_hi, z, p.dark);
        }
    }

    // T23 76mm armored mantlet
    turret.fill(-5, 1, -11, 5, 6, -9, p.dark);
    turret.fill(-4, 2, -11, 4, 5, -11, p.main);
    turret.set(3, 4, -12, p.trim); // coax MG
    turret.set(-3, 4, -11, p.optic); // gun sight

    // Raised all-round vision commander cupola + loader hatch + .50 cal pintle + whip antenna
    draw_cupola(&mut turret, p, 3.5, 2.0, 8, 3, 2.8);
    draw_roof_hatch(&mut turret, p, -3.5, 2.0, 9, 2.2, 2.0);
    draw_50cal_pintle(&mut turret, p, 0, 9, 9);
    draw_radio_antenna(&mut turret, p, -5, 8, 15, 10);

    // White US stars on T23 turret flanks
    draw_white_star_flanks(&mut turret, p, 9, 4, 0);

    // Iconic long 76mm M1A2 barrel with T-Shaped muzzle brake!
    let barrel = build_detailed_barrel(p, 36, false, 5, MuzzleBrakeStyle::TShaped);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 16.0, -2.0),
        Vec3::new(0.0, 3.5, -11.0),
        36.0,
    )
}

// ---------------------------------------------------------------------------
// 6. M4A3E2 Sherman Jumbo 76 (Tier VI Heavy Assault) — ~6.0m long x 3.0m wide
// ---------------------------------------------------------------------------

fn build_m4a3e2_76() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(16, 22, 31);

    // Heavy extended-end-connector ("duckbill") tracks + armored VVSS bogies
    build_running_gear(&mut hull, p, 9, 14, -27, 27, 8, 6, 2.0, 3, 9);
    draw_vvss_bogies(&mut hull, p, 14, &[-16, 0, 16], 8);
    // Extended-end-connector "duckbill" track cleats along outer edge
    for z in (-25..=25).step_by(3) {
        hull.set_sym(15, 0, z, p.track);
        hull.set_sym(15, 8, z, p.track);
    }

    // Heavy reinforced cast differential nose housing (`p.dark`, extra thick!)
    hull.fill(-9, 3, -25, 9, 8, 25, p.dark);
    for z in -29..=-24 {
        let t = (z + 29) as f32 / 5.0;
        let y_lo = lerp_i32(5, 3, t);
        hull.fill(-9, y_lo, z, 9, 8, z, p.dark);
    }
    hull.fill(-8, 6, -29, 8, 6, -29, p.trim);
    draw_tow_shackles(&mut hull, p, 7, 5, -30, -1);
    draw_spare_track_bar(&mut hull, p, 5, 6, 8, -28);

    // Core welded hull + THICK SIDE SPONSON APPLIQUE ARMOR SLABS (`x = ±14`)
    hull.fill(-13, 9, -15, 13, 15, 26, p.main);
    // Thick bolted 38mm applique side sponson slabs (making Jumbo visibly wider/beefier!)
    hull.fill_sym(14, 10, -15, 14, 15, 15, p.main);
    hull.fill_sym(14, 10, -15, 14, 10, 15, p.dark); // lower weld seam
    hull.fill_sym(14, 15, -15, 14, 15, 15, p.dark); // upper weld seam

    // THICK OVERHANGING WELDED APPLIQUE GLACIS PLATE (visibly protruding extra-thick front slab!)
    for z in -26..=-14 {
        let t = (z + 26) as f32 / 12.0;
        let y_top = lerp_i32(8, 16, t);
        hull.fill(-13, 9, z, 13, y_top, z, p.main);
        // Thick overhanging outer applique border in dark steel-olive
        hull.set_sym(13, y_top, z, p.dark);
        if z == -26 || z == -14 {
            hull.fill(-13, y_top, z, 13, y_top, z, p.dark);
        }
    }
    // Overhanging upper glacis armor brow lip (`y = 16, z = -15..-14`)
    hull.fill(-13, 16, -15, 13, 16, -14, p.dark);

    // Twin heavy welded crew hatches + periscope slots
    draw_roof_hatch(&mut hull, p, -5.5, -12.0, 16, 2.5, 2.2);
    draw_roof_hatch(&mut hull, p, 5.5, -12.0, 16, 2.5, 2.2);
    hull.fill_sym(5, 15, -16, 6, 15, -16, p.optic);

    // Recessed bow MG in thick glacis applique + heavy guarded headlights
    draw_bow_mg(&mut hull, p, 6, 11, -23);
    draw_brush_guard_headlight(&mut hull, p, -12, 10, -26);
    draw_brush_guard_headlight(&mut hull, p, 12, 10, -26);

    // Rear engine deck grilles & twin exhausts
    draw_engine_grilles(&mut hull, p, 9, 15, 11, 24);
    hull.fill(-12, 9, 27, 12, 14, 27, p.dark);
    draw_rear_exhausts(&mut hull, p, 7, 9, 28);

    // US white stars on thick applique side slabs & front glacis
    draw_white_star_flanks(&mut hull, p, 14, 12, 0);
    draw_white_star_front(&mut hull, p, 0, 12, -21);

    draw_turret_ring(&mut hull, p, -2.0, 15, 8.0);

    // MASSIVE HEAVY BOXY-CHEEKED T23-JUMBO TURRET (noticeably wider, taller, thicker slab-sided!)
    let mut turret = TankCanvas::new(12, 16, 15);
    for y in 0..=9 {
        let inset = if y >= 8 { y - 7 } else { 0 };
        let hw = 10 - inset;
        for z in -9..=13 {
            // Chunky boxy-cheeked heavy cast walls tapering only subtly at rear bustle
            let row_hw = if z <= 5 {
                hw
            } else {
                lerp_i32(hw, (hw - 3).max(5), (z - 5) as f32 / 8.0)
            };
            // Round only the extreme outer corners so it reads unmistakably slab-sided & massive
            let corner_cut = if z == -9 || z == 13 { 2 } else if z == -8 || z == 12 { 1 } else { 0 };
            let eff_hw = (row_hw - corner_cut).max(4);
            let y_lo = if z >= 8 { 2 } else { 0 };
            if y >= y_lo {
                turret.fill(-eff_hw, y, z, eff_hw, y, z, p.main);
            }
        }
    }
    // Dark cast roof rim for heavy turret presence
    turret.fill(-8, 9, -8, 8, 9, 11, p.dark);
    turret.fill(-7, 9, -7, 7, 9, 10, p.main);

    // GIGANTIC 177mm PROTRUDING HEAVY SQUARE ARMORED MANTLET BLOCK (signature Jumbo 76 face!)
    turret.fill(-6, 1, -13, 6, 8, -9, p.dark);
    turret.fill(-5, 2, -13, 5, 7, -13, p.main); // massive flat-faced 177mm block!
    // Heavy bolted mantlet collar framing
    turret.fill(-6, 1, -13, 6, 1, -13, p.trim);
    turret.fill(-6, 8, -13, 6, 8, -13, p.trim);
    turret.set(4, 5, -14, p.trim); // coax MG
    turret.set(-4, 5, -13, p.optic); // gun sight

    // Commander vision cupola + loader's heavy hatch + rear radio whip
    draw_cupola(&mut turret, p, 4.0, 2.0, 9, 3, 2.9);
    draw_roof_hatch(&mut turret, p, -4.0, 2.0, 10, 2.2, 2.0);
    draw_radio_antenna(&mut turret, p, -7, 9, 15, 9);

    // White US stars on thick Jumbo turret flanks
    draw_white_star_flanks(&mut turret, p, 10, 4, 1);

    // Long 76mm barrel (late-war Jumbo 76 clean muzzle)
    let barrel = build_detailed_barrel(p, 35, false, 5, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 16.0, -2.0),
        Vec3::new(0.0, 4.5, -13.0),
        35.0,
    )
}

// ---------------------------------------------------------------------------
// 7. M26 Pershing Heavy/Medium Tank (Tier VII) — ~6.3m long x 3.5m wide
// ---------------------------------------------------------------------------

fn build_m26() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(18, 20, 33);

    // Wide heavy tracks + 6 LARGE dual torsion-bar road wheels + 5 return rollers
    build_running_gear(&mut hull, p, 11, 16, -29, 29, 8, 6, 2.6, 5, 9);

    // Subtle iconic curved Pershing bow fenders curving over front track horns
    hull.fill_sym(11, 9, -31, 17, 9, -29, p.dark);
    hull.fill_sym(11, 8, -31, 17, 8, -31, p.dark);
    hull.fill_sym(12, 7, -32, 16, 7, -32, p.dark);

    // Wide, low, muscular post-war-style hull (`y_roof = 13`, width `x = -15..=15`)
    hull.fill(-10, 3, -27, 10, 8, 28, p.dark);
    // Sharp angled lower glacis nose plate
    for z in -29..=-25 {
        let t = (z + 29) as f32 / 4.0;
        let y_lo = lerp_i32(6, 3, t);
        hull.fill(-10, y_lo, z, 10, 7, z, p.dark);
    }
    draw_tow_shackles(&mut hull, p, 8, 6, -30, -1);

    // Wide low main hull superstructure (`y = 9..=13`)
    hull.fill(-15, 9, -16, 15, 13, 28, p.main);
    // Sleek sloped welded Pershing glacis (`z = -29..-16`, rising smoothly `y = 7..=13`)
    for z in -29..=-16 {
        let t = (z + 29) as f32 / 13.0;
        let y_top = lerp_i32(7, 13, t);
        let hw = lerp_i32(11, 15, t);
        hull.fill(-hw, 8, z, hw, y_top, z, p.main);
        hull.set_sym(hw, y_top, z, p.dark);
    }

    // Subtle sculpted cast driver/co-driver brow bulges + flat roof hatches
    hull.fill_sym(3, 12, -18, 7, 13, -15, p.dark);
    hull.fill_sym(4, 13, -18, 6, 13, -18, p.optic);
    draw_roof_hatch(&mut hull, p, -5.5, -13.0, 14, 2.6, 2.2);
    draw_roof_hatch(&mut hull, p, 5.5, -13.0, 14, 2.6, 2.2);

    // Bow MG on right glacis + dual headlights in brush guards on bow fenders
    draw_bow_mg(&mut hull, p, 6, 10, -23);
    draw_brush_guard_headlight(&mut hull, p, -13, 10, -28);
    draw_brush_guard_headlight(&mut hull, p, 13, 10, -28);

    // Iconic Pershing side fender stowage bins along rear mudguards
    hull.fill_sym(14, 10, 3, 16, 12, 23, p.dark);
    hull.fill_sym(15, 11, 5, 16, 11, 21, p.main);

    // Wide rear engine deck with Pershing cooling louver banks & rear armored overhang
    draw_engine_grilles(&mut hull, p, 11, 13, 12, 26);
    hull.fill(-14, 8, 29, 14, 12, 30, p.dark);
    draw_rear_exhausts(&mut hull, p, 8, 9, 31);

    // US white stars on wide hull flanks & sleek front glacis
    draw_white_star_flanks(&mut hull, p, 15, 11, -3);
    draw_white_star_front(&mut hull, p, 0, 10, -22);

    draw_turret_ring(&mut hull, p, -3.0, 13, 9.0);

    // Wide low cast rounded Pershing turret + huge deep full-width curved mantlet + long rear bustle
    let mut turret = TankCanvas::new(13, 15, 19);
    for y in 0..=8 {
        let (rx, rz) = match y {
            0 => (10.5, 9.6),
            1..=4 => (11.5, 10.6),
            5 => (11.2, 10.4),
            6 => (10.5, 9.8),
            7 => (9.5, 9.0),
            _ => (8.2, 8.0),
        };
        turret.fill_ellipse_y(0.0, -0.5, rx, rz, y, y, p.main);
    }

    // Prominent LONG rear radio bustle overhang (`z = 5..=17`, sharply undercut at `y = 0..=2`!)
    for z in 5..=17 {
        let t = (z - 5) as f32 / 12.0;
        let hw = lerp_i32(10, 6, t);
        let y_lo = if z <= 8 { 1 } else if z <= 12 { 2 } else { 3 };
        let y_hi = if z >= 15 { 7 } else { 8 };
        turret.fill(-hw, y_lo, z, hw, y_hi, z, p.main);
        if z == 17 {
            turret.fill(-hw, y_lo, z, hw, y_hi, z, p.dark);
        }
    }

    // HUGE DEEP FULL-WIDTH CURVED PERSHING MANTLET (`x in -8..=8, y = 1..=7, z = -13..-9`)
    for y in 1..=7 {
        let bulge = match y {
            3..=5 => -13,
            2 | 6 => -12,
            _ => -11,
        };
        let hw = if y == 1 || y == 7 { 7 } else { 8 };
        turret.fill(-hw, y, bulge, hw, y, -9, p.dark);
        if (3..=5).contains(&y) {
            turret.fill(-6, y, -13, 6, y, -13, p.main);
        }
    }
    turret.set(4, 4, -14, p.trim); // coax MG
    turret.set(-4, 4, -13, p.optic); // telescopic gun sight

    // All-round vision commander cupola + loader roof hatch + .50 cal pintle + twin radio whips
    draw_cupola(&mut turret, p, 4.5, 1.0, 8, 3, 3.0);
    draw_roof_hatch(&mut turret, p, -4.5, 1.5, 9, 2.5, 2.2);
    draw_50cal_pintle(&mut turret, p, 0, 9, 11);
    draw_radio_antenna(&mut turret, p, -6, 8, 14, 13);
    draw_radio_antenna(&mut turret, p, 6, 8, 14, 13);

    // White US stars on wide Pershing turret cheeks
    draw_white_star_flanks(&mut turret, p, 11, 4, 0);

    // Massive 90mm M3 cannon with iconic T-Shaped muzzle brake!
    let barrel = build_detailed_barrel(p, 42, true, 6, MuzzleBrakeStyle::TShaped);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -3.0),
        Vec3::new(0.0, 4.5, -13.0),
        42.0,
    )
}

// ---------------------------------------------------------------------------
// 8. T34 Heavy Tank (Tier VIII) — ~7.4m long x 3.7m wide
// ---------------------------------------------------------------------------

fn build_t34() -> TankVoxelModels {
    let p = &TankPalette::USA;
    let mut hull = TankCanvas::new(19, 22, 38);

    // Colossal stretched Pershing heavy chassis with 7 LARGE road wheels & 6 return rollers!
    build_running_gear(&mut hull, p, 12, 17, -34, 34, 9, 7, 2.7, 6, 10);

    // Wide reinforced mudguard skirts & heavy curved bow fender horns
    hull.fill_sym(17, 9, -34, 18, 11, 34, p.dark);
    hull.fill_sym(12, 10, -36, 18, 10, -34, p.dark);
    hull.fill_sym(12, 8, -36, 18, 9, -36, p.dark);
    // Bolted armored side-skirt panel lines
    for z in (-28..=28).step_by(8) {
        hull.fill_sym(18, 9, z, 18, 11, z, p.trim);
    }

    // Heavy stretched lower hull tub & bold angled lower nose plate
    hull.fill(-11, 3, -32, 11, 9, 33, p.dark);
    for z in -35..=-31 {
        let t = (z + 35) as f32 / 4.0;
        let y_lo = lerp_i32(7, 3, t);
        hull.fill(-11, y_lo, z, 11, 8, z, p.dark);
    }
    // Heavy twin front tow shackles & armored lifting hooks
    draw_tow_shackles(&mut hull, p, 9, 6, -36, -1);
    hull.set_sym(9, 8, -35, p.trim);

    // Colossal wide heavy superstructure (`y = 10..=15`, `x in -16..=16`)
    hull.fill(-16, 10, -19, 16, 15, 33, p.main);

    // Massive steeply sloped heavy glacis (`z = -34..-19`, rising `y = 7..=15`)
    for z in -34..=-19 {
        let t = (z + 34) as f32 / 15.0;
        let y_top = lerp_i32(7, 15, t);
        let hw = lerp_i32(12, 16, t);
        hull.fill(-hw, 9, z, hw, y_top, z, p.main);
        hull.set_sym(hw, y_top, z, p.dark);
    }
    // Reinforced cast driver & co-driver brow hoods + heavy roof hatches
    hull.fill_sym(3, 14, -21, 7, 15, -18, p.dark);
    hull.fill_sym(4, 15, -21, 6, 15, -21, p.optic);
    draw_roof_hatch(&mut hull, p, -5.5, -16.0, 16, 2.8, 2.4);
    draw_roof_hatch(&mut hull, p, 5.5, -16.0, 16, 2.8, 2.4);

    // Heavy bow MG mount + dual armored headlights in brush guards
    draw_bow_mg(&mut hull, p, 7, 11, -27);
    draw_brush_guard_headlight(&mut hull, p, -14, 11, -33);
    draw_brush_guard_headlight(&mut hull, p, 14, 11, -33);

    // Long heavy fender stowage bins along flanks
    hull.fill_sym(15, 11, 0, 17, 13, 28, p.dark);
    hull.fill_sym(16, 12, 3, 17, 12, 25, p.main);

    // Colossal stretched rear engine deck with massive twin cooling louver banks & exhausts
    draw_engine_grilles(&mut hull, p, 12, 15, 14, 31);
    hull.fill(-15, 9, 34, 15, 14, 35, p.dark);
    draw_rear_exhausts(&mut hull, p, 9, 10, 36);
    draw_tow_shackles(&mut hull, p, 9, 10, 36, 1);

    // White US stars on heavy hull flanks & massive glacis
    draw_white_star_flanks(&mut hull, p, 16, 12, -7);
    draw_white_star_front(&mut hull, p, 0, 11, -26);

    draw_turret_ring(&mut hull, p, -5.0, 15, 10.0);

    // GIGANTIC ELONGATED HEAVY CAST T34 TURRET + LEGENDARY WELDED 101mm REAR COUNTERWEIGHT BOX!
    let mut turret = TankCanvas::new(14, 17, 25);

    // Towering wide muscular heavy cast main turret (`y = 0..=10`)
    for y in 0..=10 {
        let (rx, rz) = match y {
            0 => (11.5, 11.2),
            1..=5 => (12.5, 12.2),
            6..=7 => (12.0, 11.8),
            8 => (11.2, 11.0),
            9 => (10.2, 10.2),
            _ => (9.0, 9.2),
        };
        turret.fill_ellipse_y(0.0, 0.0, rx, rz, y, y, p.main);
    }

    // Long overhanging heavy cast rear bustle (`z = 7..=19`, cleanly undercut at `y = 0..=2`)
    for z in 7..=19 {
        let t = (z - 7) as f32 / 12.0;
        let hw = lerp_i32(11, 8, t);
        let y_lo = if z <= 10 { 1 } else { 2 };
        let y_hi = if z >= 17 { 9 } else { 10 };
        turret.fill(-hw, y_lo, z, hw, y_hi, z, p.main);
    }

    // LEGENDARY HUGE RECTANGULAR WELDED 101mm COUNTERWEIGHT ARMOR BOX BOLTED TO REAR BUSTLE!
    // Unmistakable T34 silhouette from side, 3/4, and rear!
    turret.fill(-8, 2, 19, 8, 9, 24, p.dark);
    turret.fill(-7, 3, 20, 7, 8, 24, p.main);
    // Heavy steel weld seams & lifting lugs on the 101mm counterweight slab
    turret.fill(-8, 2, 24, 8, 2, 24, p.trim);
    turret.fill(-8, 9, 24, 8, 9, 24, p.trim);
    turret.fill_sym(8, 2, 24, 8, 9, 24, p.trim);
    turret.set_sym(7, 10, 21, p.trim);

    // TOWERING THICK HEAVY CAST MANTLET WITH TWIN RECOIL BUMPS
    for y in 1..=9 {
        let bulge = match y {
            3..=7 => -15,
            2 | 8 => -14,
            _ => -13,
        };
        let hw = if y == 1 || y == 9 { 8 } else { 9 };
        turret.fill(-hw, y, bulge, hw, y, -11, p.dark);
        if (3..=7).contains(&y) {
            turret.fill(-7, y, -15, 7, y, -15, p.main);
        }
    }
    // Signature twin armored recoil cylinder bumps above gun barrel on mantlet face!
    turret.fill_sym(3, 7, -16, 4, 8, -15, p.dark);
    turret.set_sym(3, 8, -16, p.trim);
    turret.set(6, 5, -16, p.trim); // heavy coax MG
    turret.set(-6, 5, -15, p.optic); // telescopic sight

    // Large raised vision commander cupola + heavy loader hatch + rangefinder ears + .50 cal + dual whips
    draw_cupola(&mut turret, p, 5.0, 1.0, 10, 3, 3.2);
    draw_roof_hatch(&mut turret, p, -5.0, 1.5, 11, 2.6, 2.4);
    // Subtle heavy side turret rangefinder blisters
    turret.fill_sym(12, 7, -3, 13, 8, -1, p.dark);
    turret.set_sym(13, 8, -3, p.optic);
    draw_50cal_pintle(&mut turret, p, 0, 11, 13);
    draw_radio_antenna(&mut turret, p, -7, 10, 16, 16);
    draw_radio_antenna(&mut turret, p, 7, 10, 16, 16);

    // White US stars on massive T34 turret cheeks
    draw_white_star_flanks(&mut turret, p, 12, 5, 2);

    // Monstrous 120mm T53 barrel with heavy T-Shaped muzzle brake!
    let barrel = build_detailed_barrel(p, 54, true, 8, MuzzleBrakeStyle::TShaped);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 16.0, -5.0),
        Vec3::new(0.0, 5.5, -15.0),
        54.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialRegistry;
    use crate::mesher::mesh_voxel_model;

    #[test]
    fn all_8_usa_tanks_build_and_mesh_within_u16_budget() {
        let registry = MaterialRegistry::new();
        let ids = [
            "us_m2a4",
            "us_m3_stuart",
            "us_m4a1",
            "us_m10",
            "us_m4a3e8",
            "us_m4a3e2_76",
            "us_m26",
            "us_t34",
        ];

        for id in ids {
            let tank = build_usa_vehicle(id).unwrap_or_else(|| panic!("missing USA tank {id}"));
            assert!(tank.hull.non_air_count() > 500, "{id} hull too sparse");
            assert!(tank.turret.non_air_count() > 100, "{id} turret too sparse");
            assert!(tank.barrel.non_air_count() > 20, "{id} barrel too sparse");

            let hull_mesh = mesh_voxel_model(&tank.hull, &registry);
            let turret_mesh = mesh_voxel_model(&tank.turret, &registry);
            let barrel_mesh = mesh_voxel_model(&tank.barrel, &registry);

            let total_verts = hull_mesh.positions.len()
                + turret_mesh.positions.len()
                + barrel_mesh.positions.len();

            assert!(
                hull_mesh.positions.len() < u16::MAX as usize,
                "{id} hull exceeds u16::MAX verts: {}",
                hull_mesh.positions.len()
            );
            assert!(
                turret_mesh.positions.len() < u16::MAX as usize,
                "{id} turret exceeds u16::MAX verts: {}",
                turret_mesh.positions.len()
            );
            assert!(
                barrel_mesh.positions.len() < u16::MAX as usize,
                "{id} barrel exceeds u16::MAX verts: {}",
                barrel_mesh.positions.len()
            );
            assert!(
                total_verts < u16::MAX as usize,
                "{id} total verts too high: {total_verts}"
            );
        }

        assert!(build_usa_vehicle("unknown_id").is_none());
    }
}
