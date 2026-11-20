//! High-detail `0.10 m` bespoke voxel vehicle sculptors for all 24 Stage 5 tanks
//! (8 USA, 8 Germany, 8 USSR), engineered for unmistakable frontal, side, and rear silhouettes.

pub mod germany;
pub mod usa;
pub mod ussr;

use glam::{IVec3, UVec3, Vec3};

use crate::material::VoxelId;
use crate::model::{MuzzleBrakeStyle, TankVoxelModels, VoxelModel};

/// High-resolution tank voxel size (`0.10 m` = 10 cm voxels, 8x finer detail than `0.20 m`).
pub const DETAIL_VOXEL_SIZE: f32 = 0.10;

/// National armor paint palette baked directly into tank voxels so white/red emblems,
/// headlights, exhausts, wooden logs, and dark steel trim render with true un-tinted colors.
#[derive(Copy, Clone, Debug)]
pub struct TankPalette {
    pub main: VoxelId,
    pub dark: VoxelId,
    pub gun: VoxelId,
    pub track: VoxelId,
    pub wheel: VoxelId,
    pub trim: VoxelId,
    pub optic: VoxelId,
    pub light: VoxelId,
    pub rust: VoxelId,
    pub wood: VoxelId,
    pub white: VoxelId,
    pub red: VoxelId,
    pub black: VoxelId,
}

impl TankPalette {
    pub const USA: Self = Self {
        main: VoxelId::ARMOR_US,
        dark: VoxelId::ARMOR_US_DARK,
        gun: VoxelId::ARMOR_US_DARK,
        track: VoxelId::STEEL_TRACK,
        wheel: VoxelId::RUBBER_WHEEL,
        trim: VoxelId::STEEL_DARK_TRIM,
        optic: VoxelId::GLASS_OPTIC,
        light: VoxelId::HEADLIGHT_GLASS,
        rust: VoxelId::EXHAUST_RUST,
        wood: VoxelId::WOOD_TOOL,
        white: VoxelId::EMBLEM_WHITE,
        red: VoxelId::EMBLEM_RED,
        black: VoxelId::EMBLEM_BLACK,
    };

    pub const GERMANY_GREY: Self = Self {
        main: VoxelId::ARMOR_DE_GREY,
        dark: VoxelId::ARMOR_DE_GREY_DARK,
        gun: VoxelId::STEEL_DARK_TRIM,
        track: VoxelId::STEEL_TRACK,
        wheel: VoxelId::RUBBER_WHEEL,
        trim: VoxelId::STEEL_DARK_TRIM,
        optic: VoxelId::GLASS_OPTIC,
        light: VoxelId::HEADLIGHT_GLASS,
        rust: VoxelId::EXHAUST_RUST,
        wood: VoxelId::WOOD_TOOL,
        white: VoxelId::EMBLEM_WHITE,
        red: VoxelId::EMBLEM_RED,
        black: VoxelId::EMBLEM_BLACK,
    };

    pub const GERMANY_TAN: Self = Self {
        main: VoxelId::ARMOR_DE_TAN,
        dark: VoxelId::ARMOR_DE_TAN_DARK,
        gun: VoxelId::ARMOR_DE_TAN_DARK,
        track: VoxelId::STEEL_TRACK,
        wheel: VoxelId::RUBBER_WHEEL,
        trim: VoxelId::STEEL_DARK_TRIM,
        optic: VoxelId::GLASS_OPTIC,
        light: VoxelId::HEADLIGHT_GLASS,
        rust: VoxelId::EXHAUST_RUST,
        wood: VoxelId::WOOD_TOOL,
        white: VoxelId::EMBLEM_WHITE,
        red: VoxelId::EMBLEM_RED,
        black: VoxelId::EMBLEM_BLACK,
    };

    pub const USSR: Self = Self {
        main: VoxelId::ARMOR_SU,
        dark: VoxelId::ARMOR_SU_DARK,
        gun: VoxelId::ARMOR_SU_DARK,
        track: VoxelId::STEEL_TRACK,
        wheel: VoxelId::RUBBER_WHEEL,
        trim: VoxelId::STEEL_DARK_TRIM,
        optic: VoxelId::GLASS_OPTIC,
        light: VoxelId::HEADLIGHT_GLASS,
        rust: VoxelId::EXHAUST_RUST,
        wood: VoxelId::WOOD_TOOL,
        white: VoxelId::EMBLEM_WHITE,
        red: VoxelId::EMBLEM_RED,
        black: VoxelId::EMBLEM_BLACK,
    };
}

