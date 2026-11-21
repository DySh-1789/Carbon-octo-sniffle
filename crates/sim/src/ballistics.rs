use voxel::VoxelId;
use voxel::glam::Vec3;

use crate::battle::{TankId, Team};
use crate::shell::ShellSpec;

/// Standard gravity acceleration in m/s².
pub const GRAVITY: f32 = 9.81;

/// Fixed simulation timestep (60 Hz) used by both headless tests and Bevy `FixedUpdate`.
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Active ballistic projectile in flight.
#[derive(Clone, Debug)]
pub struct Projectile {
    pub id: u64,
    pub owner: TankId,
    pub owner_team: Team,
    pub shell: ShellSpec,
    /// True once this projectile has ricocheted off armor (prevents infinite bounce loops).
    pub bounced: bool,
    pub position: Vec3,
    pub prev_position: Vec3,
    pub velocity: Vec3,
    pub drag_coeff: f32,
    pub age: f32,
    pub max_lifetime: f32,
}

impl Projectile {
    /// Integrates gravity and linear aerodynamic drag (semi-implicit Euler) and returns the
    /// displacement for this tick without moving the shell (collision is resolved by the caller).
    #[inline]
    pub fn integrate(&mut self, dt: f32) -> Vec3 {
        self.age += dt;
        self.prev_position = self.position;
        let drag = (1.0 - self.drag_coeff * dt).max(0.0);
        self.velocity *= drag;
        self.velocity.y -= GRAVITY * dt;
        self.velocity * dt
    }
}

/// Event emitted when a projectile collides with voxel geometry (including Stage 7 destruction debris).
#[derive(Clone, Debug)]
pub struct ImpactEvent {
    pub projectile_id: u64,
    pub position: Vec3,
    pub normal: Vec3,
    pub material: VoxelId,
    pub impact_speed: f32,
    pub blast_radius_m: f32,
    pub is_he: bool,
    pub destroyed_voxels: u32,
    pub debris: Vec<(Vec3, VoxelId)>,
}
