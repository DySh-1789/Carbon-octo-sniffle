use glam::{IVec3, UVec3, Vec3};
use serde::{Deserialize, Serialize};

use crate::chunk::TANK_VOXEL_SIZE;
use crate::material::VoxelId;

/// Finite 3D voxel grid for movable rigid vehicle parts (hull, turret, gun barrel).
#[derive(Clone, Debug)]
pub struct VoxelModel {
    pub size: UVec3,
    /// Pivot offset in voxel coordinates; vertex positions are shifted by `-pivot * voxel_size`.
    pub pivot: Vec3,
    pub voxel_size: f32,
    pub voxels: Vec<VoxelId>,
}

impl VoxelModel {
    pub fn new(size: UVec3, pivot: Vec3, voxel_size: f32) -> Self {
        let len = (size.x * size.y * size.z) as usize;
        Self {
            size,
            pivot,
            voxel_size,
            voxels: vec![VoxelId::AIR; len],
        }
    }

    #[inline]
    fn idx(&self, x: u32, y: u32, z: u32) -> usize {
        (x + z * self.size.x + y * self.size.x * self.size.z) as usize
    }

    #[inline]
    pub fn in_bounds(&self, pos: IVec3) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && pos.z >= 0
            && (pos.x as u32) < self.size.x
            && (pos.y as u32) < self.size.y
            && (pos.z as u32) < self.size.z
    }

    #[inline]
    pub fn get(&self, pos: IVec3) -> VoxelId {
        if !self.in_bounds(pos) {
            return VoxelId::AIR;
        }
        self.voxels[self.idx(pos.x as u32, pos.y as u32, pos.z as u32)]
    }

    #[inline]
    pub fn set(&mut self, pos: IVec3, id: VoxelId) {
        if self.in_bounds(pos) {
            let idx = self.idx(pos.x as u32, pos.y as u32, pos.z as u32);
            self.voxels[idx] = id;
        }
    }

    /// Inclusive integer box fill.
    pub fn fill_box(&mut self, min: IVec3, max: IVec3, id: VoxelId) {
        let min = min.max(IVec3::ZERO);
        let max = max.min(self.size.as_ivec3() - IVec3::ONE);
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                for x in min.x..=max.x {
                    self.set(IVec3::new(x, y, z), id);
                }
            }
        }
    }

    pub fn non_air_count(&self) -> usize {
        self.voxels.iter().filter(|v| !v.is_air()).count()
    }
}

/// Complete articulated voxel tank visual assembly (built in 0.2 m tank-voxel scale).
#[derive(Clone, Debug)]
pub struct TankVoxelModels {
    pub hull: VoxelModel,
    pub turret: VoxelModel,
    pub barrel: VoxelModel,
    /// Turret pivot offset relative to hull origin (in meters).
    pub turret_mount_offset: Vec3,
    /// Gun trunnion pivot offset relative to turret pivot (in meters).
    pub gun_mount_offset: Vec3,
    /// Muzzle tip offset relative to gun trunnion pivot along `-Z` (in meters).
    pub muzzle_offset: Vec3,
}

/// Iconic hull bow & glacis geometry archetypes researched from real WWII vehicles.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HullFrontStyle {
    /// Vertical/stepped driver visor plate (Pz.II, Pz.III, Pz.IV, Tiger H1, KV-1, M2A4, M3 Stuart).
    BoxyStepped,
    /// Clean sloped upper front glacis (T-34, T-34-85, Panther, Tiger II, T-44-100, M4A3E8/E2, M10, M26, T34).
    SlopedGlacis,
    /// Rounded cast Sherman hull shoulders (M4A1 Sherman).
    CastRounded,
    /// Iconic Soviet V-shaped pike nose (IS-3).
    PikeNose,
    /// Narrow tapered Christie light-tank bow (BT-5, T-50).
    TruncatedPyramid,
    /// Steep heavy bow with stepped driver visor block (IS-2).
    SteppedHeavy,
}

/// Iconic turret & casemate superstructure archetypes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurretStyle {
    /// Compact cylindrical light turret (BT-5, Pz.II).
    CylindricalSmall,
    /// Sloped hexagonal Soviet early-war turret (T-34 1941, T-50).
    HexagonalSloped,
    /// Rounded cast late-war Soviet turret (T-34-85, IS-2, T-44-100).
    RoundedSoviet,
    /// Flattened hemispherical frying-pan dome turret (IS-3).
    LowDomeIS3,
    /// Angular German medium turret with rear Rommelkiste stowage bin (Pz.III J, Pz.IV F2).
    BoxyGerman,
    /// Wide horseshoe heavy turret with drum cupola (Tiger H1).
    HorseshoeTiger,
    /// Long sloped German heavy turret with angled cheeks (Panther A, Tiger II H).
    SlopedGermanHeavy,
    /// Rounded US Sherman cast turret (M4A1, M4A3E8, M4A3E2 Jumbo).
    RoundedSherman,
    /// Heavy US cast turret with elongated counterweight bustle (M26 Pershing, T34 Heavy).
    HeavyAmerican,
    /// Open-top pentagonal tank destroyer turret with rear counterweights (M10 Wolverine).
    OpenTopPentagonal,
    /// Tall boxy welded heavy turret with wide gun shield (KV-1 ZiS-5).
    TallBoxyKV,
    /// Low-profile assault gun casemate with cast Saukopf mantlet (StuG III Ausf. G).
    CasemateLow,
    /// Massive heavy casemate superstructure on Tiger II hull (Jagdtiger).
    CasemateHeavy,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuzzleBrakeStyle {
    None,
    DoubleBaffle,
    TShaped,
}

