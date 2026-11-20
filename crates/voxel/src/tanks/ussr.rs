//! Bespoke high-detail `0.10 m` voxel models for all 8 Soviet armored vehicles (Tiers I–VIII).
//!
//! Vehicles implemented:
//! - Tier I Light:   `su_bt5`      (BT-5 — pointed Christie prow, 4 huge road wheels, cylindrical turret + rear bustle)
//! - Tier II Light:  `su_t50`      (T-50 — compact mini-T-34 sloped hull, tall cylindrical commander cupola)
//! - Tier III Med:   `su_t34_41`   (T-34 1941 — 60° glacis, wide driver hatch, 5 Christie wheels, pie-slice turret + F-34 chin)
//! - Tier IV Heavy:  `su_kv1_zis5` (KV-1 ZiS-5 — stepped bolted nose, fender bins, towering boxy welded turret + rear MG)
//! - Tier V Med:     `su_t34_85`   (T-34-85 — 60° glacis, huge rounded 3-man cast turret + cupola, long 85mm ZiS-S-53)
//! - Tier VI Heavy:  `su_is2`      (IS-2 — stepped cast heavy bow + mail-slot driver block, 122mm D-25T + DoubleBaffle)
//! - Tier VII Med:   `su_t44_100`  (T-44-100 — ultra-low clean 60° glacis, full anti-HEAT side skirts, roof DShK, 100mm D-10T)
//! - Tier VIII Heavy:`su_is3`      (IS-3 — 3D V-shaped Pike Nose, beveled side sponsons, ultra-low frying-pan dome, 122mm D-25T)

use glam::Vec3;

use crate::model::{MuzzleBrakeStyle, TankVoxelModels};

use super::{
    TankCanvas, TankPalette, assemble_tank, build_detailed_barrel, build_running_gear, draw_bow_mg,
    draw_cupola, draw_engine_grilles, draw_headlight, draw_red_star_flanks, draw_soviet_fuel_drums,
    draw_unditching_log,
};

/// Builds a bespoke high-detail `0.10 m` Soviet armored vehicle by catalog `vehicle_id`.
pub fn build_ussr_vehicle(vehicle_id: &str) -> Option<TankVoxelModels> {
    match vehicle_id {
        "su_bt5" => Some(build_bt5()),
        "su_t50" => Some(build_t50()),
        "su_t34_41" => Some(build_t34_41()),
        "su_kv1_zis5" => Some(build_kv1_zis5()),
        "su_t34_85" => Some(build_t34_85()),
        "su_is2" => Some(build_is2()),
        "su_t44_100" => Some(build_t44_100()),
        "su_is3" => Some(build_is3()),
        _ => None,
    }
}

/// Paints a crisp 5-voxel Soviet red star conformed flush onto the outer surface of sloped/curved
/// turret flanks at `(y = cy, z = cz)`, preventing buried or floating emblem voxels.
fn draw_red_star_surface(c: &mut TankCanvas, p: &TankPalette, cy: i32, cz: i32) {
    const STAR: [(i32, i32); 5] = [(0, 0), (1, 0), (-1, 0), (0, -1), (0, 1)];
    for (dy, dz) in STAR {
        let y = cy + dy;
        let z = cz + dz;
        for x in (1..=c.half_x).rev() {
            if !c.get(x, y, z).is_air() {
                c.set_sym(x, y, z, p.red);
                break;
            }
        }
    }
}

/// Draws symmetric heavy Soviet bow/stern tow hooks in dark steel trim.
fn draw_tow_hooks(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32) {
    c.set_sym(x, y, z, p.trim);
    c.set_sym(x, y - 1, z, p.trim);
}

/// Draws spare track links strapped horizontally across the lower glacis / bow beam.
fn draw_spare_tracks(c: &mut TankCanvas, p: &TankPalette, half_w: i32, y0: i32, y1: i32, z: i32) {
    for x in -half_w..=half_w {
        let id = if (x + half_w) % 2 == 0 {
            p.track
        } else {
            p.trim
        };
        for y in y0..=y1 {
            c.set(x, y, z, id);
        }
    }
}

/// Draws twin Soviet rear exhaust stubs / armored shrouds in rusty exhaust steel.
fn draw_rear_exhausts(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z0: i32, z1: i32) {
    c.fill_sym(x, y, z0, x + 1, y + 1, z1, p.rust);
    c.set_sym(x, y, z1 + 1, p.black);
}

/// Draws a raised circular/oval roof hatch with dark lid and steel hinge bar.
fn draw_roof_hatch(
    c: &mut TankCanvas,
    p: &TankPalette,
    cx: f32,
    cz: f32,
    y: i32,
    rx: f32,
    rz: f32,
) {
    c.fill_ellipse_y(cx, cz, rx, rz, y, y, p.dark);
    c.set(cx.round() as i32, y, (cz + rz).round() as i32, p.trim);
}

/// Draws a crisp 12.7mm DShK roof AA heavy machine gun on a pintle mount (iconic on T-44-100).
fn draw_dshk_aa_mg(c: &mut TankCanvas, p: &TankPalette, x: i32, y_base: i32, z_pivot: i32) {
    // Pintle pedestal post
    c.fill(x, y_base, z_pivot, x, y_base + 2, z_pivot, p.trim);
    // Receiver body & top cover
    let y_gun = y_base + 3;
    c.fill(x, y_gun - 1, z_pivot - 2, x, y_gun, z_pivot + 2, p.trim);
    // Side 12.7mm ammunition box on right of receiver
    c.fill(x + 1, y_gun - 1, z_pivot - 1, x + 2, y_gun, z_pivot + 1, p.dark);
    // Spade grips at rear
    c.set(x, y_gun - 1, z_pivot + 3, p.black);
    c.set(x, y_gun, z_pivot + 3, p.black);
    // Long finned heavy barrel pointing forward (-Z)
    c.fill(x, y_gun, z_pivot - 11, x, y_gun, z_pivot - 3, p.trim);
    // Signature flat DShK muzzle brake at tip
    c.fill(x - 1, y_gun, z_pivot - 12, x + 1, y_gun, z_pivot - 12, p.black);
}