/// Ergonomic signed-coordinate voxel canvas for high-detail vehicle sculpting.
///
/// Coordinate convention (exact match with simulation & renderer):
/// - `x`: `-` = Left, `+` = Right, `0` = centerline
/// - `y`: `0` = bottom (ground for hull, turret ring for turret), `+` = Up
/// - `z`: `-` = FRONT (bow / gun direction), `+` = REAR (engine deck)
#[derive(Clone, Debug)]
pub struct TankCanvas {
    pub half_x: i32,
    pub max_y: i32,
    pub half_z: i32,
    model: VoxelModel,
}

impl TankCanvas {
    /// Creates a canvas covering `x in [-half_x..=half_x]`, `y in [0..max_y]`, `z in [-half_z..=half_z]`.
    pub fn new(half_x: i32, max_y: i32, half_z: i32) -> Self {
        let sx = (half_x * 2 + 1) as u32;
        let sy = max_y.max(1) as u32;
        let sz = (half_z * 2 + 1) as u32;
        // Pivot places signed coordinate (0, 0, 0) at local origin (0, 0, 0) in meters
        let pivot = Vec3::new(half_x as f32 + 0.5, 0.0, half_z as f32 + 0.5);
        Self {
            half_x,
            max_y,
            half_z,
            model: VoxelModel::new(UVec3::new(sx, sy, sz), pivot, DETAIL_VOXEL_SIZE),
        }
    }

    #[inline]
    fn to_grid(&self, x: i32, y: i32, z: i32) -> IVec3 {
        IVec3::new(x + self.half_x, y, z + self.half_z)
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, id: VoxelId) {
        self.model.set(self.to_grid(x, y, z), id);
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> VoxelId {
        self.model.get(self.to_grid(x, y, z))
    }

    /// Sets `(x, y, z)` and its mirrored `(-x, y, z)`.
    #[inline]
    pub fn set_sym(&mut self, x: i32, y: i32, z: i32, id: VoxelId) {
        self.set(x, y, z, id);
        if x != 0 {
            self.set(-x, y, z, id);
        }
    }

    /// Fills an inclusive box `[x0..=x1] x [y0..=y1] x [z0..=z1]` in signed coordinates.
    #[allow(clippy::too_many_arguments)]
    pub fn fill(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, id: VoxelId) {
        let (ax, bx) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (ay, by) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        let (az, bz) = if z0 <= z1 { (z0, z1) } else { (z1, z0) };
        for y in ay..=by {
            for z in az..=bz {
                for x in ax..=bx {
                    self.set(x, y, z, id);
                }
            }
        }
    }

    /// Fills a symmetric box on both left (`-x1..=-x0`) and right (`x0..=x1`) flanks.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_sym(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, id: VoxelId) {
        self.fill(x0, y0, z0, x1, y1, z1, id);
        self.fill(-x1, y0, z0, -x0, y1, z1, id);
    }

