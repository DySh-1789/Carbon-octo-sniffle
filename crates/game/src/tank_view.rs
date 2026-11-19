use bevy::prelude::*;
use sim::{TankId, Team};
use voxel::{TankVoxelModels, mesh_voxel_model};

use crate::bridge::{SimResource, to_bevy_quat, to_bevy_vec3};
use crate::voxel_render::voxel_mesh_to_bevy;

#[derive(Resource)]
pub struct TankModelsResource(pub TankVoxelModels);

#[derive(Resource)]
struct TankMaterials {
    allies: Handle<StandardMaterial>,
    axis: Handle<StandardMaterial>,
    wreck: Handle<StandardMaterial>,
    alive_cache: Vec<bool>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TankPartKind {
    HullRoot,
    TurretPivot,
    GunPivot,
    BarrelMesh,
}

#[derive(Component, Copy, Clone)]
struct TankVisualPart {
    id: TankId,
    kind: TankPartKind,
}

#[derive(Component, Copy, Clone)]
struct TankMaterialSlot {
    id: TankId,
    team: Team,
}

pub struct TankViewPlugin;

impl Plugin for TankViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_battle_tanks).add_systems(
            PostUpdate,
            (sync_all_tank_transforms, sync_tank_wreck_materials),
        );
    }
}

fn spawn_battle_tanks(
    mut commands: Commands,
    sim: Res<SimResource>,
    models: Res<TankModelsResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Share ONLY 3 GPU meshes and 3 PBR materials across all 12 battle tanks
    let allies_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.62,
        metallic: 0.28,
        reflectance: 0.35,
        ..default()
    });
    // Multiply olive steel vertices into warm Wehrmacht Dunkelgelb / desert tan
    let axis_mat = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(1.65, 1.15, 0.78),
        perceptual_roughness: 0.64,
        metallic: 0.28,
        reflectance: 0.35,
        ..default()
    });
    let wreck_mat = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.20, 0.19, 0.19),
        perceptual_roughness: 0.94,
        metallic: 0.12,
        reflectance: 0.15,
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

    let mut alive_cache = Vec::with_capacity(sim.0.tanks.len());
    for unit in &sim.0.tanks {
        alive_cache.push(unit.alive);
        let mat = match (unit.alive, unit.team) {
            (false, _) => wreck_mat.clone(),
            (true, Team::Allies) => allies_mat.clone(),
            (true, Team::Axis) => axis_mat.clone(),
        };
        let slot = TankMaterialSlot {
            id: unit.id,
            team: unit.team,
        };
        let tank = &unit.state;

        commands
            .spawn((
                TankVisualPart {
                    id: unit.id,
                    kind: TankPartKind::HullRoot,
                },
                Transform::from_translation(to_bevy_vec3(tank.position))
                    .with_rotation(to_bevy_quat(tank.hull_rotation())),
                Visibility::default(),
            ))
            .with_children(|hull| {
                hull.spawn((
                    Mesh3d(hull_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    slot,
                    Transform::IDENTITY,
                ));

                hull.spawn((
                    TankVisualPart {
                        id: unit.id,
                        kind: TankPartKind::TurretPivot,
                    },
                    Transform::from_translation(to_bevy_vec3(models.0.turret_mount_offset))
                        .with_rotation(Quat::from_rotation_y(tank.turret_yaw_local)),
                    Visibility::default(),
                ))
                .with_children(|turret| {
                    turret.spawn((
                        Mesh3d(turret_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        slot,
                        Transform::IDENTITY,
                    ));

                    turret
                        .spawn((
                            TankVisualPart {
                                id: unit.id,
                                kind: TankPartKind::GunPivot,
                            },
                            Transform::from_translation(to_bevy_vec3(models.0.gun_mount_offset))
                                .with_rotation(Quat::from_rotation_x(tank.gun_pitch_local)),
                            Visibility::default(),
                        ))
                        .with_children(|gun| {
                            gun.spawn((
                                TankVisualPart {
                                    id: unit.id,
                                    kind: TankPartKind::BarrelMesh,
                                },
                                Mesh3d(barrel_mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                slot,
                                Transform::from_xyz(0.0, 0.0, tank.recoil_offset),
                            ));
                        });
                });
            });
    }

    commands.insert_resource(TankMaterials {
        allies: allies_mat,
        axis: axis_mat,
        wreck: wreck_mat,
        alive_cache,
    });
}

fn sync_all_tank_transforms(
    sim: Res<SimResource>,
    mut parts: Query<(&TankVisualPart, &mut Transform)>,
) {
    let tanks = &sim.0.tanks;
    for (part, mut tf) in &mut parts {
        let Some(unit) = tanks.get(part.id.index()) else {
            continue;
        };
        let tank = &unit.state;
        match part.kind {
            TankPartKind::HullRoot => {
                tf.translation = to_bevy_vec3(tank.position);
                tf.rotation = to_bevy_quat(tank.hull_rotation());
            }
            TankPartKind::TurretPivot => {
                tf.rotation = Quat::from_rotation_y(tank.turret_yaw_local);
            }
            TankPartKind::GunPivot => {
                tf.rotation = Quat::from_rotation_x(tank.gun_pitch_local);
            }
            TankPartKind::BarrelMesh => {
                tf.translation = Vec3::new(0.0, 0.0, tank.recoil_offset);
            }
        }
    }
}

fn sync_tank_wreck_materials(
    sim: Res<SimResource>,
    mut mats: ResMut<TankMaterials>,
    mut slots: Query<(&TankMaterialSlot, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let mut changed = false;
    for (i, unit) in sim.0.tanks.iter().enumerate() {
        if mats.alive_cache.get(i).copied() != Some(unit.alive) {
            changed = true;
            if let Some(entry) = mats.alive_cache.get_mut(i) {
                *entry = unit.alive;
            }
        }
    }
    if !changed {
        return;
    }

    for (slot, mut mat) in &mut slots {
        let alive = sim
            .0
            .tanks
            .get(slot.id.index())
            .is_some_and(|u| u.alive);
        mat.0 = if !alive {
            mats.wreck.clone()
        } else if slot.team == Team::Allies {
            mats.allies.clone()
        } else {
            mats.axis.clone()
        };
    }
}
