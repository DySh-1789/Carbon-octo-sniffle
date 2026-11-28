//! `0.05 m` (5 cm) Super-Resolution Tank Refinement & Micro-Detailing Engine.
//!
//! Upsamples sculpted `0.10 m` hull and turret canvases to `0.05 m` (8x volumetric density)
//! while preserving exact local-meter coordinates and bounding boxes, then applies:
//! 1. Smooth half-step slope beveling on glacis plates, hull sides, pike noses, and rounded turrets.
//! 2. Micro-sculpted running gear: track tread grousers, mud weathering, sprocket teeth, and bolted wheel hubs.
//! 3. High-resolution `0.05 m` national insignia (5-point US/USSR stars, authentic 10x10 German Balkenkreuz).
//! 4. Authentic late-war German 3-tone Wehrmacht camouflage (Dunkelgelb + Olivgrün + Schokoladenbraun).
//! 5. Fine 5 cm engine deck louvers, optic glints, headlight bezels, edge highlights, pioneer tools,
//!    tow cables, rolled canvas tarps, rear convoy tail lamps, smoke grenade launchers, and Soviet grab rails.

use glam::{IVec3, UVec3, Vec3};

use crate::material::VoxelId;
use crate::model::VoxelModel;

/// Super-resolution tank voxel size (`0.05 m` = 5 cm voxels, 8x finer than `0.10 m`).
pub const MICRO_VOXEL_SIZE: f32 = 0.05;

/// Padding (in `0.05 m` sub-voxels) added around `-X/+X/+Y/-Z/+Z` so exterior micro-fittings
/// (smoke launchers, handrails, tail lamps, tools) always stay cleanly inside bounds.
const PAD_XZ: i32 = 2;
const PAD_TOP_Y: i32 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ArmorTheme {
    Usa,
    GermanyGrey,
    GermanyTan,
    Ussr,
}

impl ArmorTheme {
    fn main(self) -> VoxelId {
        match self {
            Self::Usa => VoxelId::ARMOR_US,
            Self::GermanyGrey => VoxelId::ARMOR_DE_GREY,
            Self::GermanyTan => VoxelId::ARMOR_DE_TAN,
            Self::Ussr => VoxelId::ARMOR_SU,
        }
    }

    fn dark(self) -> VoxelId {
        match self {
            Self::Usa => VoxelId::ARMOR_US_DARK,
            Self::GermanyGrey => VoxelId::ARMOR_DE_GREY_DARK,
            Self::GermanyTan => VoxelId::ARMOR_DE_TAN_DARK,
            Self::Ussr => VoxelId::ARMOR_SU_DARK,
        }
    }

    fn light(self) -> VoxelId {
        match self {
            Self::Usa => VoxelId::ARMOR_US_LIGHT,
            Self::GermanyGrey => VoxelId::ARMOR_DE_GREY_LIGHT,
            Self::GermanyTan => VoxelId::ARMOR_DE_TAN_LIGHT,
            Self::Ussr => VoxelId::ARMOR_SU_LIGHT,
        }
    }

    #[inline]
    fn is_armor(self, id: VoxelId) -> bool {
        id == self.main() || id == self.dark()
    }
}

/// Refines a paired `0.10 m` hull and turret into `0.05 m` super-resolution models.
pub fn refine_hull_and_turret(hull_lo: &VoxelModel, turret_lo: &VoxelModel) -> (VoxelModel, VoxelModel) {
    let theme = detect_theme(hull_lo);

    let mut hull = upsample_2x_and_bevel(hull_lo, theme);
    refine_running_gear(hull_lo, &mut hull, theme);
    refine_emblems(hull_lo, &mut hull, theme);
    if theme == ArmorTheme::GermanyTan {
        apply_german_three_tone_camo(&mut hull, false);
    }
    refine_optics_headlights_louvers_and_edges(hull_lo, &mut hull, theme, true);
    add_hull_stowage_and_fittings(hull_lo, &mut hull, theme);

    let mut turret = upsample_2x_and_bevel(turret_lo, theme);
    refine_emblems(turret_lo, &mut turret, theme);
    if theme == ArmorTheme::GermanyTan {
        apply_german_three_tone_camo(&mut turret, true);
    }
    refine_optics_headlights_louvers_and_edges(turret_lo, &mut turret, theme, false);
    add_turret_micro_fittings(turret_lo, &mut turret, theme);

    (hull, turret)
}

fn detect_theme(hull: &VoxelModel) -> ArmorTheme {
    let mut us = 0usize;
    let mut de_grey = 0usize;
    let mut de_tan = 0usize;
    let mut su = 0usize;
    for y in 0..hull.size.y as i32 {
        for z in 0..hull.size.z as i32 {
            for x in 0..hull.size.x as i32 {
                match hull.get(IVec3::new(x, y, z)) {
                    VoxelId::ARMOR_US => us += 1,
                    VoxelId::ARMOR_DE_GREY => de_grey += 1,
                    VoxelId::ARMOR_DE_TAN => de_tan += 1,
                    VoxelId::ARMOR_SU => su += 1,
                    _ => {}
                }
            }
        }
    }
    if de_tan >= us && de_tan >= de_grey && de_tan >= su {
        ArmorTheme::GermanyTan
    } else if de_grey >= us && de_grey >= su {
        ArmorTheme::GermanyGrey
    } else if su >= us {
        ArmorTheme::Ussr
    } else {
        ArmorTheme::Usa
    }
}

