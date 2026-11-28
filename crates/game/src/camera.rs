use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use sim::Team;
use voxel::raycast_world;

use crate::bridge::{
    PendingSimEvents, PlayerInputResource, SimResource, to_bevy_quat, to_bevy_vec3, to_sim_vec3,
};
use crate::state::{AppState, BattleEntity};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CameraMode {
    ThirdPerson,
    Sniper,
}

/// Interactive camera state supporting War Thunder / WoT 3rd-person orbit + gunner sniper optic,
/// smooth zoom interpolation, cannon recoil kick, impact shake, and post-death allied spectating.
#[derive(Resource)]
pub struct TankCameraController {
    pub mode: CameraMode,
    /// World yaw in radians (`0` looks along `-Z`).
    pub yaw: f32,
    /// Pitch in radians (`+` looks up, `-` looks down).
    pub pitch: f32,
    /// Target third-person orbit radius in meters.
    pub distance: f32,
    /// Smoothed current orbit radius for silky zoom.
    pub current_distance: f32,
    /// Target sniper mode vertical FOV in degrees.
    pub sniper_fov_deg: f32,
    /// Smoothed active vertical FOV in degrees.
    pub current_fov_deg: f32,
    /// True while `C` is held to inspect surroundings without turning the turret.
    pub free_look: bool,
    pub cursor_locked: bool,
    /// True on the exact frame cursor transitioned from unlocked -> locked (prevents accidental shot).
    pub just_locked_this_frame: bool,
    pub sensitivity: f32,
    /// Active tank index followed by the camera (`0` = player, or living ally while spectating).
    pub spectate_tank_idx: usize,
    /// Decaying upward cannon-recoil kick in radians.
    recoil_kick: f32,
    /// Decaying impact screen-shake intensity (`0.0..=1.0`).
    shake_trauma: f32,
    shake_phase: f32,
}

impl Default for TankCameraController {
    fn default() -> Self {
        Self {
            mode: CameraMode::ThirdPerson,
            yaw: 0.0,
            pitch: -0.18,
            distance: 11.5,
            current_distance: 11.5,
            sniper_fov_deg: 22.0,
            current_fov_deg: 60.0,
            free_look: false,
            cursor_locked: false,
            just_locked_this_frame: false,
            sensitivity: 0.0028,
            spectate_tank_idx: 0,
            recoil_kick: 0.0,
            shake_trauma: 0.0,
            shake_phase: 0.0,
        }
    }
}

#[derive(Component)]
pub struct MainTankCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap { size: 1024 })
            .init_resource::<TankCameraController>()
            .add_systems(OnEnter(AppState::InBattle), setup_scene_lighting_and_camera)
            .add_systems(OnExit(AppState::InBattle), unlock_cursor_on_battle_exit)
            .add_systems(
                Update,
                (handle_camera_controls, update_camera_and_aim)
                    .chain()
                    .run_if(in_state(AppState::InBattle)),
            );
    }
}

fn setup_scene_lighting_and_camera(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut cam_ctrl: ResMut<TankCameraController>,
) {
    // Align initial camera orbit direction with the player tank's spawn heading
    cam_ctrl.yaw = sim.0.player_tank().hull_yaw;
    cam_ctrl.pitch = -0.18;
    cam_ctrl.mode = CameraMode::ThirdPerson;
    cam_ctrl.distance = 11.5;
    cam_ctrl.current_distance = 11.5;
    cam_ctrl.current_fov_deg = 60.0;
    cam_ctrl.free_look = false;
    cam_ctrl.cursor_locked = false;
    cam_ctrl.just_locked_this_frame = false;
    cam_ctrl.spectate_tank_idx = 0;
    cam_ctrl.recoil_kick = 0.0;
    cam_ctrl.shake_trauma = 0.0;

    let atmo = sim.0.map.atmosphere;

    // Biome-tailored sky & ambient light matching the selected map's horizon fog
    commands.insert_resource(ClearColor(Color::srgb(
        atmo.sky_rgb[0],
        atmo.sky_rgb[1],
        atmo.sky_rgb[2],
    )));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(atmo.ambient_rgb[0], atmo.ambient_rgb[1], atmo.ambient_rgb[2]),
        brightness: atmo.ambient_brightness,
        affects_lightmapped_meshes: true,
    });

    // Primary sun directional light with memory-efficient 2-cascade 1024px shadow maps
    // (~8 MB vs Bevy's default 64 MB 4-cascade 2048px shadows)
    commands.spawn((
        BattleEntity,
        DirectionalLight {
            illuminance: atmo.sun_illuminance,
            shadow_maps_enabled: true,
            color: Color::srgb(atmo.sun_rgb[0], atmo.sun_rgb[1], atmo.sun_rgb[2]),
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 2,
            first_cascade_far_bound: 34.0,
            maximum_distance: 165.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(atmo.sun_dir[0], atmo.sun_dir[1], atmo.sun_dir[2])
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let spawn_pos = to_bevy_vec3(sim.0.player_tank().position);

    // Main 3D tank camera (`Msaa::Off` eliminates heavy 4x multisampled render targets)
    commands.spawn((
        BattleEntity,
        MainTankCamera,
        Camera3d::default(),
        Msaa::Off,
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0f32.to_radians(),
            near: 0.15,
            far: 450.0,
            ..default()
        }),
        DistanceFog {
            color: Color::srgba(atmo.fog_rgb[0], atmo.fog_rgb[1], atmo.fog_rgb[2], 1.0),
            directional_light_color: Color::srgba(
                atmo.sun_rgb[0],
                atmo.sun_rgb[1],
                atmo.sun_rgb[2],
                0.35,
            ),
            directional_light_exponent: 24.0,
            falloff: FogFalloff::Linear {
                start: atmo.fog_start,
                end: atmo.fog_end,
            },
        },
        Transform::from_translation(spawn_pos + Vec3::new(0.0, 5.5, 11.5))
            .looking_at(spawn_pos + Vec3::new(0.0, 1.8, 0.0), Vec3::Y),
    ));
}

