use std::f32::consts::{PI, TAU};

use voxel::glam::{IVec3, Quat, Vec3};
use voxel::{TankVoxelModels, VoxelWorld, raycast_world};

use crate::battle::SimRng;
use crate::catalog::{Nation, VehicleClass};
use crate::damage::{ArmorProfile, DamageModifiers};
use crate::shell::ShellSpec;

/// Static performance and geometry parameters for a tank.
#[derive(Clone, Debug)]
pub struct TankConfig {
    /// Catalog identifier (e.g. `"us_m4a3e2_76"`, `"su_t44_100"`, or `"basic_medium"`).
    pub vehicle_id: String,
    pub name: String,
    pub nation: Nation,
    /// Tech tree tier `1..=8`.
    pub tier: u8,
    pub class: VehicleClass,
    /// Maximum forward speed in m/s (~13.9 m/s ≈ 50 km/h).
    pub max_forward_speed: f32,
    /// Maximum reverse speed in m/s (~5.0 m/s ≈ 18 km/h).
    pub max_reverse_speed: f32,
    /// Forward/reverse linear acceleration in m/s².
    pub acceleration: f32,
    /// Braking deceleration when opposing velocity in m/s².
    pub brake_decel: f32,
    /// Natural rolling resistance deceleration in m/s².
    pub coast_drag: f32,
    /// Hull yaw turn rate in rad/s.
    pub hull_turn_rate: f32,
    /// Turret traverse rate in rad/s.
    pub turret_yaw_rate: f32,
    /// Optional symmetric horizontal gun traverse limit in radians for casemate TDs (`None` = 360°).
    pub turret_yaw_limit: Option<f32>,
    /// Gun elevation/depression rate in rad/s.
    pub gun_pitch_rate: f32,
    /// Minimum gun depression angle in radians (negative).
    pub gun_min_pitch: f32,
    /// Maximum gun elevation angle in radians (positive).
    pub gun_max_pitch: f32,
    /// Gun reload duration in seconds.
    pub reload_time: f32,
    /// Projectile muzzle velocity in m/s.
    pub muzzle_velocity: f32,
    /// Projectile aerodynamic drag factor (`1/s` velocity damping).
    pub shell_drag: f32,
    /// Primary cannon shell specification (Stage 4: solid kinetic AP).
    pub shell: ShellSpec,
    /// Historical plate thicknesses, slopes, armor types, and crew/transmission layout.
    pub armor: ArmorProfile,
    /// Horizontal bounding half-extent used to keep hull inside playable arena.
    pub hull_radius: f32,
    /// Half-width of tank hull/tracks in meters for obstacle collision & suspension.
    pub hull_half_width: f32,
    /// Half-length of tank hull/tracks in meters for obstacle collision & suspension.
    pub hull_half_length: f32,
    pub turret_mount_offset: Vec3,
    pub gun_mount_offset: Vec3,
    pub muzzle_offset: Vec3,
}

impl TankConfig {
    /// Balanced WWII medium tank profile matching the base voxel tank.
    pub fn stage1_basic(models: &TankVoxelModels) -> Self {
        let shell = ShellSpec::basic_75mm_ap();
        Self {
            vehicle_id: "basic_medium".to_string(),
            name: "Base Medium Tank".to_string(),
            nation: Nation::Usa,
            tier: 3,
            class: VehicleClass::Medium,
            max_forward_speed: 13.9,
            max_reverse_speed: 5.0,
            acceleration: 5.2,
            brake_decel: 12.0,
            coast_drag: 2.8,
            hull_turn_rate: 1.15,
            turret_yaw_rate: 0.72,
            turret_yaw_limit: None,
            gun_pitch_rate: 0.45,
            gun_min_pitch: (-8.0f32).to_radians(),
            gun_max_pitch: (20.0f32).to_radians(),
            reload_time: 4.5,
            muzzle_velocity: shell.muzzle_velocity,
            shell_drag: shell.drag_coeff,
            shell,
            armor: ArmorProfile::basic_medium_tank(),
            hull_radius: 3.2,
            hull_half_width: 1.55,
            hull_half_length: 2.65,
            turret_mount_offset: models.turret_mount_offset,
            gun_mount_offset: models.gun_mount_offset,
            muzzle_offset: models.muzzle_offset,
        }
    }
}