#[inline]
fn to_hi(x: i32, y: i32, z: i32) -> IVec3 {
    IVec3::new(x * 2 + PAD_XZ, y * 2, z * 2 + PAD_XZ)
}

/// Exact 2x upsampling (`0.10 m` -> `0.05 m`) + smooth half-step slope beveling on armor slabs.
fn upsample_2x_and_bevel(src: &VoxelModel, theme: ArmorTheme) -> VoxelModel {
    let sx = src.size.x as i32;
    let sy = src.size.y as i32;
    let sz = src.size.z as i32;

    let size_hi = UVec3::new(
        (sx * 2 + PAD_XZ * 2) as u32,
        (sy * 2 + PAD_TOP_Y) as u32,
        (sz * 2 + PAD_XZ * 2) as u32,
    );
    let pivot_hi = Vec3::new(
        src.pivot.x * 2.0 + PAD_XZ as f32,
        src.pivot.y * 2.0,
        src.pivot.z * 2.0 + PAD_XZ as f32,
    );
    let mut dst = VoxelModel::new(size_hi, pivot_hi, MICRO_VOXEL_SIZE);

    // 1. Exact 2x2x2 volumetric copy
    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                let id = src.get(IVec3::new(x, y, z));
                if id.is_air() {
                    continue;
                }
                let base = to_hi(x, y, z);
                for dy in 0..2 {
                    for dz in 0..2 {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, dy, dz), id);
                        }
                    }
                }
            }
        }
    }

    // 2. Half-voxel (0.05 m) slope chamfering on armor staircases
    for y in 1..sy {
        for z in 0..sz {
            for x in 0..sx {
                let p = IVec3::new(x, y, z);
                if !src.get(p).is_air() {
                    continue;
                }
                let base = to_hi(x, y, z);

                let below = src.get(p + IVec3::NEG_Y);
                let above_air = src.get(p + IVec3::Y).is_air();

                // Forward-facing glacis slope stair (solid armor below + rear, continuing slope ahead)
                if theme.is_armor(below) && above_air {
                    let rear = src.get(p + IVec3::Z);
                    let fwd_below = src.get(p + IVec3::NEG_Y + IVec3::NEG_Z);
                    if theme.is_armor(rear) && (theme.is_armor(fwd_below) || fwd_below.is_air()) && src.get(p + IVec3::NEG_Z).is_air() {
                        // Only bevel true sloped transitions (rear block has air above or slopes further back)
                        let rear_above = src.get(p + IVec3::Z + IVec3::Y);
                        if rear_above.is_air() || theme.is_armor(src.get(p + IVec3::Z * 2 + IVec3::Y)) {
                            dst.set(base + IVec3::new(0, 0, 1), rear);
                            dst.set(base + IVec3::new(1, 0, 1), rear);
                        }
                    }

                    // Rear-facing sloped deck / rear turret slope
                    let fwd = src.get(p + IVec3::NEG_Z);
                    let rear_below = src.get(p + IVec3::NEG_Y + IVec3::Z);
                    if theme.is_armor(fwd) && (theme.is_armor(rear_below) || rear_below.is_air()) && src.get(p + IVec3::Z).is_air() {
                        let fwd_above = src.get(p + IVec3::NEG_Z + IVec3::Y);
                        if fwd_above.is_air() || theme.is_armor(src.get(p + IVec3::NEG_Z * 2 + IVec3::Y)) {
                            dst.set(base + IVec3::new(0, 0, 0), fwd);
                            dst.set(base + IVec3::new(1, 0, 0), fwd);
                        }
                    }

                    // Sloped right side armor (+X stair)
                    let left_in = src.get(p + IVec3::NEG_X);
                    if theme.is_armor(left_in) && src.get(p + IVec3::X).is_air() {
                        let out_below = src.get(p + IVec3::NEG_Y + IVec3::X);
                        if theme.is_armor(out_below) || out_below.is_air() {
                            let in_above = src.get(p + IVec3::NEG_X + IVec3::Y);
                            if in_above.is_air() || theme.is_armor(src.get(p + IVec3::NEG_X * 2 + IVec3::Y)) {
                                dst.set(base + IVec3::new(0, 0, 0), left_in);
                                dst.set(base + IVec3::new(0, 0, 1), left_in);
                            }
                        }
                    }

                    // Sloped left side armor (-X stair)
                    let right_in = src.get(p + IVec3::X);
                    if theme.is_armor(right_in) && src.get(p + IVec3::NEG_X).is_air() {
                        let out_below = src.get(p + IVec3::NEG_Y + IVec3::NEG_X);
                        if theme.is_armor(out_below) || out_below.is_air() {
                            let in_above = src.get(p + IVec3::X + IVec3::Y);
                            if in_above.is_air() || theme.is_armor(src.get(p + IVec3::X * 2 + IVec3::Y)) {
                                dst.set(base + IVec3::new(1, 0, 0), right_in);
                                dst.set(base + IVec3::new(1, 0, 1), right_in);
                            }
                        }
                    }
                }

                // Plan-view 45-degree corner chamfer (IS-3 pike nose, curved turrets, cast noses)
                for (dx, dz) in [(-1i32, -1i32), (1, -1), (-1, 1), (1, 1)] {
                    let nx = src.get(p + IVec3::new(dx, 0, 0));
                    let nz = src.get(p + IVec3::new(0, 0, dz));
                    let diag = src.get(p + IVec3::new(dx, 0, dz));
                    if theme.is_armor(nx)
                        && theme.is_armor(nz)
                        && theme.is_armor(diag)
                        && src.get(p + IVec3::new(-dx, 0, 0)).is_air()
                        && src.get(p + IVec3::new(0, 0, -dz)).is_air()
                    {
                        let ox = if dx > 0 { 1 } else { 0 };
                        let oz = if dz > 0 { 1 } else { 0 };
                        dst.set(base + IVec3::new(ox, 0, oz), nx);
                        dst.set(base + IVec3::new(ox, 1, oz), nx);
                    }
                }
            }
        }
    }

    dst
}