/// Shared classic T-34 60° sloped hull builder (used by both T-34 1941 and T-34-85 with subtle variants).
fn build_t34_hull(p: &TankPalette, half_z: i32, late_model: bool) -> TankCanvas {
    let z_track_f = -(half_z - 3);
    let z_track_r = half_z - 3;
    let mut hull = TankCanvas::new(16, 17, half_z + 2);

    // 5 HUGE Christie road wheels (radius 3.2, no return rollers!)
    build_running_gear(
        &mut hull,
        p,
        10,
        14,
        z_track_f,
        z_track_r,
        8,
        5,
        3.2,
        0,
        9,
    );

    // Lower hull tub between tracks
    hull.fill(-9, 3, z_track_f + 4, 9, 8, z_track_r - 2, p.main);

    // Sloped upper hull sides (iconic 40° T-34 overhanging side armor above Christie wheels)
    let z_side_f = -(half_z - 13);
    let z_side_r = half_z - 7;
    for y in 9..=13 {
        let hw = 14 - (y - 9);
        hull.fill(-hw, y, z_side_f, hw, y, z_side_r, p.main);
    }

    // Crisp 60° flat sloped front glacis + sharp lower nose beak
    let z_nose = -half_z;
    for z in z_nose..=z_side_f {
        let t = (z - z_nose) as f32 / (z_side_f - z_nose) as f32;
        let y_top = (7.0 + t * 6.0).round() as i32;
        let y_bot = if z <= z_nose + 6 {
            (7.0 - (z - z_nose) as f32 * 0.67).round() as i32
        } else {
            3
        };
        let hw = if z < z_nose + 3 { 10 } else { 11 };
        hull.fill(-hw, y_bot.max(3), z, hw, y_top.min(13), z, p.main);
        // Side cheek fairing bridging glacis into sloped upper sides
        for y in 9..=y_top.min(13) {
            let side_hw = (14 - (y - 9)).min(hw + 3);
            if z > z_nose + 5 {
                hull.fill(-side_hw, y, z, side_hw, y, z, p.main);
            }
        }
    }

    // Welded nose beam highlight + spare track links + heavy front tow hooks
    hull.fill(-10, 7, z_nose, 10, 7, z_nose, p.dark);
    draw_spare_tracks(&mut hull, p, 5, 5, 6, z_nose + 1);
    draw_tow_hooks(&mut hull, p, 7, 7, z_nose - 1);

    // ICONIC wide rectangular driver hatch centered-left on the 60° glacis
    let z_hatch0 = z_nose + 4;
    let z_hatch1 = z_nose + 9;
    for z in z_hatch0..=z_hatch1 {
        let t = (z - z_nose) as f32 / (z_side_f - z_nose) as f32;
        let y_surf = (7.0 + t * 6.0).round() as i32;
        hull.fill(-6, y_surf, z, -1, y_surf + 1, z, p.dark);
    }
    // Driver hatch top hinges & dual armored periscope visor slits
    let z_vis = z_nose + 6;
    let y_vis = (7.0 + 6.0 * 6.0 / (z_side_f - z_nose) as f32).round() as i32;
    hull.fill(-5, y_vis + 1, z_vis - 1, -2, y_vis + 1, z_vis - 1, p.optic);
    hull.set(-6, y_vis + 2, z_hatch1, p.trim);
    hull.set(-1, y_vis + 2, z_hatch1, p.trim);

    // Bow DT machine gun ball mount on right glacis
    let z_mg = z_nose + 6;
    draw_bow_mg(&mut hull, p, 5, y_vis + 1, z_mg);

    // Single iconic left headlamp on glacis / hull shoulder
    let lamp_x = if late_model { -10 } else { -8 };
    let lamp_z = if late_model { z_nose + 8 } else { z_nose + 7 };
    draw_headlight(&mut hull, p, lamp_x, y_vis + 2, lamp_z);

    // Sloped rear armor plate (45° stern)
    for z in (z_side_r + 1)..=(half_z - 1) {
        let step = z - z_side_r;
        let y_top = (13 - step).max(7);
        let hw = (10 + (13 - y_top).min(2)).min(11);
        hull.fill(-hw, 5, z, hw, y_top, z, p.main);
    }
    // Circular rear transmission inspection plate
    hull.fill(-3, 8, half_z - 3, 3, 10, half_z - 2, p.dark);

    // Rear engine deck ventilation grilles & louvers
    draw_engine_grilles(&mut hull, p, 8, 13, 9, z_side_r - 2);
    // Raised center rear engine access hatch
    hull.fill(-2, 13, 10, 2, 13, z_side_r - 3, p.dark);

    // Twin Soviet external cylindrical fuel drums along rear hull flanks
    draw_soviet_fuel_drums(&mut hull, p, 13, 11, 12, z_side_r);

    // Twin rusty exhaust pipes emerging from rear plate
    draw_rear_exhausts(&mut hull, p, 5, 9, half_z - 3, half_z - 1);

    // Wooden Soviet unditching log strapped across the stern
    draw_unditching_log(&mut hull, p, 9, 8, half_z - 1);

    hull
}