fn unlock_cursor_on_battle_exit(
    mut cam_ctrl: ResMut<TankCameraController>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    cam_ctrl.cursor_locked = false;
    cam_ctrl.just_locked_this_frame = false;
    cam_ctrl.free_look = false;
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    sim: Res<SimResource>,
    pending: Res<PendingSimEvents>,
    mut cam_ctrl: ResMut<TankCameraController>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    cam_ctrl.just_locked_this_frame = false;

    if pending.battle_restarted {
        cam_ctrl.yaw = sim.0.player_tank().hull_yaw;
        cam_ctrl.pitch = -0.18;
        cam_ctrl.mode = CameraMode::ThirdPerson;
        cam_ctrl.spectate_tank_idx = 0;
        cam_ctrl.recoil_kick = 0.0;
        cam_ctrl.shake_trauma = 0.0;
    }

    let dt = time.delta_secs().min(0.05);
    let battle_over = sim.0.battle.is_over();
    let player_id = sim.0.player_unit().id;
    let player_alive = sim.0.player_unit().alive;

    // Trigger tactile caliber-scaled recoil kick when player fires
    for shot in &pending.fired_shots {
        if shot.shooter_id == player_id {
            let cal_scale = (shot.shell.caliber_mm / 75.0).clamp(0.45, 2.0);
            cam_ctrl.recoil_kick = (cam_ctrl.recoil_kick + 0.016 * cal_scale).min(0.045);
            cam_ctrl.shake_trauma = (cam_ctrl.shake_trauma + 0.24 * cal_scale).min(0.85);
        }
    }
    // Trigger impact camera shake when player's tank takes a hit or nearby heavy HE blast
    for rep in &pending.hit_reports {
        if rep.target_id == player_id && !rep.inert_hit {
            let bump = if rep.kill_reason.is_some() {
                1.0
            } else if rep.outcome == sim::HitOutcome::Penetration {
                0.78
            } else {
                0.42
            };
            cam_ctrl.shake_trauma = (cam_ctrl.shake_trauma + bump).min(1.0);
        }
    }
    cam_ctrl.recoil_kick *= (-10.5 * dt).exp();
    if cam_ctrl.recoil_kick < 1e-4 {
        cam_ctrl.recoil_kick = 0.0;
    }
    cam_ctrl.shake_trauma = (cam_ctrl.shake_trauma - dt * 1.85).max(0.0);
    cam_ctrl.shake_phase += dt * 42.0;

    if player_alive {
        cam_ctrl.spectate_tank_idx = 0;
    } else {
        if cam_ctrl.mode == CameraMode::Sniper {
            cam_ctrl.mode = CameraMode::ThirdPerson;
        }
        // Allow cycling spectator camera across living allies + player wreck using Q / E or Left / Right
        if !battle_over {
            let step = if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::ArrowRight) {
                1i32
            } else if keys.just_pressed(KeyCode::KeyQ) || keys.just_pressed(KeyCode::ArrowLeft) {
                -1i32
            } else {
                0
            };
            if step != 0 {
                let candidates: Vec<usize> = sim
                    .0
                    .tanks
                    .iter()
                    .enumerate()
                    .filter(|(idx, t)| *idx == 0 || (t.alive && t.team == Team::Allies))
                    .map(|(idx, _)| idx)
                    .collect();
                if !candidates.is_empty() {
                    let cur_pos = candidates
                        .iter()
                        .position(|&i| i == cam_ctrl.spectate_tank_idx)
                        .unwrap_or(0);
                    let next_pos = (cur_pos as i32 + step).rem_euclid(candidates.len() as i32) as usize;
                    cam_ctrl.spectate_tank_idx = candidates[next_pos];
                }
            }
            // If currently spectated ally dies, fall back to player wreck cleanly
            if cam_ctrl.spectate_tank_idx != 0
                && !sim
                    .0
                    .tanks
                    .get(cam_ctrl.spectate_tank_idx)
                    .is_some_and(|t| t.alive)
            {
                cam_ctrl.spectate_tank_idx = 0;
            }
        }
    }

    // Unlock cursor cleanly at battle end so "Play Again" / "To Hangar" buttons can be clicked directly.
    // While unlocked, clicking top-left `< HANGAR` or bottom ammo/action bar does NOT re-lock cursor.
    let over_hud_button_zone = window_q.single().ok().is_some_and(|w| {
        w.cursor_position().is_some_and(|p| {
            (p.x <= 215.0 && p.y <= 118.0) || p.y >= (w.height() - 108.0).max(0.0)
        })
    });

    if let Ok(mut cursor) = cursor_q.single_mut() {
        if battle_over {
            if cam_ctrl.cursor_locked {
                cursor.grab_mode = CursorGrabMode::None;
                cursor.visible = true;
                cam_ctrl.cursor_locked = false;
            }
        } else if !cam_ctrl.cursor_locked
            && ((mouse_buttons.just_pressed(MouseButton::Left) && !over_hud_button_zone)
                || mouse_buttons.just_pressed(MouseButton::Right))
        {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            cam_ctrl.cursor_locked = true;
            cam_ctrl.just_locked_this_frame = true;
        } else if cam_ctrl.cursor_locked && keys.just_pressed(KeyCode::Escape) {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
            cam_ctrl.cursor_locked = false;
        }
    }

    // Shift toggles 3rd-person <-> Sniper gunner optic while player is alive
    if player_alive && (keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight)) {
        cam_ctrl.mode = match cam_ctrl.mode {
            CameraMode::ThirdPerson => CameraMode::Sniper,
            CameraMode::Sniper => CameraMode::ThirdPerson,
        };
    }

    // Hold C for free-look in 3rd-person
    cam_ctrl.free_look =
        cam_ctrl.mode == CameraMode::ThirdPerson && keys.pressed(KeyCode::KeyC);

    // Mouse look delta (works when cursor is locked OR while holding RMB)
    if (cam_ctrl.cursor_locked || mouse_buttons.pressed(MouseButton::Right))
        && mouse_motion.delta != Vec2::ZERO
    {
        let sens = if cam_ctrl.mode == CameraMode::Sniper {
            cam_ctrl.sensitivity * (cam_ctrl.sniper_fov_deg / 60.0)
        } else {
            cam_ctrl.sensitivity
        };
        cam_ctrl.yaw -= mouse_motion.delta.x * sens;
        cam_ctrl.pitch -= mouse_motion.delta.y * sens;

        let (min_p, max_p) = match cam_ctrl.mode {
            CameraMode::ThirdPerson => (-FRAC_PI_2 * 0.78, 0.42),
            CameraMode::Sniper => ((-10.0f32).to_radians(), (22.0f32).to_radians()),
        };
        cam_ctrl.pitch = cam_ctrl.pitch.clamp(min_p, max_p);
    }

    // Mouse scroll zoom
    let scroll_y = mouse_scroll.delta.y;
    if scroll_y.abs() > 0.01 {
        match cam_ctrl.mode {
            CameraMode::ThirdPerson => {
                cam_ctrl.distance = (cam_ctrl.distance - scroll_y * 1.2).clamp(5.5, 24.0);
            }
            CameraMode::Sniper => {
                cam_ctrl.sniper_fov_deg =
                    (cam_ctrl.sniper_fov_deg - scroll_y * 2.5).clamp(10.0, 38.0);
            }
        }
    }

    // Smooth camera distance & FOV interpolation
    let smooth = 1.0 - (-14.0 * dt).exp();
    cam_ctrl.current_distance += (cam_ctrl.distance - cam_ctrl.current_distance) * smooth;
    let target_fov = match cam_ctrl.mode {
        CameraMode::ThirdPerson => 60.0,
        CameraMode::Sniper => cam_ctrl.sniper_fov_deg,
    };
    cam_ctrl.current_fov_deg += (target_fov - cam_ctrl.current_fov_deg) * smooth;
}

