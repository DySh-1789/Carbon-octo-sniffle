//! Stages 4, 7 & 8 ballistic shell definitions, De Marre armor penetration math, HE overpressure,
//! and bursting-charge APHE / APHEBC / APCBC ballistics.

use serde::{Deserialize, Serialize};

/// Ammunition family across Stages 4, 7, and 8.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellKind {
    /// Solid kinetic armor-piercing shot (no explosive filler).
    Ap,
    /// High-Explosive blast shell (contact fuse, voxel destruction, War Thunder Overpressure).
    He,
    /// Armor-Piercing High-Explosive (kinetic penetrator + delayed internal bursting charge).
    Aphe,
    /// Armor-Piercing High-Explosive Ballistic Capped (Soviet blunt-nose sloped-armor specialist).
    Aphebc,
    /// Armor-Piercing Capped Ballistic Capped (low drag + piercing cap + bursting charge).
    Apcbc,
}

impl ShellKind {
    pub fn short_label(self) -> &'static str {
        match self {
            ShellKind::Ap => "AP",
            ShellKind::He => "HE",
            ShellKind::Aphe => "APHE",
            ShellKind::Aphebc => "APHEBC",
            ShellKind::Apcbc => "APCBC",
        }
    }

    #[inline]
    pub fn is_he(self) -> bool {
        self == ShellKind::He
    }

    #[inline]
    pub fn has_bursting_charge(self) -> bool {
        matches!(self, ShellKind::Aphe | ShellKind::Aphebc | ShellKind::Apcbc)
    }
}

/// Complete ballistic, armor-interaction, HE blast, and internal fuse specification for a shell.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShellSpec {
    pub name: String,
    pub kind: ShellKind,
    /// Projectile diameter in millimeters (used for caliber overmatch rules).
    pub caliber_mm: f32,
    /// Projectile mass in kilograms.
    pub mass_kg: f32,
    /// Initial muzzle velocity in m/s.
    pub muzzle_velocity: f32,
    /// Linear aerodynamic drag coefficient (`1/s`).
    pub drag_coeff: f32,
    /// Flat (0 deg) RHA penetration at muzzle velocity in millimeters (or HE blast pen for `He`).
    pub penetration_0m_mm: f32,
    /// Base slope normalization angle in degrees (bends trajectory toward armor normal).
    pub normalization_deg: f32,
    /// Impact angle from plate normal (degrees) where ricochet probability begins (> 0%).
    pub ricochet_start_deg: f32,
    /// Impact angle from plate normal (degrees) where ricochet becomes guaranteed (100%).
    pub ricochet_full_deg: f32,
    /// Base half-angle of secondary armor spall cone (degrees) after penetration.
    pub spall_cone_half_angle_deg: f32,
    /// Number of deterministic secondary spall fragment rays generated on penetration.
    pub spall_ray_count: u8,
    /// TNT-equivalent explosive filler mass in kilograms (`0.0` for solid AP).
    pub explosive_mass_kg: f32,
    /// High-explosive blast / overpressure armor penetration in mm.
    pub he_pen_mm: f32,
    /// Voxel terrain/building destruction & HE splash radius in meters.
    pub blast_radius_m: f32,
    /// Voxel material destruction power (compared against `VoxelMaterial::blast_resistance`).
    pub blast_power: f32,
    /// Minimum armor thickness (mm) required to arm an APHE/APHEBC/APCBC internal fuse.
    pub fuse_sensitivity_mm: f32,
    /// Distance (meters) traveled inside the tank after arming before bursting charge detonates.
    pub fuse_delay_m: f32,
}

impl ShellSpec {
    /// Standard WWII 75 mm solid AP shot paired with the Base Medium Tank.
    pub fn basic_75mm_ap() -> Self {
        Self {
            name: "75mm M72 AP".to_string(),
            kind: ShellKind::Ap,
            caliber_mm: 75.0,
            mass_kg: 6.32,
            muzzle_velocity: 780.0,
            drag_coeff: 0.04,
            penetration_0m_mm: 108.0,
            normalization_deg: 4.0,
            ricochet_start_deg: 60.0,
            ricochet_full_deg: 72.0,
            spall_cone_half_angle_deg: 15.0,
            spall_ray_count: 10,
            explosive_mass_kg: 0.0,
            he_pen_mm: 0.0,
            blast_radius_m: 0.90,
            blast_power: 2.8,
            fuse_sensitivity_mm: 0.0,
            fuse_delay_m: 0.0,
        }
    }