/// Refines hull tracks, sprockets, and road wheels at `0.05 m` resolution:
/// - Alternating 5 cm track tread cleats & subtle ground-contact mud
/// - Bright steel sprocket teeth accents on front/rear drive hubs
/// - Bolted road wheel hubs (center grease cap + 4 radial lug bolts)
fn refine_running_gear(src: &VoxelModel, dst: &mut VoxelModel, theme: ArmorTheme) {
    let sx = src.size.x as i32;
    let sy = src.size.y as i32;
    let sz = src.size.z as i32;

    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                let id = src.get(IVec3::new(x, y, z));
                if id.is_air() {
                    continue;
                }
                let base = to_hi(x, y, z);

                if id == VoxelId::STEEL_TRACK {
                    let above_air = src.get(IVec3::new(x, y + 1, z)).is_air();
                    let below_air = y == 0 || src.get(IVec3::new(x, y - 1, z)).is_air();
                    let front_air = src.get(IVec3::new(x, y, z - 1)).is_air();
                    let rear_air = src.get(IVec3::new(x, y, z + 1)).is_air();

                    // Top track run: alternating 5 cm track grouser bars + center guide tooth contrast
                    if above_air {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, 1, 0), VoxelId::STEEL_DARK_TRIM);
                            dst.set(base + IVec3::new(dx, 1, 1), VoxelId::STEEL_TRACK);
                        }
                    }

                    // Bottom ground-contact track run: mud-dusted cleats
                    if y == 0 {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, 0, 0), VoxelId::MUD_DUST);
                            dst.set(base + IVec3::new(dx, 0, 1), VoxelId::STEEL_DARK_TRIM);
                        }
                    } else if below_air {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, 0, 0), VoxelId::STEEL_DARK_TRIM);
                        }
                    }

                    // Vertical front/rear track wrap-up treads
                    if front_air {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, 0, 0), VoxelId::STEEL_DARK_TRIM);
                            dst.set(base + IVec3::new(dx, 1, 0), VoxelId::STEEL_TRACK);
                        }
                    }
                    if rear_air {
                        for dx in 0..2 {
                            dst.set(base + IVec3::new(dx, 0, 1), VoxelId::STEEL_DARK_TRIM);
                            dst.set(base + IVec3::new(dx, 1, 1), VoxelId::STEEL_TRACK);
                        }
                    }
                } else if id == VoxelId::STEEL_DARK_TRIM && y <= 8 {
                    // Identify road wheel center hub caps & sprocket hubs on outer flanks
                    let left_air = src.get(IVec3::new(x - 1, y, z)).is_air();
                    let right_air = src.get(IVec3::new(x + 1, y, z)).is_air();
                    if left_air || right_air {
                        let neighbor_wheel = src.get(IVec3::new(x, y + 1, z)) == theme.dark()
                            || src.get(IVec3::new(x, y - 1, z)) == theme.dark()
                            || src.get(IVec3::new(x, y, z + 1)) == VoxelId::RUBBER_WHEEL
                            || src.get(IVec3::new(x, y, z - 1)) == VoxelId::RUBBER_WHEEL;
                        if neighbor_wheel {
                            let ox = if right_air { 1 } else { 0 };
                            // Bright steel hub grease cap & recessed nut
                            dst.set(base + IVec3::new(ox, 0, 0), VoxelId::STEEL_BRIGHT);
                            dst.set(base + IVec3::new(ox, 1, 1), VoxelId::STEEL_BRIGHT);
                            // 4 radial lug bolts on surrounding wheel dish
                            for (dy, dz) in [(-1, 0), (2, 1), (0, -1), (1, 2)] {
                                let bp = base + IVec3::new(ox, dy, dz);
                                if dst.get(bp) == theme.dark() {
                                    dst.set(bp, VoxelId::STEEL_BRIGHT);
                                }
                            }
                        }
                    }
                } else if id == VoxelId::RUBBER_WHEEL {
                    // Subtle mud splash on lowest rubber wheel edge touching the bottom track
                    if y <= 1 {
                        dst.set(base + IVec3::new(0, 0, 0), VoxelId::MUD_DUST);
                        dst.set(base + IVec3::new(1, 0, 1), VoxelId::MUD_DUST);
                    }
                }
            }
        }
    }
}

