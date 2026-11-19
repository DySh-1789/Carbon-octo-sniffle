//! Stage 4 War Thunder-style damage system (strictly NO tank HP bar).
//!
//! Models oriented hull/turret/gun armor plates (`RHA` / `CHA`), external modules (tracks, barrel),
//! internal modules (ammo racks, engine, transmission, fuel tanks, breech, turret ring), 5-man
//! crew with 6-second driver/gunner replacement, fires, ammo-rack detonation with turret blow-off,
//! and solid-AP post-penetration primary ray + secondary spall cone.

use std::f32::consts::TAU;

use voxel::glam::{Quat, Vec3};

use crate::battle::{SimRng, TankId, Team};
use crate::shell::ShellSpec;
use crate::tank::TankState;

/// Crew replacement duration in seconds when a surviving crewman moves to Driver or Gunner.
pub const CREW_TRANSFER_TIME: f32 = 6.0;
/// Minimum surviving crew required to keep a tank operational (`< 2` => knocked out).
pub const MIN_OPERATIONAL_CREW: usize = 2;
/// Unextinguished fire duration (seconds) after which the tank fully burns out.
pub const FIRE_BURNOUT_TIME: f32 = 14.0;
/// Reload time multiplier when the Loader seat is vacant.
pub const NO_LOADER_RELOAD_MULT: f32 = 1.8;

/// Armor metallurgy resistance modifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArmorType {
    /// Rolled Homogeneous Armor (`1.00x`).
    Rha,
    /// Cast Homogeneous Armor (`0.94x`).
    Cha,
    /// Track links / external structural steel (`0.75x`).
    TrackSteel,
}

impl ArmorType {
    #[inline]
    pub fn multiplier(self) -> f32 {
        match self {
            ArmorType::Rha => 1.0,
            ArmorType::Cha => 0.94,
            ArmorType::TrackSteel => 0.75,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ArmorType::Rha => "RHA",
            ArmorType::Cha => "CHA",
            ArmorType::TrackSteel => "TRACK",
        }
    }
}

/// Local coordinate frame of an armor box or internal hitbox.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PartFrame {
    Hull,
    Turret,
    Gun,
}

impl PartFrame {
    #[inline]
    pub fn world_pose(self, tank: &TankState) -> (Vec3, Quat) {
        match self {
            PartFrame::Hull => (tank.position, tank.hull_rotation()),
            PartFrame::Turret => (tank.turret_world_position(), tank.turret_world_rotation()),
            PartFrame::Gun => (tank.gun_world_position(), tank.gun_world_rotation()),
        }
    }
}

/// Internal and external tank component kinds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    AmmoRack,
    Engine,
    Transmission,
    FuelTank,
    GunBreech,
    GunBarrel,
    TurretRing,
    LeftTrack,
    RightTrack,
}

impl ModuleKind {
    pub fn label(self) -> &'static str {
        match self {
            ModuleKind::AmmoRack => "Ammo Rack",
            ModuleKind::Engine => "Engine",
            ModuleKind::Transmission => "Transmission",
            ModuleKind::FuelTank => "Fuel Tank",
            ModuleKind::GunBreech => "Gun Breech",
            ModuleKind::GunBarrel => "Gun Barrel",
            ModuleKind::TurretRing => "Turret Ring",
            ModuleKind::LeftTrack => "Left Track",
            ModuleKind::RightTrack => "Right Track",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            ModuleKind::AmmoRack => "AMMO",
            ModuleKind::Engine => "ENG",
            ModuleKind::Transmission => "TRN",
            ModuleKind::FuelTank => "FUEL",
            ModuleKind::GunBreech => "BRC",
            ModuleKind::GunBarrel => "BRL",
            ModuleKind::TurretRing => "RING",
            ModuleKind::LeftTrack => "L-TRK",
            ModuleKind::RightTrack => "R-TRK",
        }
    }
}

/// Discrete War Thunder color-coded state of a single module (`Intact -> Damaged -> Destroyed`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleHealth {
    Intact,
    Damaged,
    Destroyed,
}

/// Tank crew positions.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CrewRole {
    Driver,
    Gunner,
    Loader,
    Commander,
    RadioOperator,
}

impl CrewRole {
    pub fn label(self) -> &'static str {
        match self {
            CrewRole::Driver => "Driver",
            CrewRole::Gunner => "Gunner",
            CrewRole::Loader => "Loader",
            CrewRole::Commander => "Commander",
            CrewRole::RadioOperator => "Radio Op",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            CrewRole::Driver => "DRV",
            CrewRole::Gunner => "GUN",
            CrewRole::Loader => "LDR",
            CrewRole::Commander => "CMD",
            CrewRole::RadioOperator => "RAD",
        }
    }
}

/// Exact War Thunder destruction cause (no generic HP depletion).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KillReason {
    CrewKnockedOut,
    AmmoRackDetonation,
    BurnedOut,
}

impl KillReason {
    pub fn short_label(self) -> &'static str {
        match self {
            KillReason::CrewKnockedOut => "CREW KO",
            KillReason::AmmoRackDetonation => "AMMO RACK",
            KillReason::BurnedOut => "BURNED OUT",
        }
    }
}

/// Armor interaction outcome for a shell impact.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HitOutcome {
    Penetration,
    Ricochet,
    NonPenetration,
    ExternalModuleHit,
}

impl HitOutcome {
    pub fn label(self) -> &'static str {
        match self {
            HitOutcome::Penetration => "PENETRATION",
            HitOutcome::Ricochet => "RICOCHET",
            HitOutcome::NonPenetration => "NON-PENETRATION",
            HitOutcome::ExternalModuleHit => "MODULE HIT",
        }
    }
}