/// Control inputs consumed by `TankState::step` on each fixed tick.
#[derive(Copy, Clone, Debug)]
pub struct TankInput {
    /// `-1.0` (reverse) .. `+1.0` (forward).
    pub throttle: f32,
    /// `-1.0` (turn right) .. `+1.0` (turn left).
    pub steer: f32,
    /// World-space point the player/AI wants the gun to aim at.
    pub aim_world_point: Vec3,
    /// Trigger fire if gun is reloaded.
    pub fire: bool,
}

impl Default for TankInput {
    fn default() -> Self {
        Self {
            throttle: 0.0,
            steer: 0.0,
            aim_world_point: Vec3::new(0.0, 2.0, -100.0),
            fire: false,
        }
    }
}

/// Spawn parameters emitted when a tank successfully fires its main cannon.
#[derive(Clone, Debug)]
pub struct FiredShot {
    pub muzzle_position: Vec3,
    pub direction: Vec3,
    pub velocity: Vec3,
    pub drag_coeff: f32,
    pub shell: ShellSpec,
}

/// Dynamic runtime state of a single tank in the simulation.
#[derive(Clone, Debug)]
pub struct TankState {
    pub config: TankConfig,
    /// World position of the hull bottom center.
    pub position: Vec3,
    /// Current forward signed speed along hull `-Z` axis in m/s.
    pub forward_speed: f32,
    /// Hull yaw in radians (`0` faces `-Z`, `+` turns left).
    pub hull_yaw: f32,
    /// Hull pitch in radians (used for terrain alignment from Stage 2 onward).
    pub hull_pitch: f32,
    /// Hull roll in radians (used for terrain alignment from Stage 2 onward).
    pub hull_roll: f32,
    /// Turret yaw relative to hull in radians.
    pub turret_yaw_local: f32,
    /// Gun barrel pitch relative to turret in radians (`+` up, `-` down).
    pub gun_pitch_local: f32,
    /// Seconds remaining until next shot can be fired (`0.0` = loaded).
    pub reload_remaining: f32,
    /// Visual barrel recoil displacement along `+Z` in meters (`0.0..=0.35`).
    pub recoil_offset: f32,
    /// Total shots fired (handy telemetry / testing).
    pub shots_fired: u32,
}

impl TankState {
    pub fn new(config: TankConfig, spawn_pos: Vec3, hull_yaw: f32) -> Self {
        Self {
            config,
            position: spawn_pos,
            forward_speed: 0.0,
            hull_yaw,
            hull_pitch: 0.0,
            hull_roll: 0.0,
            turret_yaw_local: 0.0,
            gun_pitch_local: 0.0,
            reload_remaining: 0.0,
            recoil_offset: 0.0,
            shots_fired: 0,
        }
    }