/// Detects national emblem patches from the `0.10 m` model and redraws crisp `0.05 m`
/// 5-pointed stars (USA/USSR) or authentic 10x10 German Balkenkreuz flush on the outer surface.
fn refine_emblems(src: &VoxelModel, dst: &mut VoxelModel, theme: ArmorTheme) {
    let sx = src.size.x as i32;
    let sy = src.size.y as i32;
    let sz = src.size.z as i32;
    let mut visited = vec![false; (sx * sy * sz) as usize];
    let idx = |x: i32, y: i32, z: i32| -> usize { ((y * sz + z) * sx + x) as usize };

    let is_emblem_mat = |id: VoxelId| -> bool {
        match theme {
            ArmorTheme::Usa => id == VoxelId::EMBLEM_WHITE,
            ArmorTheme::Ussr => id == VoxelId::EMBLEM_RED,
            ArmorTheme::GermanyGrey | ArmorTheme::GermanyTan => {
                id == VoxelId::EMBLEM_WHITE || id == VoxelId::EMBLEM_BLACK
            }
        }
    };

    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                let i = idx(x, y, z);
                if visited[i] {
                    continue;
                }
                let id = src.get(IVec3::new(x, y, z));
                if !is_emblem_mat(id) {
                    continue;
                }

                // Flood-fill connected emblem voxels in 3D
                let mut stack = vec![IVec3::new(x, y, z)];
                let mut cluster = Vec::new();
                visited[i] = true;
                let mut has_white = false;
                let mut has_red = false;

                while let Some(p) = stack.pop() {
                    let cid = src.get(p);
                    if cid == VoxelId::EMBLEM_WHITE {
                        has_white = true;
                    }
                    if cid == VoxelId::EMBLEM_RED {
                        has_red = true;
                    }
                    cluster.push(p);
                    for d in [
                        IVec3::X,
                        IVec3::NEG_X,
                        IVec3::Y,
                        IVec3::NEG_Y,
                        IVec3::Z,
                        IVec3::NEG_Z,
                    ] {
                        let np = p + d;
                        if np.x >= 0
                            && np.x < sx
                            && np.y >= 0
                            && np.y < sy
                            && np.z >= 0
                            && np.z < sz
                        {
                            let ni = idx(np.x, np.y, np.z);
                            if !visited[ni] && is_emblem_mat(src.get(np)) {
                                visited[ni] = true;
                                stack.push(np);
                            }
                        }
                    }
                }

                // Filter out engine grilles / gun bores / non-emblem clusters
                let valid_emblem = match theme {
                    ArmorTheme::Usa => has_white && cluster.len() >= 4 && cluster.len() <= 20,
                    ArmorTheme::Ussr => has_red && cluster.len() >= 4 && cluster.len() <= 20,
                    ArmorTheme::GermanyGrey | ArmorTheme::GermanyTan => {
                        has_white && cluster.len() >= 8 && cluster.len() <= 30
                    }
                };
                if !valid_emblem {
                    continue;
                }

                // Determine cluster centroid & orientation (flank-facing +-X vs front-facing -Z)
                let mut sum = Vec3::ZERO;
                let mut min_p = IVec3::splat(i32::MAX);
                let mut max_p = IVec3::splat(i32::MIN);
                for &p in &cluster {
                    sum += p.as_vec3();
                    min_p = min_p.min(p);
                    max_p = max_p.max(p);
                }
                let center_lo = sum / cluster.len() as f32;
                let span = max_p - min_p;
                let is_flank = span.z >= span.x;

                // Clear coarse 0.10 m emblem blocks back to base armor paint
                for &p in &cluster {
                    let base = to_hi(p.x, p.y, p.z);
                    for dy in 0..2 {
                        for dz in 0..2 {
                            for dx in 0..2 {
                                let hp = base + IVec3::new(dx, dy, dz);
                                if !dst.get(hp).is_air() {
                                    dst.set(hp, theme.main());
                                }
                            }
                        }
                    }
                }

                // Project razor-sharp 0.05 m insignia onto the outermost solid armor surface
                let cx_hi = (center_lo.x * 2.0 + PAD_XZ as f32 + 0.5).round() as i32;
                let cy_hi = (center_lo.y * 2.0 + 0.5).round() as i32;
                let cz_hi = (center_lo.z * 2.0 + PAD_XZ as f32 + 0.5).round() as i32;
                let right_side = center_lo.x > src.pivot.x;

                if is_flank {
                    let search_start = if right_side {
                        (dst.size.x as i32 - 1).min(cx_hi + 4)
                    } else {
                        (cx_hi - 4).max(0)
                    };
                    let search_dir = if right_side { -1 } else { 1 };
                    stamp_emblem_flank(dst, theme, cy_hi, cz_hi, search_start, search_dir);
                } else {
                    let search_start = (cz_hi - 4).max(0);
                    stamp_emblem_front(dst, theme, cx_hi, cy_hi, search_start);
                }
            }
        }
    }
}

/// High-res 9x9 pixel art for an authentic 5-pointed star (USA white / USSR red with white rim).
fn five_point_star_pixel(du: i32, dv: i32, with_white_rim: bool, fill: VoxelId) -> Option<VoxelId> {
    // 9x9 grid centered at (0, 0), dv = +4 is top point, dv = -4 is bottom legs
    const MASK: [&str; 9] = [
        "....#....",
        "....#....",
        "...###...",
        "#########",
        ".#######.",
        "..#####..",
        ".###.###.",
        ".##...##.",
        "#.......#",
    ];
    if !(-4..=4).contains(&du) || !(-4..=4).contains(&dv) {
        return None;
    }
    let row = (4 - dv) as usize;
    let col = (du + 4) as usize;
    if MASK[row].as_bytes()[col] == b'#' {
        return Some(fill);
    }
    if with_white_rim {
        // 1-voxel crisp white halo on Soviet Guards stars
        for (ou, ov) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nu = du + ou;
            let nv = dv + ov;
            if (-4..=4).contains(&nu) && (-4..=4).contains(&nv) {
                let nr = (4 - nv) as usize;
                let nc = (nu + 4) as usize;
                if MASK[nr].as_bytes()[nc] == b'#' {
                    return Some(VoxelId::EMBLEM_WHITE);
                }
            }
        }
    }
    None
}