    /// Standard WWII 75 mm High-Explosive (`HE`) shell for destroying houses, craters, and thin armor.
    pub fn basic_75mm_he() -> Self {
        Self::he_for_caliber("75mm M48 HE", 75.0, 6.62, 580.0, 0.67)
    }

    /// Constructs a caliber-authentic High-Explosive (`HE`) shell with realistic TNT filler,
    /// War Thunder overpressure penetration, and voxel cratering blast power.
    pub fn he_for_caliber(
        name: &str,
        caliber_mm: f32,
        mass_kg: f32,
        muzzle_velocity: f32,
        tnt_kg: f32,
    ) -> Self {
        // War Thunder-style HE blast penetration & destruction scaling with TNT filler mass
        let he_pen_mm = if tnt_kg >= 3.0 {
            42.0
        } else if tnt_kg >= 1.4 {
            31.0
        } else if tnt_kg >= 0.85 {
            24.0
        } else if tnt_kg >= 0.45 {
            16.0
        } else if tnt_kg >= 0.15 {
            11.0
        } else {
            6.0
        };
        let blast_radius_m = (1.5 + tnt_kg.powf(0.42) * 1.65).clamp(1.6, 4.6);
        let blast_power = (3.4 + tnt_kg.powf(0.48) * 3.4).clamp(3.6, 10.5);

        Self {
            name: name.to_string(),
            kind: ShellKind::He,
            caliber_mm,
            mass_kg,
            muzzle_velocity,
            drag_coeff: 0.055,
            penetration_0m_mm: he_pen_mm,
            normalization_deg: 0.0,
            ricochet_start_deg: 89.0,
            ricochet_full_deg: 90.0,
            spall_cone_half_angle_deg: 45.0,
            spall_ray_count: 14,
            explosive_mass_kg: tnt_kg,
            he_pen_mm,
            blast_radius_m,
            blast_power,
            fuse_sensitivity_mm: 0.1,
            fuse_delay_m: 0.0,
        }
    }

    /// Constructs an authentic bursting-charge armor-piercing shell (`APHE`, `APHEBC`, or `APCBC`).
    #[allow(clippy::too_many_arguments)]
    pub fn bursting_ap(
        name: &str,
        kind: ShellKind,
        caliber_mm: f32,
        mass_kg: f32,
        muzzle_velocity: f32,
        pen_0m_mm: f32,
        tnt_kg: f32,
    ) -> Self {
        let (drag_mult, norm_deg, rico_start, rico_full, fuse_sens, fuse_delay) = match kind {
            ShellKind::Aphe => (1.0, 4.5, 61.0, 73.0, 14.0, 1.05),
            // Soviet APHEBC blunt-nose excels vs sloped armor (superior normalization & bounce angles!)
            ShellKind::Aphebc => (0.94, 6.8, 64.0, 76.0, 14.0, 1.15),
            // German/US APCBC ballistic cap retains penetration over long range + anti-shatter cap
            ShellKind::Apcbc => (0.80, 5.6, 62.5, 74.5, 14.0, 1.10),
            _ => (1.0, 4.0, 60.0, 72.0, 0.0, 0.0),
        };
        let base_drag = if caliber_mm >= 100.0 {
            0.032
        } else if caliber_mm >= 75.0 {
            0.038
        } else {
            0.048
        };
        Self {
            name: name.to_string(),
            kind,
            caliber_mm,
            mass_kg,
            muzzle_velocity,
            drag_coeff: base_drag * drag_mult,
            penetration_0m_mm: pen_0m_mm,
            normalization_deg: norm_deg,
            ricochet_start_deg: rico_start,
            ricochet_full_deg: rico_full,
            spall_cone_half_angle_deg: if caliber_mm >= 100.0 {
                19.0
            } else if caliber_mm >= 75.0 {
                16.0
            } else {
                12.0
            },
            spall_ray_count: if caliber_mm >= 100.0 {
                14
            } else if caliber_mm >= 75.0 {
                11
            } else {
                8
            },
            explosive_mass_kg: tnt_kg,
            he_pen_mm: (tnt_kg * 18.0).clamp(0.0, 26.0),
            blast_radius_m: (0.95 + tnt_kg.powf(0.35) * 0.75).clamp(0.9, 2.1),
            blast_power: (2.8 + tnt_kg * 2.4).clamp(2.8, 5.2),
            fuse_sensitivity_mm: fuse_sens,
            fuse_delay_m: fuse_delay,
        }
    }