/// Live gameplay performance penalties derived from module and crew damage.
#[derive(Copy, Clone, Debug)]
pub struct DamageModifiers {
    pub can_drive: bool,
    pub speed_scale: f32,
    pub steer_scale: f32,
    /// Directional pull when one track is destroyed (`-1` pulls right, `+1` pulls left).
    pub broken_track_pull: f32,
    pub can_aim: bool,
    pub turret_rate_scale: f32,
    pub reload_time_scale: f32,
    pub can_fire: bool,
    /// Angular dispersion half-cone in radians (elevated when gun barrel/breech is damaged).
    pub fire_dispersion_rad: f32,
}

impl DamageModifiers {
    pub fn intact() -> Self {
        Self {
            can_drive: true,
            speed_scale: 1.0,
            steer_scale: 1.0,
            broken_track_pull: 0.0,
            can_aim: true,
            turret_rate_scale: 1.0,
            reload_time_scale: 1.0,
            can_fire: true,
            fire_dispersion_rad: 0.0,
        }
    }
}

/// Single internal or external module instance with local AABB and structural integrity.
#[derive(Clone, Debug)]
pub struct ModuleState {
    pub kind: ModuleKind,
    pub frame: PartFrame,
    pub box_min: Vec3,
    pub box_max: Vec3,
    pub integrity: f32,
    pub max_integrity: f32,
}

impl ModuleState {
    pub fn new(
        kind: ModuleKind,
        frame: PartFrame,
        box_min: Vec3,
        box_max: Vec3,
        max_integrity: f32,
    ) -> Self {
        Self {
            kind,
            frame,
            box_min,
            box_max,
            integrity: max_integrity,
            max_integrity,
        }
    }

    #[inline]
    pub fn health(&self) -> ModuleHealth {
        if self.integrity <= 0.0 {
            ModuleHealth::Destroyed
        } else if self.integrity <= self.max_integrity * 0.65 {
            ModuleHealth::Damaged
        } else {
            ModuleHealth::Intact
        }
    }

    pub fn apply_damage(&mut self, amount: f32) -> (ModuleHealth, ModuleHealth) {
        let before = self.health();
        self.integrity = (self.integrity - amount).max(0.0);
        let after = self.health();
        (before, after)
    }
}

/// Single crew member hitbox and alive state.
#[derive(Clone, Debug)]
pub struct CrewMemberState {
    pub role: CrewRole,
    /// Current seat occupied (equals `role` initially, or `Driver`/`Gunner` after replacement).
    pub active_seat: CrewRole,
    pub frame: PartFrame,
    pub box_min: Vec3,
    pub box_max: Vec3,
    pub health: f32,
    pub alive: bool,
}

impl CrewMemberState {
    pub fn new(role: CrewRole, frame: PartFrame, box_min: Vec3, box_max: Vec3) -> Self {
        Self {
            role,
            active_seat: role,
            frame,
            box_min,
            box_max,
            health: 100.0,
            alive: true,
        }
    }

    /// Applies kinetic/spall injury. Returns `true` if this damage knocked out the crewman.
    pub fn apply_damage(&mut self, amount: f32) -> bool {
        if !self.alive {
            return false;
        }
        self.health = (self.health - amount).max(0.0);
        if self.health <= 0.0 {
            self.alive = false;
            return true;
        }
        false
    }
}

/// Complete War Thunder internal state of a single tank (modules, crew, transfers, fire, turret pop).
#[derive(Clone, Debug)]
pub struct TankDamageState {
    pub modules: Vec<ModuleState>,
    pub crew: Vec<CrewMemberState>,
    /// Countdown (seconds) while a spare crewman moves to replace a knocked-out Driver.
    pub driver_transfer_timer: f32,
    /// Countdown (seconds) while a spare crewman moves to replace a knocked-out Gunner.
    pub gunner_transfer_timer: f32,
    pub on_fire: bool,
    pub fire_elapsed: f32,
    fire_tick_accum: f32,
    /// True if ammo-rack detonation blew the turret off the hull.
    pub turret_blown_off: bool,
    /// Total hits received (used by AI to react defensively without an HP bar).
    pub damage_events_received: u32,
    /// Shooter who ignited the current fire (credited if fire kills the tank).
    pub fire_instigator: Option<(TankId, Team)>,
}