    #[inline]
    pub fn hull_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.hull_yaw)
            * Quat::from_rotation_x(self.hull_pitch)
            * Quat::from_rotation_z(self.hull_roll)
    }

    #[inline]
    pub fn hull_forward(&self) -> Vec3 {
        self.hull_rotation() * Vec3::NEG_Z
    }

    #[inline]
    pub fn turret_world_position(&self) -> Vec3 {
        self.position + self.hull_rotation() * self.config.turret_mount_offset
    }

    #[inline]
    pub fn turret_world_rotation(&self) -> Quat {
        self.hull_rotation() * Quat::from_rotation_y(self.turret_yaw_local)
    }

    #[inline]
    pub fn gun_world_position(&self) -> Vec3 {
        self.turret_world_position() + self.turret_world_rotation() * self.config.gun_mount_offset
    }

    #[inline]
    pub fn gun_world_rotation(&self) -> Quat {
        self.turret_world_rotation() * Quat::from_rotation_x(self.gun_pitch_local)
    }

    #[inline]
    pub fn gun_forward(&self) -> Vec3 {
        (self.gun_world_rotation() * Vec3::NEG_Z).normalize()
    }

    #[inline]
    pub fn muzzle_world_position(&self) -> Vec3 {
        self.gun_world_position() + self.gun_world_rotation() * self.config.muzzle_offset
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.reload_remaining <= 0.0
    }

    #[inline]
    pub fn reload_progress(&self) -> f32 {
        if self.config.reload_time <= 0.0 {
            1.0
        } else {
            (1.0 - self.reload_remaining / self.config.reload_time).clamp(0.0, 1.0)
        }
    }

    #[inline]
    pub fn speed_kmh(&self) -> f32 {
        self.forward_speed * 3.6
    }

    /// Human-readable gear indicator (`R`, `N`, `F1`..`F4`) for HUD telemetry.
    pub fn gear_label(&self) -> &'static str {
        let kmh = self.speed_kmh();
        if kmh < -1.0 {
            "R"
        } else if kmh.abs() <= 1.0 {
            "N"
        } else if kmh < 12.0 {
            "F1"
        } else if kmh < 25.0 {
            "F2"
        } else if kmh < 38.0 {
            "F3"
        } else {
            "F4"
        }
    }

    /// Raycasts from the gun muzzle along its actual current bore axis to locate where the barrel
    /// is really pointing (used for the trailing War Thunder/WoT aim circle).
    pub fn actual_gun_aim_point(&self, world: &VoxelWorld, max_range: f32) -> Vec3 {
        let origin = self.muzzle_world_position();
        let dir = self.gun_forward();
        if let Some(hit) = raycast_world(world, origin, dir, max_range) {
            hit.point
        } else {
            origin + dir * max_range
        }
    }

    /// Immediately aligns tank height, pitch, and roll to the terrain beneath its 6 track probes.
    /// Called once on spawn so the tank sits flush on slopes from the very first frame.
    pub fn settle_on_terrain(&mut self, world: &VoxelWorld) {
        if let Some(surface_y) = world.surface_height_at(self.position.x, self.position.z) {
            self.position.y = surface_y;
        }
        let (target_y, target_pitch, target_roll) = self.compute_suspension_targets(world);
        self.position.y = target_y;
        self.hull_pitch = target_pitch;
        self.hull_roll = target_roll;
    }

    /// Returns true if the tank tracks are currently standing on shallow fordable water.
    pub fn is_on_water(&self, world: &VoxelWorld) -> bool {
        let inv = 1.0 / world.voxel_size;
        let vx = (self.position.x * inv).floor() as i32;
        let vy = ((self.position.y - 0.15) * inv).floor() as i32;
        let vz = (self.position.z * inv).floor() as i32;
        world.get_voxel(IVec3::new(vx, vy, vz)).is_water()
    }

    /// Advances tank movement, 6-point terrain suspension, obstacle collisions, turret slewing,
    /// reload, and recoil by `dt` seconds (intact vehicle; used by solo tests).
    pub fn step(&mut self, dt: f32, input: TankInput, world: &VoxelWorld) -> Option<FiredShot> {
        let mut rng = SimRng::new(1);
        self.step_with_modifiers(dt, input, world, DamageModifiers::intact(), &mut rng)
    }

    /// Advances tank simulation with active War Thunder module & crew damage penalties.
    pub fn step_with_modifiers(
        &mut self,
        dt: f32,
        input: TankInput,
        world: &VoxelWorld,
        mods: DamageModifiers,
        rng: &mut SimRng,
    ) -> Option<FiredShot> {
        let throttle = if mods.can_drive {
            input.throttle.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let mut steer = if mods.can_drive {
            input.steer.clamp(-1.0, 1.0) * mods.steer_scale
        } else {
            0.0
        };
        if mods.can_drive && mods.broken_track_pull.abs() > 0.01 && throttle.abs() > 0.05 {
            steer = (steer + mods.broken_track_pull * throttle.signum()).clamp(-1.0, 1.0);
        }
        let in_water = self.is_on_water(world);

        // 1. Longitudinal tracked acceleration / braking / slope grade physics / water ford drag
        let slope_sin = self.hull_pitch.sin();
        if throttle.abs() > 0.01 {
            let opposing = (throttle > 0.0 && self.forward_speed < -0.1)
                || (throttle < 0.0 && self.forward_speed > 0.1);
            let accel = if opposing {
                self.config.brake_decel
            } else {
                self.config.acceleration * mods.speed_scale
            };
            self.forward_speed += throttle * accel * dt;
            // Uphill pitch reduces climbing acceleration; downhill adds slight gravity momentum
            self.forward_speed -= slope_sin * 5.6 * dt;
        } else {
            let drag_mult = if mods.can_drive { 1.0 } else { 2.6 };
            let decel = self.config.coast_drag * drag_mult * dt;
            if self.forward_speed.abs() <= decel {
                // Tracked hill-hold brake keeps stationary tank steady on hull-down slopes
                self.forward_speed = 0.0;
            } else {
                self.forward_speed -= self.forward_speed.signum() * decel;
                self.forward_speed -= slope_sin * 3.8 * dt;
            }
        }

        // Dynamic slope, water & damage speed envelope
        let uphill_factor = (1.0 - 0.90 * slope_sin.max(0.0)).clamp(0.45, 1.0);
        let downhill_boost = 1.0 + 0.14 * (-slope_sin).max(0.0);
        let water_factor = if in_water { 0.72 } else { 1.0 };
        let eff_max_fwd =
            self.config.max_forward_speed * uphill_factor * downhill_boost * water_factor * mods.speed_scale;
        let eff_max_rev = self.config.max_reverse_speed * water_factor * mods.speed_scale;
        self.forward_speed = self.forward_speed.clamp(-eff_max_rev, eff_max_fwd);

        // 2. Hull steering (neutral pivot turn + reverse steering inversion + wall-wedge protection)
        if steer.abs() > 0.01 {
            let reverse_flip = if self.forward_speed < -0.4 { -1.0 } else { 1.0 };
            let speed_ratio =
                (self.forward_speed.abs() / self.config.max_forward_speed).clamp(0.0, 1.0);
            let turn_factor = 1.0 - 0.25 * speed_ratio;
            let delta_yaw = steer * reverse_flip * self.config.hull_turn_rate * turn_factor * dt;
            let cand_yaw = wrap_angle(self.hull_yaw + delta_yaw);

            let current_hits = self.count_hull_collisions(
                world,
                self.position.x,
                self.position.z,
                self.hull_yaw,
            );
            let cand_hits =
                self.count_hull_collisions(world, self.position.x, self.position.z, cand_yaw);
            if cand_hits == 0 || cand_hits <= current_hits {
                self.hull_yaw = cand_yaw;
            }
            // Turning tracks bleeds a small amount of forward speed
            self.forward_speed *= (1.0 - 0.35 * steer.abs() * dt).max(0.0);
        }

        // 3. Horizontal movement with sliding voxel building/rock/cliff collision resolution
        let horiz_fwd = Vec3::new(-self.hull_yaw.sin(), 0.0, -self.hull_yaw.cos());
        let move_delta = horiz_fwd * self.forward_speed * dt;
        let old_x = self.position.x;
        let old_z = self.position.z;

        let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
        let pad = self.config.hull_radius + 1.0;
        let cand_x = (old_x + move_delta.x).clamp(min_x + pad, max_x - pad);
        let cand_z = (old_z + move_delta.z).clamp(min_z + pad, max_z - pad);

        let bound_hit = (cand_x - (old_x + move_delta.x)).abs() > 1e-4
            || (cand_z - (old_z + move_delta.z)).abs() > 1e-4;

        if self.count_hull_collisions(world, cand_x, cand_z, self.hull_yaw) == 0 {
            self.position.x = cand_x;
            self.position.z = cand_z;
            if bound_hit {
                self.forward_speed *= 0.2;
            }
        } else if (cand_x - old_x).abs() > 1e-5
            && self.count_hull_collisions(world, cand_x, old_z, self.hull_yaw) == 0
        {
            // Slide smoothly along X facade
            self.position.x = cand_x;
            self.forward_speed *= 0.86;
        } else if (cand_z - old_z).abs() > 1e-5
            && self.count_hull_collisions(world, old_x, cand_z, self.hull_yaw) == 0
        {
            // Slide smoothly along Z facade
            self.position.z = cand_z;
            self.forward_speed *= 0.86;
        } else {
            // Solid wall/boulder impact — stop forward motion cleanly
            self.forward_speed = 0.0;
        }

        // 4. 6-point tracked suspension over voxel terrain (smooth height, pitch, and roll)
        let (target_y, target_pitch, target_roll) = self.compute_suspension_targets(world);
        let y_rate = if target_y >= self.position.y { 14.0 } else { 10.0 };
        let y_alpha = 1.0 - (-y_rate * dt).exp();
        self.position.y += (target_y - self.position.y) * y_alpha;

        let att_alpha = 1.0 - (-9.5 * dt).exp();
        self.hull_pitch += (target_pitch - self.hull_pitch) * att_alpha;
        self.hull_roll += (target_roll - self.hull_roll) * att_alpha;

        // 5. Independent turret yaw & gun pitch tracking (respects dead Gunner / jammed TurretRing)
        if mods.can_aim {
            self.update_turret_and_gun(dt, input.aim_world_point, mods.turret_rate_scale);
        }

        // 6. Reload (slowed when Loader dead or Breech damaged) & visual recoil decay
        let reload_rate = (1.0 / mods.reload_time_scale.max(0.1)).clamp(0.1, 2.0);
        self.reload_remaining = (self.reload_remaining - dt * reload_rate).max(0.0);
        self.recoil_offset = (self.recoil_offset - 1.1 * dt).max(0.0);

        // 7. Main gun firing (blocked if Gunner absent or Breech/Barrel destroyed)
        if input.fire && self.is_loaded() && mods.can_fire {
            self.reload_remaining = self.config.reload_time;
            self.recoil_offset = 0.32;
            self.shots_fired += 1;

            let mut dir = self.gun_forward();
            if mods.fire_dispersion_rad > 1e-4 {
                let yaw_off = rng.signed() * mods.fire_dispersion_rad;
                let pitch_off = rng.signed() * mods.fire_dispersion_rad;
                let rot = Quat::from_rotation_y(yaw_off) * Quat::from_rotation_x(pitch_off);
                dir = (rot * dir).normalize();
            }
            let hull_vel = horiz_fwd * self.forward_speed;
            return Some(FiredShot {
                muzzle_position: self.muzzle_world_position(),
                direction: dir,
                velocity: dir * self.config.muzzle_velocity + hull_vel,
                drag_coeff: self.config.shell_drag,
                shell: self.config.shell.clone(),
            });
        }

        None
    }

    /// Computes target `(height_y, hull_pitch, hull_roll)` from 6 bilinear-smoothed track contact probes.
    fn compute_suspension_targets(&self, world: &VoxelWorld) -> (f32, f32, f32) {
        let track_x = (self.config.hull_half_width - 0.20).max(0.9);
        let track_z = (self.config.hull_half_length - 0.65).max(1.5);
        let sin_y = self.hull_yaw.sin();
        let cos_y = self.hull_yaw.cos();
        let sin_p = self.hull_pitch.sin();
        let sin_r = self.hull_roll.sin();

        let probes = [
            (-track_x, -track_z), // 0: left front
            (-track_x, 0.0),      // 1: left mid
            (-track_x, track_z),  // 2: left rear
            (track_x, -track_z),  // 3: right front
            (track_x, 0.0),       // 4: right mid
            (track_x, track_z),   // 5: right rear
        ];

        let mut h = [self.position.y; 6];
        let mut valid_count = 0u32;
        for (i, &(lx, lz)) in probes.iter().enumerate() {
            let wx = self.position.x + lx * cos_y + lz * sin_y;
            let wz = self.position.z - lx * sin_y + lz * cos_y;
            let expected_y = self.position.y - lz * sin_p + lx * sin_r;
            if let Some(gy) = sample_smooth_ground(world, wx, wz, expected_y) {
                h[i] = gy;
                valid_count += 1;
            }
        }

        if valid_count == 0 {
            return (self.position.y, 0.0, 0.0);
        }

        let avg_y = h.iter().sum::<f32>() / 6.0;
        let mid_y = (h[1] + h[4]) * 0.5;
        let target_y = avg_y.max(mid_y - 0.08);

        let front_y = (h[0] + h[3]) * 0.5;
        let rear_y = (h[2] + h[5]) * 0.5;
        let left_y = (h[0] + h[1] + h[2]) / 3.0;
        let right_y = (h[3] + h[4] + h[5]) / 3.0;

        // Positive pitch = nose UP (+Y around +X); positive roll = right side UP (+Y around +Z)
        let target_pitch = (front_y - rear_y)
            .atan2(2.0 * track_z)
            .clamp(-0.58, 0.58);
        let target_roll = (right_y - left_y)
            .atan2(2.0 * track_x)
            .clamp(-0.52, 0.52);

        (target_y, target_pitch, target_roll)
    }

    /// Counts how many oriented hull-perimeter probes intersect solid obstacles `>= 2` voxels
    /// (`> 0.65 m`) above local track clearance, such as buildings, rocks, walls, tree trunks, or cliffs.
    pub fn count_hull_collisions(&self, world: &VoxelWorld, cx: f32, cz: f32, yaw: f32) -> usize {
        let hw = self.config.hull_half_width;
        let hl = self.config.hull_half_length;
        let sin_y = yaw.sin();
        let cos_y = yaw.cos();
        let sin_p = self.hull_pitch.sin();
        let sin_r = self.hull_roll.sin();
        let inv_vs = 1.0 / world.voxel_size;

        // 16 tightly spaced perimeter probes so even single-voxel (0.5 m) pillars/trunks cannot slip through
        let perimeter: [(f32, f32); 16] = [
            // Front bumper
            (-hw, -hl),
            (-hw * 0.5, -hl),
            (0.0, -hl),
            (hw * 0.5, -hl),
            (hw, -hl),
            // Rear bumper
            (-hw, hl),
            (-hw * 0.5, hl),
            (0.0, hl),
            (hw * 0.5, hl),
            (hw, hl),
            // Left track flank
            (-hw, -hl * 0.5),
            (-hw, 0.0),
            (-hw, hl * 0.5),
            // Right track flank
            (hw, -hl * 0.5),
            (hw, 0.0),
            (hw, hl * 0.5),
        ];

        let mut hits = 0usize;
        for &(lx, lz) in &perimeter {
            let wx = cx + lx * cos_y + lz * sin_y;
            let wz = cz - lx * sin_y + lz * cos_y;
            let probe_ground = self.position.y - lz * sin_p + lx * sin_r;
            // Use the higher of hull center or sloped probe plane so a nose-down dip entering an
            // uphill valley transition never treats a climbable 1-voxel (0.5 m) step as a wall.
            let ref_y = self.position.y.max(probe_ground);
            let vx = (wx * inv_vs).floor() as i32;
            let vz = (wz * inv_vs).floor() as i32;

            // Check 3 vertical bands above 1-voxel (0.5 m) step clearance: blocks walls/buildings/
            // boulders/parapets/trunks >= 2 voxels (1.0 m) tall while staying below tree canopies.
            let mut blocked = false;
            for dy in [0.75f32, 1.25, 1.75] {
                let vy = ((ref_y + dy) * inv_vs).floor() as i32;
                if world.is_solid(IVec3::new(vx, vy, vz)) {
                    blocked = true;
                    break;
                }
            }
            if blocked {
                hits += 1;
            }
        }
        hits
    }

    fn update_turret_and_gun(&mut self, dt: f32, aim_world_point: Vec3, turret_rate_scale: f32) {
        let gun_pos = self.gun_world_position();
        let to_target_world = aim_world_point - gun_pos;
        if to_target_world.length_squared() < 0.25 {
            return;
        }

        // Express aim vector in hull-local frame so turret rotates smoothly as hull turns
        let to_target_hull = self.hull_rotation().inverse() * to_target_world;
        let desired_yaw = (-to_target_hull.x).atan2(-to_target_hull.z);
        let max_yaw_step = self.config.turret_yaw_rate * turret_rate_scale * dt;
        if let Some(limit) = self.config.turret_yaw_limit {
            let clamped_desired = desired_yaw.clamp(-limit, limit);
            let yaw_err = clamped_desired - self.turret_yaw_local;
            self.turret_yaw_local =
                (self.turret_yaw_local + yaw_err.clamp(-max_yaw_step, max_yaw_step))
                    .clamp(-limit, limit);
        } else {
            let yaw_err = wrap_angle(desired_yaw - self.turret_yaw_local);
            self.turret_yaw_local =
                wrap_angle(self.turret_yaw_local + yaw_err.clamp(-max_yaw_step, max_yaw_step));
        }

        // Express aim vector in turret-local frame for gun elevation/depression
        let to_target_turret = self.turret_world_rotation().inverse() * to_target_world;
        let horiz = (to_target_turret.x * to_target_turret.x
            + to_target_turret.z * to_target_turret.z)
            .sqrt()
            .max(1e-4);
        let desired_pitch = to_target_turret
            .y
            .atan2(horiz)
            .clamp(self.config.gun_min_pitch, self.config.gun_max_pitch);
        let pitch_err = desired_pitch - self.gun_pitch_local;
        let max_pitch_step = self.config.gun_pitch_rate * dt;
        self.gun_pitch_local = (self.gun_pitch_local
            + pitch_err.clamp(-max_pitch_step, max_pitch_step))
        .clamp(self.config.gun_min_pitch, self.config.gun_max_pitch);
    }
}

