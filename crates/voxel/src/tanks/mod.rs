//! High-detail `0.10 m` bespoke voxel vehicle sculptors for all 24 Stage 5 tanks
//! (8 USA, 8 Germany, 8 USSR), engineered for unmistakable frontal, side, and rear silhouettes.

pub mod germany;
pub mod refine;
pub mod usa;
pub mod ussr;

use glam::{IVec3, UVec3, Vec3};

use crate::material::VoxelId;
use crate::model::{MuzzleBrakeStyle, TankVoxelModels, VoxelModel};
pub use refine::MICRO_VOXEL_SIZE;

/// Base sculptor grid size (`0.10 m`), refined automatically to `MICRO_VOXEL_SIZE` (`0.05 m`) on assembly.
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

/// Builds a crisp, super-detailed `0.05 m` (5 cm) gun barrel model with bolted recoil sleeve,
/// tapered octagonal tube, bore evacuator on long heavy guns, dark recessed bore shadow,
/// and hollow-vented authentic muzzle brake (`None`, `DoubleBaffle`, `TShaped`).
///
/// `length` and `mantlet_collar_len` are specified in `0.10 m` base units (converted internally
/// to `0.05 m` micro-voxels) so callers keep exact physical proportions.
/// The pivot is at `(0, 0, 0)` (gun trunnion) and the barrel extends forward along `-Z`.
pub fn build_detailed_barrel(
    p: &TankPalette,
    length: i32,
    thick: bool,
    mantlet_collar_len: i32,
    brake: MuzzleBrakeStyle,
) -> VoxelModel {
    let len_hi = (length * 2).max(6);
    let collar_hi = (mantlet_collar_len * 2).min(len_hi);
    let half_x: i32 = if thick { 5 } else { 4 };
    let half_y: i32 = if thick { 5 } else { 4 };
    let total_z = len_hi + 8;

    let mut model = VoxelModel::new(
        UVec3::new(
            (half_x * 2 + 1) as u32,
            (half_y * 2 + 1) as u32,
            total_z as u32,
        ),
        Vec3::new(
            half_x as f32 + 0.5,
            half_y as f32 + 0.5,
            (total_z - 3) as f32,
        ),
        MICRO_VOXEL_SIZE,
    );

    let set = |m: &mut VoxelModel, x: i32, y: i32, z_from_trunnion: i32, id: VoxelId| {
        // z_from_trunnion = 0 at trunnion, positive values go FORWARD (-Z in model)
        let gz = (total_z - 3) - z_from_trunnion;
        m.set(IVec3::new(x + half_x, y + half_y, gz), id);
    };

    // Helper: fill a circular cross-section at `z` with radius `r` in 0.05 m voxels
    let fill_disk = |m: &mut VoxelModel, z: i32, r: f32, id: VoxelId| {
        let ir = r.ceil() as i32;
        let r2 = r * r;
        for dy in -ir..=ir {
            for dx in -ir..=ir {
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 <= r2 {
                    set(m, dx, dy, z, id);
                }
            }
        }
    };

    // 1. Heavy armored recoil sleeve / mantlet collar + canvas dust boot + retaining bolts
    let collar_r: f32 = if thick { 3.15 } else { 2.15 };
    for z in 0..collar_hi {
        let mat = if z <= 1 {
            VoxelId::CANVAS_TARP
        } else if z == collar_hi - 1 {
            p.trim
        } else {
            p.dark
        };
        fill_disk(&mut model, z, collar_r, mat);
    }
    // Bright steel retaining bolt heads around front lip of recoil sleeve
    if collar_hi >= 3 {
        let bolt_r = collar_r.floor() as i32;
        set(&mut model, bolt_r, 0, collar_hi - 1, VoxelId::STEEL_BRIGHT);
        set(&mut model, -bolt_r, 0, collar_hi - 1, VoxelId::STEEL_BRIGHT);
        set(&mut model, 0, bolt_r, collar_hi - 1, VoxelId::STEEL_BRIGHT);
    }

    // 2. Stepped tapered circular cannon barrel tube
    let bore_evac_start = (len_hi * 44) / 100;
    let bore_evac_end = (len_hi * 58) / 100;
    let has_bore_evac = thick && length >= 34;

    for z in collar_hi..len_hi {
        let t = (z - collar_hi) as f32 / (len_hi - collar_hi).max(1) as f32;
        let mut r = if thick {
            if t < 0.35 {
                2.15
            } else if t < 0.72 {
                1.75
            } else {
                1.45
            }
        } else if t < 0.38 {
            1.45
        } else {
            1.05
        };

        let mut mat = p.gun;
        // Authentic mid-barrel bore evacuator (fume extractor) bulge on long heavy guns
        if has_bore_evac && (bore_evac_start..=bore_evac_end).contains(&z) {
            r = if z == bore_evac_start || z == bore_evac_end {
                2.15
            } else {
                2.50
            };
            mat = p.dark;
        }
        // Subtle machined step ring where barrel tapers
        if (thick && (z == collar_hi + ((len_hi - collar_hi) * 35) / 100))
            || (!thick && z == collar_hi + ((len_hi - collar_hi) * 38) / 100)
        {
            mat = p.dark;
        }

        fill_disk(&mut model, z, r, mat);
    }

    // 3. Authentic 0.05 m muzzle brake styles (or machined muzzle collar ring)
    match brake {
        MuzzleBrakeStyle::None => {
            // Crisp machined muzzle collar ring + recessed dark rifled bore
            let ring_r = if thick { 1.80 } else { 1.35 };
            fill_disk(&mut model, len_hi - 1, ring_r, p.dark);
            fill_disk(&mut model, len_hi, ring_r, p.trim);
            set(&mut model, 0, 0, len_hi, p.black);
            set(&mut model, 0, 0, len_hi + 1, p.black);
        }
        MuzzleBrakeStyle::DoubleBaffle => {
            // Iconic German KwK 40 / KwK 36 / KwK 43 & Soviet D-25T bulbous double-baffle brake
            let z0 = (len_hi - 8).max(4);
            let z1 = len_hi + 1;
            let bx_max: i32 = if thick { 3 } else { 2 };
            let by_max: i32 = 2;
            for z in z0..=z1 {
                let is_rear_collar = z == z0;
                let is_baffle_wall = z == z0 + 1 || z == z0 + 4 || z == z0 + 7 || z == z1;
                let is_vent_chamber = (z == z0 + 2 || z == z0 + 3) || (z == z0 + 5 || z == z0 + 6);
                for dy in -by_max..=by_max {
                    for dx in -bx_max..=bx_max {
                        if dx.abs() == bx_max && dy.abs() == by_max {
                            continue; // Octagonal chamfered profile
                        }
                        if is_rear_collar {
                            if dx.abs() < bx_max && dy.abs() < by_max {
                                set(&mut model, dx, dy, z, p.dark);
                            }
                        } else if is_vent_chamber {
                            // Hollow side blast slots: leave outer mid-height vents open Air!
                            if dy.abs() <= 1 && dx.abs() >= 1 {
                                if dx.abs() == 1 && dy == 0 {
                                    set(&mut model, dx, dy, z, p.black);
                                } else {
                                    set(&mut model, dx, dy, z, VoxelId::AIR);
                                }
                            } else {
                                set(&mut model, dx, dy, z, p.trim);
                            }
                        } else if is_baffle_wall {
                            let mat = if z == z1 && (dx.abs() == bx_max || dy.abs() == by_max) {
                                VoxelId::STEEL_BRIGHT
                            } else {
                                p.trim
                            };
                            set(&mut model, dx, dy, z, mat);
                        }
                    }
                }
            }
            // Deep recessed dark bore opening at muzzle face
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx.abs() + dy.abs() <= 1 {
                        set(&mut model, dx, dy, z1, p.black);
                    }
                }
            }
        }
        MuzzleBrakeStyle::TShaped => {
            // Iconic US 76mm M1A2 / 90mm M3 / 120mm T53 T-shaped twin-port muzzle brake
            let z0 = (len_hi - 5).max(4);
            let z1 = len_hi + 1;
            let wing_x: i32 = if thick { 4 } else { 3 };
            for z in z0..=z1 {
                let is_port = z == z0 + 2 || z == z0 + 3 || z == z0 + 4;
                for dy in -2i32..=2 {
                    for dx in -wing_x..=wing_x {
                        if is_port {
                            // Wide T-wings with hollow horizontal side blast ports
                            if dy == 0 && dx.abs() >= 2 {
                                set(&mut model, dx, dy, z, VoxelId::AIR);
                            } else if dy.abs() <= 1 || dx.abs() <= 1 {
                                let mat = if dx.abs() == wing_x && dy.abs() == 1 {
                                    VoxelId::STEEL_BRIGHT
                                } else {
                                    p.trim
                                };
                                set(&mut model, dx, dy, z, mat);
                            }
                        } else if (z == z0 || z == z0 + 1) && dx.abs() <= 2 && dy.abs() <= 2 && dx.abs() + dy.abs() <= 3 {
                            set(&mut model, dx, dy, z, p.dark);
                        } else if z == z1 && dy.abs() <= 1 && dx.abs() <= wing_x {
                            set(&mut model, dx, dy, z, p.trim);
                        }
                    }
                }
            }
            // Dark recessed bore opening
            set(&mut model, 0, 0, z1, p.black);
            set(&mut model, 1, 0, z1, p.black);
            set(&mut model, -1, 0, z1, p.black);
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
    let hull_lo = hull.into_model();
    let turret_lo = turret.into_model();
    let (hull_hi, turret_hi) = refine::refine_hull_and_turret(&hull_lo, &turret_lo);
    TankVoxelModels {
        hull: hull_hi,
        turret: turret_hi,
        barrel,
        turret_mount_offset: turret_mount_vox * vs,
        gun_mount_offset: gun_mount_vox * vs,
        muzzle_offset: Vec3::new(0.0, 0.0, -barrel_len_vox * vs),
    }
}

/// Dispatches a Stage 5 catalog `vehicle_id` to its bespoke `0.05 m` super-detail voxel sculptor.
pub fn build_vehicle_models(vehicle_id: &str) -> Option<TankVoxelModels> {
    usa::build_usa_vehicle(vehicle_id)
        .or_else(|| germany::build_germany_vehicle(vehicle_id))
        .or_else(|| ussr::build_ussr_vehicle(vehicle_id))
}