impl TankDamageState {
    /// Internal layout for the Base Medium Tank matching the `0.2 m` voxel hull & turret dimensions.
    pub fn basic_medium_tank() -> Self {
        let modules = vec![
            // External tracks (left & right)
            ModuleState::new(
                ModuleKind::LeftTrack,
                PartFrame::Hull,
                Vec3::new(-1.60, 0.0, -2.90),
                Vec3::new(-1.05, 0.95, 2.90),
                85.0,
            ),
            ModuleState::new(
                ModuleKind::RightTrack,
                PartFrame::Hull,
                Vec3::new(1.05, 0.0, -2.90),
                Vec3::new(1.60, 0.95, 2.90),
                85.0,
            ),
            // External gun barrel
            ModuleState::new(
                ModuleKind::GunBarrel,
                PartFrame::Gun,
                Vec3::new(-0.22, -0.22, -3.90),
                Vec3::new(0.22, 0.22, 0.0),
                75.0,
            ),
            // Front lower transmission
            ModuleState::new(
                ModuleKind::Transmission,
                PartFrame::Hull,
                Vec3::new(-0.68, 0.22, -2.35),
                Vec3::new(0.68, 0.68, -1.62),
                110.0,
            ),
            // Rear engine block
            ModuleState::new(
                ModuleKind::Engine,
                PartFrame::Hull,
                Vec3::new(-0.72, 0.30, 0.68),
                Vec3::new(0.72, 1.22, 2.28),
                120.0,
            ),
            // Left & right fuel tanks flanking the engine bay
            ModuleState::new(
                ModuleKind::FuelTank,
                PartFrame::Hull,
                Vec3::new(-1.05, 0.35, 0.35),
                Vec3::new(-0.74, 1.12, 1.95),
                80.0,
            ),
            ModuleState::new(
                ModuleKind::FuelTank,
                PartFrame::Hull,
                Vec3::new(0.74, 0.35, 0.35),
                Vec3::new(1.05, 1.12, 1.95),
                80.0,
            ),
            // Three ammunition stowage racks (hull left sponson, hull floor/right, turret bustle)
            ModuleState::new(
                ModuleKind::AmmoRack,
                PartFrame::Hull,
                Vec3::new(-1.02, 0.55, -0.78),
                Vec3::new(-0.56, 1.15, 0.18),
                70.0,
            ),
            ModuleState::new(
                ModuleKind::AmmoRack,
                PartFrame::Hull,
                Vec3::new(0.56, 0.45, -0.78),
                Vec3::new(1.02, 1.08, 0.18),
                70.0,
            ),
            ModuleState::new(
                ModuleKind::AmmoRack,
                PartFrame::Turret,
                Vec3::new(-0.55, 0.20, 0.58),
                Vec3::new(0.55, 0.78, 1.22),
                65.0,
            ),
            // Turret traverse ring at hull roof collar
            ModuleState::new(
                ModuleKind::TurretRing,
                PartFrame::Hull,
                Vec3::new(-0.82, 1.22, -1.15),
                Vec3::new(0.82, 1.44, 0.35),
                90.0,
            ),
            // Cannon breech directly behind mantlet
            ModuleState::new(
                ModuleKind::GunBreech,
                PartFrame::Turret,
                Vec3::new(-0.28, 0.18, -1.05),
                Vec3::new(0.28, 0.74, -0.05),
                90.0,
            ),
        ];

        let crew = vec![
            // Hull front-left Driver
            CrewMemberState::new(
                CrewRole::Driver,
                PartFrame::Hull,
                Vec3::new(-0.72, 0.42, -1.58),
                Vec3::new(-0.14, 1.25, -0.92),
            ),
            // Hull front-right Radio Operator / bow gunner
            CrewMemberState::new(
                CrewRole::RadioOperator,
                PartFrame::Hull,
                Vec3::new(0.14, 0.42, -1.58),
                Vec3::new(0.72, 1.25, -0.92),
            ),
            // Turret left-front Gunner
            CrewMemberState::new(
                CrewRole::Gunner,
                PartFrame::Turret,
                Vec3::new(-0.76, 0.06, -0.78),
                Vec3::new(-0.26, 0.86, -0.10),
            ),
            // Turret left-rear Commander under cupola
            CrewMemberState::new(
                CrewRole::Commander,
                PartFrame::Turret,
                Vec3::new(-0.72, 0.12, -0.04),
                Vec3::new(-0.20, 1.04, 0.56),
            ),
            // Turret right Loader
            CrewMemberState::new(
                CrewRole::Loader,
                PartFrame::Turret,
                Vec3::new(0.26, 0.06, -0.62),
                Vec3::new(0.78, 0.88, 0.32),
            ),
        ];

        Self {
            modules,
            crew,
            driver_transfer_timer: 0.0,
            gunner_transfer_timer: 0.0,
            on_fire: false,
            fire_elapsed: 0.0,
            fire_tick_accum: 0.0,
            turret_blown_off: false,
            damage_events_received: 0,
            fire_instigator: None,
        }
    }

    #[inline]
    pub fn alive_crew_count(&self) -> usize {
        self.crew.iter().filter(|c| c.alive).count()
    }

    #[inline]
    pub fn total_crew_count(&self) -> usize {
        self.crew.len()
    }

    #[inline]
    pub fn has_active_seat(&self, seat: CrewRole) -> bool {
        self.crew.iter().any(|c| c.alive && c.active_seat == seat)
    }

    /// Returns the worst damage state among all instances of `kind` (e.g. any ammo rack or fuel tank).
    pub fn module_worst_health(&self, kind: ModuleKind) -> ModuleHealth {
        self.modules
            .iter()
            .filter(|m| m.kind == kind)
            .map(ModuleState::health)
            .max()
            .unwrap_or(ModuleHealth::Intact)
    }

    /// Computes live driving, traverse, reload, and firing modifiers from module & crew damage.
    pub fn mobility_modifiers(&self) -> DamageModifiers {
        let mut mods = DamageModifiers::intact();

        let left_trk = self.module_worst_health(ModuleKind::LeftTrack);
        let right_trk = self.module_worst_health(ModuleKind::RightTrack);
        let engine = self.module_worst_health(ModuleKind::Engine);
        let trans = self.module_worst_health(ModuleKind::Transmission);
        let ring = self.module_worst_health(ModuleKind::TurretRing);
        let breech = self.module_worst_health(ModuleKind::GunBreech);
        let barrel = self.module_worst_health(ModuleKind::GunBarrel);

        // 1. Mobility (driver, engine, transmission, tracks)
        if !self.has_active_seat(CrewRole::Driver)
            || engine == ModuleHealth::Destroyed
            || trans == ModuleHealth::Destroyed
            || (left_trk == ModuleHealth::Destroyed && right_trk == ModuleHealth::Destroyed)
        {
            mods.can_drive = false;
            mods.speed_scale = 0.0;
            mods.steer_scale = 0.0;
        } else {
            if engine == ModuleHealth::Damaged {
                mods.speed_scale *= 0.62;
            }
            if trans == ModuleHealth::Damaged {
                mods.speed_scale *= 0.72;
            }
            if left_trk == ModuleHealth::Destroyed {
                mods.speed_scale *= 0.25;
                mods.steer_scale *= 0.45;
                mods.broken_track_pull = 0.55;
            } else if right_trk == ModuleHealth::Destroyed {
                mods.speed_scale *= 0.25;
                mods.steer_scale *= 0.45;
                mods.broken_track_pull = -0.55;
            } else if left_trk == ModuleHealth::Damaged || right_trk == ModuleHealth::Damaged {
                mods.speed_scale *= 0.85;
            }
        }

        // 2. Turret traverse & gunner presence
        if !self.has_active_seat(CrewRole::Gunner) {
            mods.can_aim = false;
            mods.can_fire = false;
            mods.turret_rate_scale = 0.0;
        } else if ring == ModuleHealth::Destroyed {
            mods.turret_rate_scale = 0.0;
        } else if ring == ModuleHealth::Damaged {
            mods.turret_rate_scale = 0.45;
        }

        // 3. Reload speed (loader presence + damaged breech)
        if !self.has_active_seat(CrewRole::Loader) {
            mods.reload_time_scale *= NO_LOADER_RELOAD_MULT;
        }
        if breech == ModuleHealth::Damaged {
            mods.reload_time_scale *= 1.35;
        }

        // 4. Gun firing capability & dispersion
        if breech == ModuleHealth::Destroyed || barrel == ModuleHealth::Destroyed {
            mods.can_fire = false;
        } else {
            if barrel == ModuleHealth::Damaged {
                mods.fire_dispersion_rad += 2.2f32.to_radians();
            }
            if breech == ModuleHealth::Damaged {
                mods.fire_dispersion_rad += 0.8f32.to_radians();
            }
        }

        mods
    }

