//! Bespoke high-detail `0.10 m` voxel models for all 8 German armored vehicles (Tiers I–VIII).
//!
//! Vehicles implemented:
//! 1. `de_pz2c`      — Pz.Kpfw. II Ausf. C (Tier I Light, Panzergrau)
//! 2. `de_pz3j`      — Pz.Kpfw. III Ausf. J (Tier II Medium, Panzergrau)
//! 3. `de_pz4_f2`    — Pz.Kpfw. IV Ausf. F2 (Tier III Medium, Panzergrau)
//! 4. `de_stug3g`    — StuG III Ausf. G (Tier IV Tank Destroyer, Dunkelgelb)
//! 5. `de_tiger_h1`  — Tiger I Ausf. H1 (Tier V Heavy, Dunkelgelb)
//! 6. `de_panther_a` — Panther Ausf. A (Tier VI Medium, Dunkelgelb)
//! 7. `de_tiger2_h`  — Tiger II Ausf. H King Tiger (Tier VII Heavy, Dunkelgelb)
//! 8. `de_jagdtiger` — Jagdtiger Sd.Kfz. 186 (Tier VIII Heavy TD, Dunkelgelb)

use glam::Vec3;

use crate::material::VoxelId;
use crate::model::{MuzzleBrakeStyle, TankVoxelModels};

use super::{
    TankCanvas, TankPalette, assemble_tank, build_detailed_barrel, build_running_gear,
    draw_balkenkreuz_flanks, draw_bow_mg, draw_cupola, draw_engine_grilles, draw_headlight,
};

/// Dispatches a German catalog `vehicle_id` to its bespoke `0.10 m` voxel sculptor.
pub fn build_germany_vehicle(vehicle_id: &str) -> Option<TankVoxelModels> {
    match vehicle_id {
        "de_pz2c" => Some(build_pz2c()),
        "de_pz3j" => Some(build_pz3j()),
        "de_pz4_f2" => Some(build_pz4_f2()),
        "de_stug3g" => Some(build_stug3g()),
        "de_tiger_h1" => Some(build_tiger_h1()),
        "de_panther_a" => Some(build_panther_a()),
        "de_tiger2_h" => Some(build_tiger2_h()),
        "de_jagdtiger" => Some(build_jagdtiger()),
        _ => None,
    }
}

// ============================================================================
// SHARED GERMAN ARMOR SCULPTING HELPERS
// ============================================================================

/// Draws a classic German armored driver's visor slit (`Sehklappe`) facing `-Z`:
/// glowing blue-grey optic glass recessed inside a crisp dark steel visor brow.
fn draw_driver_visor(c: &mut TankCanvas, p: &TankPalette, cx: i32, y: i32, z: i32, half_w: i32) {
    // Upper & lower armored visor hood lips
    c.fill(cx - half_w - 1, y + 1, z, cx + half_w + 1, y + 1, z, p.trim);
    c.fill(cx - half_w - 1, y - 1, z, cx + half_w + 1, y - 1, z, p.dark);
    c.set(cx - half_w - 1, y, z, p.trim);
    c.set(cx + half_w + 1, y, z, p.trim);
    // Vision glass slit
    c.fill(cx - half_w, y, z, cx + half_w, y, z, p.optic);
}

/// Draws spare track links mounted horizontally across a front bow plate or hull face (`z`).
fn draw_spare_track_bow(
    c: &mut TankCanvas,
    p: &TankPalette,
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
    z: i32,
) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            let link = if (x - x0) % 2 == 0 { p.track } else { p.rust };
            c.set(x, y, z, link);
        }
    }
    // Retaining steel bar across center
    let bar_y = (y0 + y1) / 2;
    c.fill(x0, bar_y, z - 1, x1, bar_y, z - 1, p.trim);
}

/// Draws heavy German tow shackles protruding symmetrically from front (`dir = -1`) or rear (`dir = 1`).
fn draw_tow_shackles(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32, dir: i32) {
    c.set_sym(x, y, z, p.trim);
    c.set_sym(x, y, z + dir, p.trim);
    c.set_sym(x, y - 1, z + dir, p.trim);
}

/// Draws a classic German horizontal cylindrical exhaust muffler across the rear hull.
fn draw_rear_muffler(c: &mut TankCanvas, p: &TankPalette, half_w: i32, cy: f32, cz: f32, r: f32) {
    c.fill_cyl_x(-half_w, half_w, cy, cz, r, p.rust);
    // Mounting steel straps
    let strap_x = (half_w * 2 / 3).max(2);
    let iy0 = (cy - r).round() as i32;
    let iy1 = (cy + r).round() as i32;
    let iz = cz.round() as i32;
    c.fill_sym(strap_x, iy0, iz, strap_x, iy1, iz, p.trim);
    // Exhaust pipe tips curving down on outer ends
    c.set_sym(half_w, iy0 - 1, iz, p.trim);
}

/// Draws iconic German `Schürzen` (5mm spaced armor side skirts) hanging cleanly outside tracks
/// with top mounting rail, brackets to the fender, and crisp vertical panel seams.
#[allow(clippy::too_many_arguments)]
fn draw_schuerzen(
    c: &mut TankCanvas,
    p: &TankPalette,
    skirt_x: i32,
    fender_x: i32,
    y0: i32,
    y1: i32,
    z0: i32,
    z1: i32,
    seams: &[i32],
) {
    // Main hanging armor plates
    c.fill_sym(skirt_x, y0 + 1, z0, skirt_x, y1 - 1, z1, p.main);
    // Slightly weathered darker lower lip & upper steel hanger rail
    c.fill_sym(skirt_x, y0, z0, skirt_x, y0, z1, p.dark);
    c.fill_sym(skirt_x, y1, z0, skirt_x, y1, z1, p.trim);
    // Mounting brackets connecting rail cleanly back to fender
    let step = ((z1 - z0) / 4).max(6);
    let mut bz = z0 + 2;
    while bz <= z1 - 2 {
        c.fill_sym(fender_x, y1, bz, skirt_x, y1, bz, p.trim);
        bz += step;
    }
    // Crisp dark vertical seams separating individual hanging Schürzen panels
    for &sz in seams {
        if sz > z0 && sz < z1 {
            c.fill_sym(skirt_x, y0, sz, skirt_x, y1 - 1, sz, p.dark);
        }
    }
}

/// Paints a crisp 5×5 German Balkenkreuz flush onto a sloped or curved armor flank by finding
/// the outermost solid voxel at each `(cy + dy, cz + dz)` cell.
fn draw_balkenkreuz_on_sloped_flanks(
    c: &mut TankCanvas,
    p: &TankPalette,
    cy: i32,
    cz: i32,
    max_search_x: i32,
) {
    for dy in -2i32..=2 {
        for dz in -2i32..=2 {
            let on_arm = dy.abs() <= 1 || dz.abs() <= 1;
            let on_core = dy == 0 || dz == 0;
            let color = if on_core {
                Some(p.black)
            } else if on_arm && !(dy.abs() == 2 || dz.abs() == 2) {
                Some(p.white)
            } else {
                None
            };
            if let Some(id) = color {
                let y = cy + dy;
                let z = cz + dz;
                for x in (1..=max_search_x).rev() {
                    if !c.get(x, y, z).is_air() {
                        c.set_sym(x, y, z, id);
                        break;
                    }
                }
            }
        }
    }
}