    /// Fills a vertical elliptical cylinder (`y0..=y1`) centered at `(cx, cz)` with radii `(rx, rz)`.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_ellipse_y(
        &mut self,
        cx: f32,
        cz: f32,
        rx: f32,
        rz: f32,
        y0: i32,
        y1: i32,
        id: VoxelId,
    ) {
        let ix0 = (cx - rx).floor() as i32;
        let ix1 = (cx + rx).ceil() as i32;
        let iz0 = (cz - rz).floor() as i32;
        let iz1 = (cz + rz).ceil() as i32;
        let rx2 = (rx * rx).max(0.25);
        let rz2 = (rz * rz).max(0.25);
        for y in y0..=y1 {
            for z in iz0..=iz1 {
                for x in ix0..=ix1 {
                    let dx = x as f32 - cx;
                    let dz = z as f32 - cz;
                    if (dx * dx) / rx2 + (dz * dz) / rz2 <= 1.0 {
                        self.set(x, y, z, id);
                    }
                }
            }
        }
    }

    /// Fills a horizontal circular cylinder along the X axis (`x0..=x1`, centered at `cy, cz`, radius `r`).
    /// Ideal for road wheels, sprockets, idler wheels, rounded mantlets, and rear fuel drums.
    pub fn fill_cyl_x(&mut self, x0: i32, x1: i32, cy: f32, cz: f32, r: f32, id: VoxelId) {
        let iy0 = (cy - r).floor() as i32;
        let iy1 = (cy + r).ceil() as i32;
        let iz0 = (cz - r).floor() as i32;
        let iz1 = (cz + r).ceil() as i32;
        let r2 = r * r;
        let (ax, bx) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for y in iy0..=iy1 {
            for z in iz0..=iz1 {
                let dy = y as f32 - cy;
                let dz = z as f32 - cz;
                if dy * dy + dz * dz <= r2 {
                    for x in ax..=bx {
                        self.set(x, y, z, id);
                    }
                }
            }
        }
    }

    /// Symmetric horizontal X-cylinder on both left and right sides (`[x0..=x1]` and `[-x1..=-x0]`).
    pub fn fill_cyl_x_sym(&mut self, x0: i32, x1: i32, cy: f32, cz: f32, r: f32, id: VoxelId) {
        self.fill_cyl_x(x0, x1, cy, cz, r, id);
        self.fill_cyl_x(-x1, -x0, cy, cz, r, id);
    }

    /// Fills a horizontal cylinder along the Z axis (`z0..=z1`, centered at `cx, cy`, radius `r`).
    /// Ideal for cylindrical exhaust mufflers, Soviet side fuel drums, and gun recoil sleeves.
    pub fn fill_cyl_z(&mut self, cx: f32, cy: f32, z0: i32, z1: i32, r: f32, id: VoxelId) {
        let ix0 = (cx - r).floor() as i32;
        let ix1 = (cx + r).ceil() as i32;
        let iy0 = (cy - r).floor() as i32;
        let iy1 = (cy + r).ceil() as i32;
        let r2 = r * r;
        let (az, bz) = if z0 <= z1 { (z0, z1) } else { (z1, z0) };
        for z in az..=bz {
            for y in iy0..=iy1 {
                for x in ix0..=ix1 {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    if dx * dx + dy * dy <= r2 {
                        self.set(x, y, z, id);
                    }
                }
            }
        }
    }

    /// Consumes the canvas and returns the built `VoxelModel`.
    pub fn into_model(self) -> VoxelModel {
        self.model
    }
}

/// Shared track & road-wheel running-gear builder at `0.10 m` voxel resolution.
///
/// Builds left & right track loops (`inner_x..=outer_x`, `z_front..=z_rear`, height `0..=track_top_y`),
/// drive sprocket (front or rear), idler wheel, evenly spaced road wheels with rubber tires & painted hubs,
/// optional return rollers, and mudguard fenders above the tracks!
#[allow(clippy::too_many_arguments)]
pub fn build_running_gear(
    c: &mut TankCanvas,
    p: &TankPalette,
    inner_x: i32,
    outer_x: i32,
    z_front: i32,
    z_rear: i32,
    track_top_y: i32,
    wheel_count: usize,
    wheel_radius: f32,
    return_rollers: usize,
    fender_y: i32,
) {
    // 1. Bottom ground run & top return run of track belt
    c.fill_sym(inner_x, 0, z_front + 3, outer_x, 0, z_rear - 3, p.track);
    c.fill_sym(
        inner_x,
        track_top_y,
        z_front + 1,
        outer_x,
        track_top_y,
        z_rear - 1,
        p.track,
    );

    // 2. Angled front & rear track wrap-up around sprocket & idler
    for step in 0..=3 {
        let y_lo = step.min(track_top_y - 1);
        c.fill_sym(
            inner_x,
            y_lo,
            z_front + (3 - step),
            outer_x,
            (y_lo + 1).min(track_top_y),
            z_front + (3 - step),
            p.track,
        );
        c.fill_sym(
            inner_x,
            y_lo,
            z_rear - (3 - step),
            outer_x,
            (y_lo + 1).min(track_top_y),
            z_rear - (3 - step),
            p.track,
        );
    }
    // Front/rear sprocket vertical ends
    let sprocket_y = (track_top_y - 2).max(2);
    c.fill_sym(inner_x, sprocket_y, z_front, outer_x, track_top_y - 1, z_front, p.track);
    c.fill_sym(inner_x, sprocket_y, z_rear, outer_x, track_top_y - 1, z_rear, p.track);

    // Sprocket & idler hubs (metallic toothed sprocket look)
    let wheel_inner = inner_x + 1;
    let wheel_outer = (outer_x - 1).max(wheel_inner);
    c.fill_cyl_x_sym(
        wheel_inner,
        wheel_outer,
        sprocket_y as f32 + 0.5,
        (z_front + 2) as f32,
        2.1,
        p.dark,
    );
    c.fill_cyl_x_sym(
        wheel_inner,
        wheel_outer,
        sprocket_y as f32 + 0.5,
        (z_rear - 2) as f32,
        2.0,
        p.dark,
    );
    c.set_sym(outer_x, sprocket_y, z_front + 2, p.trim);
    c.set_sym(outer_x, sprocket_y, z_rear - 2, p.trim);

    // 3. Evenly spaced road wheels with rubber tires & recessed painted steel rims
    let z_w0 = (z_front + 6) as f32;
    let z_w1 = (z_rear - 6) as f32;
    let wheel_cy = wheel_radius + 0.45;
    for i in 0..wheel_count.max(2) {
        let t = if wheel_count <= 1 {
            0.5
        } else {
            i as f32 / (wheel_count - 1) as f32
        };
        let wz = z_w0 + (z_w1 - z_w0) * t;
        // Dark rubber outer tire
        c.fill_cyl_x_sym(wheel_inner, outer_x, wheel_cy, wz, wheel_radius, p.wheel);
        // Painted steel inner dish
        if wheel_radius >= 2.2 {
            c.fill_cyl_x_sym(
                outer_x,
                outer_x,
                wheel_cy,
                wz,
                (wheel_radius - 1.0).max(1.0),
                p.dark,
            );
            c.set_sym(outer_x, wheel_cy.round() as i32, wz.round() as i32, p.trim);
        }
    }

    // 4. Optional upper return rollers
    if return_rollers > 0 {
        let ry = track_top_y - 1;
        for i in 0..return_rollers {
            let t = (i as f32 + 0.5) / return_rollers as f32;
            let rz = (z_front + 6) as f32 + ((z_rear - z_front - 12) as f32) * t;
            c.fill_sym(
                wheel_inner,
                ry - 1,
                rz.round() as i32,
                outer_x,
                ry,
                rz.round() as i32,
                p.wheel,
            );
        }
    }

    // 5. Armored fenders / mudguards covering track tops with front/rear mudflaps
    c.fill_sym(inner_x, fender_y, z_front - 1, outer_x + 1, fender_y, z_rear + 1, p.dark);
    // Slight front & rear curved mudflap drop
    c.fill_sym(inner_x, fender_y - 1, z_front - 1, outer_x + 1, fender_y - 1, z_front - 1, p.dark);
    c.fill_sym(inner_x, fender_y - 1, z_rear + 1, outer_x + 1, fender_y - 1, z_rear + 1, p.dark);
}