/// High-res 10x10 authentic Wehrmacht Balkenkreuz (black cross + crisp white L-brackets).
fn balkenkreuz_pixel(du: i32, dv: i32) -> Option<VoxelId> {
    // Centered on [-4..=5] -> map to symmetrical distance using abs
    let ax = du.abs();
    let ay = dv.abs();
    if ax > 4 || ay > 4 {
        return None;
    }
    // Inner black cross arms (width 3 -> |u| <= 1 or |v| <= 1)
    if ax <= 1 || ay <= 1 {
        return Some(VoxelId::EMBLEM_BLACK);
    }
    // Crisp white L-brackets right outside the black cross (|u| == 2 or |v| == 2, up to 4)
    if (ax == 2 && ay <= 4) || (ay == 2 && ax <= 4) {
        return Some(VoxelId::EMBLEM_WHITE);
    }
    None
}

fn stamp_emblem_flank(
    dst: &mut VoxelModel,
    theme: ArmorTheme,
    cy: i32,
    cz: i32,
    search_start_x: i32,
    dir_x: i32,
) {
    for dv in -4..=4 {
        for du in -4..=4 {
            let pixel = match theme {
                ArmorTheme::Usa => five_point_star_pixel(du, dv, false, VoxelId::EMBLEM_WHITE),
                ArmorTheme::Ussr => five_point_star_pixel(du, dv, true, VoxelId::EMBLEM_RED),
                ArmorTheme::GermanyGrey | ArmorTheme::GermanyTan => balkenkreuz_pixel(du, dv),
            };
            let Some(color) = pixel else {
                continue;
            };
            let y = cy + dv;
            let z = cz + du;
            for step in 0..10 {
                let x = search_start_x + dir_x * step;
                let p = IVec3::new(x, y, z);
                let cur = dst.get(p);
                if !cur.is_air() {
                    if theme.is_armor(cur) || cur == VoxelId::EMBLEM_WHITE || cur == VoxelId::EMBLEM_BLACK || cur == VoxelId::EMBLEM_RED {
                        dst.set(p, color);
                    }
                    break;
                }
            }
        }
    }
}

fn stamp_emblem_front(
    dst: &mut VoxelModel,
    theme: ArmorTheme,
    cx: i32,
    cy: i32,
    search_start_z: i32,
) {
    for dv in -4..=4 {
        for du in -4..=4 {
            let pixel = match theme {
                ArmorTheme::Usa => five_point_star_pixel(du, dv, false, VoxelId::EMBLEM_WHITE),
                ArmorTheme::Ussr => five_point_star_pixel(du, dv, true, VoxelId::EMBLEM_RED),
                ArmorTheme::GermanyGrey | ArmorTheme::GermanyTan => balkenkreuz_pixel(du, dv),
            };
            let Some(color) = pixel else {
                continue;
            };
            let x = cx + du;
            let y = cy + dv;
            for step in 0..10 {
                let z = search_start_z + step;
                let p = IVec3::new(x, y, z);
                let cur = dst.get(p);
                if !cur.is_air() {
                    if theme.is_armor(cur) || cur == VoxelId::EMBLEM_WHITE || cur == VoxelId::EMBLEM_BLACK || cur == VoxelId::EMBLEM_RED {
                        dst.set(p, color);
                    }
                    break;
                }
            }
        }
    }
}

/// Authentic late-war Wehrmacht 3-tone camo bands (Olivgrün + Schokoladenbraun over Dunkelgelb).
/// Uses smooth macro-scale trigonometric bands so greedy meshing stays super compact.
fn apply_german_three_tone_camo(dst: &mut VoxelModel, is_turret: bool) {
    let sx = dst.size.x as i32;
    let sy = dst.size.y as i32;
    let sz = dst.size.z as i32;
    let phase = if is_turret { 1.35 } else { 0.0 };

    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                let p = IVec3::new(x, y, z);
                if dst.get(p) != VoxelId::ARMOR_DE_TAN {
                    continue;
                }
                // Meter coordinates relative to model pivot
                let mx = (x as f32 - dst.pivot.x) * MICRO_VOXEL_SIZE;
                let my = (y as f32 - dst.pivot.y) * MICRO_VOXEL_SIZE;
                let mz = (z as f32 - dst.pivot.z) * MICRO_VOXEL_SIZE;

                // Smooth organic diagonal Wehrmacht spray bands (~0.55 m wavelength)
                let wave_g = (mx * 1.45 + mz * 1.15 - my * 0.90 + phase).sin()
                    + 0.45 * (mx * 0.85 - mz * 1.55 + my * 0.70).cos();
                let wave_b = (mx * 1.10 - mz * 1.35 + my * 1.05 + 2.4 + phase).sin()
                    + 0.40 * (mx * 1.65 + mz * 0.75).cos();

                if wave_g > 0.78 {
                    dst.set(p, VoxelId::ARMOR_CAMO_GREEN);
                } else if wave_b > 0.86 {
                    dst.set(p, VoxelId::ARMOR_CAMO_BROWN);
                }
            }
        }
    }
}