    #[inline]
    pub fn can_move(&self) -> bool {
        self.mobility_modifiers().can_drive
    }

    #[inline]
    pub fn can_fire(&self) -> bool {
        self.mobility_modifiers().can_fire
    }

    /// Starts crew-replacement timers whenever Driver or Gunner is absent and a spare crewman exists.
    pub fn ensure_crew_transfers_started(&mut self) {
        if self.alive_crew_count() < MIN_OPERATIONAL_CREW {
            self.driver_transfer_timer = 0.0;
            self.gunner_transfer_timer = 0.0;
            return;
        }
        if !self.has_active_seat(CrewRole::Driver)
            && self.driver_transfer_timer <= 0.0
            && self.find_replacement_candidate(CrewRole::Driver).is_some()
        {
            self.driver_transfer_timer = CREW_TRANSFER_TIME;
        }
        if !self.has_active_seat(CrewRole::Gunner)
            && self.gunner_transfer_timer <= 0.0
            && self.find_replacement_candidate(CrewRole::Gunner).is_some()
        {
            self.gunner_transfer_timer = CREW_TRANSFER_TIME;
        }
    }

    /// Advances crew-replacement timers and ongoing fire damage. Returns `Some(KillReason)` if
    /// the tank is destroyed this tick (fire burn-out, fire-induced ammo detonation, or `< 2` crew).
    pub fn tick_ongoing(&mut self, dt: f32, rng: &mut SimRng) -> Option<KillReason> {
        if self.alive_crew_count() < MIN_OPERATIONAL_CREW {
            return Some(KillReason::CrewKnockedOut);
        }

        // 1. Crew replacement timers (6.0 s)
        self.ensure_crew_transfers_started();

        if self.driver_transfer_timer > 0.0 {
            self.driver_transfer_timer = (self.driver_transfer_timer - dt).max(0.0);
            if self.driver_transfer_timer <= 0.0
                && !self.has_active_seat(CrewRole::Driver)
                && let Some(idx) = self.find_replacement_candidate(CrewRole::Driver)
            {
                self.reassign_crew_to_seat(idx, CrewRole::Driver);
            }
        }
        if self.gunner_transfer_timer > 0.0 {
            self.gunner_transfer_timer = (self.gunner_transfer_timer - dt).max(0.0);
            if self.gunner_transfer_timer <= 0.0
                && !self.has_active_seat(CrewRole::Gunner)
                && let Some(idx) = self.find_replacement_candidate(CrewRole::Gunner)
            {
                self.reassign_crew_to_seat(idx, CrewRole::Gunner);
            }
        }

        // 2. Ongoing fire burn-out tick
        if self.on_fire {
            self.fire_elapsed += dt;
            self.fire_tick_accum += dt;
            while self.fire_tick_accum >= 1.0 {
                self.fire_tick_accum -= 1.0;
                // Thermal damage spreads to Engine, FuelTank, and AmmoRack
                let mut ammo_detonated = false;
                for m in &mut self.modules {
                    match m.kind {
                        ModuleKind::Engine | ModuleKind::FuelTank => {
                            m.apply_damage(18.0);
                        }
                        ModuleKind::AmmoRack => {
                            let (before, after) = m.apply_damage(11.0);
                            if before != ModuleHealth::Destroyed
                                && after == ModuleHealth::Destroyed
                                && rng.next_f32() < 0.85
                            {
                                ammo_detonated = true;
                            }
                        }
                        _ => {}
                    }
                }
                if ammo_detonated {
                    self.turret_blown_off = true;
                    return Some(KillReason::AmmoRackDetonation);
                }

                // Smoke and heat injure surviving crewmen
                for c in &mut self.crew {
                    if c.alive {
                        c.apply_damage(9.5);
                    }
                }
                if self.alive_crew_count() < MIN_OPERATIONAL_CREW {
                    return Some(KillReason::CrewKnockedOut);
                }
            }
            if self.fire_elapsed >= FIRE_BURNOUT_TIME {
                return Some(KillReason::BurnedOut);
            }
        }

        None
    }

    fn find_replacement_candidate(&self, needed: CrewRole) -> Option<usize> {
        let priority: &[CrewRole] = match needed {
            CrewRole::Driver => &[CrewRole::RadioOperator, CrewRole::Commander, CrewRole::Loader],
            CrewRole::Gunner => &[CrewRole::Commander, CrewRole::RadioOperator, CrewRole::Loader],
            _ => &[],
        };
        for &seat in priority {
            if let Some(idx) = self
                .crew
                .iter()
                .position(|c| c.alive && c.active_seat == seat)
            {
                return Some(idx);
            }
        }
        None
    }

    fn reassign_crew_to_seat(&mut self, idx: usize, seat: CrewRole) {
        let (frame, bmin, bmax) = match seat {
            CrewRole::Driver => (
                PartFrame::Hull,
                Vec3::new(-0.72, 0.42, -1.58),
                Vec3::new(-0.14, 1.25, -0.92),
            ),
            CrewRole::Gunner => (
                PartFrame::Turret,
                Vec3::new(-0.76, 0.06, -0.78),
                Vec3::new(-0.26, 0.86, -0.10),
            ),
            _ => return,
        };
        let c = &mut self.crew[idx];
        c.active_seat = seat;
        c.frame = frame;
        c.box_min = bmin;
        c.box_max = bmax;
    }
}