/// Builds a crisp, detailed `0.10 m` gun barrel model with optional recoil sleeve, tapered tube,
/// dark hollow bore shadow at tip, and authentic muzzle brake (`None`, `DoubleBaffle`, `TShaped`).
///
/// The pivot is at `(0, 0, 0)` (gun trunnion) and the barrel extends forward along `-Z` to `-length`.
pub fn build_detailed_barrel(
    p: &TankPalette,
    length: i32,
    thick: bool,
    mantlet_collar_len: i32,
    brake: MuzzleBrakeStyle,
) -> VoxelModel {
    let half_x = if thick { 3 } else { 2 };
    let max_y = if thick { 7 } else { 5 };
    let cy = max_y / 2;
    let total_z = length + 4;
    let mut model = VoxelModel::new(
        UVec3::new((half_x * 2 + 1) as u32, max_y as u32, total_z as u32),
        Vec3::new(half_x as f32 + 0.5, cy as f32 + 0.5, (total_z - 2) as f32),
        DETAIL_VOXEL_SIZE,
    );

    let set = |m: &mut VoxelModel, x: i32, y: i32, z_from_trunnion: i32, id: VoxelId| {
        // z_from_trunnion = 0 at trunnion, positive values go FORWARD (-Z in model)
        let gz = (total_z - 2) - z_from_trunnion;
        m.set(IVec3::new(x + half_x, y + cy, gz), id);
    };

    // Armored recoil sleeve / mantlet collar near turret face
    let collar_r: i32 = if thick { 2 } else { 1 };
    for z in 0..mantlet_collar_len.min(length) {
        for dy in -collar_r..=collar_r {
            for dx in -collar_r..=collar_r {
                if dx.abs() + dy.abs() < collar_r * 2 || !thick {
                    set(&mut model, dx, dy, z, p.dark);
                }
            }
        }
    }

    // Main barrel tube
    for z in mantlet_collar_len..length {
        if thick {
            // Thick octagonal 2x2 / cross-section tube for 88mm / 100mm / 120mm / 122mm / 128mm guns
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx.abs() + dy.abs() <= 1 || z < length * 2 / 3 {
                        set(&mut model, dx, dy, z, p.gun);
                    }
                }
            }
        } else {
            // Crisp slender tube for 20mm / 37mm / 45mm / 50mm / 75mm / 76mm guns
            set(&mut model, 0, 0, z, p.gun);
            set(&mut model, 0, 1, z, p.gun);
            if z < length / 2 {
                set(&mut model, -1, 0, z, p.gun);
                set(&mut model, 1, 0, z, p.gun);
            }
        }
    }

    // Dark bore opening shadow right at muzzle tip
    set(&mut model, 0, 0, length, p.black);

    // Authentic muzzle brake styles
    match brake {
        MuzzleBrakeStyle::None => {}
        MuzzleBrakeStyle::DoubleBaffle => {
            // Iconic German KwK 40 / KwK 36 / KwK 43 or Soviet D-25T bulbous double-baffle brake
            let z0 = (length - 4).max(2);
            let z1 = length;
            for z in z0..=z1 {
                let is_baffle_slot = z == z0 + 1 || z == z0 + 3;
                for dy in -1i32..=1 {
                    for dx in -2i32..=2 {
                        if is_baffle_slot && dx.abs() >= 1 && dy == 0 {
                            set(&mut model, dx, dy, z, p.black);
                        } else if dx.abs() + dy.abs() <= 2 {
                            set(&mut model, dx, dy, z, p.trim);
                        }
                    }
                }
            }
            set(&mut model, 0, 0, z1, p.black);
        }
        MuzzleBrakeStyle::TShaped => {
            // Iconic US 76mm M1A2 / 90mm M3 T-shaped muzzle brake
            let z0 = (length - 2).max(2);
            for z in z0..=length {
                for dy in -1i32..=1 {
                    for dx in -2i32..=2 {
                        if z == z0 + 1 && dx.abs() == 2 && dy == 0 {
                            set(&mut model, dx, dy, z, p.black);
                        } else if dy == 0 || dx.abs() <= 1 {
                            set(&mut model, dx, dy, z, p.trim);
                        }
                    }
                }
            }
            set(&mut model, 0, 0, length, p.black);
        }
    }

    model
}