/// Refines headlights, optic glints, 5 cm engine deck louvers, and subtle roof edge dry-brush highlights.
fn refine_optics_headlights_louvers_and_edges(
    src: &VoxelModel,
    dst: &mut VoxelModel,
    theme: ArmorTheme,
    is_hull: bool,
) {
    let sx = src.size.x as i32;
    let sy = src.size.y as i32;
    let sz = src.size.z as i32;

    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                let id = src.get(IVec3::new(x, y, z));
                let base = to_hi(x, y, z);

                if id == VoxelId::HEADLIGHT_GLASS {
                    // Add blackout hood brow on top & chrome steel bezel corners around glowing lamp
                    dst.set(base + IVec3::new(0, 1, 0), VoxelId::STEEL_DARK_TRIM);
                    dst.set(base + IVec3::new(1, 1, 0), VoxelId::STEEL_DARK_TRIM);
                    dst.set(base + IVec3::new(0, 0, 1), VoxelId::STEEL_BRIGHT);
                    dst.set(base + IVec3::new(1, 0, 1), VoxelId::STEEL_BRIGHT);
                } else if id == VoxelId::GLASS_OPTIC {
                    // Anti-reflective cyan glass glint in upper-left sub-voxel of each prism
                    dst.set(base + IVec3::new(0, 1, 0), VoxelId::OPTIC_GLINT);
                    dst.set(base + IVec3::new(0, 1, 1), VoxelId::OPTIC_GLINT);
                } else if is_hull && id == VoxelId::EMBLEM_BLACK && src.get(IVec3::new(x, y + 1, z)).is_air() {
                    // Rear engine deck cooling louver slats at 5 cm pitch
                    let z_rel = z as f32 - src.pivot.z;
                    if z_rel > 0.0 {
                        dst.set(base + IVec3::new(0, 1, 0), VoxelId::EMBLEM_BLACK);
                        dst.set(base + IVec3::new(1, 1, 0), VoxelId::EMBLEM_BLACK);
                        dst.set(base + IVec3::new(0, 1, 1), VoxelId::STEEL_DARK_TRIM);
                        dst.set(base + IVec3::new(1, 1, 1), VoxelId::STEEL_DARK_TRIM);
                    }
                }
            }
        }
    }

    // Subtle top-lip dry-brush edge highlights on flat roof/glacis/hatch rims
    let d_sx = dst.size.x as i32;
    let d_sy = dst.size.y as i32;
    let d_sz = dst.size.z as i32;
    let main_id = theme.main();
    let light_id = theme.light();

    for y in 4..(d_sy - 1) {
        for z in 1..(d_sz - 1) {
            for x in 1..(d_sx - 1) {
                let p = IVec3::new(x, y, z);
                if dst.get(p) != main_id {
                    continue;
                }
                if !dst.get(p + IVec3::Y).is_air() {
                    continue;
                }
                // Highlight crisp exposed upper roof edges (air on one lateral side, armor on opposite)
                let edge_x = (dst.get(p + IVec3::X).is_air() && !dst.get(p + IVec3::NEG_X).is_air())
                    || (dst.get(p + IVec3::NEG_X).is_air() && !dst.get(p + IVec3::X).is_air());
                let edge_z = (dst.get(p + IVec3::Z).is_air() && !dst.get(p + IVec3::NEG_Z).is_air())
                    || (dst.get(p + IVec3::NEG_Z).is_air() && !dst.get(p + IVec3::Z).is_air());
                if edge_x || edge_z {
                    dst.set(p, light_id);
                }
            }
        }
    }
}

