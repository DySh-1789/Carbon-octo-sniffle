//! Stage 4 ballistic shell definitions and De Marre armor penetration math.
//!
//! Stage 4 introduces solid kinetic Armor-Piercing (`AP`) ammunition with velocity-dependent
//! penetration, slope normalization, caliber overmatch, ricochet probability curves, and
//! post-penetration spall cone parameters. (`HE` is added in Stage 7; `APHE`/`APHEBC`/`APCBC`
//! bursting-charge shells are added in Stage 8.)

use serde::{Deserialize, Serialize};

/// Ammunition family (strictly `Ap` during Stage 4; extended in Stages 7 and 8).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellKind {
    /// Solid kinetic armor-piercing shot (no explosive filler).
    Ap,
}

impl ShellKind {
    pub fn short_label(self) -> &'static str {
        match self {
            ShellKind::Ap => "AP",
        }
    }
}

/// Complete ballistic and armor-interaction specification for a tank cannon shell.
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
    /// Flat (0 deg) RHA penetration at muzzle velocity in millimeters.
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
        }
    }

    /// Kinetic penetration in millimeters of RHA at `speed_mps` using the De Marre velocity law
    /// (`P ∝ v^1.43`). Penetration decays naturally with flight distance as drag slows the shell.
    #[inline]
    pub fn penetration_at_speed(&self, speed_mps: f32) -> f32 {
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
    ///
    /// Applies the classic War Thunder caliber overmatch rules:
    /// - `caliber >= 3 * thickness`: triple overmatch — ricochet impossible (`0.0`).
    /// - `caliber >= 2 * thickness`: double overmatch — ricochet threshold shifts by `+8 deg` and
    ///   probability is halved.
    pub fn ricochet_chance(&self, impact_angle_deg: f32, nominal_thickness_mm: f32) -> f32 {
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
        // Smooth S-curve matching steeper bounce probabilities near full ricochet angle
        let smooth = t * t * (3.0 - 2.0 * t);
        (smooth * prob_scale).clamp(0.0, 1.0)
    }

    /// Line-of-sight effective armor thickness in mm accounting for armor type (`RHA`/`CHA`),
    /// obliquity angle `impact_angle_deg`, and overmatch-adjusted normalization:
    /// `T_eff = (T_nom * armor_mult) / cos(max(0, theta - alpha_norm))`.
    #[inline]
    pub fn effective_armor_mm(
        &self,
        nominal_thickness_mm: f32,
        impact_angle_deg: f32,
        armor_multiplier: f32,
    ) -> f32 {
        let norm = self.effective_normalization_deg(nominal_thickness_mm);
        let eff_angle = (impact_angle_deg - norm).clamp(0.0, 86.0);
        (nominal_thickness_mm * armor_multiplier) / eff_angle.to_radians().cos()
    }
}