/// Draws a crisp white/black German Balkenkreuz (5x5 cross) on both outer hull or turret flanks at `x = ±flank_x`.
pub fn draw_balkenkreuz_flanks(c: &mut TankCanvas, p: &TankPalette, flank_x: i32, cy: i32, cz: i32) {
    for dy in -2i32..=2 {
        for dz in -2i32..=2 {
            let on_arm = dy.abs() <= 1 || dz.abs() <= 1;
            let on_core = dy == 0 || dz == 0;
            if on_core {
                c.set_sym(flank_x, cy + dy, cz + dz, p.black);
            } else if on_arm && (dy.abs() == 2 || dz.abs() == 2) {
                // Keep outer corners clean
            } else if on_arm {
                c.set_sym(flank_x, cy + dy, cz + dz, p.white);
            }
        }
    }
}

/// Draws a clean US white star emblem (`3x3` bright cross-star) on both sides at `x = ±flank_x`.
pub fn draw_white_star_flanks(c: &mut TankCanvas, p: &TankPalette, flank_x: i32, cy: i32, cz: i32) {
    c.set_sym(flank_x, cy, cz, p.white);
    c.set_sym(flank_x, cy + 1, cz, p.white);
    c.set_sym(flank_x, cy - 1, cz, p.white);
    c.set_sym(flank_x, cy, cz - 1, p.white);
    c.set_sym(flank_x, cy, cz + 1, p.white);
}

/// Draws a Soviet red star with subtle dark outline on both sides at `x = ±flank_x`.
pub fn draw_red_star_flanks(c: &mut TankCanvas, p: &TankPalette, flank_x: i32, cy: i32, cz: i32) {
    c.set_sym(flank_x, cy, cz, p.red);
    c.set_sym(flank_x, cy + 1, cz, p.red);
    c.set_sym(flank_x, cy - 1, cz, p.red);
    c.set_sym(flank_x, cy, cz - 1, p.red);
    c.set_sym(flank_x, cy, cz + 1, p.red);
}

/// Draws engine deck ventilation grilles & louvers on top of the rear hull roof (`y = roof_y`).
pub fn draw_engine_grilles(
    c: &mut TankCanvas,
    p: &TankPalette,
    x_half: i32,
    roof_y: i32,
    z0: i32,
    z1: i32,
) {
    for z in z0..=z1 {
        let slat = if (z - z0) % 2 == 0 { p.black } else { p.dark };
        for x in 2..=x_half {
            c.set_sym(x, roof_y, z, slat);
        }
    }
}