/// Adds realistic `0.05 m` hull stowage & battlefield fittings safely anchored to each tank's actual geometry:
/// - Rear red convoy tail lamps on both rear fender corners
/// - Pioneer shovel (wooden handle + bright steel spade) on left fender top
/// - Coiled steel tow cable with looped eyes on right fender top
/// - Rolled canvas tarp with leather straps across rear upper deck edge
fn add_hull_stowage_and_fittings(src: &VoxelModel, dst: &mut VoxelModel, theme: ArmorTheme) {
    let sx = dst.size.x as i32;
    let sy = dst.size.y as i32;
    let sz = dst.size.z as i32;
    let cx = dst.pivot.x.round() as i32;

    // Find true solid hull Z bounds & outer fender X bounds
    let mut z_min = sz;
    let mut z_max = 0;
    let mut max_half_x = 0;
    for y in 4..sy {
        for z in 0..sz {
            for x in 0..sx {
                if !dst.get(IVec3::new(x, y, z)).is_air() {
                    z_min = z_min.min(z);
                    z_max = z_max.max(z);
                    max_half_x = max_half_x.max((x - cx).abs());
                }
            }
        }
    }
    if z_max <= z_min + 20 || max_half_x < 14 {
        return;
    }

    // 1. Rear red convoy tail lamps on left & right rear corners
    let tail_x_off = (max_half_x - 4).max(8);
    for &sign in &[-1, 1] {
        let tx = cx + sign * tail_x_off;
        // Scan inward from z_max to find solid rear armor plate at mid-upper hull height
        'tail_scan: for z in (z_max - 8..=z_max).rev() {
            for y in (12..(sy - 4).min(28)).rev() {
                if !dst.get(IVec3::new(tx, y, z)).is_air()
                    && dst.get(IVec3::new(tx, y, z + 1)).is_air()
                {
                    dst.set(IVec3::new(tx, y, z + 1), VoxelId::EMBLEM_RED);
                    dst.set(IVec3::new(tx, y + 1, z + 1), VoxelId::STEEL_DARK_TRIM);
                    break 'tail_scan;
                }
            }
        }
    }

    // Helper: find top solid surface Y at (x, z) inside fender zone (y in 12..=24)
    let fender_top_y = |m: &VoxelModel, x: i32, z: i32| -> Option<i32> {
        for y in (12..=24).rev() {
            if !m.get(IVec3::new(x, y, z)).is_air() && m.get(IVec3::new(x, y + 1, z)).is_air() {
                // Ensure 2 clear voxels above so we don't clip into overhanging sponson/skirts
                if m.get(IVec3::new(x, y + 2, z)).is_air() {
                    return Some(y);
                }
                return None;
            }
        }
        None
    };

    // 2. Left fender pioneer shovel (wooden handle + bright steel spade blade)
    let left_fx = cx - (max_half_x - 2);
    let mid_z = (z_min + z_max) / 2;
    if let Some(fy) = fender_top_y(dst, left_fx, mid_z) {
        let clear = (-6..=6).all(|dz| {
            !dst.get(IVec3::new(left_fx, fy, mid_z + dz)).is_air()
                && dst.get(IVec3::new(left_fx, fy + 1, mid_z + dz)).is_air()
                && dst.get(IVec3::new(left_fx, fy + 2, mid_z + dz)).is_air()
        });
        if clear {
            // Wooden shovel shaft
            for dz in -2..=5 {
                dst.set(IVec3::new(left_fx, fy + 1, mid_z + dz), VoxelId::WOOD_TOOL);
            }
            // Steel mounting clamp
            dst.set(IVec3::new(left_fx, fy + 1, mid_z + 1), VoxelId::STEEL_DARK_TRIM);
            // Bright steel spade blade at front
            for dz in -6..=-3 {
                dst.set(IVec3::new(left_fx, fy + 1, mid_z + dz), VoxelId::STEEL_BRIGHT);
                if left_fx + 1 < sx && dst.get(IVec3::new(left_fx + 1, fy + 1, mid_z + dz)).is_air() {
                    dst.set(IVec3::new(left_fx + 1, fy + 1, mid_z + dz), VoxelId::STEEL_BRIGHT);
                }
            }
        }
    }

    // 3. Right fender braided steel tow cable with looped end eyes
    let right_fx = cx + (max_half_x - 2);
    if let Some(fy) = fender_top_y(dst, right_fx, mid_z) {
        let clear = (-8..=8).all(|dz| {
            !dst.get(IVec3::new(right_fx, fy, mid_z + dz)).is_air()
                && dst.get(IVec3::new(right_fx, fy + 1, mid_z + dz)).is_air()
                && dst.get(IVec3::new(right_fx, fy + 2, mid_z + dz)).is_air()
        });
        if clear {
            for dz in -8i32..=8 {
                let mat = if dz.abs() >= 7 {
                    VoxelId::STEEL_BRIGHT
                } else if (dz + 8) % 2 == 0 {
                    VoxelId::STEEL_DARK_TRIM
                } else {
                    VoxelId::STEEL_BRIGHT
                };
                dst.set(IVec3::new(right_fx, fy + 1, mid_z + dz), mat);
            }
        }
    }

    // 4. Rolled canvas field tarp strapped across the rear engine deck edge (non-USSR, since USSR has unditching logs!)
    if theme != ArmorTheme::Ussr {
        let tarp_z = z_max - 5;
        let half_w = (max_half_x / 3).clamp(5, 8);
        // Find uniform flat deck height near rear centerline
        let mut deck_y = None;
        for y in (14..=28).rev() {
            if !dst.get(IVec3::new(cx, y, tarp_z)).is_air()
                && dst.get(IVec3::new(cx, y + 1, tarp_z)).is_air()
            {
                deck_y = Some(y);
                break;
            }
        }
        if let Some(dy) = deck_y {
            let flat_and_clear = (-half_w..=half_w).all(|dx| {
                !dst.get(IVec3::new(cx + dx, dy, tarp_z)).is_air()
                    && dst.get(IVec3::new(cx + dx, dy + 1, tarp_z)).is_air()
                    && dst.get(IVec3::new(cx + dx, dy + 2, tarp_z)).is_air()
                    && dst.get(IVec3::new(cx + dx, dy + 1, tarp_z + 1)).is_air()
            });
            if flat_and_clear {
                for dx in -half_w..=half_w {
                    let is_strap = dx.abs() == half_w / 2;
                    let mat = if is_strap {
                        VoxelId::STEEL_DARK_TRIM
                    } else {
                        VoxelId::CANVAS_TARP
                    };
                    dst.set(IVec3::new(cx + dx, dy + 1, tarp_z), mat);
                    dst.set(IVec3::new(cx + dx, dy + 1, tarp_z + 1), mat);
                    if !is_strap {
                        dst.set(IVec3::new(cx + dx, dy + 2, tarp_z), VoxelId::CANVAS_TARP);
                    } else {
                        dst.set(IVec3::new(cx + dx, dy + 2, tarp_z), VoxelId::STEEL_DARK_TRIM);
                    }
                }
            }
        }
    }
    let _ = src;
}

