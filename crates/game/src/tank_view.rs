use bevy::prelude::*;
use voxel::{TankVoxelModels, mesh_voxel_model};

use crate::bridge::{SimResource, to_bevy_quat, to_bevy_vec3};
use crate::voxel_render::voxel_mesh_to_bevy;

#[derive(Resource)]
pub struct TankModelsResource(pub TankVoxelModels);

#[derive(Component)]
pub struct PlayerHullRoot;

#[derive(Component)]
pub struct PlayerTurretPivot;

#[derive(Component)]
pub struct PlayerGunPivot;

#[derive(Component)]
pub struct PlayerBarrelMesh;

pub struct TankViewPlugin;

impl Plugin for TankViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player_tank)
            .add_systems(PostUpdate, sync_player_tank_transforms);
    }
}

fn spawn_player_tank(
    mut commands: Commands,
    sim: Res<SimResource>,
    models: Res<TankModelsResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tank_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.62,
        metallic: 0.28,
        reflectance: 0.35,
        ..default()
    });

    let hull_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.0.hull,
        &sim.0.voxels.materials,
    )));
    let turret_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.0.turret,
        &sim.0.voxels.materials,
    )));
    let barrel_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.0.barrel,
        &sim.0.voxels.materials,
    )));

    let tank = &sim.0.player_tank;

    commands
        .spawn((
            PlayerHullRoot,
            Transform::from_translation(to_bevy_vec3(tank.position))
                .with_rotation(to_bevy_quat(tank.hull_rotation())),
            Visibility::default(),
        ))
        .with_children(|hull| {
            hull.spawn((
                Mesh3d(hull_mesh),
                MeshMaterial3d(tank_mat.clone()),
                Transform::IDENTITY,
            ));

            hull.spawn((
                PlayerTurretPivot,
                Transform::from_translation(to_bevy_vec3(models.0.turret_mount_offset)),
                Visibility::default(),
            ))
            .with_children(|turret| {
                turret.spawn((
                    Mesh3d(turret_mesh),
                    MeshMaterial3d(tank_mat.clone()),
                    Transform::IDENTITY,
                ));

                turret
                    .spawn((
                        PlayerGunPivot,
                        Transform::from_translation(to_bevy_vec3(models.0.gun_mount_offset)),
                        Visibility::default(),
                    ))
                    .with_children(|gun| {
                        gun.spawn((
                            PlayerBarrelMesh,
                            Mesh3d(barrel_mesh),
                            MeshMaterial3d(tank_mat.clone()),
                            Transform::IDENTITY,
                        ));
                    });
            });
        });
}

#[allow(clippy::type_complexity)]
fn sync_player_tank_transforms(
    sim: Res<SimResource>,
    mut parts: ParamSet<(
        Query<&mut Transform, With<PlayerHullRoot>>,
        Query<&mut Transform, With<PlayerTurretPivot>>,
        Query<&mut Transform, With<PlayerGunPivot>>,
        Query<&mut Transform, With<PlayerBarrelMesh>>,
    )>,
) {
    let tank = &sim.0.player_tank;

    if let Ok(mut hull_tf) = parts.p0().single_mut() {
        hull_tf.translation = to_bevy_vec3(tank.position);
        hull_tf.rotation = to_bevy_quat(tank.hull_rotation());
    }
    if let Ok(mut turret_tf) = parts.p1().single_mut() {
        turret_tf.rotation = Quat::from_rotation_y(tank.turret_yaw_local);
    }
    if let Ok(mut gun_tf) = parts.p2().single_mut() {
        gun_tf.rotation = Quat::from_rotation_x(tank.gun_pitch_local);
    }
    if let Ok(mut barrel_tf) = parts.p3().single_mut() {
        // Barrel kicks back along +Z on firing and smoothly returns
        barrel_tf.translation = Vec3::new(0.0, 0.0, tank.recoil_offset);
    }
}