// ============================================================================
// 1. TIER I LIGHT: BT-5 ("su_bt5")
// ============================================================================
fn build_bt5() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(12, 16, 29);

    // 4 HUGE solid Christie road wheels (radius 3.4!) + sleek fenders
    build_running_gear(&mut hull, p, 8, 10, -23, 23, 8, 4, 3.4, 0, 9);

    // Trim front fender slightly back so the sharp pointed Christie prow stands out head-on
    hull.fill_sym(8, 8, -25, 11, 9, -23, crate::material::VoxelId::AIR);

    // Lower hull tub
    hull.fill(-7, 3, -20, 7, 9, 23, p.main);

    // Sharp tapered pointed boat-like Christie prow (z = -28..=-14)
    for z in -28..=-14 {
        let t = (z - (-28)) as f32 / 14.0;
        let hw = (2.0 + t * 6.0).round() as i32;
        let y_top = (6.0 + t * 6.0).round() as i32;
        let y_bot = if z < -20 {
            (5.0 - (z + 28) as f32 * 0.28).round() as i32
        } else {
            3
        };
        hull.fill(-hw, y_bot.max(3), z, hw, y_top.min(12), z, p.main);
    }
    // Prow nose cap & tow shackles
    hull.fill(-2, 5, -28, 2, 6, -28, p.dark);
    draw_tow_hooks(&mut hull, p, 2, 6, -29);

    // Prominent driver visor hood centered on the pointed glacis
    hull.fill(-3, 9, -19, 3, 12, -14, p.dark);
    hull.fill(-2, 10, -20, 2, 11, -20, p.trim);
    hull.fill(-1, 10, -20, 1, 10, -20, p.optic);

    // Single left headlamp on prow cheek
    draw_headlight(&mut hull, p, -6, 10, -21);

    // Fighting compartment (z = -14..=7, roof at y = 12)
    hull.fill(-8, 9, -14, 8, 11, 7, p.main);
    hull.fill(-7, 12, -14, 7, 12, 7, p.main);

    // Slightly lower rear engine deck (z = 8..=24) with grilles & twin exhausts
    hull.fill(-7, 9, 8, 7, 11, 21, p.main);
    for z in 22..=26 {
        let y_top = (11 - (z - 21)).max(7);
        hull.fill(-7, 5, z, 7, y_top, z, p.main);
    }
    draw_engine_grilles(&mut hull, p, 6, 11, 10, 19);
    // Iconic external BT exhaust mufflers along rear hull sides
    hull.fill_sym(8, 10, 13, 9, 10, 22, p.rust);
    hull.set_sym(9, 10, 23, p.black);
    draw_tow_hooks(&mut hull, p, 5, 7, 26);

    // Forward-mounted small cylindrical BT-5 turret with prominent overhanging rear bustle
    let mut turret = TankCanvas::new(8, 11, 12);
    // Main cylindrical turret drum
    turret.fill_ellipse_y(0.0, -1.0, 5.8, 5.8, 0, 6, p.main);
    turret.fill_ellipse_y(0.0, -1.0, 5.2, 5.2, 7, 7, p.main);
    // Prominent overhanging boxy/tapered rear bustle
    turret.fill(-4, 1, 3, 4, 6, 9, p.main);
    turret.fill(-3, 2, 10, 3, 6, 10, p.main);
    turret.fill(-3, 7, 3, 3, 7, 8, p.main);

    // Narrow front mantlet + 45mm gun housing + coaxial DT MG
    turret.fill(-2, 1, -7, 2, 5, -5, p.dark);
    turret.set(2, 3, -8, p.trim);
    turret.set(-2, 4, -8, p.optic);

    // Dual top hatches
    turret.fill(-3, 8, -3, -1, 8, 1, p.dark);
    turret.fill(1, 8, -3, 3, 8, 1, p.dark);

    // Classic BT-5 horseshoe command aerial rail wrapping around turret sides & bustle
    turret.fill_sym(6, 7, -4, 6, 7, 7, p.trim);
    turret.fill(-5, 7, 8, 5, 7, 8, p.trim);
    // Vertical standoffs for the horseshoe rail
    turret.set_sym(6, 6, -3, p.trim);
    turret.set_sym(6, 6, 6, p.trim);

    // Red star on turret sides (cylindrical side sits cleanly at x = ±6, z = -1)
    draw_red_star_flanks(&mut turret, p, 6, 3, -1);

    let barrel = build_detailed_barrel(p, 21, false, 3, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 13.0, -6.0),
        Vec3::new(0.0, 3.5, -5.0),
        21.0,
    )
}

// ============================================================================
// 2. TIER II LIGHT: T-50 ("su_t50")
// ============================================================================
fn build_t50() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(13, 16, 27);

    // 6 small torsion road wheels + 3 return rollers
    build_running_gear(&mut hull, p, 8, 11, -23, 23, 7, 6, 2.0, 3, 8);

    // Lower hull tub
    hull.fill(-8, 3, -20, 8, 7, 22, p.main);

    // Clean sloped mini-T-34 upper hull sides (y = 8..=11)
    for y in 8..=11 {
        let hw = 11 - (y - 8);
        hull.fill(-hw, y, -14, hw, y, 19, p.main);
    }

    // Crisp sloped all-around front glacis (z = -25..=-14)
    for z in -25..=-14 {
        let t = (z + 25) as f32 / 11.0;
        let y_top = (6.0 + t * 5.0).round() as i32;
        let y_bot = if z < -20 {
            (6.0 - (z + 25) as f32 * 0.6).round() as i32
        } else {
            3
        };
        let hw = (8.0 + t * 2.0).round() as i32;
        hull.fill(-hw, y_bot.max(3), z, hw, y_top.min(11), z, p.main);
    }
    hull.fill(-8, 6, -25, 8, 6, -25, p.dark);
    draw_tow_hooks(&mut hull, p, 5, 6, -26);

    // Prominent bolted driver visor hatch right on the glacis
    hull.fill(-3, 8, -21, 3, 10, -17, p.dark);
    hull.fill(-2, 9, -22, 2, 9, -22, p.trim);
    hull.fill(-1, 9, -22, 1, 9, -22, p.optic);
    // Corner bolts on driver hatch
    hull.set_sym(3, 10, -21, p.trim);
    hull.set_sym(3, 8, -21, p.trim);

    // Single left headlamp
    draw_headlight(&mut hull, p, -8, 9, -22);

    // Fender stowage boxes
    hull.fill_sym(10, 9, 3, 11, 10, 14, p.dark);

    // Rear engine deck & sloped rear louver plate
    draw_engine_grilles(&mut hull, p, 6, 11, 9, 18);
    for z in 20..=25 {
        let y_top = (11 - (z - 19)).max(6);
        hull.fill(-8, 5, z, 8, y_top, z, p.main);
    }
    draw_rear_exhausts(&mut hull, p, 5, 8, 22, 24);
    draw_tow_hooks(&mut hull, p, 6, 7, 25);

    // Sloped compact hexagonal turret crowned by signature tall cylindrical commander cupola
    let mut turret = TankCanvas::new(8, 13, 11);
    for y in 0..=6 {
        let inset = (y as f32) * 0.32;
        for z in -7..=8 {
            let base_hw = if z < -2 {
                3.5 + ((z + 7) as f32 / 5.0) * 3.5
            } else if z <= 2 {
                7.0
            } else {
                7.0 - ((z - 2) as f32 / 6.0) * 3.2
            };
            let hw = (base_hw - inset).round() as i32;
            if hw >= 1 {
                turret.fill(-hw, y, z, hw, y, z, p.main);
            }
        }
    }

    // Curved front gun mantlet + coaxial MG + gunner optic
    turret.fill_cyl_x(-3, 3, 3.2, -7.2, 1.8, p.dark);
    turret.set(2, 3, -9, p.trim);
    turret.set(-2, 4, -9, p.optic);

    // SIGNATURE tall cylindrical armored commander cupola with 360° vision ports
    draw_cupola(&mut turret, p, 0.0, 1.0, 7, 4, 2.8);

    // Red star cleanly conformed onto sloped hexagonal flanks
    draw_red_star_surface(&mut turret, p, 3, 0);

    let barrel = build_detailed_barrel(p, 22, false, 3, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 12.0, -4.0),
        Vec3::new(0.0, 3.5, -6.0),
        22.0,
    )
}