/// Builds authentic German **Schachtellaufwerk** (overlapping interleaved road wheels) running gear
/// used on Tiger I, Panther, Tiger II, and Jagdtiger — featuring wide tracks, toothed front drive
/// sprockets, rear idlers, outer & inner staggered large road wheels, and armored fenders.
#[allow(clippy::too_many_arguments)]
fn build_schachtellaufwerk_gear(
    c: &mut TankCanvas,
    p: &TankPalette,
    inner_x: i32,
    outer_x: i32,
    z_front: i32,
    z_rear: i32,
    track_top_y: i32,
    outer_wheels: usize,
    inner_wheels: usize,
    wheel_radius: f32,
    fender_y: i32,
    steel_rimmed: bool,
) {
    // 1. Wide bottom ground run & upper track return run
    c.fill_sym(inner_x, 0, z_front + 4, outer_x, 0, z_rear - 4, p.track);
    c.fill_sym(
        inner_x,
        track_top_y,
        z_front + 1,
        outer_x,
        track_top_y,
        z_rear - 1,
        p.track,
    );

    // 2. Steep front sprocket & rear idler track wrap-up
    for step in 0..=4 {
        let y_lo = (step * (track_top_y - 1) / 4).min(track_top_y - 1);
        let y_hi = (y_lo + 1).min(track_top_y);
        c.fill_sym(
            inner_x,
            y_lo,
            z_front + (4 - step),
            outer_x,
            y_hi,
            z_front + (4 - step),
            p.track,
        );
        c.fill_sym(
            inner_x,
            y_lo,
            z_rear - (4 - step),
            outer_x,
            y_hi,
            z_rear - (4 - step),
            p.track,
        );
    }
    let sprocket_y = (track_top_y - 3).max(3);
    c.fill_sym(inner_x, sprocket_y, z_front, outer_x, track_top_y - 1, z_front, p.track);
    c.fill_sym(inner_x, sprocket_y, z_rear, outer_x, track_top_y - 1, z_rear, p.track);

    // 3. Large front toothed drive sprocket & rear idler wheel
    let mid_x = (inner_x + outer_x) / 2;
    c.fill_cyl_x_sym(
        inner_x + 1,
        outer_x,
        sprocket_y as f32 + 0.6,
        (z_front + 2) as f32,
        2.9,
        p.dark,
    );
    c.fill_cyl_x_sym(
        outer_x,
        outer_x,
        sprocket_y as f32 + 0.6,
        (z_front + 2) as f32,
        1.6,
        p.trim,
    );
    c.fill_cyl_x_sym(
        inner_x + 1,
        outer_x - 1,
        sprocket_y as f32 + 0.5,
        (z_rear - 2) as f32,
        2.6,
        p.dark,
    );
    c.set_sym(outer_x, sprocket_y, z_rear - 2, p.trim);

    // 4. Inner interleaved row of large overlapping road wheels (staggered between outer wheels)
    let wheel_cy = wheel_radius + 0.45;
    let z_span_in0 = (z_front + 7) as f32;
    let z_span_in1 = (z_rear - 7) as f32;
    let tire_mat = if steel_rimmed { p.trim } else { p.wheel };
    for i in 0..inner_wheels.max(2) {
        let t = i as f32 / (inner_wheels.max(2) - 1) as f32;
        let wz = z_span_in0 + (z_span_in1 - z_span_in0) * t;
        c.fill_cyl_x_sym(inner_x + 1, mid_x + 1, wheel_cy, wz, wheel_radius, tire_mat);
        c.fill_cyl_x_sym(
            mid_x + 1,
            mid_x + 1,
            wheel_cy,
            wz,
            (wheel_radius - 1.1).max(1.2),
            p.dark,
        );
    }

    // 5. Outer overlapping row of huge Schachtellaufwerk road wheels
    let z_span_out0 = (z_front + 10) as f32;
    let z_span_out1 = (z_rear - 10) as f32;
    for i in 0..outer_wheels.max(2) {
        let t = i as f32 / (outer_wheels.max(2) - 1) as f32;
        let wz = z_span_out0 + (z_span_out1 - z_span_out0) * t;
        // Dark rubber (or steel-rimmed Tiger II) outer tire
        c.fill_cyl_x_sym(mid_x + 1, outer_x, wheel_cy, wz, wheel_radius, tire_mat);
        // Painted recessed steel wheel dish
        c.fill_cyl_x_sym(
            outer_x,
            outer_x,
            wheel_cy,
            wz,
            (wheel_radius - 1.1).max(1.4),
            p.main,
        );
        // Inner bolted rim ring + central hubcap
        c.fill_cyl_x_sym(
            outer_x,
            outer_x,
            wheel_cy,
            wz,
            (wheel_radius - 2.3).max(0.9),
            p.dark,
        );
        c.set_sym(outer_x, wheel_cy.round() as i32, wz.round() as i32, p.trim);
    }

    // 6. Armored mudguard fenders covering wide track tops
    let fender_outer = (outer_x + 1).min(c.half_x);
    let fz0 = (z_front - 1).max(-c.half_z);
    let fz1 = (z_rear + 1).min(c.half_z);
    c.fill_sym(inner_x, fender_y, fz0, fender_outer, fender_y, fz1, p.dark);
    // Hinged front & rear mudflaps
    c.fill_sym(inner_x, fender_y - 2, fz0, fender_outer, fender_y - 1, fz0, p.dark);
    c.fill_sym(inner_x, fender_y - 1, fz1, fender_outer, fender_y - 1, fz1, p.dark);
}

/// Helper to linearly interpolate an `i32` across integer `y in y0..=y1`.
#[inline]
fn lerp_i32(y: i32, y0: i32, y1: i32, v0: i32, v1: i32) -> i32 {
    if y1 == y0 {
        return v0;
    }
    let t = (y - y0) as f32 / (y1 - y0) as f32;
    (v0 as f32 + (v1 - v0) as f32 * t).round() as i32
}

/// Fills a solid sloped lower-glacis + upper-glacis German wedge nose from `y_belly..=y_deck`.
#[allow(clippy::too_many_arguments)]
fn fill_sloped_glacis_hull(
    c: &mut TankCanvas,
    p: &TankPalette,
    half_w_belly: i32,
    half_w_sponson: i32,
    half_w_roof: i32,
    y_belly: i32,
    y_nose: i32,
    y_sponson: i32,
    y_deck: i32,
    z_belly_front: i32,
    z_nose: i32,
    z_deck_front: i32,
    z_rear_bottom: i32,
    z_rear_top: i32,
) {
    // Lower hull tub + sloped lower glacis belly (y_belly..=y_nose)
    for y in y_belly..=y_nose {
        let zf = lerp_i32(y, y_belly, y_nose, z_belly_front, z_nose);
        let zr = lerp_i32(y, y_belly, y_nose, z_rear_bottom - 2, z_rear_bottom);
        c.fill(-half_w_belly, y, zf, half_w_belly, y, zr, p.dark);
    }

    // Mid-hull lower sponson section (y_nose+1..=y_sponson-1)
    for y in (y_nose + 1)..y_sponson {
        let zf = lerp_i32(y, y_nose, y_deck, z_nose, z_deck_front);
        c.fill(-half_w_belly, y, zf, half_w_belly, y, z_rear_bottom, p.main);
    }

    // Upper wide sloped hull casemate/glacis (y_sponson..=y_deck)
    for y in y_sponson..=y_deck {
        let hw = lerp_i32(y, y_sponson, y_deck, half_w_sponson, half_w_roof);
        let zf = lerp_i32(y, y_nose, y_deck, z_nose, z_deck_front);
        let zr = lerp_i32(y, y_sponson, y_deck, z_rear_bottom, z_rear_top);
        c.fill(-hw, y, zf, hw, y, zr, p.main);
    }
}

/// Draws a German rear-turret `Rommelkiste` (Gepäckkasten stowage bin) bolted onto turret rear.
fn draw_rommelkiste(
    c: &mut TankCanvas,
    p: &TankPalette,
    half_w: i32,
    y0: i32,
    y1: i32,
    z0: i32,
    z1: i32,
) {
    // Tapered rear profile
    c.fill(-half_w, y0, z0, half_w, y1, z1 - 1, p.dark);
    c.fill(-(half_w - 1), y0 + 1, z1, half_w - 1, y1, z1, p.dark);
    // Hinged top lid
    c.fill(-half_w, y1, z0, half_w, y1, z1 - 1, p.main);
    // Clasp latches on rear face
    c.set_sym((half_w / 2).max(1), y1 - 1, z1, p.trim);
}