/// Draws a cylindrical commander cupola with dark vision ports around its rim.
pub fn draw_cupola(c: &mut TankCanvas, p: &TankPalette, cx: f32, cz: f32, y_base: i32, h: i32, r: f32) {
    c.fill_ellipse_y(cx, cz, r, r, y_base, y_base + h - 1, p.dark);
    // Vision slit band near top
    let slit_y = y_base + h - 1;
    let ix = cx.round() as i32;
    let iz = cz.round() as i32;
    let ir = r.round() as i32;
    c.set(ix, slit_y, iz - ir, p.optic);
    c.set(ix + ir, slit_y, iz, p.black);
    c.set(ix - ir, slit_y, iz, p.black);
    c.set(ix, slit_y, iz + ir, p.black);
    // Hatch lid on top
    c.fill_ellipse_y(cx, cz, (r - 0.5).max(0.8), (r - 0.5).max(0.8), y_base + h, y_base + h, p.main);
}

/// Draws a crisp front headlight housing with bright warm lens facing `-Z`.
pub fn draw_headlight(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32) {
    c.set(x, y, z, p.trim);
    c.set(x, y, z - 1, p.light);
}

/// Draws a bow machine gun ball mount + protruding short gunmetal barrel pointing `-Z`.
pub fn draw_bow_mg(c: &mut TankCanvas, p: &TankPalette, x: i32, y: i32, z: i32) {
    c.fill(x - 1, y - 1, z, x + 1, y + 1, z, p.dark);
    c.set(x, y, z - 1, p.trim);
    c.set(x, y, z - 2, p.trim);
}

/// Draws iconic Soviet twin cylindrical external fuel drums along rear fenders.
pub fn draw_soviet_fuel_drums(c: &mut TankCanvas, p: &TankPalette, flank_x: i32, y: i32, z0: i32, z1: i32) {
    for &sign in &[-1, 1] {
        let cx = (flank_x * sign) as f32;
        c.fill_cyl_z(cx, y as f32, z0, z1, 1.5, p.dark);
        // Straps around drums
        c.fill(flank_x * sign - 1, y - 1, z0 + 2, flank_x * sign + 1, y + 1, z0 + 2, p.trim);
        c.fill(flank_x * sign - 1, y - 1, z1 - 2, flank_x * sign + 1, y + 1, z1 - 2, p.trim);
    }
}

/// Draws a wooden Soviet unditching log strapped horizontally across the rear hull plate.
pub fn draw_unditching_log(c: &mut TankCanvas, p: &TankPalette, half_w: i32, y: i32, z: i32) {
    c.fill(-half_w, y, z, half_w, y + 1, z + 1, p.wood);
    // Steel mounting straps
    c.fill_sym(half_w / 2, y, z, half_w / 2, y + 1, z + 1, p.trim);
}

/// Assembles final `TankVoxelModels` from signed-coordinate `TankCanvas`es + barrel model.
///
/// - `turret_mount_vox`: `(x, y, z)` in hull signed voxel coordinates where turret pivot sits.
/// - `gun_mount_vox`: `(x, y, z)` in turret signed voxel coordinates where gun trunnion sits.
/// - `barrel_len_vox`: barrel length in `0.10 m` voxels forward along `-Z`.
pub fn assemble_tank(
    hull: TankCanvas,
    turret: TankCanvas,
    barrel: VoxelModel,
    turret_mount_vox: Vec3,
    gun_mount_vox: Vec3,
    barrel_len_vox: f32,
) -> TankVoxelModels {
    let vs = DETAIL_VOXEL_SIZE;
    TankVoxelModels {
        hull: hull.into_model(),
        turret: turret.into_model(),
        barrel,
        turret_mount_offset: turret_mount_vox * vs,
        gun_mount_offset: gun_mount_vox * vs,
        muzzle_offset: Vec3::new(0.0, 0.0, -barrel_len_vox * vs),
    }
}

/// Dispatches a Stage 5 catalog `vehicle_id` to its bespoke high-detail `0.10 m` voxel sculptor.
pub fn build_vehicle_models(vehicle_id: &str) -> Option<TankVoxelModels> {
    usa::build_usa_vehicle(vehicle_id)
        .or_else(|| germany::build_germany_vehicle(vehicle_id))
        .or_else(|| ussr::build_ussr_vehicle(vehicle_id))
}