// ============================================================================
// 3. TIER III MEDIUM: T-34 (1941) ("su_t34_41")
// ============================================================================
fn build_t34_41() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let hull = build_t34_hull(p, 29, false);

    // Forward-mounted compact sloped hexagonal "pie-slice" welded turret (1941)
    let mut turret = TankCanvas::new(9, 11, 13);
    for y in 0..=7 {
        let inset = (y as f32) * 0.30;
        for z in -8..=10 {
            let base_hw = if z < -2 {
                4.2 + ((z + 8) as f32 / 6.0) * 3.8
            } else if z <= 2 {
                8.0
            } else {
                8.0 - ((z - 2) as f32 / 8.0) * 4.2
            };
            let hw = (base_hw - inset).round() as i32;
            if hw >= 1 {
                turret.fill(-hw, y, z, hw, y, z, p.main);
            }
        }
    }

    // Iconic boxy protruding F-34 recoil chin & curved mantlet under barrel
    turret.fill(-3, 2, -10, 3, 5, -8, p.dark);
    // Protruding boxy lower recoil guard ("chin")
    turret.fill(-2, 1, -11, 2, 2, -9, p.dark);
    turret.set(-2, 4, -10, p.optic);
    turret.set(2, 3, -11, p.trim);

    // Single large rectangular forward-opening roof hatch (1941 signature, no cupola!)
    turret.fill(-4, 8, -3, 4, 8, 3, p.dark);
    turret.fill(-3, 8, 4, 3, 8, 4, p.trim);
    // Commander's PT-4-7 periscope hood forward-left
    turret.fill(-3, 9, -4, -1, 9, -2, p.trim);
    turret.set(-2, 9, -5, p.optic);

    // Rear turret pistol port & lifting hooks
    turret.set_sym(3, 6, 8, p.trim);
    turret.set(0, 4, 10, p.trim);

    // Crisp red star on sloped pie-slice turret flanks
    draw_red_star_surface(&mut turret, p, 3, 0);

    let barrel = build_detailed_barrel(p, 31, false, 4, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -7.0),
        Vec3::new(0.0, 3.5, -8.0),
        31.0,
    )
}