// ============================================================================
// 1. PANZER II AUSF. C (Tier I Light — GERMANY_GREY)
// ============================================================================
fn build_pz2c() -> TankVoxelModels {
    let p = TankPalette::GERMANY_GREY;
    let mut hull = TankCanvas::new(11, 15, 24);

    // Running gear: narrow early-war tracks, 5 road wheels + 4 return rollers
    build_running_gear(&mut hull, &p, 8, 10, -22, 22, 6, 5, 2.0, 4, 7);

    // Lower hull tub & sloped nose wedge
    hull.fill(-7, 2, -19, 7, 7, 21, p.dark);
    for y in 2..=6 {
        let zf = lerp_i32(y, 2, 6, -19, -23);
        hull.fill(-7, y, zf, 7, y, -19, p.dark);
    }
    // Sloped front transmission deck (z = -23..-13, y = 6..=8)
    for z in -23..=-13 {
        let y_top = lerp_i32(z, -23, -13, 6, 8);
        hull.fill(-7, 6, z, 7, y_top, z, p.main);
    }
    // Transmission access hatches on front slope
    hull.fill(-5, 7, -20, -2, 7, -16, p.dark);
    hull.fill(2, 7, -20, 5, 7, -16, p.dark);

    // Stepped angular upper fighting compartment (y = 8..=12, z = -12..=10)
    hull.fill(-9, 8, -12, 9, 12, 10, p.main);
    // Bolted front driver plate trim & riveted seams
    hull.fill(-9, 8, -12, 9, 8, -12, p.dark);
    hull.fill(-9, 12, -12, 9, 12, -12, p.dark);
    // Driver armored visor slit (left) & bow viewport (right)
    draw_driver_visor(&mut hull, &p, -4, 10, -12, 2);
    hull.fill(3, 10, -12, 5, 10, -12, p.trim);
    hull.set(4, 10, -12, p.optic);

    // Bosch headlamp on left front fender & front tow hooks
    draw_headlight(&mut hull, &p, -9, 8, -20);
    draw_tow_shackles(&mut hull, &p, 6, 6, -23, -1);

    // Slightly lower rear engine deck (y = 8..=10, z = 11..=21)
    hull.fill(-8, 8, 11, 8, 10, 21, p.main);
    hull.fill(-7, 11, 11, 7, 11, 13, p.main);
    draw_engine_grilles(&mut hull, &p, 6, 10, 14, 20);
    // Horizontal cylindrical rear exhaust muffler + tail lamp
    draw_rear_muffler(&mut hull, &p, 6, 8.0, 22.5, 1.2);
    hull.set(-8, 8, 22, p.red);

    // Crisp Balkenkreuz on hull flanks (y = 8..=12 centered at cy = 10)
    draw_balkenkreuz_flanks(&mut hull, &p, 9, 10, -1);

    // Pioneer tools along right fender
    hull.fill(9, 8, -8, 10, 8, 6, p.wood);
    hull.fill(9, 8, 7, 10, 8, 9, p.trim);

    // Iconic LEFT-OFFSET small angular early-war Pz.II turret
    let mut turret = TankCanvas::new(7, 11, 12);
    for y in 0..=5 {
        let inset = if y >= 4 { 1 } else { 0 };
        let hw = 5 - inset;
        // Front narrower angular cheeks widening toward mid-turret
        turret.fill(-hw + 1, y, -6, hw - 1, y, -4, p.main);
        turret.fill(-hw, y, -3, hw, y, 4, p.main);
        turret.fill(-hw + 1, y, 5, hw - 1, y, 6, p.main);
    }
    // Turret side vision slits & pistol ports
    turret.set_sym(5, 3, -1, p.optic);
    turret.set_sym(5, 3, 2, p.trim);
    // Compact early-war commander hatch/cupola on turret rear roof
    draw_cupola(&mut turret, &p, 0.0, 1.5, 6, 2, 2.3);

    // Armored front mantlet housing side-by-side 20mm KwK 30 + coaxial MG34
    turret.fill(-3, 1, -7, 3, 4, -6, p.dark);
    turret.fill(-3, 1, -7, 3, 1, -7, p.trim);
    // Protruding coaxial MG34 barrel right next to main 20mm gun (at x = +1)
    turret.fill(1, 2, -11, 1, 2, -8, p.trim);
    // Gunner's telescopic sight port on left
    turret.set(-3, 3, -7, p.optic);

    // 20mm KwK 30 barrel: len 18, thin, no muzzle brake, offset to x = -1 alongside MG34
    let barrel = build_detailed_barrel(&p, 18, false, 3, MuzzleBrakeStyle::None);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(-1.5, 13.0, -3.0),
        Vec3::new(-1.0, 2.0, -7.0),
        18.0,
    )
}

// ============================================================================
// 2. PANZER III AUSF. J (Tier II Medium — GERMANY_GREY)
// ============================================================================
fn build_pz3j() -> TankVoxelModels {
    let p = TankPalette::GERMANY_GREY;
    let mut hull = TankCanvas::new(14, 16, 27);

    // Running gear: 6 torsion-bar road wheels + 3 return rollers
    build_running_gear(&mut hull, &p, 10, 13, -25, 25, 7, 6, 2.4, 3, 8);

    // Lower hull tub & sloped lower nose glacis
    hull.fill(-9, 2, -22, 9, 8, 24, p.dark);
    for y in 2..=7 {
        let zf = lerp_i32(y, 2, 7, -22, -25);
        hull.fill(-9, y, zf, 9, y, -22, p.dark);
    }
    // Central spare track rack strapped across lower bow + front tow shackles
    draw_spare_track_bow(&mut hull, &p, -6, 6, 4, 6, -25);
    draw_tow_shackles(&mut hull, &p, 8, 6, -25, -1);

    // Sloped front transmission deck (z = -25..-15, y = 7..=9)
    for z in -25..=-15 {
        let y_top = lerp_i32(z, -25, -15, 7, 9);
        hull.fill(-10, 7, z, 10, y_top, z, p.main);
    }
    // Armored brake inspection hatches on transmission glacis
    hull.fill(-7, 8, -22, -3, 8, -17, p.dark);
    hull.fill(3, 8, -22, 7, 8, -17, p.dark);

    // Classic German boxy stepped fighting compartment (y = 9..=13, z = -14..=10)
    hull.fill(-12, 9, -14, 12, 13, 10, p.main);
    // Vertical 50mm front driver plate detail
    hull.fill(-12, 9, -15, 12, 13, -15, p.main);
    hull.fill(-12, 9, -15, 12, 9, -15, p.dark);
    // Left armored driver visor slit + right bow MG34 ball mount
    draw_driver_visor(&mut hull, &p, -6, 11, -15, 2);
    draw_bow_mg(&mut hull, &p, 6, 11, -15);
    // Bosch headlamp on left fender + Notek blackout lamp on right
    draw_headlight(&mut hull, &p, -12, 9, -23);
    hull.set(12, 9, -23, p.trim);
    hull.set(12, 9, -24, p.dark);

    // Side hull escape hatches & Balkenkreuz on hull flanks
    draw_balkenkreuz_flanks(&mut hull, &p, 12, 11, -2);
    // Pioneer tools & antenna trough along fenders
    hull.fill(-13, 9, -6, -13, 9, 10, p.wood);
    hull.fill(13, 9, -8, 13, 9, 8, p.trim);

    // Rear engine deck (y = 9..=12, z = 11..=25) with side intake cowls & grilles
    hull.fill(-11, 9, 11, 11, 12, 25, p.main);
    draw_engine_grilles(&mut hull, &p, 8, 12, 13, 22);
    // Angled rear air intake scoops on engine flanks
    hull.fill_sym(11, 10, 14, 12, 11, 21, p.dark);
    // Horizontal rear exhaust muffler & tail lamp
    draw_rear_muffler(&mut hull, &p, 8, 8.5, 26.0, 1.3);
    hull.set(-11, 9, 25, p.red);

    // Sharp hexagonal Pz.III turret with tall cupola, spaced mantlet & rear Rommelkiste
    let mut turret = TankCanvas::new(10, 13, 15);
    for y in 0..=6 {
        let top_inset = if y == 6 { 1 } else { 0 };
        for z in -8..=9 {
            let base_hw = if z < -2 {
                lerp_i32(z, -8, -2, 6, 8)
            } else if z <= 4 {
                8
            } else {
                lerp_i32(z, 4, 9, 8, 6)
            };
            let hw = (base_hw - top_inset).max(4);
            turret.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Balkenkreuz flush on vertical turret flanks (x = ±8, y = 1..=5, z = -1..=3)
    draw_balkenkreuz_flanks(&mut turret, &p, 8, 3, 1);
    // Front cheek vision slits
    turret.set_sym(7, 3, -5, p.optic);

    // Wide external spaced armored mantlet (Pz.III J 50mm front shield)
    turret.fill(-5, 1, -10, 5, 5, -9, p.dark);
    turret.fill(-4, 2, -10, 4, 4, -10, p.main);
    // Gunner binocular optic left + coaxial MG34 port right
    turret.set(-3, 3, -10, p.optic);
    turret.set(3, 3, -10, p.black);
    turret.set(3, 3, -11, p.trim);

    // Rear Rommelkiste stowage box
    draw_rommelkiste(&mut turret, &p, 5, 1, 5, 10, 14);

    // Tall cylindrical commander drum cupola
    draw_cupola(&mut turret, &p, 0.0, 4.5, 7, 4, 3.0);

    // 50mm KwK 38 L/42 barrel: len 22, thin, no muzzle brake
    let barrel = build_detailed_barrel(&p, 22, false, 4, MuzzleBrakeStyle::None);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -3.0),
        Vec3::new(0.0, 3.0, -10.0),
        22.0,
    )
}