/// Adds crisp `0.05 m` nation-specific turret fittings on full-size turrets (skipped on casemate mantlets):
/// - Germany: Side-mounted `Nebelkerzen` smoke grenade launcher tubes on both front cheeks
/// - USSR: Iconic `desant` infantry grab handrails along both rear turret flanks
/// - USA: Rolled canvas bedroll strapped across rear turret bustle + bright antenna spring mount
fn add_turret_micro_fittings(src: &VoxelModel, dst: &mut VoxelModel, theme: ArmorTheme) {
    let sx = dst.size.x as i32;
    let sy = dst.size.y as i32;
    let sz = dst.size.z as i32;
    let cx = dst.pivot.x.round() as i32;
    let cz = dst.pivot.z.round() as i32;

    // Measure actual solid turret bounding box in 0.05 m voxels
    let mut min_x = sx;
    let mut max_x = 0;
    let mut min_y = sy;
    let mut max_y = 0;
    let mut min_z = sz;
    let mut max_z = 0;
    for y in 0..sy {
        for z in 0..sz {
            for x in 0..sx {
                if !dst.get(IVec3::new(x, y, z)).is_air() {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    min_z = min_z.min(z);
                    max_z = max_z.max(z);
                }
            }
        }
    }
    let w = max_x - min_x;
    let h = max_y - min_y;
    let l = max_z - min_z;

    // Skip compact casemate gun mantlets (StuG III G, Jagdtiger)
    if w < 24 || l < 26 || h < 10 {
        return;
    }

    let mid_y = (min_y + (h * 45) / 100).clamp(min_y + 3, max_y - 3);

    match theme {
        ArmorTheme::GermanyGrey | ArmorTheme::GermanyTan => {
            // Twin/triple `Nebelkerzen` smoke grenade launcher racks on both front turret cheeks
            let smoke_z = (min_z + l / 4).min(cz - 2);
            for &sign in &[-1, 1] {
                let start_x = if sign > 0 { max_x } else { min_x };
                // Find outer turret cheek surface at (mid_y, smoke_z)
                for step in 0..10 {
                    let x = start_x - sign * step;
                    if !dst.get(IVec3::new(x, mid_y, smoke_z)).is_air()
                        && dst.get(IVec3::new(x + sign, mid_y, smoke_z)).is_air()
                    {
                        let ox = x + sign;
                        let ox2 = (x + sign * 2).clamp(0, sx - 1);
                        // Mounting steel bracket
                        dst.set(IVec3::new(ox, mid_y, smoke_z), VoxelId::STEEL_DARK_TRIM);
                        dst.set(IVec3::new(ox, mid_y, smoke_z + 1), VoxelId::STEEL_DARK_TRIM);
                        // 3 angled smoke mortar tubes pointing forward-up
                        for (i, dz) in [-1i32, 0, 1].iter().enumerate() {
                            let ty = mid_y + 1 + (i as i32 % 2);
                            let tz = smoke_z + dz;
                            dst.set(IVec3::new(ox2, ty, tz), VoxelId::STEEL_DARK_TRIM);
                            dst.set(IVec3::new(ox2, ty + 1, tz - 1), VoxelId::EMBLEM_BLACK);
                        }
                        break;
                    }
                }
            }
        }
        ArmorTheme::Ussr => {
            // Iconic Soviet `desant` infantry grab handrails along both rear turret flanks
            let rail_z0 = (cz - 2).max(min_z + 6);
            let rail_z1 = (max_z - 5).min(rail_z0 + 10);
            if rail_z1 > rail_z0 + 4 {
                for &sign in &[-1, 1] {
                    // Find outermost armor surface along the rail span
                    let mut rail_x: Option<i32> = None;
                    for z in rail_z0..=rail_z1 {
                        let start_x = if sign > 0 { max_x } else { min_x };
                        for step in 0..8 {
                            let x = start_x - sign * step;
                            if !dst.get(IVec3::new(x, mid_y, z)).is_air() {
                                let candidate = x + sign * 2;
                                rail_x = Some(match rail_x {
                                    Some(prev) => {
                                        if sign > 0 {
                                            prev.max(candidate)
                                        } else {
                                            prev.min(candidate)
                                        }
                                    }
                                    None => candidate,
                                });
                                break;
                            }
                        }
                    }
                    if let Some(rx) = rail_x
                        && rx >= 1
                        && rx < sx - 1
                    {
                        // Front & rear standoff brackets into turret armor
                        for &bz in &[rail_z0, rail_z1] {
                            dst.set(IVec3::new(rx - sign, mid_y, bz), VoxelId::STEEL_DARK_TRIM);
                            dst.set(IVec3::new(rx, mid_y, bz), VoxelId::STEEL_DARK_TRIM);
                        }
                        // Horizontal grab rail bar in bright worn steel
                        for z in rail_z0..=rail_z1 {
                            if dst.get(IVec3::new(rx, mid_y, z)).is_air() {
                                dst.set(IVec3::new(rx, mid_y, z), VoxelId::STEEL_BRIGHT);
                            }
                        }
                    }
                }
            }
        }
        ArmorTheme::Usa => {
            // Rolled canvas stowage bedroll strapped across rear turret bustle wall
            let half_w = (w / 4).clamp(4, 7);
            let bustle_y = (min_y + h / 2).clamp(min_y + 3, max_y - 3);
            let mut rear_z = None;
            for z in (cz..=max_z).rev() {
                if !dst.get(IVec3::new(cx, bustle_y, z)).is_air()
                    && dst.get(IVec3::new(cx, bustle_y, z + 1)).is_air()
                {
                    rear_z = Some(z);
                    break;
                }
            }
            if let Some(rz) = rear_z
                && rz + 2 < sz
            {
                let supported = (-half_w..=half_w)
                    .all(|dx| !dst.get(IVec3::new(cx + dx, bustle_y, rz)).is_air());
                if supported {
                    for dx in -half_w..=half_w {
                        let is_strap = dx.abs() == half_w / 2;
                        let mat = if is_strap {
                            VoxelId::STEEL_DARK_TRIM
                        } else {
                            VoxelId::CANVAS_TARP
                        };
                        dst.set(IVec3::new(cx + dx, bustle_y, rz + 1), mat);
                        dst.set(IVec3::new(cx + dx, bustle_y + 1, rz + 1), mat);
                    }
                }
            }
        }
    }
    let _ = src;
}