/// Result of raycasting against a tank's exterior armor profile.
#[derive(Copy, Clone, Debug)]
pub struct ArmorHit {
    pub distance: f32,
    pub world_point: Vec3,
    /// Sloped armor outward normal in world space (used for obliquity & ricochet).
    pub plate_normal_world: Vec3,
    /// Geometric box face normal in world space (for spark visuals).
    pub geom_normal_world: Vec3,
    pub plate_name: &'static str,
    pub nominal_mm: f32,
    pub armor_type: ArmorType,
    pub external_module: Option<ModuleKind>,
    pub frame: PartFrame,
}

/// Complete War Thunder Hit-Cam report emitted for every shell-vs-tank interaction.
#[derive(Clone, Debug)]
pub struct HitReport {
    pub shooter_id: TankId,
    pub target_id: TankId,
    pub target_name: String,
    pub target_team: Team,
    pub outcome: HitOutcome,
    pub plate_name: &'static str,
    pub armor_type: ArmorType,
    pub nominal_armor_mm: f32,
    pub impact_angle_deg: f32,
    pub effective_armor_mm: f32,
    pub penetration_mm: f32,
    pub damaged_modules: Vec<(ModuleKind, ModuleHealth)>,
    pub killed_crew: Vec<CrewRole>,
    pub started_fire: bool,
    pub kill_reason: Option<KillReason>,
    pub position: Vec3,
    pub normal: Vec3,
    pub ricochet_dir: Option<Vec3>,
    /// True when friendly fire or hitting an already-destroyed wreck.
    pub inert_hit: bool,
}

/// Slab ray-vs-AABB intersection in a local coordinate frame. Returns `(t_entry, local_normal)`.
pub fn ray_vs_aabb(
    origin: Vec3,
    dir: Vec3,
    min: Vec3,
    max: Vec3,
    max_t: f32,
) -> Option<(f32, Vec3)> {
    let mut t_min = 0.0f32;
    let mut t_max = max_t;
    let mut normal = Vec3::ZERO;
    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        if d.abs() < 1e-8 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
            continue;
        }
        let inv = 1.0 / d;
        let mut t0 = (min[axis] - o) * inv;
        let mut t1 = (max[axis] - o) * inv;
        let mut n = Vec3::ZERO;
        n[axis] = -d.signum();
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > t_min {
            t_min = t0;
            normal = n;
        }
        t_max = t_max.min(t1);
        if t_min > t_max {
            return None;
        }
    }
    if normal == Vec3::ZERO {
        normal = -dir;
    }
    Some((t_min, normal))
}

fn ray_vs_oriented_box(
    origin: Vec3,
    dir: Vec3,
    max_t: f32,
    frame_pos: Vec3,
    frame_rot: Quat,
    min: Vec3,
    max: Vec3,
) -> Option<(f32, Vec3, Vec3)> {
    let inv = frame_rot.inverse();
    let lo = inv * (origin - frame_pos);
    let ld = inv * dir;
    ray_vs_aabb(lo, ld, min, max, max_t).map(|(t, ln)| (t, lo + ld * t, ln))
}

/// Classifies an entry point on the Base Medium Tank hull armor box into historical plate specs.
fn classify_hull_plate(local_hit: Vec3, local_normal: Vec3) -> (&'static str, f32, ArmorType, Vec3) {
    if local_normal.z < -0.5 {
        // Front hull split into sloped Upper Front Plate (UFP) & Lower Front Plate (LFP)
        if local_hit.y >= 0.72 {
            // 52 deg sloped upper glacis: ~90 mm eff straight-on, >112 mm angled 40 deg
            let sloped = Vec3::new(0.0, 52.0f32.to_radians().sin(), -52.0f32.to_radians().cos());
            ("Hull UFP", 60.0, ArmorType::Rha, sloped.normalize())
        } else {
            // 35 deg downward lower glacis weakspot (~56 mm eff)
            let sloped = Vec3::new(0.0, -35.0f32.to_radians().sin(), -35.0f32.to_radians().cos());
            ("Hull LFP", 48.0, ArmorType::Rha, sloped.normalize())
        }
    } else if local_normal.z > 0.5 {
        let sloped = Vec3::new(0.0, 15.0f32.to_radians().sin(), 15.0f32.to_radians().cos());
        ("Hull Rear", 35.0, ArmorType::Rha, sloped.normalize())
    } else if local_normal.x.abs() > 0.5 {
        ("Hull Side", 42.0, ArmorType::Rha, local_normal)
    } else if local_normal.y > 0.5 {
        ("Hull Roof", 20.0, ArmorType::Rha, Vec3::Y)
    } else {
        ("Hull Belly", 18.0, ArmorType::Rha, Vec3::NEG_Y)
    }
}

/// Classifies an entry point on the Base Medium Tank turret armor box.
fn classify_turret_plate(
    local_hit: Vec3,
    local_normal: Vec3,
) -> (&'static str, f32, ArmorType, Vec3) {
    if local_normal.z < -0.5 {
        if local_hit.x.abs() <= 0.45 && (0.14..=0.86).contains(&local_hit.y) {
            let sloped = Vec3::new(0.0, 16.0f32.to_radians().sin(), -16.0f32.to_radians().cos());
            ("Gun Mantlet", 88.0, ArmorType::Cha, sloped.normalize())
        } else {
            let side_s = local_hit.x.signum() * 0.34;
            let sloped = Vec3::new(side_s, 0.32, -0.88).normalize();
            ("Turret Cheek", 65.0, ArmorType::Cha, sloped)
        }
    } else if local_normal.z > 0.5 {
        ("Turret Rear", 45.0, ArmorType::Cha, Vec3::Z)
    } else if local_normal.x.abs() > 0.5 {
        let sx = local_normal.x.signum();
        let sloped = Vec3::new(sx * 0.94, 0.34, 0.0).normalize();
        ("Turret Side", 48.0, ArmorType::Cha, sloped)
    } else if local_normal.y > 0.5 {
        ("Turret Roof", 20.0, ArmorType::Rha, Vec3::Y)
    } else {
        ("Turret Underside", 25.0, ArmorType::Cha, Vec3::NEG_Y)
    }
}