// ============================================================================
// 3. PANZER IV AUSF. F2 (Tier III Medium — GERMANY_GREY)
// ============================================================================
fn build_pz4_f2() -> TankVoxelModels {
    let p = TankPalette::GERMANY_GREY;
    let mut hull = TankCanvas::new(14, 16, 29);

    // Running gear: iconic 8 paired small road wheels + 4 return rollers
    build_running_gear(&mut hull, &p, 10, 13, -27, 27, 7, 8, 1.95, 4, 8);
    // Leaf-spring paired bogie suspension housings between each pair of road wheels (4 pairs!)
    for &pair_z in &[-18, -6, 6, 18] {
        hull.fill_sym(12, 3, pair_z - 1, 13, 4, pair_z + 1, p.trim);
    }

    // Lower hull tub & sloped lower bow glacis
    hull.fill(-9, 2, -24, 9, 8, 26, p.dark);
    for y in 2..=7 {
        let zf = lerp_i32(y, 2, 7, -24, -27);
        hull.fill(-9, y, zf, 9, y, -24, p.dark);
    }
    // Spare track links bolted across lower bow + heavy tow shackles
    draw_spare_track_bow(&mut hull, &p, -7, 7, 4, 6, -27);
    draw_tow_shackles(&mut hull, &p, 9, 6, -27, -1);

    // Sloped front transmission glacis (z = -27..-16, y = 7..=9)
    for z in -27..=-16 {
        let y_top = lerp_i32(z, -27, -16, 7, 9);
        hull.fill(-10, 7, z, 10, y_top, z, p.main);
    }
    hull.fill(-7, 8, -24, -2, 8, -18, p.dark);
    hull.fill(2, 8, -24, 7, 8, -18, p.dark);

    // Sharp German boxy stepped fighting compartment (y = 9..=13, z = -16..=10)
    hull.fill(-12, 9, -15, 12, 13, 10, p.main);
    // Vertical 50mm front driver plate
    hull.fill(-12, 9, -16, 12, 13, -16, p.main);
    hull.fill(-12, 9, -16, 12, 9, -16, p.dark);
    draw_driver_visor(&mut hull, &p, -6, 11, -16, 2);
    draw_bow_mg(&mut hull, &p, 6, 11, -16);
    // Twin Bosch headlamps on front fenders
    draw_headlight(&mut hull, &p, -12, 9, -25);
    draw_headlight(&mut hull, &p, 12, 9, -25);

    // Balkenkreuz on hull flanks (y = 9..=13 centered at cy = 11)
    draw_balkenkreuz_flanks(&mut hull, &p, 12, 11, -3);
    // Spare road wheels on left fender + wooden tools on right fender
    hull.fill(-13, 9, -11, -13, 10, -8, p.wheel);
    hull.fill(-13, 9, -6, -13, 10, -3, p.wheel);
    hull.fill(13, 9, -10, 13, 9, 8, p.wood);
    hull.fill(13, 9, 9, 13, 9, 11, p.trim);

    // Rear engine deck (y = 9..=12, z = 11..=26)
    hull.fill(-11, 9, 11, 11, 12, 26, p.main);
    draw_engine_grilles(&mut hull, &p, 8, 12, 13, 24);
    hull.fill_sym(11, 10, 15, 12, 11, 23, p.dark);

    // Prominent iconic Pz.IV horizontal cylindrical exhaust muffler across rear hull
    draw_rear_muffler(&mut hull, &p, 9, 8.5, 27.5, 1.5);
    // Smaller auxiliary starter-motor muffler mounted on left rear
    hull.fill_cyl_x(-9, -4, 11.2, 27.5, 0.9, p.rust);
    hull.set(-11, 9, 27, p.red);

    // Crisp angular Pz.IV turret with OVERSIZED rear Rommelkiste bin & tall drum cupola
    let mut turret = TankCanvas::new(10, 13, 17);
    for y in 0..=6 {
        let top_inset = if y == 6 { 1 } else { 0 };
        for z in -8..=9 {
            let base_hw = if z < -4 {
                lerp_i32(z, -8, -4, 6, 8)
            } else if z <= 6 {
                8
            } else {
                lerp_i32(z, 6, 9, 8, 7)
            };
            let hw = (base_hw - top_inset).max(4);
            turret.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Side armored hatch doors with vision slits & pistol ports
    turret.fill_sym(8, 1, -2, 8, 5, 3, p.dark);
    turret.set_sym(8, 3, 0, p.optic);
    turret.set_sym(8, 3, 5, p.trim);
    // Front cheek vision ports
    turret.set_sym(7, 3, -6, p.optic);

    // External wide Pz.IV gun mantlet with central armored sleeve
    turret.fill(-5, 1, -10, 5, 5, -9, p.dark);
    turret.fill(-3, 2, -10, 3, 4, -10, p.main);
    turret.set(-3, 3, -10, p.optic);
    turret.set(3, 3, -10, p.black);
    turret.set(3, 3, -11, p.trim);

    // Oversized Pz.IV rear Rommelkiste stowage bin spanning nearly full turret rear
    draw_rommelkiste(&mut turret, &p, 7, 1, 5, 10, 16);

    // Tall cylindrical commander drum cupola
    draw_cupola(&mut turret, &p, 0.0, 4.5, 7, 4, 3.2);

    // Unmistakable long 75mm KwK 40 L/43 barrel: len 34, thin, DoubleBaffle muzzle brake
    let barrel = build_detailed_barrel(&p, 34, false, 4, MuzzleBrakeStyle::DoubleBaffle);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -3.5),
        Vec3::new(0.0, 3.0, -10.0),
        34.0,
    )
}

