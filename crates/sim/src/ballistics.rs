use voxel::glam::Vec3;
use voxel::{VoxelId, VoxelWorld, raycast_world};

use crate::tank::{FiredShot, TankConfig, TankInput, TankState};

/// Standard gravity acceleration in m/s².
pub const GRAVITY: f32 = 9.81;

/// Fixed simulation timestep (60 Hz) used by both headless tests and Bevy `FixedUpdate`.
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Active ballistic projectile in flight.
#[derive(Copy, Clone, Debug)]
pub struct Projectile {
    pub id: u64,
    pub position: Vec3,
    pub prev_position: Vec3,
    pub velocity: Vec3,
    pub drag_coeff: f32,
    pub age: f32,
    pub max_lifetime: f32,
}

/// Event emitted when a projectile collides with voxel geometry.
#[derive(Copy, Clone, Debug)]
pub struct ImpactEvent {
    pub projectile_id: u64,
    pub position: Vec3,
    pub normal: Vec3,
    pub material: VoxelId,
    pub impact_speed: f32,
}

/// Complete headless simulation state for Stage 1.
pub struct SimWorld {
    pub voxels: VoxelWorld,
    pub player_tank: TankState,
    pub projectiles: Vec<Projectile>,
    pub recent_impacts: Vec<ImpactEvent>,
    pub last_shot: Option<FiredShot>,
    pub tick: u64,
    next_projectile_id: u64,
}

impl SimWorld {
    pub fn new(voxels: VoxelWorld, tank_config: TankConfig) -> Self {
        let spawn_y = voxels.surface_height_at(0.0, 0.0).unwrap_or(0.0);
        let player_tank = TankState::new(tank_config, Vec3::new(0.0, spawn_y, 0.0), 0.0);
        Self {
            voxels,
            player_tank,
            projectiles: Vec::new(),
            recent_impacts: Vec::new(),
            last_shot: None,
            tick: 0,
            next_projectile_id: 1,
        }
    }

    /// Advances the deterministic simulation by `dt` seconds.
    pub fn step_fixed(&mut self, dt: f32, player_input: TankInput) {
        self.tick += 1;
        self.recent_impacts.clear();
        self.last_shot = None;

        // 1. Step player tank & spawn shell if fired
        if let Some(shot) = self.player_tank.step(dt, player_input, &self.voxels) {
            let id = self.next_projectile_id;
            self.next_projectile_id += 1;
            self.projectiles.push(Projectile {
                id,
                position: shot.muzzle_position,
                prev_position: shot.muzzle_position,
                velocity: shot.velocity,
                drag_coeff: shot.drag_coeff,
                age: 0.0,
                max_lifetime: 8.0,
            });
            self.last_shot = Some(shot);
        }

        // 2. Step ballistic projectiles with swept 3D DDA collision so fast shells never tunnel
        let mut surviving = Vec::with_capacity(self.projectiles.len());
        let (min_x, max_x, min_z, max_z) = self.voxels.playable_bounds_xz();
        let margin = 120.0;

        for mut proj in self.projectiles.drain(..) {
            proj.age += dt;
            if proj.age > proj.max_lifetime {
                continue;
            }

            proj.prev_position = proj.position;

            // Semi-implicit Euler with gravity and linear aerodynamic drag
            let drag = (1.0 - proj.drag_coeff * dt).max(0.0);
            proj.velocity *= drag;
            proj.velocity.y -= GRAVITY * dt;

            let step_vec = proj.velocity * dt;
            let step_dist = step_vec.length();

            if step_dist > 1e-5
                && let Some(hit) = raycast_world(&self.voxels, proj.prev_position, step_vec, step_dist)
            {
                self.recent_impacts.push(ImpactEvent {
                    projectile_id: proj.id,
                    position: hit.point,
                    normal: hit.normal.as_vec3(),
                    material: hit.material,
                    impact_speed: proj.velocity.length(),
                });
                continue;
            }

            proj.position = proj.prev_position + step_vec;

            // Despawn projectiles that leave the extended world volume
            let out_of_bounds = proj.position.y < -40.0
                || proj.position.x < min_x - margin
                || proj.position.x > max_x + margin
                || proj.position.z < min_z - margin
                || proj.position.z > max_z + margin;
            if !out_of_bounds {
                surviving.push(proj);
            }
        }
        self.projectiles = surviving;
    }
}