// ============================================================================
// 4. TIER IV HEAVY: KV-1 (ZiS-5) ("su_kv1_zis5")
// ============================================================================
fn build_kv1_zis5() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(17, 16, 35);

    // 6 heavy cast road wheels + 3 return rollers
    build_running_gear(&mut hull, p, 11, 15, -31, 31, 8, 6, 2.5, 3, 9);

    // Massive slab-sided wide rectangular heavy hull
    hull.fill(-10, 3, -21, 10, 12, 26, p.main);

    // ICONIC KV-1 STEPPED BOLTED BOW:
    // 1. Angled lower cast nose (z = -32..=-27)
    for z in -32..=-27 {
        let t = (z + 32) as f32 / 5.0;
        let y_bot = (7.0 - t * 4.0).round() as i32;
        hull.fill(-10, y_bot.max(3), z, 10, 8, z, p.main);
    }
    hull.fill(-10, 7, -32, 10, 8, -32, p.dark);
    draw_tow_hooks(&mut hull, p, 7, 7, -33);

    // 2. Distinct horizontal/gently-sloped shelf step between lower nose & driver plate!
    hull.fill(-10, 3, -27, 10, 8, -22, p.main);
    hull.fill(-10, 9, -26, 10, 9, -22, p.main);
    draw_spare_tracks(&mut hull, p, 6, 9, 9, -28);

    // 3. Steep vertical bolted 75+25mm appliqué upper front breastplate (z = -22..=-20)
    hull.fill(-10, 9, -22, 10, 12, -20, p.dark);
    // Heavy steel appliqué bolts across upper front plate
    for x in [-9, -5, 5, 9] {
        hull.set(x, 12, -23, p.trim);
        hull.set(x, 9, -23, p.trim);
    }
    // Central armored driver visor slit with brow
    hull.fill(-2, 10, -23, 2, 12, -23, p.dark);
    hull.fill(-1, 11, -23, 1, 11, -23, p.optic);
    // Right bow DT machine gun ball
    draw_bow_mg(&mut hull, p, 6, 11, -23);
    // Left round headlamp mounted on left front shelf/fender
    draw_headlight(&mut hull, p, -8, 10, -29);

    // Wide mudguard stowage boxes along both fenders (classic KV-1 heavy fender bins!)
    hull.fill_sym(12, 10, -14, 15, 11, 16, p.dark);
    for z in [-10, -2, 6, 14] {
        hull.fill_sym(12, 10, z, 15, 11, z, p.trim);
    }

    // Rear engine deck ventilation grilles & rounded stern plate
    draw_engine_grilles(&mut hull, p, 8, 12, 10, 23);
    for z in 27..=32 {
        let step = z - 26;
        let y_top = (12 - step).max(6);
        hull.fill(-10, 5, z, 10, y_top, z, p.main);
    }
    draw_rear_exhausts(&mut hull, p, 6, 12, 23, 27);
    draw_unditching_log(&mut hull, p, 10, 8, 32);

    // TOWERING TALL BOXY WELDED HEAVY KV-1 (ZiS-5) TURRET
    let mut turret = TankCanvas::new(11, 14, 17);
    for y in 0..=10 {
        // Very slight upward taper to keep the unmistakable slab-sided tall boxy silhouette
        let inset = if y >= 7 { 1 } else { 0 };
        // Front-to-mid massive angular box (z = -10..=5)
        turret.fill(-(8 - inset), y, -10, 8 - inset, y, 5, p.main);
        // Elongated boxy bustle with rounded stern (z = 6..=14)
        for z in 6..=14 {
            let t = (z - 5) as f32 / 9.0;
            let hw = ((8.0 - inset as f32) - t * 2.8).round() as i32;
            turret.fill(-hw, y, z, hw, y, z, p.main);
        }
    }
    // Thick bolted 25mm appliqué armor slabs on turret flanks
    turret.fill_sym(9, 1, -8, 9, 6, 6, p.dark);
    for z in [-7, 0, 5] {
        turret.set_sym(9, 6, z, p.trim);
        turret.set_sym(9, 1, z, p.trim);
    }

    // MASSIVE wide bolted rectangular ZiS-5 gun shield / mantlet
    turret.fill(-5, 2, -12, 5, 8, -10, p.dark);
    turret.set_sym(4, 7, -13, p.trim);
    turret.set_sym(4, 3, -13, p.trim);
    // Coaxial MG & gunner sight opening
    turret.set(3, 5, -13, p.trim);
    turret.set(-3, 6, -13, p.optic);

    // Iconic rear turret DT machine gun ball nub pointing rearward (+Z)
    turret.fill(-1, 4, 14, 1, 6, 15, p.dark);
    turret.set(0, 5, 16, p.trim);

    // Heavy circular roof hatch + twin armored periscopes
    draw_roof_hatch(&mut turret, p, 0.0, 1.0, 11, 3.0, 3.0);
    turret.fill_sym(3, 11, -5, 4, 12, -4, p.trim);
    turret.set_sym(3, 12, -5, p.optic);

    // Crisp red star on the tall flat turret flank slabs (outer appliqué sits at x = ±9)
    draw_red_star_flanks(&mut turret, p, 9, 4, -1);

    let barrel = build_detailed_barrel(p, 32, false, 4, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 13.0, -5.0),
        Vec3::new(0.0, 5.5, -10.0),
        32.0,
    )
}

// ============================================================================
// 5. TIER V MEDIUM: T-34-85 ("su_t34_85")
// ============================================================================
fn build_t34_85() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let hull = build_t34_hull(p, 30, true);

    // HUGE WIDE ROUNDED CAST 3-MAN T-34-85 TURRET
    let mut turret = TankCanvas::new(12, 15, 16);
    for y in 0..=9 {
        let ny = y as f32 / 9.5;
        // Bulging cast cheeks near lower/mid wall, smoothly rounding inward to roof
        let bulge = 1.0 - (ny * ny * 0.28);
        let rx = 10.6 * bulge;
        let rz = 12.2 * bulge;
        let cz = 1.5;
        for z in -10..=14 {
            for x in -11..=11 {
                let dx = x as f32 / rx;
                let dz = (z as f32 - cz) / rz;
                // Slightly fuller cast rear bustle & slightly narrower front nose
                let front_pinch = if z < -3 {
                    1.0 + ((-3 - z) as f32 * 0.035)
                } else {
                    1.0
                };
                if (dx * front_pinch).powi(2) + dz * dz <= 1.0 {
                    turret.set(x, y, z, p.main);
                }
            }
        }
    }

    // Wide curved cast 85mm gun mantlet
    turret.fill_cyl_x(-4, 4, 4.5, -10.2, 2.3, p.dark);
    turret.set(3, 4, -13, p.trim);
    turret.set(-3, 5, -12, p.optic);

    // Prominent commander's cupola on left roof + loader's oval hatch on right
    draw_cupola(&mut turret, p, -3.5, 2.5, 10, 3, 2.6);
    draw_roof_hatch(&mut turret, p, 3.5, 1.5, 10, 2.3, 2.5);

    // Signature twin mushroom ventilator domes at turret rear roof
    turret.fill_sym(2, 10, 8, 3, 10, 9, p.dark);
    // Turret cheek grab handrails
    turret.fill_sym(10, 6, 3, 10, 6, 7, p.trim);

    // Crisp red star on the huge rounded cast turret flanks
    draw_red_star_surface(&mut turret, p, 4, 1);

    // Long 85mm ZiS-S-53 barrel (len 40, thin, None)
    let barrel = build_detailed_barrel(p, 40, false, 5, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -5.0),
        Vec3::new(0.0, 4.5, -9.0),
        40.0,
    )
}