// ============================================================================
// 4. STUG III AUSF. G (Tier IV Tank Destroyer — GERMANY_TAN)
// ============================================================================
fn build_stug3g() -> TankVoxelModels {
    let p = TankPalette::GERMANY_TAN;
    let mut hull = TankCanvas::new(15, 22, 28);

    // Running gear (Pz.III chassis): 6 road wheels + 3 return rollers
    build_running_gear(&mut hull, &p, 10, 13, -25, 25, 7, 6, 2.4, 3, 8);

    // Lower hull tub & sloped nose glacis
    hull.fill(-9, 2, -22, 9, 8, 24, p.dark);
    for y in 2..=7 {
        let zf = lerp_i32(y, 2, 7, -22, -26);
        hull.fill(-9, y, zf, 9, y, -22, p.dark);
    }
    // Spare track links bolted across front bow + heavy tow shackles
    draw_spare_track_bow(&mut hull, &p, -6, 6, 4, 6, -26);
    draw_tow_shackles(&mut hull, &p, 8, 6, -26, -1);

    // Sloped front transmission deck (z = -26..-14, y = 7..=9)
    for z in -26..=-14 {
        let y_top = lerp_i32(z, -26, -14, 7, 9);
        hull.fill(-10, 7, z, 10, y_top, z, p.main);
    }
    hull.fill(-7, 8, -23, -3, 8, -17, p.dark);
    hull.fill(3, 8, -23, 7, 8, -17, p.dark);
    // Bosch headlamp on left front fender
    draw_headlight(&mut hull, &p, -12, 9, -24);

    // Low rear engine deck (y = 9..=12, z = 8..=25)
    hull.fill(-11, 9, 8, 11, 12, 25, p.main);
    draw_engine_grilles(&mut hull, &p, 8, 12, 11, 21);
    // Spare track stowage rack across rear engine deck
    hull.fill(-8, 13, 22, 8, 13, 24, p.track);
    hull.fill(-8, 13, 23, 8, 13, 23, p.rust);
    // Rear horizontal cylindrical muffler
    draw_rear_muffler(&mut hull, &p, 8, 8.5, 26.5, 1.3);
    hull.set(-11, 9, 26, p.red);

    // ULTRA-LOW SLEEK FIXED CASEMATE SUPERSTRUCTURE (part of hull — never rotates!)
    // Lower overhanging angled side panniers (y = 9..=12, z = -14..=8)
    hull.fill(-13, 9, -14, 13, 12, 8, p.main);
    // Sharply sloped upper casemate walls (y = 13..=16)
    for y in 13..=16 {
        let hw = lerp_i32(y, 12, 16, 13, 9);
        let zf = lerp_i32(y, 12, 16, -14, -10);
        let zr = lerp_i32(y, 12, 16, 8, 7);
        hull.fill(-hw, y, zf, hw, y, zr, p.main);
    }
    // Crisp dark weld trim along casemate roof rim
    hull.fill(-9, 16, -10, 9, 16, -10, p.dark);
    hull.fill_sym(9, 16, -10, 9, 16, 7, p.dark);

    // Left armored driver visor hood protruding cleanly on front-left casemate
    hull.fill(-9, 11, -15, -5, 13, -13, p.main);
    draw_driver_visor(&mut hull, &p, -7, 12, -15, 1);

    // Commander's periscope cupola on LEFT roof (low profile StuG III G cupola)
    draw_cupola(&mut hull, &p, -5.5, 1.5, 17, 3, 2.5);

    // Iconic FOLDING MG34 ARMORED SHIELD + loader's hatch on RIGHT roof
    hull.fill(2, 17, 2, 7, 17, 6, p.dark); // Loader hatch frame
    hull.fill(2, 17, -1, 7, 20, -1, p.main); // Angled armored MG shield
    hull.fill(2, 20, -1, 7, 20, -1, p.dark);
    hull.set(4, 19, -1, p.black); // Firing slot
    hull.fill(4, 19, -4, 4, 19, -2, p.trim); // MG34 barrel pointing forward

    // Full hanging SCHÜRZEN side armor skirts outside tracks (x = ±15, y = 3..=9)
    draw_schuerzen(&mut hull, &p, 15, 13, 3, 9, -21, 22, &[-10, 1, 12]);
    // Crisp Balkenkreuz painted right onto the Schürzen side skirts!
    draw_balkenkreuz_flanks(&mut hull, &p, 15, 6, -4);

    // Carve clean RECESSED CENTRAL FRONT GUN EMBRASURE for the Saukopf mantlet
    // Floor at y = 10, roof at y = 16 -> clear y = 11..=15, x = -3..=3, z = -15..=-10
    hull.fill(-3, 11, -15, 3, 15, -10, VoxelId::AIR);
    // Solid dark armored inner recoil backing wall at z = -9 so zero daylight ever leaks
    hull.fill(-4, 10, -9, 4, 15, -9, p.dark);

    // TURRET CANVAS: ONLY the iconic cast curved SAUKOPF ("pig's head") mantlet!
    // Mounts at (0, 11, -11): y = 0..=4 maps to hull y = 11..=15 (flush, zero Z-fighting!)
    let mut turret = TankCanvas::new(4, 6, 6);
    // Inner swivel collar sitting cleanly inside embrasure with ±10° yaw clearance
    turret.fill_cyl_z(0.0, 2.0, -1, 1, 2.2, p.dark);
    // Flanged cast Saukopf shield sealing the front embrasure cleanly (z_local = -2..-3)
    turret.fill(-3, 1, -3, 3, 3, -2, p.dark);
    turret.fill(-2, 0, -3, 2, 4, -2, p.dark);
    // Smooth tapered conical "pig's head" snout projecting forward (z_local = -4..-5)
    turret.fill_cyl_z(0.0, 2.0, -4, -4, 2.1, p.main);
    turret.fill_cyl_z(0.0, 2.0, -5, -5, 1.4, p.dark);
    // Coaxial MG port on right side of Saukopf
    turret.set(2, 2, -3, p.black);

    // Long 75mm StuK 40 L/48 barrel: len 36, thin, DoubleBaffle muzzle brake
    let barrel = build_detailed_barrel(&p, 36, false, 4, MuzzleBrakeStyle::DoubleBaffle);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 11.0, -11.0),
        Vec3::new(0.0, 2.0, -5.0),
        36.0,
    )
}