    #[inline]
    pub fn is_he(&self) -> bool {
        self.kind.is_he()
    }

    #[inline]
    pub fn has_bursting_charge(&self) -> bool {
        self.kind.has_bursting_charge() && self.explosive_mass_kg > 0.005
    }

    /// Kinetic penetration in millimeters of RHA at `speed_mps` using the De Marre velocity law
    /// (`P ∝ v^1.43`). HE shells keep constant chemical blast penetration regardless of speed.
    #[inline]
    pub fn penetration_at_speed(&self, speed_mps: f32) -> f32 {
        if self.is_he() {
            return self.he_pen_mm;
        }
        if self.muzzle_velocity <= 1.0 || speed_mps <= 0.0 {
            return 0.0;
        }
        let ratio = (speed_mps / self.muzzle_velocity).clamp(0.0, 1.25);
        self.penetration_0m_mm * ratio.powf(1.43)
    }

    /// Caliber-to-thickness overmatch ratio (`caliber_mm / nominal_thickness_mm`).
    #[inline]
    pub fn overmatch_ratio(&self, nominal_thickness_mm: f32) -> f32 {
        self.caliber_mm / nominal_thickness_mm.max(1.0)
    }

    /// Effective normalization angle in degrees, amplified by caliber overmatch (`>= 2x`, `>= 3x`).
    #[inline]
    pub fn effective_normalization_deg(&self, nominal_thickness_mm: f32) -> f32 {
        if self.is_he() {
            return 0.0;
        }
        let ratio = self.overmatch_ratio(nominal_thickness_mm);
        let mult = if ratio >= 3.0 {
            2.6
        } else if ratio >= 2.0 {
            1.75
        } else if ratio >= 1.3 {
            1.0 + (ratio - 1.3) * 0.65
        } else {
            1.0
        };
        (self.normalization_deg * mult).min(24.0)
    }

    /// Ricochet probability in `[0.0, 1.0]` for impact angle `impact_angle_deg` (`0` = perpendicular,
    /// `90` = grazing) against a plate of `nominal_thickness_mm`.
    pub fn ricochet_chance(&self, impact_angle_deg: f32, nominal_thickness_mm: f32) -> f32 {
        if self.is_he() {
            return 0.0;
        }
        let ratio = self.overmatch_ratio(nominal_thickness_mm);
        if ratio >= 3.0 {
            return 0.0;
        }
        let (angle_bonus, prob_scale) = if ratio >= 2.0 {
            (8.0, 0.5)
        } else {
            (0.0, 1.0)
        };
        let start = self.ricochet_start_deg + angle_bonus;
        let full = (self.ricochet_full_deg + angle_bonus).min(88.0);
        if impact_angle_deg <= start {
            return 0.0;
        }
        if impact_angle_deg >= full {
            return prob_scale;
        }
        let t = ((impact_angle_deg - start) / (full - start).max(0.1)).clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t);
        (smooth * prob_scale).clamp(0.0, 1.0)
    }

    /// Line-of-sight effective armor thickness in mm accounting for armor type (`RHA`/`CHA`),
    /// obliquity angle `impact_angle_deg`, and overmatch-adjusted normalization.
    #[inline]
    pub fn effective_armor_mm(
        &self,
        nominal_thickness_mm: f32,
        impact_angle_deg: f32,
        armor_multiplier: f32,
    ) -> f32 {
        if self.is_he() {
            return nominal_thickness_mm * armor_multiplier;
        }
        let norm = self.effective_normalization_deg(nominal_thickness_mm);
        let eff_angle = (impact_angle_deg - norm).clamp(0.0, 86.0);
        (nominal_thickness_mm * armor_multiplier) / eff_angle.to_radians().cos()
    }
}