// ============================================================================
// 6. TIER VI HEAVY: IS-2 ("su_is2")
// ============================================================================
fn build_is2() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(16, 17, 35);

    // 6 heavy cast road wheels + 3 return rollers
    build_running_gear(&mut hull, p, 10, 14, -31, 31, 8, 6, 2.4, 3, 9);

    // Massive heavy hull tub & slightly tapered cast upper shoulders
    hull.fill(-10, 3, -24, 10, 10, 26, p.main);
    for y in 11..=13 {
        let hw = 10 - (y - 11) / 2;
        hull.fill(-hw, y, -18, hw, y, 26, p.main);
    }

    // ICONIC IS-2 STEPPED HEAVY BOW:
    // 1. Cast angled lower bow tapering slightly inward toward the nose tip (z = -33..=-24)
    for z in -33..=-24 {
        let t = (z + 33) as f32 / 9.0;
        let hw = (7.5 + t * 2.5).round() as i32;
        let y_bot = if z < -27 {
            (7.0 - (z + 33) as f32 * 0.67).round() as i32
        } else {
            3
        };
        let y_top = (8.0 + t * 2.0).round() as i32;
        hull.fill(-hw, y_bot.max(3), z, hw, y_top, z, p.main);
    }
    hull.fill(-7, 7, -33, 7, 8, -33, p.dark);
    draw_spare_tracks(&mut hull, p, 5, 5, 6, -32);
    draw_tow_hooks(&mut hull, p, 6, 7, -34);

    // 2. Steep vertical thick armored driver visor block with recessed mail-slot optic!
    hull.fill(-6, 10, -24, 6, 13, -18, p.main);
    // Cast cheek transition shoulders wrapping cleanly behind driver block
    for z in -23..=-18 {
        let hw = 6 + ((z + 23) * 4 / 5);
        hull.fill(-hw, 10, z, hw, 12, z, p.main);
    }
    // Recessed horizontal mail-slot driver vision port right in the center of the vertical visor
    hull.fill(-3, 11, -24, 3, 12, -24, p.dark);
    hull.fill(-2, 11, -25, 2, 11, -25, p.black);
    hull.fill(-1, 11, -25, 1, 11, -25, p.optic);
    // Armored brow lip directly over mail-slot
    hull.fill(-3, 12, -25, 3, 12, -25, p.trim);

    // Single round left headlamp on left glacis cheek
    draw_headlight(&mut hull, p, -8, 11, -26);

    // Rear engine deck grilles & sloped heavy rear stern
    draw_engine_grilles(&mut hull, p, 8, 13, 10, 23);
    for z in 27..=32 {
        let step = z - 26;
        let y_top = (13 - step).max(6);
        hull.fill(-9, 5, z, 9, y_top, z, p.main);
    }
    draw_rear_exhausts(&mut hull, p, 6, 12, 23, 28);

    // Twin cylindrical rear external fuel drums along rear fenders + wooden unditching log
    draw_soviet_fuel_drums(&mut hull, p, 13, 11, 12, 24);
    draw_unditching_log(&mut hull, p, 9, 8, 32);

    // Elongated rounded narrow-cheeked cast IS-2 heavy turret
    let mut turret = TankCanvas::new(12, 15, 17);
    for y in 0..=9 {
        let ny = y as f32 / 9.5;
        let scale = 1.0 - ny * ny * 0.26;
        let cz = 1.5;
        let rz = 13.0 * scale;
        for z in -11..=15 {
            // Narrower front cheeks widening smoothly toward rear counterweight bustle
            let width_profile = if z < 0 {
                7.2 + ((z + 11) as f32 / 11.0) * 3.0
            } else {
                10.2
            };
            let rx = width_profile * scale;
            let dz = (z as f32 - cz) / rz;
            for x in -11..=11 {
                let dx = x as f32 / rx;
                if dx * dx + dz * dz <= 1.0 {
                    turret.set(x, y, z, p.main);
                }
            }
        }
    }

    // Thick curved cast armored 122mm gun mantlet
    turret.fill_cyl_x(-4, 4, 4.5, -11.0, 2.8, p.dark);
    turret.set(3, 4, -14, p.trim);
    turret.set(-3, 5, -14, p.optic);

    // Commander cupola on left + loader hatch on right
    draw_cupola(&mut turret, p, -3.5, 2.0, 10, 3, 2.6);
    draw_roof_hatch(&mut turret, p, 3.5, 2.0, 10, 2.2, 2.3);

    // Rear turret DT machine gun ball mount
    turret.fill(-1, 4, 14, 1, 6, 15, p.dark);
    turret.set(0, 5, 16, p.trim);

    // Crisp red star on elongated cast turret flanks
    draw_red_star_surface(&mut turret, p, 4, 2);

    // MONSTROUS long thick 122mm D-25T cannon with DoubleBaffle muzzle brake!
    let barrel = build_detailed_barrel(p, 48, true, 6, MuzzleBrakeStyle::DoubleBaffle);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 14.0, -5.0),
        Vec3::new(0.0, 4.5, -9.0),
        48.0,
    )
}