// ============================================================================
// 5. TIGER I AUSF. H1 (Tier V Heavy — GERMANY_TAN)
// ============================================================================
fn build_tiger_h1() -> TankVoxelModels {
    let p = TankPalette::GERMANY_TAN;
    let mut hull = TankCanvas::new(18, 19, 31);

    // Massive 725mm wide overhanging tracks + huge overlapping SCHACHTELLAUFWERK road wheels
    build_schachtellaufwerk_gear(&mut hull, &p, 10, 16, -29, 29, 9, 4, 5, 3.6, 10, false);
    // Side armored mudguard lip overhanging outer edge of tracks (x = ±17..=18)
    hull.fill_sym(17, 9, -29, 18, 10, 29, p.dark);

    // Lower hull tub & sloped lower front glacis
    hull.fill(-10, 2, -25, 10, 10, 27, p.dark);
    for y in 2..=7 {
        let zf = lerp_i32(y, 2, 7, -24, -29);
        hull.fill(-10, y, zf, 10, y, -24, p.dark);
    }
    // Spare track strip bolted across lower nose + massive front tow shackles
    draw_spare_track_bow(&mut hull, &p, -7, 7, 5, 6, -29);
    draw_tow_shackles(&mut hull, &p, 9, 7, -29, -1);

    // Slightly sloped front transmission deck (z = -29..-21, y = 7..=10)
    for z in -29..=-21 {
        let y_top = lerp_i32(z, -29, -21, 7, 10);
        hull.fill(-10, 7, z, 10, y_top, z, p.main);
    }

    // LEGENDARY TIGER I WIDE FULL-WIDTH BOXY UPPER HULL (x = -17..=17, y = 10..=15, z = -21..=28)
    hull.fill(-17, 10, -21, 17, 15, 28, p.main);
    // Interlocking vertical front 102mm armor plate trim seams
    hull.fill(-17, 10, -21, 17, 10, -21, p.dark);
    hull.fill(-17, 15, -21, 17, 15, -21, p.dark);
    hull.fill_sym(17, 10, -21, 17, 15, -21, p.dark);

    // Unmistakable head-on Tiger I face:
    // 1) Left horizontal armored driver visor slit
    draw_driver_visor(&mut hull, &p, -8, 13, -21, 3);
    // 2) Right bow MG34 Kugelblende ball mount
    draw_bow_mg(&mut hull, &p, 9, 13, -21);
    // 3) Iconic CENTRAL Bosch headlamp mounted dead-center on front deck edge
    draw_headlight(&mut hull, &p, 0, 16, -21);
    hull.set(0, 16, -20, p.trim); // Stalk base

    // Driver & radio operator roof hatches in front of turret
    hull.fill_ellipse_y(-8.5, -16.0, 2.2, 2.2, 16, 16, p.dark);
    hull.fill_ellipse_y(8.5, -16.0, 2.2, 2.2, 16, 16, p.dark);

    // Crisp Balkenkreuz on wide vertical Tiger I hull flanks (x = ±17, y = 11..=15 -> cy = 13)
    draw_balkenkreuz_flanks(&mut hull, &p, 17, 13, -4);

    // Symmetric rear engine deck radiator grilles + central circular cooling intake
    draw_engine_grilles(&mut hull, &p, 13, 15, 11, 25);
    hull.fill_ellipse_y(0.0, 18.0, 3.2, 3.2, 15, 15, p.black);
    hull.fill_ellipse_y(0.0, 18.0, 2.0, 2.0, 15, 15, p.dark);

    // REAR TIGER H1 SIGNATURE LANDMARKS:
    // 1) Twin vertical exhaust pipes enclosed in curved armored heat-shields
    hull.fill_sym(4, 8, 29, 6, 15, 30, p.dark); // Armored curved shrouds
    hull.fill_sym(5, 9, 29, 5, 17, 29, p.rust); // Inner vertical exhaust stacks peeking above!
    hull.set_sym(5, 17, 29, p.black); // Open exhaust tips
    // 2) Twin vertical FEIFEL cylindrical air filter canisters + intake ducts
    hull.fill_ellipse_y(-12.5, 29.5, 1.8, 1.6, 11, 16, p.dark);
    hull.fill_ellipse_y(12.5, 29.5, 1.8, 1.6, 11, 16, p.dark);
    hull.fill_sym(12, 17, 29, 13, 17, 29, p.trim);
    // Feifel intake pipes curving from canisters forward into rear engine deck
    hull.fill_sym(11, 16, 25, 13, 16, 28, p.trim);
    // Rear tow shackles
    draw_tow_shackles(&mut hull, &p, 10, 8, 29, 1);

    // ICONIC WIDE HORSESHOE CURVED TIGER I TURRET
    let mut turret = TankCanvas::new(13, 15, 16);
    // Curved U-shaped horseshoe main armor body (y = 0..=7)
    turret.fill_ellipse_y(0.0, 2.0, 11.5, 9.5, 0, 7, p.main);
    // Broad flat-cheeks front turret extension (z = -9..=2)
    for z in -9..=2 {
        let hw = if z < -5 {
            lerp_i32(z, -9, -5, 10, 12)
        } else {
            12
        };
        turret.fill(-hw, 0, z, hw, 7, z, p.main);
    }
    // Crisp dark roof border bevel
    turret.fill(-9, 7, -9, 9, 7, -9, p.dark);
    // Side vision ports & spare track links bolted to turret cheeks
    turret.set_sym(12, 4, -3, p.optic);
    for y in 2..=5 {
        turret.set_sym(12, y, 0, if y % 2 == 0 { p.track } else { p.rust });
        turret.set_sym(12, y, 2, if y % 2 == 0 { p.track } else { p.rust });
    }

    // FULL-WIDTH THICK STEPPED TIGER I GUN MANTLET (z = -12..-10)
    turret.fill(-10, 1, -11, 10, 6, -10, p.dark);
    // Thick reinforced central armored bulge around trunnion
    turret.fill(-7, 2, -12, 7, 5, -11, p.main);
    turret.fill(-7, 2, -12, 7, 2, -12, p.dark);
    // Twin binocular gunner sight holes (left) + coaxial MG34 port (right)
    turret.set(-4, 4, -12, p.optic);
    turret.set(-3, 4, -12, p.optic);
    turret.set(4, 4, -12, p.black);
    turret.set(4, 4, -13, p.trim);

    // Rear curved turret stowage bin (Rommelkiste)
    draw_rommelkiste(&mut turret, &p, 7, 1, 6, 11, 15);

    // Tall iconic cylindrical Tiger H1 drum cupola on LEFT + loader hatch on right
    draw_cupola(&mut turret, &p, -6.0, 3.0, 8, 5, 3.2);
    turret.fill_ellipse_y(5.0, 2.0, 2.2, 2.2, 8, 8, p.dark);

    // Thick 88mm KwK 36 L/56 barrel: len 42, thick, DoubleBaffle muzzle brake
    let barrel = build_detailed_barrel(&p, 42, true, 6, MuzzleBrakeStyle::DoubleBaffle);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 16.0, -3.0),
        Vec3::new(0.0, 4.0, -12.0),
        42.0,
    )
}

// ============================================================================
// 6. PANTHER AUSF. A (Tier VI Medium — GERMANY_TAN)
// ============================================================================
fn build_panther_a() -> TankVoxelModels {
    let p = TankPalette::GERMANY_TAN;
    let mut hull = TankCanvas::new(17, 19, 34);

    // Wide tracks + massive overlapping SCHACHTELLAUFWERK interleaved road wheels
    build_schachtellaufwerk_gear(&mut hull, &p, 10, 15, -31, 31, 9, 4, 4, 3.6, 10, false);

    // Wide, sleek, steeply sloped 55° Panther hull & glacis
    fill_sloped_glacis_hull(
        &mut hull, &p, 10, 16, 14, // belly, sponson, roof half-widths
        2, 7, 10, 16, // belly, nose, sponson, deck Y
        -25, -32, -18, // belly front, sharp nose tip, deck front Z
        32, 28, // rear bottom, rear top Z (sloped rear!)
    );

    // Sharp interlocking bow nose weld seam & heavy front tow clevises
    hull.fill(-10, 7, -32, 10, 7, -32, p.dark);
    draw_tow_shackles(&mut hull, &p, 10, 7, -32, -1);

    // Head-on Panther Ausf. A signature details on 55° glacis:
    // 1) Right Kugelblende / letterbox MG34 ball mount on upper glacis (y = 13 -> z ≈ -23)
    draw_bow_mg(&mut hull, &p, 7, 13, -23);
    // 2) Left armored driver periscope slots & roof hatches
    hull.fill(-8, 17, -19, -5, 17, -18, p.dark);
    hull.set(-7, 17, -19, p.optic);
    hull.set(-6, 17, -19, p.optic);
    hull.fill(-9, 17, -16, -4, 17, -13, p.dark);
    hull.fill(4, 17, -16, 9, 17, -13, p.dark);
    // 3) Welded Bosch/Notek headlamp on left front fender
    draw_headlight(&mut hull, &p, -13, 11, -30);

    // Hanging SCHÜRZEN side skirts above the interleaved road wheels (y = 5..=10, x = ±17)
    draw_schuerzen(&mut hull, &p, 17, 15, 5, 10, -25, 25, &[-13, -1, 11]);

    // Rear engine deck cooling fan grilles + side louvers
    draw_engine_grilles(&mut hull, &p, 11, 16, 9, 26);
    hull.fill_ellipse_y(-6.0, 18.0, 2.8, 2.8, 16, 16, p.black);
    hull.fill_ellipse_y(6.0, 18.0, 2.8, 2.8, 16, 16, p.black);
    hull.fill_ellipse_y(-6.0, 18.0, 1.5, 1.5, 16, 16, p.dark);
    hull.fill_ellipse_y(6.0, 18.0, 1.5, 1.5, 16, 16, p.dark);

    // Twin angled rear exhaust pipes + twin rectangular rear stowage boxes
    for y in 10..=16 {
        let z_pipe = lerp_i32(y, 10, 16, 32, 33);
        hull.set_sym(4, y, z_pipe, p.rust);
        hull.set_sym(5, y, z_pipe, p.rust);
    }
    hull.fill_sym(3, 10, 32, 6, 12, 32, p.dark); // Armored exhaust bases
    hull.fill_sym(9, 10, 30, 14, 14, 33, p.dark); // Rear hull stowage boxes
    hull.fill_sym(10, 14, 30, 13, 14, 32, p.main);

    // Crisp angled Panther Ausf. A turret with iconic semi-cylindrical curved mantlet
    let mut turret = TankCanvas::new(12, 15, 16);
    for y in 0..=8 {
        let tumblehome = lerp_i32(y, 0, 8, 0, 3);
        let z_front = lerp_i32(y, 0, 8, -10, -8);
        let z_rear = lerp_i32(y, 0, 8, 12, 10);
        for z in z_front..=z_rear {
            let base_hw = if z < -2 {
                lerp_i32(z, -10, -2, 8, 11)
            } else if z <= 4 {
                11
            } else {
                lerp_i32(z, 4, 12, 11, 8)
            };
            let hw = (base_hw - tumblehome).max(4);
            turret.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Crisp Balkenkreuz painted flush onto sloped Panther turret cheeks!
    draw_balkenkreuz_on_sloped_flanks(&mut turret, &p, 4, 2, 11);

    // Iconic SEMI-CYLINDRICAL CURVED PANTHER MANTLET spanning full turret front
    turret.fill_cyl_x(-8, 8, 4.0, -10.5, 3.2, p.dark);
    turret.fill_cyl_x(-6, 6, 4.0, -10.8, 2.8, p.main);
    // Twin gunner sight holes (left) & coaxial MG hole (right) on curved mantlet
    turret.set(-3, 5, -13, p.optic);
    turret.set(-2, 5, -13, p.optic);
    turret.set(3, 4, -13, p.black);

    // Tall cylindrical cast Panther Ausf. A commander cupola on LEFT
    draw_cupola(&mut turret, &p, -4.5, 4.5, 9, 4, 3.0);
    // Circular rear turret escape hatch on back wall
    turret.fill_cyl_z(0.0, 4.0, 11, 12, 2.2, p.dark);

    // Super-long slender 75mm KwK 42 L/70 barrel: len 48, thin, DoubleBaffle muzzle brake
    let barrel = build_detailed_barrel(&p, 48, false, 5, MuzzleBrakeStyle::DoubleBaffle);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 17.0, -3.5),
        Vec3::new(0.0, 4.0, -13.0),
        48.0,
    )
}

