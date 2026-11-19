use bevy::prelude::*;
use sim::{FiredShot, HitReport, ImpactEvent, KillEvent, SimWorld, TankInput};
use voxel::glam;

/// Converts a `voxel`/`sim` `Vec3` into Bevy's `Vec3`.
#[inline]
pub fn to_bevy_vec3(v: glam::Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// Converts Bevy's `Vec3` into a `voxel`/`sim` `Vec3`.
#[inline]
pub fn to_sim_vec3(v: Vec3) -> glam::Vec3 {
    glam::Vec3::new(v.x, v.y, v.z)
}

/// Converts a `voxel`/`sim` `Quat` into Bevy's `Quat`.
#[inline]
pub fn to_bevy_quat(q: glam::Quat) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

/// Main headless simulation state wrapped as a Bevy resource.
#[derive(Resource)]
pub struct SimResource(pub SimWorld);

/// Current player tank control snapshot consumed by `FixedUpdate`.
#[derive(Resource, Default)]
pub struct PlayerInputResource {
    pub throttle: f32,
    pub steer: f32,
    pub aim_world_point: glam::Vec3,
    /// Latched fire request so brief clicks between fixed ticks are never dropped.
    pub fire_queued: bool,
}

impl PlayerInputResource {
    pub fn take_for_step(&mut self) -> TankInput {
        let fire = self.fire_queued;
        self.fire_queued = false;
        TankInput {
            throttle: self.throttle,
            steer: self.steer,
            aim_world_point: self.aim_world_point,
            fire,
        }
    }
}

/// Battle lifecycle control signals bridged from UI/keyboard into `FixedUpdate` / state machine.
#[derive(Resource, Default)]
pub struct BattleControlResource {
    pub restart_requested: bool,
    pub return_to_hangar: bool,
}

/// Buffered frame events bridged from `FixedUpdate` to render-frame VFX/HUD systems.
/// Read by all `Update` consumers and cleared in `PostUpdate`.
#[derive(Resource, Default)]
pub struct PendingSimEvents {
    pub fired_shots: Vec<FiredShot>,
    pub impacts: Vec<ImpactEvent>,
    pub hit_reports: Vec<HitReport>,
    pub kills: Vec<KillEvent>,
    pub battle_restarted: bool,
}

impl PendingSimEvents {
    pub fn clear_frame_events(&mut self) {
        self.fired_shots.clear();
        self.impacts.clear();
        self.hit_reports.clear();
        self.kills.clear();
        self.battle_restarted = false;
    }
}
