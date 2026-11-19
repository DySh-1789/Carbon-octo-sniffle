use std::f32::consts::{PI, TAU};

use voxel::glam::{Quat, Vec3};
use voxel::{TankVoxelModels, VoxelWorld, raycast_world};

/// Static performance and geometry parameters for a tank.
#[derive(Clone, Debug)]
pub struct TankConfig {
    pub name: String,
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
    /// Horizontal bounding half-extent used to keep hull inside playable arena.
    pub hull_radius: f32,
    pub turret_mount_offset: Vec3,
    pub gun_mount_offset: Vec3,
    pub muzzle_offset: Vec3,
}

impl TankConfig {
    /// Balanced WWII medium tank profile matching the Stage 1 base voxel tank.
    pub fn stage1_basic(models: &TankVoxelModels) -> Self {
        Self {
            name: "Base Medium Tank".to_string(),
            max_forward_speed: 13.9,
            max_reverse_speed: 5.0,
            acceleration: 5.2,
            brake_decel: 12.0,
            coast_drag: 2.8,
            hull_turn_rate: 1.15,
            turret_yaw_rate: 0.72,
            gun_pitch_rate: 0.45,
            gun_min_pitch: (-8.0f32).to_radians(),
            gun_max_pitch: (20.0f32).to_radians(),
            reload_time: 4.5,
            muzzle_velocity: 780.0,
            shell_drag: 0.04,
            hull_radius: 3.2,
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
#[derive(Copy, Clone, Debug)]
pub struct FiredShot {
    pub muzzle_position: Vec3,
    pub direction: Vec3,
    pub velocity: Vec3,
    pub drag_coeff: f32,
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

    /// Advances tank movement, turret slewing, reload, and recoil by `dt` seconds.
    pub fn step(&mut self, dt: f32, input: TankInput, world: &VoxelWorld) -> Option<FiredShot> {
        let throttle = input.throttle.clamp(-1.0, 1.0);
        let steer = input.steer.clamp(-1.0, 1.0);

        // 1. Longitudinal tracked acceleration / braking / rolling drag
        if throttle.abs() > 0.01 {
            let opposing = (throttle > 0.0 && self.forward_speed < -0.1)
                || (throttle < 0.0 && self.forward_speed > 0.1);
            let accel = if opposing {
                self.config.brake_decel
            } else {
                self.config.acceleration
            };
            self.forward_speed += throttle * accel * dt;
        } else {
            let decel = self.config.coast_drag * dt;
            if self.forward_speed.abs() <= decel {
                self.forward_speed = 0.0;
            } else {
                self.forward_speed -= self.forward_speed.signum() * decel;
            }
        }
        self.forward_speed = self
            .forward_speed
            .clamp(-self.config.max_reverse_speed, self.config.max_forward_speed);

        // 2. Hull steering (neutral pivot turn when stationary, reverse steering inversion)
        if steer.abs() > 0.01 {
            let reverse_flip = if self.forward_speed < -0.4 { -1.0 } else { 1.0 };
            // Slightly slower turn at full speed to preserve momentum feel
            let speed_ratio =
                (self.forward_speed.abs() / self.config.max_forward_speed).clamp(0.0, 1.0);
            let turn_factor = 1.0 - 0.25 * speed_ratio;
            let delta_yaw = steer * reverse_flip * self.config.hull_turn_rate * turn_factor * dt;
            self.hull_yaw = wrap_angle(self.hull_yaw + delta_yaw);
            // Turning tracks bleeds a small amount of forward speed
            self.forward_speed *= (1.0 - 0.35 * steer.abs() * dt).max(0.0);
        }

        // 3. Integrate horizontal movement and clamp to playable arena bounds
        let forward = self.hull_forward();
        let horiz_fwd = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        self.position += horiz_fwd * self.forward_speed * dt;

        let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
        let pad = self.config.hull_radius + 1.0;
        let clamped_x = self.position.x.clamp(min_x + pad, max_x - pad);
        let clamped_z = self.position.z.clamp(min_z + pad, max_z - pad);
        if (clamped_x - self.position.x).abs() > 1e-4 || (clamped_z - self.position.z).abs() > 1e-4
        {
            self.position.x = clamped_x;
            self.position.z = clamped_z;
            self.forward_speed *= 0.2;
        }

        // Ground snap (Stage 1 flat ground; expanded to 6-point suspension on Stage 2)
        if let Some(surface_y) = world.surface_height_at(self.position.x, self.position.z) {
            self.position.y = surface_y;
        }

        // 4. Independent turret yaw & gun pitch tracking toward `aim_world_point`
        self.update_turret_and_gun(dt, input.aim_world_point);

        // 5. Reload & visual barrel recoil decay
        self.reload_remaining = (self.reload_remaining - dt).max(0.0);
        self.recoil_offset = (self.recoil_offset - 1.1 * dt).max(0.0);

        // 6. Main gun firing
        if input.fire && self.is_loaded() {
            self.reload_remaining = self.config.reload_time;
            self.recoil_offset = 0.32;
            self.shots_fired += 1;

            let dir = self.gun_forward();
            let hull_vel = horiz_fwd * self.forward_speed;
            return Some(FiredShot {
                muzzle_position: self.muzzle_world_position(),
                direction: dir,
                velocity: dir * self.config.muzzle_velocity + hull_vel,
                drag_coeff: self.config.shell_drag,
            });
        }

        None
    }

    fn update_turret_and_gun(&mut self, dt: f32, aim_world_point: Vec3) {
        let gun_pos = self.gun_world_position();
        let to_target_world = aim_world_point - gun_pos;
        if to_target_world.length_squared() < 0.25 {
            return;
        }

        // Express aim vector in hull-local frame so turret rotates smoothly as hull turns
        let to_target_hull = self.hull_rotation().inverse() * to_target_world;
        let desired_yaw = (-to_target_hull.x).atan2(-to_target_hull.z);
        let yaw_err = wrap_angle(desired_yaw - self.turret_yaw_local);
        let max_yaw_step = self.config.turret_yaw_rate * dt;
        self.turret_yaw_local =
            wrap_angle(self.turret_yaw_local + yaw_err.clamp(-max_yaw_step, max_yaw_step));

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

/// Normalizes an angle in radians into `(-PI, PI]`.
#[inline]
pub fn wrap_angle(mut angle: f32) -> f32 {
    angle = (angle + PI).rem_euclid(TAU) - PI;
    if angle <= -PI { angle + TAU } else { angle }
}