/// Raycasts against all armor volumes of a tank (optionally skipping external track/barrel hits
/// when tracing pass-through into the main hull behind a penetrated track).
pub fn raycast_tank_armor(
    tank: &TankState,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    include_external: bool,
    turret_attached: bool,
) -> Option<ArmorHit> {
    // Broad phase bounding sphere
    let center = tank.position + Vec3::new(0.0, 1.35, 0.0);
    let to_c = center - origin;
    let along = to_c.dot(dir).clamp(0.0, max_dist);
    let closest = origin + dir * along;
    if closest.distance_squared(center) > 5.8 * 5.8 {
        return None;
    }

    let mut best: Option<ArmorHit> = None;
    let mut consider = |hit: ArmorHit| {
        if best.is_none_or(|b| hit.distance < b.distance) {
            best = Some(hit);
        }
    };

    // 1. Main armored hull body
    let (h_pos, h_rot) = PartFrame::Hull.world_pose(tank);
    if let Some((t, lhit, lnorm)) = ray_vs_oriented_box(
        origin,
        dir,
        max_dist,
        h_pos,
        h_rot,
        Vec3::new(-1.08, 0.24, -2.60),
        Vec3::new(1.08, 1.42, 2.60),
    ) {
        let (plate_name, nominal_mm, armor_type, plate_lnorm) = classify_hull_plate(lhit, lnorm);
        consider(ArmorHit {
            distance: t,
            world_point: origin + dir * t,
            plate_normal_world: (h_rot * plate_lnorm).normalize(),
            geom_normal_world: (h_rot * lnorm).normalize(),
            plate_name,
            nominal_mm,
            armor_type,
            external_module: None,
            frame: PartFrame::Hull,
        });
    }

    // 2. External left & right tracks
    if include_external {
        for (min, max, name, kind) in [
            (
                Vec3::new(-1.60, 0.0, -2.90),
                Vec3::new(-1.05, 0.95, 2.90),
                "Left Track",
                ModuleKind::LeftTrack,
            ),
            (
                Vec3::new(1.05, 0.0, -2.90),
                Vec3::new(1.60, 0.95, 2.90),
                "Right Track",
                ModuleKind::RightTrack,
            ),
        ] {
            if let Some((t, _, lnorm)) =
                ray_vs_oriented_box(origin, dir, max_dist, h_pos, h_rot, min, max)
            {
                let wn = (h_rot * lnorm).normalize();
                consider(ArmorHit {
                    distance: t,
                    world_point: origin + dir * t,
                    plate_normal_world: wn,
                    geom_normal_world: wn,
                    plate_name: name,
                    nominal_mm: 22.0,
                    armor_type: ArmorType::TrackSteel,
                    external_module: Some(kind),
                    frame: PartFrame::Hull,
                });
            }
        }
    }

    // 3. Turret main body, commander cupola, and external gun barrel (skipped if turret blew off)
    if turret_attached {
        let (t_pos, t_rot) = PartFrame::Turret.world_pose(tank);
        if let Some((t, lhit, lnorm)) = ray_vs_oriented_box(
            origin,
            dir,
            max_dist,
            t_pos,
            t_rot,
            Vec3::new(-1.0, 0.0, -1.25),
            Vec3::new(1.0, 1.0, 1.35),
        ) {
            let (plate_name, nominal_mm, armor_type, plate_lnorm) =
                classify_turret_plate(lhit, lnorm);
            consider(ArmorHit {
                distance: t,
                world_point: origin + dir * t,
                plate_normal_world: (t_rot * plate_lnorm).normalize(),
                geom_normal_world: (t_rot * lnorm).normalize(),
                plate_name,
                nominal_mm,
                armor_type,
                external_module: None,
                frame: PartFrame::Turret,
            });
        }

        // Commander cupola
        if let Some((t, _, lnorm)) = ray_vs_oriented_box(
            origin,
            dir,
            max_dist,
            t_pos,
            t_rot,
            Vec3::new(-0.56, 1.0, -0.05),
            Vec3::new(0.04, 1.24, 0.55),
        ) {
            let wn = (t_rot * lnorm).normalize();
            consider(ArmorHit {
                distance: t,
                world_point: origin + dir * t,
                plate_normal_world: wn,
                geom_normal_world: wn,
                plate_name: "Cupola",
                nominal_mm: 45.0,
                armor_type: ArmorType::Cha,
                external_module: None,
                frame: PartFrame::Turret,
            });
        }

        // External gun barrel
        if include_external {
            let (g_pos, g_rot) = PartFrame::Gun.world_pose(tank);
            if let Some((t, _, lnorm)) = ray_vs_oriented_box(
                origin,
                dir,
                max_dist,
                g_pos,
                g_rot,
                Vec3::new(-0.22, -0.22, -3.90),
                Vec3::new(0.22, 0.22, 0.0),
            ) {
                let wn = (g_rot * lnorm).normalize();
                consider(ArmorHit {
                    distance: t,
                    world_point: origin + dir * t,
                    plate_normal_world: wn,
                    geom_normal_world: wn,
                    plate_name: "Gun Barrel",
                    nominal_mm: 25.0,
                    armor_type: ArmorType::Rha,
                    external_module: Some(ModuleKind::GunBarrel),
                    frame: PartFrame::Gun,
                });
            }
        }
    }

    best
}

