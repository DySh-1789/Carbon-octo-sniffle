use bevy::prelude::*;
use sim::{GRAVITY, TankId, Team};
use voxel::{TankVoxelModels, mesh_voxel_model};

use crate::bridge::{PendingSimEvents, SimResource, to_bevy_quat, to_bevy_vec3};
use crate::voxel_render::voxel_mesh_to_bevy;

#[derive(Resource)]
pub struct TankModelsResource(pub TankVoxelModels);

#[derive(Resource)]
struct TankRenderAssets {
    allies_mat: Handle<StandardMaterial>,
    axis_mat: Handle<StandardMaterial>,
    wreck_mat: Handle<StandardMaterial>,
    turret_mesh: Handle<Mesh>,
    barrel_mesh: Handle<Mesh>,
    gun_mount_offset: Vec3,
    alive_cache: Vec<bool>,
    turret_popped_cache: Vec<bool>,
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

/// Ballistic blown-off turret launched into the air on ammo-rack detonation.
#[derive(Component)]
struct DetachedTurret {
    velocity: Vec3,
    angular_vel: Vec3,
    settled: bool,
}

pub struct TankViewPlugin;

impl Plugin for TankViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_battle_tanks)
            .add_systems(
                Update,
                (
                    clear_detached_turrets_on_restart,
                    sync_turret_blow_offs,
                    step_detached_turrets,
                )
                    .chain(),
            )
            .add_systems(
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
    // Share ONLY 3 GPU meshes and 3 PBR materials across all 12 battle tanks + blown-off turrets
    let allies_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.62,
        metallic: 0.28,
        reflectance: 0.35,
        ..default()
    });
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
    let mut turret_popped_cache = Vec::with_capacity(sim.0.tanks.len());
    for unit in &sim.0.tanks {
        alive_cache.push(unit.alive);
        turret_popped_cache.push(unit.damage.turret_blown_off);
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

    commands.insert_resource(TankRenderAssets {
        allies_mat,
        axis_mat,
        wreck_mat,
        turret_mesh,
        barrel_mesh,
        gun_mount_offset: to_bevy_vec3(models.0.gun_mount_offset),
        alive_cache,
        turret_popped_cache,
    });
}

fn clear_detached_turrets_on_restart(
    mut commands: Commands,
    pending: Res<PendingSimEvents>,
    mut assets: ResMut<TankRenderAssets>,
    detached_q: Query<Entity, With<DetachedTurret>>,
    mut parts_q: Query<(&TankVisualPart, &mut Visibility)>,
) {
    if !pending.battle_restarted {
        return;
    }
    for entity in &detached_q {
        commands.entity(entity).despawn();
    }
    assets.turret_popped_cache.fill(false);
    for (part, mut vis) in &mut parts_q {
        if part.kind == TankPartKind::TurretPivot {
            *vis = Visibility::Inherited;
        }
    }
}

fn sync_turret_blow_offs(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut assets: ResMut<TankRenderAssets>,
    mut parts_q: Query<(&TankVisualPart, &mut Visibility)>,
) {
    for (i, unit) in sim.0.tanks.iter().enumerate() {
        let popped = unit.damage.turret_blown_off;
        let was_popped = assets.turret_popped_cache.get(i).copied().unwrap_or(false);
        if popped == was_popped {
            continue;
        }
        if let Some(entry) = assets.turret_popped_cache.get_mut(i) {
            *entry = popped;
        }

        for (part, mut vis) in &mut parts_q {
            if part.id == unit.id && part.kind == TankPartKind::TurretPivot {
                *vis = if popped {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
            }
        }

        if popped {
            let turret_pos = to_bevy_vec3(unit.state.turret_world_position()) + Vec3::Y * 0.25;
            let turret_rot = to_bevy_quat(unit.state.turret_world_rotation());
            let seed = (unit.id.0 as f32) * 1.73 + (sim.0.tick as f32) * 0.13;
            let vel = Vec3::new(seed.cos() * 3.4, 9.8 + (seed.sin().abs() * 2.8), seed.sin() * 3.4);
            let ang = Vec3::new(2.8, 1.9 * seed.cos().signum(), -2.4);

            commands
                .spawn((
                    DetachedTurret {
                        velocity: vel,
                        angular_vel: ang,
                        settled: false,
                    },
                    Transform::from_translation(turret_pos).with_rotation(turret_rot),
                    Visibility::default(),
                ))
                .with_children(|t| {
                    t.spawn((
                        Mesh3d(assets.turret_mesh.clone()),
                        MeshMaterial3d(assets.wreck_mat.clone()),
                        Transform::IDENTITY,
                    ));
                    t.spawn((
                        Mesh3d(assets.barrel_mesh.clone()),
                        MeshMaterial3d(assets.wreck_mat.clone()),
                        Transform::from_translation(assets.gun_mount_offset)
                            .with_rotation(Quat::from_rotation_x(unit.state.gun_pitch_local)),
                    ));
                });
        }
    }
}

fn step_detached_turrets(
    time: Res<Time>,
    sim: Res<SimResource>,
    mut q: Query<(&mut Transform, &mut DetachedTurret)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut tf, mut dt_turret) in &mut q {
        if dt_turret.settled {
            continue;
        }
        dt_turret.velocity.y -= GRAVITY * dt;
        tf.translation += dt_turret.velocity * dt;

        let spin = Quat::from_euler(
            EulerRot::XYZ,
            dt_turret.angular_vel.x * dt,
            dt_turret.angular_vel.y * dt,
            dt_turret.angular_vel.z * dt,
        );
        tf.rotation = (spin * tf.rotation).normalize();

        let floor_y = sim
            .0
            .voxels
            .surface_height_at(tf.translation.x, tf.translation.z)
            .unwrap_or(0.0)
            + 0.45;
        if tf.translation.y <= floor_y {
            tf.translation.y = floor_y;
            if dt_turret.velocity.y.abs() < 1.8 {
                dt_turret.velocity = Vec3::ZERO;
                dt_turret.angular_vel = Vec3::ZERO;
                dt_turret.settled = true;
            } else {
                dt_turret.velocity.y = -dt_turret.velocity.y * 0.28;
                dt_turret.velocity.x *= 0.55;
                dt_turret.velocity.z *= 0.55;
                dt_turret.angular_vel *= 0.45;
            }
        }
    }
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
    mut assets: ResMut<TankRenderAssets>,
    mut slots: Query<(&TankMaterialSlot, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let mut changed = false;
    for (i, unit) in sim.0.tanks.iter().enumerate() {
        if assets.alive_cache.get(i).copied() != Some(unit.alive) {
            changed = true;
            if let Some(entry) = assets.alive_cache.get_mut(i) {
                *entry = unit.alive;
            }
        }
    }
    if !changed {
        return;
    }

    for (slot, mut mat) in &mut slots {
        let alive = sim.0.tanks.get(slot.id.index()).is_some_and(|u| u.alive);
        mat.0 = if !alive {
            assets.wreck_mat.clone()
        } else if slot.team == Team::Allies {
            assets.allies_mat.clone()
        } else {
            assets.axis_mat.clone()
        };
    }
}