// ============================================================================
// 7. TIER VII MEDIUM: T-44-100 ("su_t44_100") — USER FAVORITE!
// ============================================================================
fn build_t44_100() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(17, 15, 32);

    // 5 large starfish torsion road wheels (NO return rollers, ultra-low track profile!)
    build_running_gear(&mut hull, p, 10, 14, -28, 28, 7, 5, 2.9, 0, 8);

    // Ultra-low, wide, sleek hull tub & upper superstructure (roof at only y = 11!)
    hull.fill(-10, 2, -23, 10, 7, 25, p.main);
    hull.fill(-11, 8, -16, 11, 11, 26, p.main);

    // CLEAN UNINTERRUPTED FLAT 60° GLACIS (NO driver hatch bulge, NO bow MG!)
    for z in -30..=-16 {
        let t = (z + 30) as f32 / 14.0;
        let y_top = (6.0 + t * 5.0).round() as i32;
        let y_bot = if z < -24 {
            (6.0 - (z + 30) as f32 * 0.67).round() as i32
        } else {
            2
        };
        hull.fill(-11, y_bot.max(2), z, 11, y_top.min(11), z, p.main);
    }
    // Clean welded lower nose edge + spare track links + tow hooks
    hull.fill(-11, 6, -30, 11, 6, -30, p.dark);
    draw_spare_tracks(&mut hull, p, 6, 4, 5, -29);
    draw_tow_hooks(&mut hull, p, 8, 6, -31);

    // Flush recessed driver periscope slot (zero bulge!) & flat driver roof hatch
    hull.fill(-6, 10, -20, -4, 10, -19, p.black);
    hull.set(-5, 10, -20, p.optic);
    hull.fill(-7, 11, -15, -3, 11, -12, p.dark);

    // Single left headlamp with dark armored guard on left front fender
    draw_headlight(&mut hull, p, -12, 9, -28);
    hull.set(-13, 9, -28, p.trim);
    hull.set(-12, 10, -28, p.trim);

    // SIGNATURE FULL-LENGTH 6mm ANTI-HEAT STEEL SIDE SKIRTS along BOTH flanks!
    // Covers upper half of tracks & 5 large road wheels (y = 4..=8) at x = ±16
    hull.fill_sym(16, 4, -27, 16, 8, 27, p.main);
    // Front & rear angled skirt corner steps
    hull.fill_sym(16, 5, -28, 16, 8, -28, p.main);
    hull.fill_sym(16, 5, 28, 16, 8, 28, p.main);
    // Top steel hinge rail & crisp vertical panel seams between the 6mm skirt plates
    hull.fill_sym(16, 8, -28, 16, 8, 28, p.trim);
    for z in [-20, -12, -4, 4, 12, 20] {
        hull.fill_sym(16, 4, z, 16, 7, z, p.dark);
    }
    // Lower skirt bottom edge trim
    hull.fill_sym(16, 4, -27, 16, 4, 27, p.dark);

    // Ultra-low flat rear engine deck grilles, external rear fuel drums & wooden unditching log
    draw_engine_grilles(&mut hull, p, 9, 11, 11, 23);
    draw_soviet_fuel_drums(&mut hull, p, 13, 10, 14, 25);
    for z in 27..=30 {
        let y_top = (11 - (z - 26)).max(6);
        hull.fill(-11, 4, z, 11, y_top, z, p.main);
    }
    draw_unditching_log(&mut hull, p, 10, 7, 29);

    // Centrally mounted sleek rounded cast T-44-100 turret + 12.7mm DShK roof AA MG!
    let mut turret = TankCanvas::new(12, 16, 16);
    for y in 0..=8 {
        let ny = y as f32 / 8.6;
        let scale = 1.0 - ny * ny * 0.30;
        let rx = 10.4 * scale;
        let rz = 12.0 * scale;
        let cz = 1.0;
        for z in -10..=13 {
            let front_pinch = if z < -3 {
                1.0 + ((-3 - z) as f32 * 0.032)
            } else {
                1.0
            };
            let dz = (z as f32 - cz) / rz;
            for x in -11..=11 {
                let dx = (x as f32 / rx) * front_pinch;
                if dx * dx + dz * dz <= 1.0 {
                    turret.set(x, y, z, p.main);
                }
            }
        }
    }

    // Thick recessed cast mantlet for the 100mm D-10T
    turret.fill_cyl_x(-4, 4, 4.0, -10.5, 2.5, p.dark);
    turret.set(3, 4, -13, p.trim);
    turret.set(-3, 5, -13, p.optic);

    // Low sleek post-war commander cupola on left + loader hatch on right
    draw_cupola(&mut turret, p, -3.5, 2.0, 9, 2, 2.5);
    draw_roof_hatch(&mut turret, p, 3.5, 1.5, 9, 2.2, 2.2);

    // SIGNATURE 12.7mm DShK roof AA machine gun mounted on the turret roof!
    draw_dshk_aa_mg(&mut turret, p, 3, 10, 2);

    // Turret cheek grab rails & rear radio antenna stub
    turret.fill_sym(9, 5, 3, 9, 5, 7, p.trim);
    turret.fill(-5, 9, 7, -5, 14, 7, p.trim);

    // Crisp red star on sleek rounded turret flanks
    draw_red_star_surface(&mut turret, p, 4, 1);

    // Super-long thick 100mm D-10T cannon (len 48, thick, MuzzleBrakeStyle::None — authentic!)
    let barrel = build_detailed_barrel(p, 48, true, 6, MuzzleBrakeStyle::None);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 12.0, -1.0),
        Vec3::new(0.0, 4.0, -9.0),
        48.0,
    )
}