/// Resolves a kinetic `AP` shell impact against a tank using the full War Thunder armor,
/// overmatch, ricochet, spall cone, internal module, crew, fire, and ammo-rack pipeline.
#[allow(clippy::too_many_arguments)]
pub fn resolve_ap_impact(
    shooter_id: TankId,
    shooter_team: Team,
    target_id: TankId,
    target_name: &str,
    target_team: Team,
    target_alive: bool,
    target_state: &TankState,
    damage: &mut TankDamageState,
    shell: &ShellSpec,
    hit: ArmorHit,
    incoming_dir: Vec3,
    impact_speed: f32,
    rng: &mut SimRng,
) -> HitReport {
    let dir = incoming_dir.normalize_or_zero();
    let mut report = HitReport {
        shooter_id,
        target_id,
        target_name: target_name.to_string(),
        target_team,
        outcome: HitOutcome::NonPenetration,
        plate_name: hit.plate_name,
        armor_type: hit.armor_type,
        nominal_armor_mm: hit.nominal_mm,
        impact_angle_deg: 0.0,
        effective_armor_mm: hit.nominal_mm,
        penetration_mm: shell.penetration_at_speed(impact_speed),
        damaged_modules: Vec::new(),
        killed_crew: Vec::new(),
        started_fire: false,
        kill_reason: None,
        position: hit.world_point,
        normal: hit.geom_normal_world,
        ricochet_dir: None,
        inert_hit: !target_alive || shooter_team == target_team,
    };

    if report.inert_hit {
        return report;
    }

    damage.damage_events_received += 1;

    let mut remaining_pen = report.penetration_mm;
    let mut active_hit = hit;

    // Step 1: If the shell struck an external track or gun barrel, damage that module first.
    if let Some(ext_kind) = hit.external_module {
        let ext_dmg = if ext_kind == ModuleKind::GunBarrel {
            58.0
        } else {
            68.0
        };
        if let Some(m) = damage.modules.iter_mut().find(|m| m.kind == ext_kind) {
            let (_, after) = m.apply_damage(ext_dmg);
            record_module_hit(&mut report.damaged_modules, ext_kind, after);
        }
        report.outcome = HitOutcome::ExternalModuleHit;

        // Tracks are spaced exterior armor: if the AP shell penetrates the track (~22 mm),
        // continue the ray into the main hull/turret behind it!
        if ext_kind == ModuleKind::LeftTrack || ext_kind == ModuleKind::RightTrack {
            remaining_pen = (remaining_pen - hit.nominal_mm * hit.armor_type.multiplier()).max(0.0);
            let cont_origin = hit.world_point + dir * 0.06;
            if remaining_pen > 10.0
                && let Some(inner) = raycast_tank_armor(
                    target_state,
                    cont_origin,
                    dir,
                    4.5,
                    false,
                    !damage.turret_blown_off,
                )
            {
                active_hit = inner;
                report.plate_name = inner.plate_name;
                report.armor_type = inner.armor_type;
                report.nominal_armor_mm = inner.nominal_mm;
                report.penetration_mm = remaining_pen;
                report.position = inner.world_point;
                report.normal = inner.geom_normal_world;
            } else {
                return report;
            }
        } else {
            // Gun barrel hit ends on the barrel (no interior compartment behind barrel)
            return report;
        }
    }

    // Step 2: Obliquity angle & ricochet check (with caliber overmatch rules)
    let raw_cos = (-dir).dot(active_hit.plate_normal_world);
    let plate_n = if raw_cos > 0.03 {
        active_hit.plate_normal_world
    } else {
        active_hit.geom_normal_world
    };
    let cos_theta = (-dir).dot(plate_n).clamp(0.03, 1.0);
    let impact_angle_deg = cos_theta.acos().to_degrees();
    report.impact_angle_deg = impact_angle_deg;

    let eff_armor = shell.effective_armor_mm(
        active_hit.nominal_mm,
        impact_angle_deg,
        active_hit.armor_type.multiplier(),
    );
    report.effective_armor_mm = eff_armor;
    report.penetration_mm = remaining_pen;

    let rico_chance = shell.ricochet_chance(impact_angle_deg, active_hit.nominal_mm);
    if rico_chance > 0.0 && (rico_chance >= 0.999 || rng.next_f32() < rico_chance) {
        report.outcome = HitOutcome::Ricochet;
        let refl = (dir - 2.0 * dir.dot(plate_n) * plate_n).normalize_or_zero();
        report.ricochet_dir = Some(if refl.length_squared() > 0.1 { refl } else { plate_n });
        return report;
    }

    // Step 3: Armor penetration check
    if remaining_pen < eff_armor {
        if report.outcome != HitOutcome::ExternalModuleHit {
            report.outcome = HitOutcome::NonPenetration;
        }
        return report;
    }

    // Step 4: PENETRATION! Normalize post-pen axis slightly toward inward armor normal
    report.outcome = HitOutcome::Penetration;
    let residual_pen = (remaining_pen - eff_armor).max(0.0);
    let residual_ratio = (residual_pen / remaining_pen.max(1.0)).clamp(0.0, 1.0);
    let norm_weight = (shell.effective_normalization_deg(active_hit.nominal_mm) / 45.0).clamp(0.0, 0.35);
    let pen_dir = (dir * (1.0 - norm_weight) + (-plate_n) * norm_weight).normalize();
    let entry_pos = active_hit.world_point + pen_dir * 0.06;

    // Track ammo-rack / fire triggers across primary ray + spall cone
    let mut ammo_destroyed_now = false;
    let mut fire_chance = 0.0f32;

    // 4a. Primary AP core penetrator ray (high kinetic damage deep through the tank)
    let core_damage = 80.0 + residual_ratio * 55.0;
    trace_internal_ray(
        target_state,
        damage,
        entry_pos,
        pen_dir,
        5.4,
        core_damage,
        true,
        &mut report,
        &mut ammo_destroyed_now,
        &mut fire_chance,
    );

    // 4b. Secondary armor spall cone (narrow directional cone for solid AP)
    let spall_count = shell.spall_ray_count.max(4) as usize;
    let cone_rad = (shell.spall_cone_half_angle_deg + residual_ratio * 9.0).to_radians();
    let (tangent, bitangent) = orthonormal_basis(pen_dir);
    let spall_damage = 28.0 + residual_ratio * 34.0;
    let spall_range = 2.0 + residual_ratio * 1.4;

    for i in 0..spall_count {
        // Golden-angle spiral inside cone with small deterministic jitter
        let frac = (i as f32 + 0.5) / spall_count as f32;
        let radial = cone_rad * frac.sqrt();
        let az = i as f32 * 2.399_963 + rng.next_f32() * (TAU / spall_count as f32) * 0.35;
        let spall_dir = (pen_dir * radial.cos()
            + (tangent * az.cos() + bitangent * az.sin()) * radial.sin())
            .normalize();
        trace_internal_ray(
            target_state,
            damage,
            entry_pos,
            spall_dir,
            spall_range,
            spall_damage,
            false,
            &mut report,
            &mut ammo_destroyed_now,
            &mut fire_chance,
        );
    }

    // Step 5: Evaluate fire ignition, ammo-rack detonation, and < 2 crew knockout
    if fire_chance > 0.0 && !damage.on_fire && rng.next_f32() < fire_chance.min(0.75) {
        damage.on_fire = true;
        damage.fire_elapsed = 0.0;
        damage.fire_tick_accum = 0.0;
        damage.fire_instigator = Some((shooter_id, shooter_team));
        report.started_fire = true;
    }

    if ammo_destroyed_now && rng.next_f32() < 0.78 {
        damage.turret_blown_off = true;
        report.kill_reason = Some(KillReason::AmmoRackDetonation);
        return report;
    }

    if damage.alive_crew_count() < MIN_OPERATIONAL_CREW {
        report.kill_reason = Some(KillReason::CrewKnockedOut);
        return report;
    }

    damage.ensure_crew_transfers_started();
    report
}