// ============================================================================
// 7. TIGER II AUSF. H — KING TIGER HENSCHEL (Tier VII Heavy — GERMANY_TAN)
// ============================================================================
fn build_tiger2_h() -> TankVoxelModels {
    let p = TankPalette::GERMANY_TAN;
    let mut hull = TankCanvas::new(18, 20, 36);

    // Massive 800mm wide tracks + huge overlapping STEEL-RIMMED Schachtellaufwerk road wheels
    build_schachtellaufwerk_gear(&mut hull, &p, 10, 17, -33, 33, 10, 5, 4, 3.8, 11, true);
    // Overhanging armored side mudguard skirts (x = ±18, y = 9..=11)
    hull.fill_sym(18, 9, -33, 18, 11, 33, p.dark);
    hull.fill_sym(18, 10, -30, 18, 10, 30, p.main);

    // Enormous long sharply sloped 50° massive Tiger II glacis hull
    fill_sloped_glacis_hull(
        &mut hull, &p, 10, 17, 15, // belly, sponson, roof half-widths
        2, 7, 11, 17, // belly, nose, sponson, deck Y
        -28, -35, -21, // belly front, sharp interlocking bow nose, deck front Z
        35, 31, // sloped rear hull Z
    );

    // Interlocking bow nose armor tooth-weld accent + massive tow shackles
    for x in -10..=10 {
        hull.set(x, 7, -35, if x % 2 == 0 { p.dark } else { p.trim });
    }
    draw_tow_shackles(&mut hull, &p, 11, 7, -35, -1);

    // Head-on King Tiger glacis landmarks:
    // 1) Right bow MG34 Kugelblende ball mount (y = 13 -> z ≈ -27)
    draw_bow_mg(&mut hull, &p, 9, 13, -27);
    // 2) Iconic CENTRAL Bosch headlamp mounted on front upper glacis
    draw_headlight(&mut hull, &p, 0, 13, -27);
    // 3) Left armored driver periscope slots + twin forward hull crew hatches
    hull.fill(-9, 18, -22, -6, 18, -21, p.dark);
    hull.set(-8, 18, -22, p.optic);
    hull.set(-7, 18, -22, p.optic);
    hull.fill_ellipse_y(-8.0, -17.0, 2.3, 2.3, 18, 18, p.dark);
    hull.fill_ellipse_y(8.0, -17.0, 2.3, 2.3, 18, 18, p.dark);

    // Rear engine deck cooling fan grilles + circular intakes
    draw_engine_grilles(&mut hull, &p, 12, 17, 11, 29);
    hull.fill_ellipse_y(-6.5, 20.0, 3.0, 3.0, 17, 17, p.black);
    hull.fill_ellipse_y(6.5, 20.0, 3.0, 3.0, 17, 17, p.black);
    hull.fill_ellipse_y(-6.5, 20.0, 1.5, 1.5, 17, 17, p.dark);
    hull.fill_ellipse_y(6.5, 20.0, 1.5, 1.5, 17, 17, p.dark);

    // Twin heavy rear exhaust pipes with armored guards + rear tow shackles
    hull.fill_sym(4, 10, 35, 6, 13, 35, p.dark);
    hull.fill_sym(5, 10, 36, 5, 15, 36, p.rust);
    hull.set_sym(5, 15, 36, p.black);
    draw_tow_shackles(&mut hull, &p, 10, 9, 35, 1);

    // ICONIC HENSCHEL FLAT-FRONT ANGLED-CHEEK LONG HEAVY TURRET
    let mut turret = TankCanvas::new(13, 14, 21);
    for y in 0..=8 {
        let tumblehome = lerp_i32(y, 0, 8, 0, 3);
        let z_rear = lerp_i32(y, 0, 8, 19, 17);
        for z in -14..=z_rear {
            // Crisp narrow vertical flat front plate at z = -14 widening to long heavy cheeks & bustle
            let base_hw = if z < -4 {
                lerp_i32(z, -14, -4, 7, 12)
            } else if z <= 5 {
                12
            } else {
                lerp_i32(z, 5, 19, 12, 9)
            };
            // Keep the narrow vertical 185mm front plate crisp with minimal taper
            let effective_inset = if z <= -12 { tumblehome / 2 } else { tumblehome };
            let hw = (base_hw - effective_inset).max(4);
            turret.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Crisp vertical 185mm Henschel flat front plate bevel border
    turret.fill(-7, 0, -14, 7, 0, -14, p.dark);
    turret.fill(-6, 8, -14, 6, 8, -14, p.dark);
    // Sight aperture left + coaxial MG port right directly on the flat front plate
    turret.set(-4, 5, -14, p.optic);
    turret.set(4, 4, -14, p.black);

    // Tapered conical cast SAUKOPF mantlet collar projecting proudly from flat front plate
    turret.fill_cyl_z(0.0, 4.0, -15, -15, 3.4, p.dark);
    turret.fill_cyl_z(0.0, 4.0, -16, -16, 3.0, p.main);
    turret.fill_cyl_z(0.0, 4.0, -17, -17, 2.6, p.main);
    turret.fill_cyl_z(0.0, 4.0, -18, -18, 2.2, p.dark);
    turret.fill_cyl_z(0.0, 4.0, -19, -19, 1.8, p.dark);
    turret.fill_cyl_z(0.0, 4.0, -20, -20, 1.5, p.trim);

    // Crisp Balkenkreuz flush on sloped turret front-mid flanks
    draw_balkenkreuz_on_sloped_flanks(&mut turret, &p, 4, -4, 12);

    // Spare track links hung neatly along the long rear bustle flanks
    for &tz in &[3, 6, 9, 12] {
        for y in 2..=6 {
            for x in (1..=12).rev() {
                if !turret.get(x, y, tz).is_air() {
                    let link = if y % 2 == 0 { p.track } else { p.rust };
                    turret.set_sym(x + 1, y, tz, link);
                    break;
                }
            }
        }
    }

    // Sleek low-profile Tiger II commander cupola on LEFT + loader hatch on right
    draw_cupola(&mut turret, &p, -4.5, 2.5, 9, 3, 2.9);
    turret.fill_ellipse_y(4.5, 1.5, 2.2, 2.2, 9, 9, p.dark);
    // Rear bustle escape hatch
    turret.fill(-3, 2, 18, 3, 6, 19, p.dark);

    // Mammoth 88mm KwK 43 L/71 barrel: len 54, thick, DoubleBaffle muzzle brake
    let barrel = build_detailed_barrel(&p, 54, true, 7, MuzzleBrakeStyle::DoubleBaffle);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 18.0, -3.5),
        Vec3::new(0.0, 4.5, -20.0),
        54.0,
    )
}