/// Parametric `0.2 m` voxel recipe describing a historical tank's recognizable silhouette.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoxelTankBlueprint {
    pub hull_w: u32,
    pub hull_h: u32,
    pub hull_l: u32,
    pub track_w: u32,
    pub hull_front: HullFrontStyle,
    pub side_skirts: bool,
    pub rear_fuel_drums: bool,
    pub hull_mg_port: bool,
    pub turret_style: TurretStyle,
    pub turret_w: u32,
    pub turret_h: u32,
    pub turret_l: u32,
    /// Turret Z offset from hull center in voxels (negative = forward, positive = rear).
    pub turret_z_offset: i32,
    /// Oversized heavy gun mantlet (Jumbo 76, Tiger H1, T34, Jagdtiger).
    pub heavy_mantlet: bool,
    pub barrel_len: u32,
    pub barrel_thick: bool,
    pub muzzle_brake: MuzzleBrakeStyle,
}

/// Builds a detailed `0.2 m` voxel tank assembly (`hull`, `turret`, `barrel`) from a historical
/// [`VoxelTankBlueprint`].
pub fn build_tank_models(bp: &VoxelTankBlueprint) -> TankVoxelModels {
    let vs = TANK_VOXEL_SIZE;

    let hw = bp.hull_w.max(11);
    let hh = bp.hull_h.max(6);
    let hl = bp.hull_l.max(20);
    let tw = bp.track_w.clamp(2, 4);
    let is_casemate = matches!(
        bp.turret_style,
        TurretStyle::CasemateLow | TurretStyle::CasemateHeavy
    );

    // For casemates, the fighting compartment is sculpted directly onto the hull roof!
    let extra_casemate_h = match bp.turret_style {
        TurretStyle::CasemateLow => 3,
        TurretStyle::CasemateHeavy => 5,
        _ => 1,
    };
    let hull_grid_h = hh + extra_casemate_h;
    let mut hull = VoxelModel::new(
        UVec3::new(hw, hull_grid_h, hl),
        Vec3::new(hw as f32 * 0.5, 0.0, hl as f32 * 0.5),
        vs,
    );

    let w_i = hw as i32;
    let h_i = hh as i32;
    let l_i = hl as i32;
    let tw_i = tw as i32;
    let inner_x0 = tw_i;
    let inner_x1 = w_i - 1 - tw_i;

    // ------------------------------------------------------------------
    // 1. TRACKS, ROAD WHEELS, FENDERS & OPTIONAL SIDE SKIRTS
    // ------------------------------------------------------------------
    for &(tx0, tx1, is_left) in &[(0, tw_i - 1, true), (w_i - tw_i, w_i - 1, false)] {
        let track_top_y = (h_i / 2).clamp(3, 4);
        // Bottom & top runs
        hull.fill_box(IVec3::new(tx0, 0, 3), IVec3::new(tx1, 0, l_i - 4), VoxelId::STEEL_TRACK);
        hull.fill_box(
            IVec3::new(tx0, track_top_y, 1),
            IVec3::new(tx1, track_top_y, l_i - 2),
            VoxelId::STEEL_TRACK,
        );
        // Front & rear sloped idler/sprocket ascents
        hull.fill_box(IVec3::new(tx0, 1, 1), IVec3::new(tx1, 2, 2), VoxelId::STEEL_TRACK);
        hull.fill_box(
            IVec3::new(tx0, 2, 0),
            IVec3::new(tx1, track_top_y, 1),
            VoxelId::STEEL_TRACK,
        );
        hull.fill_box(
            IVec3::new(tx0, 1, l_i - 3),
            IVec3::new(tx1, 2, l_i - 2),
            VoxelId::STEEL_TRACK,
        );
        hull.fill_box(
            IVec3::new(tx0, 2, l_i - 2),
            IVec3::new(tx1, track_top_y, l_i - 1),
            VoxelId::STEEL_TRACK,
        );

        // Suspension road wheels
        hull.fill_box(
            IVec3::new(tx0, 1, 3),
            IVec3::new(tx1, track_top_y - 1, l_i - 4),
            VoxelId::STEEL_OLIVE_DARK,
        );
        let outer_x = if is_left { 0 } else { w_i - 1 };
        let mut wz = 4;
        while wz <= l_i - 5 {
            hull.fill_box(
                IVec3::new(tx0, 1, wz - 1),
                IVec3::new(tx1, (track_top_y - 1).max(1), wz),
                VoxelId::RUBBER_WHEEL,
            );
            hull.set(IVec3::new(outer_x, 1, wz), VoxelId::STEEL_ACCENT);
            wz += 4;
        }

        // Mudguard fenders
        let fender_y = (track_top_y + 1).min(h_i - 2);
        hull.fill_box(
            IVec3::new(tx0, fender_y, 1),
            IVec3::new(tx1, fender_y, l_i - 2),
            VoxelId::STEEL_OLIVE,
        );

        // Optional armored side skirts (Schürzen on StuG/Panther/Tiger II/Jagdtiger, skirts on T-44-100)
        if bp.side_skirts {
            hull.fill_box(
                IVec3::new(outer_x, (track_top_y - 1).max(2), 3),
                IVec3::new(outer_x, fender_y, l_i - 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
        }
    }

    // ------------------------------------------------------------------
    // 2. HULL BODY & ARCHETYPE-SPECIFIC FRONT GLACIS
    // ------------------------------------------------------------------
    let lower_top = (h_i / 2).clamp(3, 4);
    let roof_y = h_i - 1;
    // Lower hull tub
    hull.fill_box(
        IVec3::new(inner_x0, 1, 4),
        IVec3::new(inner_x1, lower_top, l_i - 3),
        VoxelId::STEEL_OLIVE_DARK,
    );
    // Main upper hull fighting & engine compartment
    hull.fill_box(
        IVec3::new(inner_x0, lower_top + 1, 5),
        IVec3::new(inner_x1, roof_y, l_i - 3),
        VoxelId::STEEL_OLIVE,
    );
    // Sponson overhangs over tracks for wider hulls
    if inner_x0 > 1 {
        hull.fill_box(
            IVec3::new(inner_x0 - 1, lower_top + 1, 6),
            IVec3::new(inner_x0 - 1, roof_y - 1, l_i - 5),
            VoxelId::STEEL_OLIVE,
        );
        hull.fill_box(
            IVec3::new(inner_x1 + 1, lower_top + 1, 6),
            IVec3::new(inner_x1 + 1, roof_y - 1, l_i - 5),
            VoxelId::STEEL_OLIVE,
        );
    }

    match bp.hull_front {
        HullFrontStyle::BoxyStepped => {
            // Lower nose plate
            hull.fill_box(
                IVec3::new(inner_x0, 2, 1),
                IVec3::new(inner_x1, lower_top, 3),
                VoxelId::STEEL_OLIVE_DARK,
            );
            // Vertical driver plate at z = 4..5
            hull.fill_box(
                IVec3::new(inner_x0, lower_top + 1, 4),
                IVec3::new(inner_x1, roof_y, 5),
                VoxelId::STEEL_OLIVE,
            );
            // Driver vision slit
            hull.fill_box(
                IVec3::new(inner_x0 + 1, roof_y - 1, 4),
                IVec3::new(inner_x0 + 3, roof_y - 1, 4),
                VoxelId::GLASS_OPTIC,
            );
        }
        HullFrontStyle::SlopedGlacis => {
            // Sloped lower glacis
            hull.fill_box(
                IVec3::new(inner_x0, 2, 2),
                IVec3::new(inner_x1, lower_top, 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
            // Multi-step clean sloped upper glacis (T-34, Panther, Tiger II, Sherman E8/Jumbo, T-44-100)
            let steps = (roof_y - lower_top).max(2);
            for s in 0..steps {
                let y = lower_top + 1 + s;
                let z0 = 1 + s;
                hull.fill_box(
                    IVec3::new(inner_x0, y, z0),
                    IVec3::new(inner_x1, y, 5),
                    VoxelId::STEEL_OLIVE,
                );
            }
            hull.fill_box(
                IVec3::new(inner_x0 + 1, roof_y - 1, 2.min(steps)),
                IVec3::new(inner_x0 + 2, roof_y - 1, 2.min(steps)),
                VoxelId::GLASS_OPTIC,
            );
        }
        HullFrontStyle::CastRounded => {
            // Rounded cast M4A1 Sherman hull with curved upper corners
            hull.fill_box(
                IVec3::new(inner_x0, 2, 1),
                IVec3::new(inner_x1, lower_top, 3),
                VoxelId::STEEL_OLIVE_DARK,
            );
            hull.fill_box(
                IVec3::new(inner_x0, lower_top + 1, 2),
                IVec3::new(inner_x1, lower_top + 1, 5),
                VoxelId::STEEL_OLIVE,
            );
            hull.fill_box(
                IVec3::new(inner_x0 + 1, lower_top + 2, 3),
                IVec3::new(inner_x1 - 1, roof_y, 5),
                VoxelId::STEEL_OLIVE,
            );
            // Round off upper roof edges
            hull.fill_box(
                IVec3::new(inner_x0, roof_y, 5),
                IVec3::new(inner_x0, roof_y, l_i - 5),
                VoxelId::AIR,
            );
            hull.fill_box(
                IVec3::new(inner_x1, roof_y, 5),
                IVec3::new(inner_x1, roof_y, l_i - 5),
                VoxelId::AIR,
            );
            hull.set(IVec3::new(inner_x0 + 2, roof_y - 1, 3), VoxelId::GLASS_OPTIC);
        }
        HullFrontStyle::PikeNose => {
            // Unmistakable IS-3 V-shaped angled pike nose!
            let mid_x = w_i / 2;
            for x in inner_x0..=inner_x1 {
                let dist_center = (x - mid_x).abs();
                let z_nose = (dist_center / 2).clamp(0, 4);
                hull.fill_box(
                    IVec3::new(x, 2, z_nose + 1),
                    IVec3::new(x, lower_top, 5),
                    VoxelId::STEEL_OLIVE_DARK,
                );
                for y in (lower_top + 1)..=roof_y {
                    let y_off = y - (lower_top + 1);
                    hull.fill_box(
                        IVec3::new(x, y, (z_nose + y_off).min(5)),
                        IVec3::new(x, y, 5),
                        VoxelId::STEEL_OLIVE,
                    );
                }
            }
            hull.set(IVec3::new(mid_x, roof_y - 1, 2), VoxelId::GLASS_OPTIC);
        }
        HullFrontStyle::TruncatedPyramid => {
            // Narrow tapered nose (BT-5 / T-50)
            let mid_x = w_i / 2;
            let half_inner = ((inner_x1 - inner_x0) / 2).max(2);
            for z in 1..=4 {
                let shrink = (4 - z).min(half_inner - 1);
                hull.fill_box(
                    IVec3::new(inner_x0 + shrink, 2, z),
                    IVec3::new(inner_x1 - shrink, roof_y - (4 - z).min(2), z),
                    VoxelId::STEEL_OLIVE,
                );
            }
            hull.set(IVec3::new(mid_x, roof_y - 1, 3), VoxelId::GLASS_OPTIC);
        }
        HullFrontStyle::SteppedHeavy => {
            // IS-2 stepped heavy glacis with distinct central armored driver visor box
            hull.fill_box(
                IVec3::new(inner_x0, 2, 1),
                IVec3::new(inner_x1, lower_top, 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
            hull.fill_box(
                IVec3::new(inner_x0, lower_top + 1, 2),
                IVec3::new(inner_x1, lower_top + 1, 5),
                VoxelId::STEEL_OLIVE,
            );
            hull.fill_box(
                IVec3::new(inner_x0 + 1, lower_top + 2, 4),
                IVec3::new(inner_x1 - 1, roof_y, 5),
                VoxelId::STEEL_OLIVE,
            );
            let mid_x = w_i / 2;
            hull.fill_box(
                IVec3::new(mid_x - 1, roof_y - 1, 3),
                IVec3::new(mid_x + 1, roof_y, 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
            hull.set(IVec3::new(mid_x, roof_y - 1, 3), VoxelId::GLASS_OPTIC);
        }
    }

    // Bow machine gun ball mount
    if bp.hull_mg_port {
        hull.set(IVec3::new(inner_x1 - 2, roof_y - 1, 3), VoxelId::STEEL_GUN);
        hull.set(IVec3::new(inner_x1 - 2, roof_y - 1, 2), VoxelId::STEEL_GUN);
    }

    // Rear engine deck cooling louvers & exhausts
    let eng_z0 = (l_i * 2 / 3).min(l_i - 6);
    let eng_z1 = l_i - 4;
    for gx in (inner_x0 + 1)..=(inner_x1 - 1) {
        if (gx - inner_x0) % 2 == 1 {
            hull.fill_box(
                IVec3::new(gx, roof_y, eng_z0),
                IVec3::new(gx, roof_y, eng_z1),
                VoxelId::STEEL_TRACK,
            );
        }
    }
    hull.fill_box(
        IVec3::new(inner_x0 + 1, roof_y - 1, l_i - 2),
        IVec3::new(inner_x0 + 2, roof_y - 1, l_i - 2),
        VoxelId::STEEL_TRACK,
    );
    hull.fill_box(
        IVec3::new(inner_x1 - 2, roof_y - 1, l_i - 2),
        IVec3::new(inner_x1 - 1, roof_y - 1, l_i - 2),
        VoxelId::STEEL_TRACK,
    );

    // Optional external cylindrical Soviet fuel drums on rear fenders
    if bp.rear_fuel_drums {
        let drum_z0 = l_i - 9;
        let drum_z1 = l_i - 5;
        hull.fill_box(
            IVec3::new(0, roof_y - 1, drum_z0),
            IVec3::new(1, roof_y, drum_z1),
            VoxelId::STEEL_OLIVE_DARK,
        );
        hull.fill_box(
            IVec3::new(w_i - 2, roof_y - 1, drum_z0),
            IVec3::new(w_i - 1, roof_y, drum_z1),
            VoxelId::STEEL_OLIVE_DARK,
        );
    }

    // Front fender headlights
    hull.set(IVec3::new(1, lower_top + 2, 2), VoxelId::STEEL_ACCENT);
    hull.set(IVec3::new(w_i - 2, lower_top + 2, 2), VoxelId::STEEL_ACCENT);

    // ------------------------------------------------------------------
    // 3. CASEMATE SUPERSTRUCTURE OR TURRET RING + TURRET MODEL
    // ------------------------------------------------------------------
    let turret_center_z = (l_i / 2 + bp.turret_z_offset).clamp(5, l_i - 6);
    let (turret, turret_mount_offset, gun_mount_offset) = if is_casemate {
        let case_h = extra_casemate_h as i32;
        let case_w_pad = if bp.turret_style == TurretStyle::CasemateHeavy {
            1
        } else {
            2
        };
        let cx0 = case_w_pad.min(inner_x0);
        let cx1 = (w_i - 1 - case_w_pad).max(inner_x1);
        let cz0 = (l_i / 4).max(5);
        let cz1 = (l_i * 2 / 3).min(l_i - 7);

        for dy in 1..=case_h {
            let inset_x = if dy >= case_h - 1 { 1 } else { 0 };
            let inset_z = dy / 2;
            hull.fill_box(
                IVec3::new(cx0 + inset_x, roof_y + dy, cz0 + inset_z),
                IVec3::new(cx1 - inset_x, roof_y + dy, cz1 - inset_z / 2),
                VoxelId::STEEL_OLIVE,
            );
        }
        // Commander hatch/cupola on casemate roof
        hull.fill_box(
            IVec3::new(cx0 + 2, roof_y + case_h, cz1 - 3),
            IVec3::new(cx0 + 4, roof_y + case_h, cz1 - 1),
            VoxelId::STEEL_OLIVE_DARK,
        );
        // Distinct gun mantlet block (Saukopf / heavy Tiger II casemate shield) on small pivoting turret stub
        let mut mantlet = VoxelModel::new(UVec3::new(5, 4, 4), Vec3::new(2.5, 2.0, 3.0), vs);
        mantlet.fill_box(IVec3::new(1, 0, 1), IVec3::new(3, 3, 3), VoxelId::STEEL_OLIVE_DARK);
        if bp.heavy_mantlet {
            mantlet.fill_box(IVec3::new(0, 0, 0), IVec3::new(4, 3, 2), VoxelId::STEEL_OLIVE_DARK);
        } else {
            // Rounded Saukopf ("pig's head") StuG mantlet
            mantlet.fill_box(IVec3::new(1, 1, 0), IVec3::new(3, 2, 1), VoxelId::STEEL_OLIVE_DARK);
        }

        let mount_y = (roof_y + 1 + case_h / 2) as f32 * vs;
        let mount_z = (cz0 as f32 - l_i as f32 * 0.5) * vs;
        (
            mantlet,
            Vec3::new(0.0, mount_y, mount_z),
            Vec3::new(0.0, 0.0, -2.5 * vs),
        )
    } else {
        // Turret ring collar on hull roof
        let ring_r = (bp.turret_w as i32 / 3).clamp(2, 4);
        let mid_x = w_i / 2;
        hull.fill_box(
            IVec3::new(mid_x - ring_r, roof_y + 1, turret_center_z - ring_r),
            IVec3::new(mid_x + ring_r, roof_y + 1, turret_center_z + ring_r),
            VoxelId::STEEL_OLIVE_DARK,
        );

        let turret = build_turret_model(bp);
        let mount_y = (roof_y + 1) as f32 * vs;
        let mount_z = (turret_center_z as f32 - l_i as f32 * 0.5) * vs;
        let gun_y = ((bp.turret_h as f32 * 0.42).max(1.8)) * vs;
        let gun_z = -((bp.turret_l as f32 * 0.5) - 0.5) * vs;
        (
            turret,
            Vec3::new(0.0, mount_y, mount_z),
            Vec3::new(0.0, gun_y, gun_z),
        )
    };

    // ------------------------------------------------------------------
    // 4. GUN BARREL + MUZZLE BRAKE
    // ------------------------------------------------------------------
    let bl = bp.barrel_len.clamp(10, 34);
    let mut barrel = VoxelModel::new(
        UVec3::new(3, 3, bl),
        Vec3::new(1.5, 1.5, (bl - 1) as f32),
        vs,
    );
    let bl_i = bl as i32;
    let breech_len = (bl_i / 5).clamp(2, 5);
    // Reinforced mantlet sleeve at trunnion
    barrel.fill_box(
        IVec3::new(0, 0, bl_i - breech_len),
        IVec3::new(2, 2, bl_i - 1),
        VoxelId::STEEL_OLIVE_DARK,
    );
    // Main gun tube
    barrel.fill_box(IVec3::new(1, 1, 1), IVec3::new(1, 1, bl_i - breech_len), VoxelId::STEEL_GUN);
    if bp.barrel_thick {
        // Heavier sleeve for 88mm / 100mm / 120mm / 122mm / 128mm cannons
        let sleeve_z = (bl_i / 2).max(4);
        barrel.fill_box(
            IVec3::new(0, 1, sleeve_z),
            IVec3::new(2, 1, bl_i - breech_len),
            VoxelId::STEEL_GUN,
        );
        barrel.fill_box(
            IVec3::new(1, 0, sleeve_z),
            IVec3::new(1, 2, bl_i - breech_len),
            VoxelId::STEEL_GUN,
        );
    } else {
        let base_z = (bl_i - breech_len - 3).max(2);
        barrel.fill_box(
            IVec3::new(0, 1, base_z),
            IVec3::new(2, 1, bl_i - breech_len),
            VoxelId::STEEL_GUN,
        );
    }

    match bp.muzzle_brake {
        MuzzleBrakeStyle::None => {
            barrel.set(IVec3::new(1, 1, 0), VoxelId::STEEL_TRACK);
        }
        MuzzleBrakeStyle::DoubleBaffle => {
            barrel.fill_box(IVec3::new(0, 1, 0), IVec3::new(2, 1, 1), VoxelId::STEEL_TRACK);
            barrel.set(IVec3::new(1, 0, 0), VoxelId::STEEL_TRACK);
            barrel.set(IVec3::new(1, 2, 0), VoxelId::STEEL_TRACK);
        }
        MuzzleBrakeStyle::TShaped => {
            // Iconic T-shaped US muzzle brake (M26 Pershing, M4A3E8, T34)
            barrel.fill_box(IVec3::new(0, 1, 0), IVec3::new(2, 1, 0), VoxelId::STEEL_TRACK);
            barrel.set(IVec3::new(1, 1, 1), VoxelId::STEEL_GUN);
        }
    }

    let muzzle_offset = Vec3::new(0.0, 0.0, -(bl as f32 - 0.5) * vs);

    TankVoxelModels {
        hull,
        turret,
        barrel,
        turret_mount_offset,
        gun_mount_offset,
        muzzle_offset,
    }
}

fn build_turret_model(bp: &VoxelTankBlueprint) -> VoxelModel {
    let vs = TANK_VOXEL_SIZE;
    let tw = bp.turret_w.clamp(6, 14);
    let th = bp.turret_h.clamp(4, 9);
    let tl = bp.turret_l.clamp(8, 22);
    let mut t = VoxelModel::new(
        UVec3::new(tw, th, tl),
        Vec3::new(tw as f32 * 0.5, 0.0, tl as f32 * 0.5),
        vs,
    );
    let w = tw as i32;
    let h = th as i32;
    let l = tl as i32;
    let mid_x = w / 2;

    match bp.turret_style {
        TurretStyle::CylindricalSmall => {
            t.fill_box(IVec3::new(1, 0, 1), IVec3::new(w - 2, h - 2, l - 2), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 2), IVec3::new(w - 1, h - 2, l - 3), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(2, h - 1, 2), IVec3::new(w - 3, h - 1, l - 3), VoxelId::STEEL_OLIVE_DARK);
        }
        TurretStyle::HexagonalSloped => {
            // Sloped T-34 1941 / T-50 turret tapering cleanly upward
            for y in 0..(h - 1) {
                let inset = (y / 2).min(2);
                t.fill_box(
                    IVec3::new(inset, y, 1 + inset),
                    IVec3::new(w - 1 - inset, y, l - 2 - inset / 2),
                    VoxelId::STEEL_OLIVE,
                );
            }
            t.fill_box(
                IVec3::new(2, h - 1, 3),
                IVec3::new(w - 3, h - 1, l - 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
        }
        TurretStyle::RoundedSoviet => {
            // Iconic T-34-85 / IS-2 / T-44-100 rounded cast turret with full rear bustle
            t.fill_box(IVec3::new(1, 0, 2), IVec3::new(w - 2, h - 2, l - 2), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 3), IVec3::new(w - 1, h - 3, l - 4), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(2, 0, 1), IVec3::new(w - 3, h - 2, 2), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(2, h - 2, 3), IVec3::new(w - 3, h - 2, l - 3), VoxelId::STEEL_OLIVE);
            // Commander cupola left-rear
            t.fill_box(
                IVec3::new(2, h - 1, l / 2),
                IVec3::new(4, h - 1, l / 2 + 2),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.set(IVec3::new(3, h - 1, l / 2), VoxelId::GLASS_OPTIC);
        }
        TurretStyle::LowDomeIS3 => {
            // Iconic flattened hemispherical IS-3 frying-pan dome!
            let cx = (w - 1) as f32 * 0.5;
            let cz = (l - 1) as f32 * 0.5;
            let rx = (w as f32 * 0.5).max(2.0);
            let rz = (l as f32 * 0.5).max(2.0);
            for y in 0..h {
                let yf = y as f32 / (h as f32).max(1.0);
                let shrink = (1.0 - yf * yf * 0.72).max(0.35);
                for z in 0..l {
                    for x in 0..w {
                        let dx = (x as f32 - cx) / (rx * shrink);
                        let dz = (z as f32 - cz) / (rz * shrink);
                        if dx * dx + dz * dz <= 1.0 {
                            t.set(IVec3::new(x, y, z), VoxelId::STEEL_OLIVE);
                        }
                    }
                }
            }
            // Low twin hatches on dome crown
            t.set(IVec3::new(mid_x - 1, h - 1, l / 2), VoxelId::STEEL_OLIVE_DARK);
            t.set(IVec3::new(mid_x + 1, h - 1, l / 2), VoxelId::STEEL_OLIVE_DARK);
        }
        TurretStyle::BoxyGerman => {
            // Pz.III J / Pz.IV F2 angular turret + distinct rear Rommelkiste stowage bin + tall cupola
            t.fill_box(IVec3::new(1, 0, 1), IVec3::new(w - 2, h - 2, l - 4), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 3), IVec3::new(w - 1, h - 2, l - 5), VoxelId::STEEL_OLIVE);
            // Rear Rommelkiste stowage box
            t.fill_box(
                IVec3::new(2, 1, l - 3),
                IVec3::new(w - 3, h - 3, l - 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
            // Tall German cylindrical commander cupola
            t.fill_box(
                IVec3::new(mid_x - 1, h - 1, l - 6),
                IVec3::new(mid_x + 1, h - 1, l - 4),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.set(IVec3::new(mid_x, h - 1, l - 6), VoxelId::GLASS_OPTIC);
        }
        TurretStyle::HorseshoeTiger => {
            // Iconic wide horseshoe Tiger H1 turret with curved rear + tall drum cupola
            t.fill_box(IVec3::new(0, 0, 2), IVec3::new(w - 1, h - 2, l - 4), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(1, 0, 1), IVec3::new(w - 2, h - 2, l - 2), VoxelId::STEEL_OLIVE);
            t.fill_box(
                IVec3::new(2, 1, l - 2),
                IVec3::new(w - 3, h - 2, l - 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
            // Tall Tiger H1 left-rear commander drum cupola
            t.fill_box(
                IVec3::new(2, h - 1, l / 2 + 1),
                IVec3::new(4, h - 1, l / 2 + 3),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.set(IVec3::new(3, h - 1, l / 2 + 1), VoxelId::GLASS_OPTIC);
        }
        TurretStyle::SlopedGermanHeavy => {
            // Panther A / Tiger II (H): long angled turret cheeks tapering forward to narrow thick face
            for z in 1..(l - 1) {
                let frac = z as f32 / l as f32;
                let side_inset = if frac < 0.25 { 2 } else if frac < 0.55 { 1 } else { 0 };
                for y in 0..(h - 1) {
                    let top_inset = if y >= h - 2 { 1 } else { 0 };
                    let x0 = (side_inset + top_inset).min(mid_x - 1);
                    let x1 = (w - 1 - side_inset - top_inset).max(mid_x + 1);
                    t.fill_box(IVec3::new(x0, y, z), IVec3::new(x1, y, z), VoxelId::STEEL_OLIVE);
                }
            }
            // Commander cupola
            t.fill_box(
                IVec3::new(2, h - 1, l / 2),
                IVec3::new(4, h - 1, l / 2 + 2),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.set(IVec3::new(3, h - 1, l / 2), VoxelId::GLASS_OPTIC);
        }
        TurretStyle::RoundedSherman => {
            // Classic rounded cast Sherman / T23 turret
            t.fill_box(IVec3::new(1, 0, 2), IVec3::new(w - 2, h - 2, l - 2), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 4), IVec3::new(w - 1, h - 3, l - 4), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(2, 0, 1), IVec3::new(w - 3, h - 2, l - 1), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(2, h - 2, 3), IVec3::new(w - 3, h - 2, l - 3), VoxelId::STEEL_OLIVE);
            t.fill_box(
                IVec3::new(mid_x + 1, h - 1, l / 2),
                IVec3::new(mid_x + 2, h - 1, l / 2 + 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
        }
        TurretStyle::HeavyAmerican => {
            // M26 Pershing / T34 Heavy: massive elongated turret + unmistakable rear counterweight bustle!
            t.fill_box(IVec3::new(1, 0, 2), IVec3::new(w - 2, h - 2, l - 5), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 3), IVec3::new(w - 1, h - 3, l - 6), VoxelId::STEEL_OLIVE);
            // Elongated armored rear bustle counterweight
            t.fill_box(
                IVec3::new(2, 1, l - 5),
                IVec3::new(w - 3, h - 2, l - 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.fill_box(
                IVec3::new(2, h - 1, l / 2),
                IVec3::new(4, h - 1, l / 2 + 2),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.set(IVec3::new(3, h - 1, l / 2), VoxelId::GLASS_OPTIC);
        }
        TurretStyle::OpenTopPentagonal => {
            // Iconic M10 Wolverine open-top sloped pentagonal turret with wedge rear counterweights!
            // Outer armored walls (open interior fighting compartment!)
            t.fill_box(IVec3::new(1, 0, 1), IVec3::new(w - 2, h - 2, l - 3), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 3), IVec3::new(w - 1, h - 3, l - 4), VoxelId::STEEL_OLIVE);
            // Carve open-top fighting pit inside turret
            t.fill_box(
                IVec3::new(2, 2, 3),
                IVec3::new(w - 3, h - 1, l - 5),
                VoxelId::AIR,
            );
            // Front partial roof strip
            t.fill_box(IVec3::new(2, h - 2, 1), IVec3::new(w - 3, h - 2, 2), VoxelId::STEEL_OLIVE);
            // Iconic duckbill wedge counterweights on rear cheeks
            t.fill_box(
                IVec3::new(0, 1, l - 3),
                IVec3::new(2, h - 2, l - 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
            t.fill_box(
                IVec3::new(w - 3, 1, l - 3),
                IVec3::new(w - 1, h - 2, l - 1),
                VoxelId::STEEL_OLIVE_DARK,
            );
        }
        TurretStyle::TallBoxyKV => {
            // KV-1 ZiS-5 tall boxy welded turret with wide heavy rear overhang
            t.fill_box(IVec3::new(1, 0, 1), IVec3::new(w - 2, h - 1, l - 1), VoxelId::STEEL_OLIVE);
            t.fill_box(IVec3::new(0, 0, 2), IVec3::new(w - 1, h - 2, l - 2), VoxelId::STEEL_OLIVE);
            t.fill_box(
                IVec3::new(mid_x - 1, h - 1, l - 4),
                IVec3::new(mid_x + 1, h - 1, l - 2),
                VoxelId::STEEL_OLIVE_DARK,
            );
        }
        TurretStyle::CasemateLow | TurretStyle::CasemateHeavy => {}
    }

    // Gun mantlet armor block at turret front (extra wide & thick on Jumbo, Tiger, T34!)
    let mantlet_half_w = if bp.heavy_mantlet {
        (w / 2 - 1).max(2)
    } else {
        (w / 4).max(1)
    };
    let mantlet_depth = if bp.heavy_mantlet { 2 } else { 1 };
    t.fill_box(
        IVec3::new(mid_x - mantlet_half_w, 1, 0),
        IVec3::new(mid_x + mantlet_half_w, (h - 2).max(2), mantlet_depth),
        VoxelId::STEEL_OLIVE_DARK,
    );

    // Side turret accent stripe
    t.fill_box(
        IVec3::new(0, 1, l / 2 - 1),
        IVec3::new(0, 1, l / 2 + 1),
        VoxelId::STEEL_ACCENT,
    );
    t.fill_box(
        IVec3::new(w - 1, 1, l / 2 - 1),
        IVec3::new(w - 1, 1, l / 2 + 1),
        VoxelId::STEEL_ACCENT,
    );

    t
}

/// Builds the default balanced WWII medium tank (used by unit tests & Stage 1–4 fallback).
pub fn build_basic_tank_models() -> TankVoxelModels {
    build_tank_models(&VoxelTankBlueprint {
        hull_w: 16,
        hull_h: 7,
        hull_l: 30,
        track_w: 3,
        hull_front: HullFrontStyle::SlopedGlacis,
        side_skirts: false,
        rear_fuel_drums: false,
        hull_mg_port: false,
        turret_style: TurretStyle::RoundedSoviet,
        turret_w: 10,
        turret_h: 6,
        turret_l: 14,
        turret_z_offset: -2,
        heavy_mantlet: false,
        barrel_len: 20,
        barrel_thick: false,
        muzzle_brake: MuzzleBrakeStyle::DoubleBaffle,
    })
}