#[allow(clippy::too_many_arguments)]
fn trace_internal_ray(
    tank: &TankState,
    damage: &mut TankDamageState,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    base_dmg: f32,
    is_primary_core: bool,
    report: &mut HitReport,
    ammo_destroyed_now: &mut bool,
    fire_chance: &mut f32,
) {
    // 1. Collect and sort intersected internal modules along the ray (energy attenuates with depth)
    let mut mod_hits: Vec<(usize, f32)> = Vec::new();
    for (idx, m) in damage.modules.iter().enumerate() {
        if matches!(
            m.kind,
            ModuleKind::LeftTrack | ModuleKind::RightTrack | ModuleKind::GunBarrel
        ) {
            continue;
        }
        let (fpos, frot) = m.frame.world_pose(tank);
        if let Some((t, _, _)) =
            ray_vs_oriented_box(origin, dir, max_dist, fpos, frot, m.box_min, m.box_max)
        {
            mod_hits.push((idx, t));
        }
    }
    mod_hits.sort_by(|a, b| a.1.total_cmp(&b.1));

    let mut energy = base_dmg;
    for (idx, _) in mod_hits {
        if energy <= 4.0 {
            break;
        }
        let m = &mut damage.modules[idx];
        let (before, after) = m.apply_damage(energy);
        if after != ModuleHealth::Intact {
            record_module_hit(&mut report.damaged_modules, m.kind, after);
        }
        match m.kind {
            ModuleKind::AmmoRack => {
                if before != ModuleHealth::Destroyed && after == ModuleHealth::Destroyed {
                    *ammo_destroyed_now = true;
                }
            }
            ModuleKind::FuelTank => {
                *fire_chance = fire_chance.max(if after == ModuleHealth::Destroyed {
                    0.55
                } else {
                    0.30
                });
            }
            ModuleKind::Engine => {
                *fire_chance = fire_chance.max(if after == ModuleHealth::Destroyed {
                    0.45
                } else {
                    0.22
                });
            }
            ModuleKind::Transmission => {
                *fire_chance = fire_chance.max(if after == ModuleHealth::Destroyed {
                    0.25
                } else {
                    0.12
                });
            }
            _ => {}
        }
        // Heavy steel blocks (engine/breech/transmission) absorb more residual energy than thin racks
        let absorb = match m.kind {
            ModuleKind::Engine | ModuleKind::GunBreech | ModuleKind::Transmission => {
                if is_primary_core { 0.55 } else { 0.85 }
            }
            _ => if is_primary_core { 0.22 } else { 0.45 },
        };
        energy *= 1.0 - absorb;
    }

    // 2. Intersect crew hitboxes along the ray
    let mut crew_hits: Vec<(usize, f32)> = Vec::new();
    for (idx, c) in damage.crew.iter().enumerate() {
        if !c.alive {
            continue;
        }
        let (fpos, frot) = c.frame.world_pose(tank);
        if let Some((t, _, _)) =
            ray_vs_oriented_box(origin, dir, max_dist, fpos, frot, c.box_min, c.box_max)
        {
            crew_hits.push((idx, t));
        }
    }
    crew_hits.sort_by(|a, b| a.1.total_cmp(&b.1));

    let mut crew_energy = base_dmg;
    for (idx, _) in crew_hits {
        if crew_energy <= 8.0 {
            break;
        }
        let c = &mut damage.crew[idx];
        let seat = c.active_seat;
        if c.apply_damage(crew_energy) && !report.killed_crew.contains(&seat) {
            report.killed_crew.push(seat);
        }
        crew_energy *= if is_primary_core { 0.72 } else { 0.45 };
    }
}

fn record_module_hit(
    list: &mut Vec<(ModuleKind, ModuleHealth)>,
    kind: ModuleKind,
    health: ModuleHealth,
) {
    if let Some(entry) = list.iter_mut().find(|(k, _)| *k == kind) {
        entry.1 = entry.1.max(health);
    } else {
        list.push((kind, health));
    }
}

fn orthonormal_basis(axis: Vec3) -> (Vec3, Vec3) {
    let helper = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let tangent = axis.cross(helper).normalize();
    let bitangent = axis.cross(tangent).normalize();
    (tangent, bitangent)
}