// ============================================================================
// 8. JAGDTIGER SD.KFZ. 186 (Tier VIII Heavy TD — GERMANY_TAN)
// ============================================================================
fn build_jagdtiger() -> TankVoxelModels {
    let p = TankPalette::GERMANY_TAN;
    let mut hull = TankCanvas::new(18, 31, 39);

    // Stretched Tiger II running gear: massive 800mm tracks + 5+5 interleaved steel road wheels
    build_schachtellaufwerk_gear(&mut hull, &p, 10, 17, -36, 36, 10, 5, 5, 3.8, 11, true);
    // Overhanging armored side mudguard skirts (x = ±18, y = 9..=11)
    hull.fill_sym(18, 9, -36, 18, 11, 36, p.dark);
    hull.fill_sym(18, 10, -33, 18, 10, 33, p.main);

    // Enormous stretched Tiger II sloped hull (50° glacis)
    fill_sloped_glacis_hull(
        &mut hull, &p, 10, 17, 15, // belly, sponson, roof half-widths
        2, 7, 11, 17, // belly, nose, sponson, deck Y
        -31, -38, -24, // belly front, bow nose, deck front Z
        38, 34, // sloped rear hull Z
    );

    // Interlocking bow nose armor weld + massive tow clevises
    for x in -10..=10 {
        hull.set(x, 7, -38, if x % 2 == 0 { p.dark } else { p.trim });
    }
    draw_tow_shackles(&mut hull, &p, 11, 7, -38, -1);

    // Front glacis details: bow MG34 right, central headlamp, driver periscope left
    draw_bow_mg(&mut hull, &p, 9, 13, -30);
    draw_headlight(&mut hull, &p, 0, 13, -30);
    hull.fill(-9, 18, -25, -6, 18, -24, p.dark);
    hull.set(-8, 18, -25, p.optic);
    hull.set(-7, 18, -25, p.optic);

    // Rear engine deck cooling fan grilles + twin heavy rear exhaust pipes
    draw_engine_grilles(&mut hull, &p, 12, 17, 14, 32);
    hull.fill_ellipse_y(-6.5, 23.0, 3.0, 3.0, 17, 17, p.black);
    hull.fill_ellipse_y(6.5, 23.0, 3.0, 3.0, 17, 17, p.black);
    hull.fill_sym(4, 10, 38, 6, 13, 38, p.dark);
    hull.fill_sym(5, 10, 39, 5, 15, 39, p.rust);
    hull.set_sym(5, 15, 39, p.black);
    draw_tow_shackles(&mut hull, &p, 10, 9, 38, 1);

    // TOWERING MASSIVE 250mm SLAB-SIDED ANGLED HEAVY CASEMATE SUPERSTRUCTURE (part of hull!)
    for y in 17..=27 {
        let hw = lerp_i32(y, 17, 27, 16, 13);
        let zf = lerp_i32(y, 17, 27, -20, -17);
        let zr = lerp_i32(y, 17, 27, 12, 11);
        hull.fill(-hw, y, zf, hw, y, zr, p.main);
    }
    // Crisp dark armor weld borders around massive 250mm front face & roof rim
    hull.fill(-13, 27, -17, 13, 27, -17, p.dark);
    hull.fill_sym(13, 27, -17, 13, 27, 11, p.dark);
    // Heavy twin rear casemate access doors on back wall (z = 11..=12)
    hull.fill(-6, 19, 12, 6, 25, 12, p.dark);
    hull.fill(0, 19, 12, 0, 25, 12, p.trim); // Center door seam

    // Crisp Balkenkreuz painted flush onto the massive sloped casemate flanks!
    draw_balkenkreuz_on_sloped_flanks(&mut hull, &p, 22, -1, 16);

    // Spare track links hung symmetrically along the sloped casemate flanks (front & rear racks)
    for &tz in &[-10, -8, -6, 4, 6, 8] {
        for y in 19..=25 {
            for x in (1..=16).rev() {
                if !hull.get(x, y, tz).is_air() {
                    let link = if y % 2 == 0 { p.track } else { p.rust };
                    hull.set_sym(x + 1, y, tz, link);
                    break;
                }
            }
        }
    }

    // Twin roof periscope hatches + commander/loader hatches + gunner sight hood on casemate roof
    draw_cupola(&mut hull, &p, -6.0, 3.0, 28, 2, 3.0);
    draw_cupola(&mut hull, &p, 6.0, 3.0, 28, 2, 2.8);
    // Front gunner telescopic periscope hood
    hull.fill(-5, 28, -13, -3, 29, -11, p.dark);
    hull.set(-4, 29, -13, p.optic);
    // Ventilator dome on roof center
    hull.fill_ellipse_y(0.0, -5.0, 1.8, 1.8, 28, 28, p.dark);

    // Carve clean CENTRAL FRONT EMBRASURE in the 250mm casemate front wall
    // Floor at y = 18, roof at y = 26 -> clear y = 19..=25, x = -5..=5, z = -21..=-14
    hull.fill(-5, 19, -21, 5, 25, -14, VoxelId::AIR);
    // Solid dark armored inner breech backing wall at z = -13 so zero daylight ever leaks
    hull.fill(-6, 18, -13, 6, 25, -13, p.dark);

    // TURRET CANVAS: ONLY the massive rounded heavy cast Jagdtiger mantlet!
    // Mounts at (0, 19, -16): y = 0..=6 maps to hull y = 19..=25 (flush, zero Z-fighting!)
    let mut turret = TankCanvas::new(6, 8, 11);
    // Inner swivel collar inside embrasure with generous ±10° yaw clearance
    turret.fill_cyl_z(0.0, 3.0, -1, 1, 3.2, p.dark);
    // Massive overlapping heavy cast oval shield sealing the embrasure (z_local = -2..-4)
    turret.fill(-5, 1, -4, 5, 5, -3, p.dark);
    turret.fill(-4, 0, -4, 4, 6, -2, p.dark);
    turret.fill_cyl_z(0.0, 3.0, -4, -3, 3.5, p.main);
    // Heavy tapered conical cast mantlet cradle projecting proudly forward (z_local = -5..-10)
    turret.fill_cyl_z(0.0, 3.0, -5, -5, 3.3, p.main);
    turret.fill_cyl_z(0.0, 3.0, -6, -6, 2.9, p.dark);
    turret.fill_cyl_z(0.0, 3.0, -7, -7, 2.5, p.dark);
    turret.fill_cyl_z(0.0, 3.0, -8, -8, 2.2, p.main);
    turret.fill_cyl_z(0.0, 3.0, -9, -9, 1.9, p.dark);
    turret.fill_cyl_z(0.0, 3.0, -10, -10, 1.6, p.trim);

    // COLOSSAL thick 128mm PaK 44 L/55 barrel: len 58, thick, NO muzzle brake (historically authentic!)
    let barrel = build_detailed_barrel(&p, 58, true, 8, MuzzleBrakeStyle::None);

    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 19.0, -16.0),
        Vec3::new(0.0, 3.5, -10.0),
        58.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialRegistry;
    use crate::mesher::mesh_voxel_model;

    #[test]
    fn all_eight_german_vehicles_build_cleanly_under_u16_vertex_budget() {
        let materials = MaterialRegistry::new();
        let ids = [
            "de_pz2c",
            "de_pz3j",
            "de_pz4_f2",
            "de_stug3g",
            "de_tiger_h1",
            "de_panther_a",
            "de_tiger2_h",
            "de_jagdtiger",
        ];

        for id in ids {
            let models = build_germany_vehicle(id)
                .unwrap_or_else(|| panic!("expected model for German vehicle {id}"));
            let hull_mesh = mesh_voxel_model(&models.hull, &materials);
            let turret_mesh = mesh_voxel_model(&models.turret, &materials);
            let barrel_mesh = mesh_voxel_model(&models.barrel, &materials);

            assert!(!hull_mesh.is_empty(), "{id} hull mesh empty");
            assert!(!turret_mesh.is_empty(), "{id} turret mesh empty");
            assert!(!barrel_mesh.is_empty(), "{id} barrel mesh empty");

            assert!(
                hull_mesh.positions.len() < u16::MAX as usize,
                "{id} hull vertices {} >= u16::MAX",
                hull_mesh.positions.len()
            );
            assert!(
                turret_mesh.positions.len() < u16::MAX as usize,
                "{id} turret vertices {} >= u16::MAX",
                turret_mesh.positions.len()
            );
            assert!(
                barrel_mesh.positions.len() < u16::MAX as usize,
                "{id} barrel vertices {} >= u16::MAX",
                barrel_mesh.positions.len()
            );
        }

        assert!(build_germany_vehicle("unknown_tank").is_none());
    }
}