// ============================================================================
// 8. TIER VIII HEAVY: IS-3 ("su_is3") — 3D PIKE NOSE + FRYING-PAN DOME!
// ============================================================================
fn build_is3() -> TankVoxelModels {
    let p = &TankPalette::USSR;
    let mut hull = TankCanvas::new(17, 15, 36);

    // 6 heavy cast road wheels + 3 return rollers
    build_running_gear(&mut hull, p, 11, 15, -30, 31, 8, 6, 2.4, 3, 9);

    // Lower heavy hull tub between tracks
    hull.fill(-10, 3, -24, 10, 8, 27, p.main);

    // STEEPLY BEVELED SPACED-ARMOR UPPER HULL SIDE SPONSONS (y = 9..=11)
    // Wide beveled shoulder overhanging the tracks (quintessential IS-3 hull profile!)
    hull.fill(-15, 9, -23, 15, 9, 27, p.main);
    hull.fill(-14, 10, -22, 14, 10, 27, p.main);
    hull.fill(-12, 11, -20, 12, 11, 27, p.main);
    // Crisp dark edge accent along the lower spaced-armor sponson chine
    hull.fill_sym(15, 9, -23, 15, 9, 27, p.dark);

    // ICONIC 3D V-SHAPED PIKE NOSE (Щучий нос)!
    // Two sharply angled glacis cheeks sloping BOTH backward (+Z) AND outward (±X),
    // meeting at a razor-sharp central prow ridge right down x = 0!
    for z in -35_i32..=-14 {
        for x in -14_i32..=14 {
            let ax = x.abs() as f32;
            // Razor-sharp V-shaped leading edge: tip at (x=0, z=-35), swept back at ~36°
            let z_edge = -35.0 + ax * 0.72;
            if (z as f32) < z_edge {
                continue;
            }
            let dist_back = z as f32 - z_edge;
            let y_top = (6.5 + dist_back * 0.42).min(11.0).round() as i32;
            let y_bot = (6.0 - dist_back * 0.55).max(3.0).round() as i32;
            // Outer cheek clip so pike blends cleanly into beveled side sponsons
            let max_x_for_y = |y: i32| -> i32 {
                if y <= 8 {
                    10
                } else if y == 9 {
                    15
                } else if y == 10 {
                    14
                } else {
                    12
                }
            };
            for y in y_bot..=y_top {
                if x.abs() <= max_x_for_y(y) {
                    let is_center_ridge = x == 0 && y == y_top && z <= -24;
                    let is_beak_edge = dist_back < 1.1 && y == 6;
                    let id = if is_center_ridge || is_beak_edge {
                        p.dark
                    } else {
                        p.main
                    };
                    hull.set(x, y, z, id);
                }
            }
        }
    }

    // Heavy bow tow hooks mounted symmetrically on the lower pike cheeks
    draw_tow_hooks(&mut hull, p, 5, 6, -32);

    // Flush armored driver's triplex periscope block right behind the pike apex
    hull.fill(-2, 11, -22, 2, 12, -19, p.dark);
    hull.fill(-1, 12, -23, 1, 12, -23, p.optic);

    // Single left headlamp on left front fender
    draw_headlight(&mut hull, p, -12, 10, -29);

    // Ultra-low flat rear engine deck, twin huge external rear fuel drums & wooden unditching log
    draw_engine_grilles(&mut hull, p, 9, 11, 11, 24);
    draw_soviet_fuel_drums(&mut hull, p, 14, 11, 14, 26);
    for z in 28..=33 {
        let step = z - 27;
        let y_top = (11 - step).max(6);
        let hw = (12 - step / 2).max(9);
        hull.fill(-hw, 5, z, hw, y_top, z, p.main);
    }
    draw_unditching_log(&mut hull, p, 10, 7, 32);

    // ICONIC ULTRA-LOW FLATTENED "FRYING-PAN" HEMISPHERICAL CAST DOME TURRET!
    let mut turret = TankCanvas::new(14, 10, 16);
    for y in 0..=7 {
        let ny = (y as f32 + 0.25) / 7.5;
        // Super-flattened hemispherical frying-pan dome curve
        let scale = (1.0 - ny.powf(2.4)).max(0.0).sqrt();
        let rx = 13.0 * scale;
        let rz = 14.4 * scale;
        let cz = 0.8;
        for z in -14..=15 {
            let dz = (z as f32 - cz) / rz.max(0.5);
            // Slightly narrower front mantlet taper for authentic IS-3 teardrop frying-pan planform
            let front_taper = if z < -5 {
                1.0 + ((-5 - z) as f32 * 0.025)
            } else {
                1.0
            };
            for x in -13..=13 {
                let dx = (x as f32 / rx.max(0.5)) * front_taper;
                if dx * dx + dz * dz <= 1.0 {
                    turret.set(x, y, z, p.main);
                }
            }
        }
    }

    // Recessed cast 122mm gun mantlet blended into the frying-pan front dome
    turret.fill_cyl_x(-3, 3, 3.0, -13.0, 2.2, p.dark);
    turret.set(3, 3, -15, p.trim);
    turret.set(-3, 4, -14, p.optic);

    // Dual flush low-profile roof hatches (NO tall cupola — ultra-low predatory silhouette!)
    draw_roof_hatch(&mut turret, p, -3.2, 0.5, 7, 2.4, 2.4);
    draw_roof_hatch(&mut turret, p, 3.2, 0.5, 7, 2.2, 2.2);
    // Low commander's periscope ring lip (just 1 voxel high!)
    turret.fill(-4, 8, -1, -2, 8, 1, p.dark);
    turret.set(-3, 8, -2, p.optic);
    // Rear dome ventilator & turret grab rails
    turret.fill(-1, 7, 6, 1, 7, 7, p.trim);
    turret.fill_sym(11, 4, 2, 11, 4, 6, p.trim);

    // Crisp red star conformed onto the curved frying-pan dome flanks
    draw_red_star_surface(&mut turret, p, 3, 0);

    // MONSTROUS thick 122mm D-25T cannon (len 50, thick, DoubleBaffle muzzle brake!)
    let barrel = build_detailed_barrel(p, 50, true, 6, MuzzleBrakeStyle::DoubleBaffle);
    assemble_tank(
        hull,
        turret,
        barrel,
        Vec3::new(0.0, 12.0, -4.0),
        Vec3::new(0.0, 3.5, -11.0),
        50.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaterialRegistry, mesh_voxel_model};

    #[test]
    fn all_eight_ussr_tanks_build_and_mesh_within_vertex_budget() {
        let registry = MaterialRegistry::new();
        let ids = [
            "su_bt5",
            "su_t50",
            "su_t34_41",
            "su_kv1_zis5",
            "su_t34_85",
            "su_is2",
            "su_t44_100",
            "su_is3",
        ];

        for id in ids {
            let models = build_ussr_vehicle(id)
                .unwrap_or_else(|| panic!("expected USSR vehicle {id} to build"));
            for (part, model) in [
                ("hull", &models.hull),
                ("turret", &models.turret),
                ("barrel", &models.barrel),
            ] {
                assert!(
                    model.non_air_count() > 20,
                    "{id} {part} has too few voxels"
                );
                let mesh = mesh_voxel_model(model, &registry);
                assert!(!mesh.is_empty(), "{id} {part} produced empty mesh");
                assert!(
                    mesh.positions.len() < u16::MAX as usize,
                    "{id} {part} exceeded u16::MAX vertices: {}",
                    mesh.positions.len()
                );
            }
        }
    }
}