fn update_camera_and_aim(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    mut player_input: ResMut<PlayerInputResource>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<MainTankCamera>>,
) {
    let Ok((mut cam_tf, mut projection)) = camera_q.single_mut() else {
        return;
    };

    let player_id = sim.0.player_unit().id;
    let follow_unit = sim
        .0
        .tanks
        .get(cam_ctrl.spectate_tank_idx)
        .unwrap_or_else(|| sim.0.player_unit());
    let tank = &follow_unit.state;

    // Apply subtle cannon recoil pitch kick + damped impact shake without corrupting user's aim angle
    let shake_sq = cam_ctrl.shake_trauma * cam_ctrl.shake_trauma;
    let shake_yaw = (cam_ctrl.shake_phase * 1.3).sin() * 0.018 * shake_sq;
    let shake_pitch = (cam_ctrl.shake_phase * 1.7 + 0.8).cos() * 0.016 * shake_sq;
    let shake_roll = (cam_ctrl.shake_phase * 1.1 + 1.5).sin() * 0.022 * shake_sq;

    let base_look_rot = Quat::from_rotation_y(cam_ctrl.yaw) * Quat::from_rotation_x(cam_ctrl.pitch);
    let base_look_dir = (base_look_rot * Vec3::NEG_Z).normalize();
    let vis_look_rot = Quat::from_rotation_y(cam_ctrl.yaw + shake_yaw)
        * Quat::from_rotation_x(cam_ctrl.pitch + cam_ctrl.recoil_kick + shake_pitch)
        * Quat::from_rotation_z(shake_roll);

    if let Projection::Perspective(ref mut persp) = *projection {
        persp.fov = cam_ctrl.current_fov_deg.to_radians();
    }

    match cam_ctrl.mode {
        CameraMode::ThirdPerson => {
            let pivot = to_bevy_vec3(tank.turret_world_position()) + Vec3::new(0.0, 1.35, 0.0);

            // Prevent camera from clipping inside houses, boulders, or hillsides via voxel raycast
            let mut safe_dist = cam_ctrl.current_distance;
            if let Some(hit) = raycast_world(
                &sim.0.voxels,
                to_sim_vec3(pivot),
                to_sim_vec3(-base_look_dir),
                cam_ctrl.current_distance + 0.6,
            ) {
                safe_dist = (hit.distance - 0.55).clamp(1.9, cam_ctrl.current_distance);
            }

            let mut cam_pos = pivot - base_look_dir * safe_dist;
            if let Some(ground_y) = sim
                .0
                .voxels
                .ground_height_near(cam_pos.x, cam_pos.z, pivot.y + 3.0, 5.0, 18.0)
            {
                cam_pos.y = cam_pos.y.max(ground_y + 0.65);
            }

            // Aim convergence uses unshaken ray so screen shake feels visceral without throwing off turret target
            let unshaken_tf =
                Transform::from_translation(cam_pos).looking_at(pivot + base_look_dir * 40.0, Vec3::Y);
            let aim_dir = unshaken_tf.forward().as_vec3();

            let shake_offset = Vec3::new(shake_yaw * 4.5, (cam_ctrl.recoil_kick + shake_pitch) * 3.5, 0.0);
            cam_tf.translation = cam_pos + shake_offset;
            cam_tf.rotation = unshaken_tf.rotation
                * Quat::from_rotation_x(cam_ctrl.recoil_kick * 0.65 + shake_pitch)
                * Quat::from_rotation_y(shake_yaw)
                * Quat::from_rotation_z(shake_roll);

            // Converge crosshair aim onto both voxels AND enemy tanks (zero skyline parallax)
            if !cam_ctrl.free_look && sim.0.player_unit().alive {
                let ray_origin = to_sim_vec3(cam_pos);
                let ray_dir = to_sim_vec3(aim_dir);
                let max_aim_range = 450.0;
                let aim_point = sim
                    .0
                    .raycast_scene(ray_origin, ray_dir, max_aim_range, Some(player_id))
                    .unwrap_or(ray_origin + ray_dir * max_aim_range);
                player_input.aim_world_point = aim_point;
            }
        }
        CameraMode::Sniper => {
            let gun_pos = to_bevy_vec3(tank.gun_world_position());
            let gun_rot = to_bevy_quat(tank.gun_world_rotation());
            let optic_pos = gun_pos + gun_rot * Vec3::new(0.42, 0.25, -0.40);

            cam_tf.translation = optic_pos;
            cam_tf.rotation = vis_look_rot;

            let ray_origin = to_sim_vec3(optic_pos);
            let ray_dir = to_sim_vec3(base_look_dir);
            let max_aim_range = 500.0;
            let aim_point = sim
                .0
                .raycast_scene(ray_origin, ray_dir, max_aim_range, Some(player_id))
                .unwrap_or(ray_origin + ray_dir * max_aim_range);
            player_input.aim_world_point = aim_point;
        }
    }
}
