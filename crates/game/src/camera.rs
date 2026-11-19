use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use voxel::raycast_world;

use crate::bridge::{PlayerInputResource, SimResource, to_bevy_quat, to_bevy_vec3, to_sim_vec3};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CameraMode {
    ThirdPerson,
    Sniper,
}

/// Interactive camera state supporting War Thunder / WoT 3rd-person orbit + gunner sniper optic.
#[derive(Resource)]
pub struct TankCameraController {
    pub mode: CameraMode,
    /// World yaw in radians (`0` looks along `-Z`).
    pub yaw: f32,
    /// Pitch in radians (`+` looks up, `-` looks down).
    pub pitch: f32,
    /// Third-person orbit radius in meters.
    pub distance: f32,
    /// Sniper mode vertical FOV in degrees.
    pub sniper_fov_deg: f32,
    /// True while `C` is held to inspect surroundings without turning the turret.
    pub free_look: bool,
    pub cursor_locked: bool,
    pub sensitivity: f32,
}

impl Default for TankCameraController {
    fn default() -> Self {
        Self {
            mode: CameraMode::ThirdPerson,
            yaw: 0.0,
            pitch: -0.18,
            distance: 11.5,
            sniper_fov_deg: 22.0,
            free_look: false,
            cursor_locked: false,
            sensitivity: 0.0028,
        }
    }
}

#[derive(Component)]
pub struct MainTankCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TankCameraController>()
            .add_systems(Startup, setup_scene_lighting_and_camera)
            .add_systems(Update, (handle_camera_controls, update_camera_and_aim).chain());
    }
}

fn setup_scene_lighting_and_camera(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut cam_ctrl: ResMut<TankCameraController>,
) {
    // Align initial camera orbit direction with the player tank's spawn heading
    cam_ctrl.yaw = sim.0.player_tank.hull_yaw;

    // Warm sunlit European valley sky matching horizon fog
    commands.insert_resource(ClearColor(Color::srgb(0.62, 0.76, 0.88)));

    // Soft sky/ground ambient bounce light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.84, 0.89, 0.96),
        brightness: 450.0,
        affects_lightmapped_meshes: true,
    });

    // Primary afternoon sun directional light with crisp shadows across hills & village
    commands.spawn((
        DirectionalLight {
            illuminance: 15_500.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.96, 0.88),
            ..default()
        },
        Transform::from_xyz(110.0, 155.0, 85.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let spawn_pos = to_bevy_vec3(sim.0.player_tank.position);

    // Main 3D tank camera with atmospheric distance fog for valley depth perception
    commands.spawn((
        MainTankCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0f32.to_radians(),
            near: 0.15,
            far: 900.0,
            ..default()
        }),
        DistanceFog {
            color: Color::srgba(0.66, 0.78, 0.88, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.84, 0.35),
            directional_light_exponent: 24.0,
            falloff: FogFalloff::Linear {
                start: 125.0,
                end: 370.0,
            },
        },
        Transform::from_translation(spawn_pos + Vec3::new(0.0, 5.5, 11.5))
            .looking_at(spawn_pos + Vec3::new(0.0, 1.8, 0.0), Vec3::Y),
    ));
}

fn handle_camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut cam_ctrl: ResMut<TankCameraController>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // Lock cursor on left/right mouse click; release on Escape
    if let Ok(mut cursor) = cursor_q.single_mut() {
        if !cam_ctrl.cursor_locked
            && (mouse_buttons.just_pressed(MouseButton::Left)
                || mouse_buttons.just_pressed(MouseButton::Right))
        {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            cam_ctrl.cursor_locked = true;
        } else if cam_ctrl.cursor_locked && keys.just_pressed(KeyCode::Escape) {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
            cam_ctrl.cursor_locked = false;
        }
    }

    // Shift toggles 3rd-person <-> Sniper gunner optic
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
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

    let tank = &sim.0.player_tank;
    let look_rot = Quat::from_rotation_y(cam_ctrl.yaw) * Quat::from_rotation_x(cam_ctrl.pitch);
    let look_dir = (look_rot * Vec3::NEG_Z).normalize();

    match cam_ctrl.mode {
        CameraMode::ThirdPerson => {
            let pivot = to_bevy_vec3(tank.turret_world_position()) + Vec3::new(0.0, 1.35, 0.0);

            // Prevent camera from clipping inside houses, boulders, or hillsides via voxel raycast
            let mut safe_dist = cam_ctrl.distance;
            if let Some(hit) = raycast_world(
                &sim.0.voxels,
                to_sim_vec3(pivot),
                to_sim_vec3(-look_dir),
                cam_ctrl.distance + 0.6,
            ) {
                safe_dist = (hit.distance - 0.55).clamp(1.9, cam_ctrl.distance);
            }

            let mut cam_pos = pivot - look_dir * safe_dist;
            // Keep camera above local voxel terrain height
            if let Some(ground_y) = sim
                .0
                .voxels
                .ground_height_near(cam_pos.x, cam_pos.z, pivot.y + 3.0, 5.0, 18.0)
            {
                cam_pos.y = cam_pos.y.max(ground_y + 0.65);
            }

            cam_tf.translation = cam_pos;
            cam_tf.look_at(pivot + look_dir * 40.0, Vec3::Y);

            if let Projection::Perspective(ref mut persp) = *projection {
                persp.fov = 60.0f32.to_radians();
            }

            // Compute desired world aim point from camera crosshair unless free-looking
            if !cam_ctrl.free_look {
                let ray_origin = to_sim_vec3(cam_pos);
                let ray_dir = to_sim_vec3((cam_tf.forward()).as_vec3());
                let max_aim_range = 450.0;
                let aim_point = raycast_world(&sim.0.voxels, ray_origin, ray_dir, max_aim_range)
                    .map(|h| h.point)
                    .unwrap_or(ray_origin + ray_dir * max_aim_range);
                player_input.aim_world_point = aim_point;
            }
        }
        CameraMode::Sniper => {
            // Gunner sight mounted next to the mantlet so the barrel doesn't obscure the optic
            let gun_pos = to_bevy_vec3(tank.gun_world_position());
            let gun_rot = to_bevy_quat(tank.gun_world_rotation());
            let optic_pos = gun_pos + gun_rot * Vec3::new(0.42, 0.25, -0.40);

            cam_tf.translation = optic_pos;
            cam_tf.rotation = look_rot;

            if let Projection::Perspective(ref mut persp) = *projection {
                persp.fov = cam_ctrl.sniper_fov_deg.to_radians();
            }

            let ray_origin = to_sim_vec3(optic_pos);
            let ray_dir = to_sim_vec3(look_dir);
            let max_aim_range = 500.0;
            let aim_point = raycast_world(&sim.0.voxels, ray_origin, ray_dir, max_aim_range)
                .map(|h| h.point)
                .unwrap_or(ray_origin + ray_dir * max_aim_range);
            player_input.aim_world_point = aim_point;
        }
    }
}