/// Bilinearly interpolates driveable voxel ground height around `(wx, wz)` near `ref_y` so
/// 0.5 m voxel terrain stairs drive like continuous hills without vertical snapping jitter.
fn sample_smooth_ground(world: &VoxelWorld, wx: f32, wz: f32, ref_y: f32) -> Option<f32> {
    let vs = world.voxel_size;
    let gx = wx / vs - 0.5;
    let gz = wz / vs - 0.5;
    let ix = gx.floor();
    let iz = gz.floor();
    let fx = (gx - ix).clamp(0.0, 1.0);
    let fz = (gz - iz).clamp(0.0, 1.0);

    let samples = [
        ((ix + 0.5) * vs, (iz + 0.5) * vs, (1.0 - fx) * (1.0 - fz)),
        ((ix + 1.5) * vs, (iz + 0.5) * vs, fx * (1.0 - fz)),
        ((ix + 0.5) * vs, (iz + 1.5) * vs, (1.0 - fx) * fz),
        ((ix + 1.5) * vs, (iz + 1.5) * vs, fx * fz),
    ];

    let mut sum = 0.0f32;
    let mut w_sum = 0.0f32;
    for &(sx, sz, w) in &samples {
        if let Some(h) = world.ground_height_near(sx, sz, ref_y, 0.75, 2.2) {
            sum += h * w;
            w_sum += w;
        }
    }

    if w_sum > 1e-4 {
        Some(sum / w_sum)
    } else {
        world.ground_height_near(wx, wz, ref_y, 0.75, 2.5)
    }
}

/// Normalizes an angle in radians into `(-PI, PI]`.
#[inline]
pub fn wrap_angle(mut angle: f32) -> f32 {
    angle = (angle + PI).rem_euclid(TAU) - PI;
    if angle <= -PI { angle + TAU } else { angle }
}
