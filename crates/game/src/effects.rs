use std::collections::HashMap;

use bevy::prelude::*;
use sim::{HitOutcome, KillReason, MapId, ShellKind, Team};
use voxel::VoxelId;

use crate::bridge::{PendingSimEvents, SimResource, to_bevy_vec3};
use crate::camera::MainTankCamera;
use crate::state::{AppState, BattleEntity};

/// Maximum camera distance for spawning dynamic transient `PointLight`s (keeps WebGL2/WGPU light clusters lean).
const MAX_DYNAMIC_LIGHT_DIST_SQ: f32 = 95.0 * 95.0;
/// Maximum camera distance for track dust & water spray puffs.
const MAX_TRACK_DUST_DIST_SQ: f32 = 72.0 * 72.0;

#[derive(Resource)]
struct EffectsAssets {
    tracer_mesh: Handle<Mesh>,
    tracer_mat: Handle<StandardMaterial>,
    he_tracer_mat: Handle<StandardMaterial>,
    ap_tracer_mat: Handle<StandardMaterial>,
    puff_mesh: Handle<Mesh>,
    beacon_post_mesh: Handle<Mesh>,
    flash_mat: Handle<StandardMaterial>,
    fire_mat: Handle<StandardMaterial>,
    spark_mat: Handle<StandardMaterial>,
    smoke_mat: Handle<StandardMaterial>,
    wreck_smoke_mat: Handle<StandardMaterial>,
    dirt_mat: Handle<StandardMaterial>,
    sand_dust_mat: Handle<StandardMaterial>,
    snow_dust_mat: Handle<StandardMaterial>,
    water_spray_mat: Handle<StandardMaterial>,
    fpe_foam_mat: Handle<StandardMaterial>,
    brick_debris_mat: Handle<StandardMaterial>,
    rubble_debris_mat: Handle<StandardMaterial>,
    wood_debris_mat: Handle<StandardMaterial>,
    zone_neutral_mat: Handle<StandardMaterial>,
    zone_allies_mat: Handle<StandardMaterial>,
    zone_axis_mat: Handle<StandardMaterial>,
    zone_contested_mat: Handle<StandardMaterial>,
}

#[derive(Component, Copy, Clone)]
struct ZoneBeaconMarker {
    zone_idx: usize,
}

#[derive(Resource, Default)]
struct TracerEntities {
    map: HashMap<u64, Entity>,
}

#[derive(Resource, Default)]
struct FireEmitTimer {
    accum: f32,
}

#[derive(Resource, Default)]
struct DustEmitTimer {
    accum: f32,
}

#[derive(Component)]
struct TransientParticle {
    velocity: Vec3,
    gravity: f32,
    age: f32,
    lifetime: f32,
    start_scale: f32,
    end_scale: f32,
}

#[derive(Component)]
struct TransientLight {
    age: f32,
    lifetime: f32,
    start_intensity: f32,
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TracerEntities>()
            .init_resource::<FireEmitTimer>()
            .init_resource::<DustEmitTimer>()
            .add_systems(Startup, setup_effects_assets)
            .add_systems(OnEnter(AppState::InBattle), spawn_capture_zone_beacons)
            .add_systems(OnExit(AppState::InBattle), clear_effects_on_exit)
            .add_systems(
                Update,
                (
                    clear_effects_on_restart,
                    sync_projectile_tracers,
                    sync_capture_zone_beacons,
                    spawn_shot_and_impact_effects,
                    emit_active_tank_fire_effects,
                    emit_track_dust_and_water_effects,
                    tick_transient_effects,
                )
                    .chain()
                    .run_if(in_state(AppState::InBattle)),
            );
    }
}

fn clear_effects_on_exit(
    mut commands: Commands,
    mut tracers: ResMut<TracerEntities>,
    mut fire_timer: ResMut<FireEmitTimer>,
    mut dust_timer: ResMut<DustEmitTimer>,
    particle_q: Query<Entity, With<TransientParticle>>,
    light_q: Query<Entity, With<TransientLight>>,
) {
    for (_, entity) in tracers.map.drain() {
        commands.entity(entity).despawn();
    }
    for entity in &particle_q {
        commands.entity(entity).despawn();
    }
    for entity in &light_q {
        commands.entity(entity).despawn();
    }
    fire_timer.accum = 0.0;
    dust_timer.accum = 0.0;
}

fn setup_effects_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tracer_mesh = meshes.add(Cuboid::new(0.14, 0.14, 2.2));
    let tracer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.35),
        emissive: LinearRgba::new(14.0, 7.0, 1.2, 1.0),
        unlit: true,
        ..default()
    });
    let he_tracer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.18),
        emissive: LinearRgba::new(16.0, 4.2, 0.6, 1.0),
        unlit: true,
        ..default()
    });
    let ap_tracer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.96, 0.72),
        emissive: LinearRgba::new(14.0, 12.0, 5.0, 1.0),
        unlit: true,
        ..default()
    });
    let puff_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let beacon_post_mesh = meshes.add(Cuboid::new(0.22, 2.2, 0.22));
    let flash_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.18),
        emissive: LinearRgba::new(10.0, 4.5, 0.8, 1.0),
        unlit: true,
        ..default()
    });
    let fire_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.42, 0.08),
        emissive: LinearRgba::new(9.5, 2.8, 0.25, 1.0),
        unlit: true,
        ..default()
    });
    let spark_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.94, 0.65),
        emissive: LinearRgba::new(16.0, 11.0, 3.0, 1.0),
        unlit: true,
        ..default()
    });
    let smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.60, 0.56),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wreck_smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.17, 0.17),
        perceptual_roughness: 1.0,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.29, 0.20),
        perceptual_roughness: 0.95,
        ..default()
    });
    let sand_dust_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.76, 0.65, 0.45),
        perceptual_roughness: 0.98,
        ..default()
    });
    let snow_dust_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.90, 0.95),
        perceptual_roughness: 0.96,
        ..default()
    });
    let water_spray_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.88, 0.96),
        perceptual_roughness: 0.65,
        ..default()
    });
    let fpe_foam_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.95, 0.98),
        perceptual_roughness: 0.85,
        ..default()
    });
    let brick_debris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.66, 0.28, 0.20),
        perceptual_roughness: 0.90,
        ..default()
    });
    let rubble_debris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.56, 0.54, 0.50),
        perceptual_roughness: 0.94,
        ..default()
    });
    let wood_debris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.30, 0.16),
        perceptual_roughness: 0.90,
        ..default()
    });
    let zone_neutral_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.84, 0.32),
        emissive: LinearRgba::new(2.6, 2.0, 0.45, 1.0),
        unlit: true,
        ..default()
    });
    let zone_allies_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.68, 1.0),
        emissive: LinearRgba::new(0.45, 2.2, 4.5, 1.0),
        unlit: true,
        ..default()
    });
    let zone_axis_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.30, 0.26),
        emissive: LinearRgba::new(4.5, 0.55, 0.40, 1.0),
        unlit: true,
        ..default()
    });
    let zone_contested_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.64, 0.16),
        emissive: LinearRgba::new(4.2, 1.8, 0.25, 1.0),
        unlit: true,
        ..default()
    });

    commands.insert_resource(EffectsAssets {
        tracer_mesh,
        tracer_mat,
        he_tracer_mat,
        ap_tracer_mat,
        puff_mesh,
        beacon_post_mesh,
        flash_mat,
        fire_mat,
        spark_mat,
        smoke_mat,
        wreck_smoke_mat,
        dirt_mat,
        sand_dust_mat,
        snow_dust_mat,
        water_spray_mat,
        fpe_foam_mat,
        brick_debris_mat,
        rubble_debris_mat,
        wood_debris_mat,
        zone_neutral_mat,
        zone_allies_mat,
        zone_axis_mat,
        zone_contested_mat,
    });
}

fn clear_effects_on_restart(
    mut commands: Commands,
    pending: Res<PendingSimEvents>,
    mut tracers: ResMut<TracerEntities>,
    particle_q: Query<Entity, With<TransientParticle>>,
    light_q: Query<Entity, With<TransientLight>>,
) {
    if !pending.battle_restarted {
        return;
    }
    for (_, entity) in tracers.map.drain() {
        commands.entity(entity).despawn();
    }
    for entity in &particle_q {
        commands.entity(entity).despawn();
    }
    for entity in &light_q {
        commands.entity(entity).despawn();
    }
}

fn sync_projectile_tracers(
    mut commands: Commands,
    sim: Res<SimResource>,
    assets: Res<EffectsAssets>,
    mut tracers: ResMut<TracerEntities>,
    mut tf_q: Query<&mut Transform>,
) {
    let active_ids: Vec<u64> = sim.0.projectiles.iter().map(|p| p.id).collect();

    tracers.map.retain(|id, entity| {
        if active_ids.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });

    for proj in &sim.0.projectiles {
        let pos = to_bevy_vec3(proj.position);
        let dir = to_bevy_vec3(proj.velocity).normalize_or_zero();
        let rot = if dir.length_squared() > 1e-4 {
            Transform::IDENTITY.looking_to(dir, Vec3::Y).rotation
        } else {
            Quat::IDENTITY
        };

        if let Some(&entity) = tracers.map.get(&proj.id) {
            if let Ok(mut tf) = tf_q.get_mut(entity) {
                tf.translation = pos;
                tf.rotation = rot;
            }
        } else {
            let mat = match proj.shell.kind {
                ShellKind::He => assets.he_tracer_mat.clone(),
                ShellKind::Ap => assets.ap_tracer_mat.clone(),
                ShellKind::Aphe | ShellKind::Aphebc | ShellKind::Apcbc => assets.tracer_mat.clone(),
            };
            let entity = commands
                .spawn((
                    Mesh3d(assets.tracer_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_translation(pos).with_rotation(rot),
                ))
                .id();
            tracers.map.insert(proj.id, entity);
        }
    }
}

/// Spawns lightweight 3D tactical perimeter beacons around each capture zone (A / B / C)
/// that tint live in Blue (Allies), Red (Axis), Gold (Neutral), or Amber (Contested).
fn spawn_capture_zone_beacons(
    mut commands: Commands,
    sim: Res<SimResource>,
    assets: Res<EffectsAssets>,
) {
    const POSTS_PER_ZONE: usize = 8;
    for (zone_idx, zone) in sim.0.battle.zones.iter().enumerate() {
        for p in 0..POSTS_PER_ZONE {
            let angle = p as f32 * std::f32::consts::TAU / POSTS_PER_ZONE as f32;
            let wx = zone.center.x + angle.cos() * zone.radius;
            let wz = zone.center.z + angle.sin() * zone.radius;
            let wy = sim
                .0
                .voxels
                .surface_height_at(wx, wz)
                .unwrap_or(zone.center.y);
            commands.spawn((
                BattleEntity,
                ZoneBeaconMarker { zone_idx },
                Mesh3d(assets.beacon_post_mesh.clone()),
                MeshMaterial3d(assets.zone_neutral_mat.clone()),
                Transform::from_xyz(wx, wy + 1.1, wz),
            ));
        }
    }
}

fn sync_capture_zone_beacons(
    sim: Res<SimResource>,
    assets: Res<EffectsAssets>,
    mut q: Query<(&ZoneBeaconMarker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (marker, mut mat) in &mut q {
        let Some(zone) = sim.0.battle.zones.get(marker.zone_idx) else {
            continue;
        };
        let desired = if zone.contested {
            &assets.zone_contested_mat
        } else {
            match zone.owner {
                Some(Team::Allies) => &assets.zone_allies_mat,
                Some(Team::Axis) => &assets.zone_axis_mat,
                None => &assets.zone_neutral_mat,
            }
        };
        if mat.0 != *desired {
            mat.0 = desired.clone();
        }
    }
}

fn spawn_shot_and_impact_effects(
    mut commands: Commands,
    sim: Res<SimResource>,
    pending: Res<PendingSimEvents>,
    assets: Res<EffectsAssets>,
    cam_q: Query<&Transform, With<MainTankCamera>>,
) {
    let cam_pos = cam_q.single().ok().map(|tf| tf.translation);
    let near_cam =
        |pos: Vec3| cam_pos.is_some_and(|c| c.distance_squared(pos) < MAX_DYNAMIC_LIGHT_DIST_SQ);

    // 1. Muzzle flash + smoke puffs on firing (player & bots)
    for shot in &pending.fired_shots {
        let muzzle = to_bevy_vec3(shot.muzzle_position);
        let dir = to_bevy_vec3(shot.direction);

        if near_cam(muzzle) {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.72, 0.25),
                    intensity: 350_000.0,
                    range: 18.0,
                    ..default()
                },
                Transform::from_translation(muzzle + dir * 0.4),
                TransientLight {
                    age: 0.0,
                    lifetime: 0.11,
                    start_intensity: 350_000.0,
                },
            ));
        }

        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(assets.flash_mat.clone()),
            Transform::from_translation(muzzle + dir * 0.5).with_scale(Vec3::splat(0.75)),
            TransientParticle {
                velocity: dir * 6.0,
                gravity: 0.0,
                age: 0.0,
                lifetime: 0.10,
                start_scale: 0.85,
                end_scale: 0.05,
            },
        ));

        for i in 0..6 {
            let angle = i as f32 * std::f32::consts::TAU / 6.0;
            let side = Vec3::new(angle.cos(), 0.35 + (i % 2) as f32 * 0.25, angle.sin());
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(assets.smoke_mat.clone()),
                Transform::from_translation(muzzle + dir * 0.6).with_scale(Vec3::splat(0.35)),
                TransientParticle {
                    velocity: dir * 4.5 + side * 2.4,
                    gravity: -1.4,
                    age: 0.0,
                    lifetime: 0.55 + (i as f32) * 0.06,
                    start_scale: 0.35,
                    end_scale: 1.35,
                },
            ));
        }
    }

    // 2. Impact voxel dust / flash burst + Stage 7 HE craters & flying voxel debris
    for impact in &pending.impacts {
        let pos = to_bevy_vec3(impact.position);
        let normal = to_bevy_vec3(impact.normal).normalize_or_zero();
        let up = if normal.length_squared() > 0.1 {
            normal
        } else {
            Vec3::Y
        };
        let blast_scale = if impact.is_he {
            (impact.blast_radius_m * 0.65).clamp(1.25, 3.2)
        } else {
            1.0 + (impact.destroyed_voxels as f32 * 0.04).min(0.6)
        };

        if near_cam(pos) {
            let intensity = if impact.is_he {
                560_000.0 * blast_scale
            } else {
                280_000.0
            };
            commands.spawn((
                PointLight {
                    color: if impact.is_he {
                        Color::srgb(1.0, 0.50, 0.12)
                    } else {
                        Color::srgb(1.0, 0.6, 0.2)
                    },
                    intensity,
                    range: 14.0 * blast_scale,
                    ..default()
                },
                Transform::from_translation(pos + up * 0.6),
                TransientLight {
                    age: 0.0,
                    lifetime: if impact.is_he { 0.34 } else { 0.16 },
                    start_intensity: intensity,
                },
            ));
        }

        // Core blast flash (massive fireball for HE shells)
        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(if impact.is_he {
                assets.fire_mat.clone()
            } else {
                assets.flash_mat.clone()
            }),
            Transform::from_translation(pos + up * (0.35 * blast_scale))
                .with_scale(Vec3::splat(0.9 * blast_scale)),
            TransientParticle {
                velocity: up * (2.2 * blast_scale),
                gravity: 0.0,
                age: 0.0,
                lifetime: if impact.is_he { 0.36 } else { 0.18 },
                start_scale: 0.95 * blast_scale,
                end_scale: if impact.is_he { 0.25 } else { 0.05 },
            },
        ));

        let puff_count = if impact.is_he { 12 } else { 8 };
        for i in 0..puff_count {
            let angle = i as f32 * std::f32::consts::TAU / puff_count as f32;
            let spread = (2.4 + (i % 3) as f32 * 1.4) * blast_scale;
            let vel = up * ((4.2 + (i % 4) as f32 * 1.8) * blast_scale.sqrt())
                + Vec3::new(angle.cos() * spread, 1.5 * blast_scale, angle.sin() * spread);
            let is_smoke = i % 2 == 0;
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(if is_smoke {
                    if impact.is_he && i % 4 == 0 {
                        assets.wreck_smoke_mat.clone()
                    } else {
                        assets.smoke_mat.clone()
                    }
                } else {
                    assets.dirt_mat.clone()
                }),
                Transform::from_translation(pos + up * 0.3).with_scale(Vec3::splat(0.35 * blast_scale)),
                TransientParticle {
                    velocity: vel,
                    gravity: if is_smoke { 1.8 } else { 14.0 },
                    age: 0.0,
                    lifetime: if is_smoke {
                        0.85 + if impact.is_he { 0.55 } else { 0.0 }
                    } else {
                        0.65
                    },
                    start_scale: (if is_smoke { 0.45 } else { 0.32 }) * blast_scale.sqrt(),
                    end_scale: (if is_smoke { 1.6 } else { 0.08 }) * blast_scale,
                },
            ));
        }

        // Flying physical voxel debris from destroyed houses, walls, roofs & terrain craters
        for (idx, &(debris_pos, voxel_id)) in impact.debris.iter().enumerate() {
            let dpos = to_bevy_vec3(debris_pos);
            let radial = (dpos - pos).normalize_or_zero();
            let angle = idx as f32 * 2.399_963;
            let burst_dir = if radial.length_squared() > 0.05 {
                radial
            } else {
                Vec3::new(angle.cos(), 0.4, angle.sin()).normalize()
            };
            let mat = match voxel_id {
                VoxelId::BRICK | VoxelId::ROOF_TILE | VoxelId::MUDBRICK => {
                    assets.brick_debris_mat.clone()
                }
                VoxelId::WOOD | VoxelId::TREE_TRUNK => assets.wood_debris_mat.clone(),
                VoxelId::PLASTER_WALL
                | VoxelId::RUBBLE
                | VoxelId::ROCK
                | VoxelId::GRANITE
                | VoxelId::CONCRETE
                | VoxelId::BUNKER_CONCRETE
                | VoxelId::SANDSTONE => assets.rubble_debris_mat.clone(),
                _ => assets.dirt_mat.clone(),
            };
            let speed = (4.8 + (idx % 4) as f32 * 2.2) * blast_scale.sqrt();
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(dpos).with_scale(Vec3::splat(0.34)),
                TransientParticle {
                    velocity: burst_dir * speed + Vec3::Y * (4.2 + (idx % 3) as f32 * 2.1),
                    gravity: 16.0,
                    age: 0.0,
                    lifetime: 0.72 + (idx % 3) as f32 * 0.16,
                    start_scale: 0.36,
                    end_scale: 0.12,
                },
            ));
        }
    }

    // 3. Stage 4/7/8 War Thunder armor hits: ricochet sparks, non-pens, penetrations, HE overpressure & APHE bursts
    for report in &pending.hit_reports {
        let pos = to_bevy_vec3(report.position);
        let normal = to_bevy_vec3(report.normal).normalize_or_zero();
        let up = if normal.length_squared() > 0.1 {
            normal
        } else {
            Vec3::Y
        };
        let is_ammo_det = report.kill_reason == Some(KillReason::AmmoRackDetonation);
        let is_explosive_burst = report.aphe_detonated || report.overpressure;
        let destroyed = report.kill_reason.is_some();

        if near_cam(pos) {
            let (intensity, range, life) = if is_ammo_det {
                (900_000.0, 28.0, 0.55)
            } else if destroyed || is_explosive_burst {
                (620_000.0, 22.0, 0.38)
            } else if report.outcome == HitOutcome::Ricochet {
                (340_000.0, 14.0, 0.14)
            } else {
                (260_000.0, 11.0, 0.12)
            };
            commands.spawn((
                PointLight {
                    color: if report.outcome == HitOutcome::Ricochet {
                        Color::srgb(1.0, 0.90, 0.55)
                    } else {
                        Color::srgb(1.0, 0.52, 0.14)
                    },
                    intensity,
                    range,
                    ..default()
                },
                Transform::from_translation(pos + up * 0.4),
                TransientLight {
                    age: 0.0,
                    lifetime: life,
                    start_intensity: intensity,
                },
            ));
        }

        // Ricochet sparks streak sharply along the deflected angle
        if report.outcome == HitOutcome::Ricochet {
            let rdir = report
                .ricochet_dir
                .map(to_bevy_vec3)
                .unwrap_or(up)
                .normalize_or_zero();
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let vel = rdir * (10.0 + (i % 3) as f32 * 4.0)
                    + Vec3::new(angle.cos() * 2.2, 1.8, angle.sin() * 2.2);
                commands.spawn((
                    Mesh3d(assets.puff_mesh.clone()),
                    MeshMaterial3d(assets.spark_mat.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::splat(0.16)),
                    TransientParticle {
                        velocity: vel,
                        gravity: 11.0,
                        age: 0.0,
                        lifetime: 0.36 + (i % 3) as f32 * 0.08,
                        start_scale: 0.18,
                        end_scale: 0.02,
                    },
                ));
            }
            continue;
        }

        // Armor flash & sparks (amplified for APHE internal detonation & HE overpressure)
        let flash_scale = if is_ammo_det {
            2.4
        } else if is_explosive_burst {
            1.75
        } else if destroyed {
            1.4
        } else if report.outcome == HitOutcome::Penetration {
            0.9
        } else {
            0.55
        };
        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(if is_ammo_det || is_explosive_burst {
                assets.fire_mat.clone()
            } else {
                assets.flash_mat.clone()
            }),
            Transform::from_translation(pos + up * 0.25).with_scale(Vec3::splat(flash_scale)),
            TransientParticle {
                velocity: Vec3::Y * if is_ammo_det || is_explosive_burst { 3.6 } else { 1.0 },
                gravity: 0.0,
                age: 0.0,
                lifetime: if is_ammo_det {
                    0.48
                } else if is_explosive_burst {
                    0.30
                } else {
                    0.16
                },
                start_scale: flash_scale,
                end_scale: 0.1,
            },
        ));

        let spark_count = if is_ammo_det {
            10
        } else if is_explosive_burst {
            8
        } else {
            5
        };
        for i in 0..spark_count {
            let angle = i as f32 * std::f32::consts::TAU / spark_count as f32;
            let vel = up * (5.5 + (i % 3) as f32 * 2.8)
                + Vec3::new(angle.cos() * 3.8, 2.5, angle.sin() * 3.8);
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(assets.spark_mat.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(0.16)),
                TransientParticle {
                    velocity: vel,
                    gravity: 12.5,
                    age: 0.0,
                    lifetime: 0.34 + (i % 2) as f32 * 0.12,
                    start_scale: 0.18,
                    end_scale: 0.02,
                },
            ));
        }

        // Massive fireball + dark smoke tower on ammo-rack detonation, overpressure, or knockout
        if destroyed || is_explosive_burst {
            let plume_n = if is_ammo_det {
                14
            } else if destroyed {
                9
            } else {
                6
            };
            for i in 0..plume_n {
                let angle = i as f32 * std::f32::consts::TAU / plume_n as f32;
                let spread = 1.2 + (i % 3) as f32 * 0.7;
                let is_flame = (is_ammo_det || is_explosive_burst) && i % 3 == 0;
                commands.spawn((
                    Mesh3d(assets.puff_mesh.clone()),
                    MeshMaterial3d(if is_flame {
                        assets.fire_mat.clone()
                    } else {
                        assets.wreck_smoke_mat.clone()
                    }),
                    Transform::from_translation(pos + Vec3::Y * 0.6).with_scale(Vec3::splat(0.65)),
                    TransientParticle {
                        velocity: Vec3::new(
                            angle.cos() * spread,
                            if is_flame { 5.2 } else { 3.8 } + (i % 4) as f32 * 1.4,
                            angle.sin() * spread,
                        ),
                        gravity: -0.7,
                        age: 0.0,
                        lifetime: if is_flame {
                            0.65
                        } else {
                            1.5 + (i % 5) as f32 * 0.30
                        },
                        start_scale: 0.55,
                        end_scale: if is_flame { 2.0 } else { 2.8 },
                    },
                ));
            }
        }
    }

    // 4. Fire Prevention Equipment (FPE) white CO2/foam burst on engine deck
    for &tank_id in &pending.fpe_uses {
        let Some(unit) = sim.0.tanks.get(tank_id.index()) else {
            continue;
        };
        let origin = to_bevy_vec3(unit.state.position)
            + Quat::from_rotation_y(unit.state.hull_yaw) * Vec3::new(0.0, 1.4, 1.25);
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::TAU / 8.0;
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(assets.fpe_foam_mat.clone()),
                Transform::from_translation(origin).with_scale(Vec3::splat(0.42)),
                TransientParticle {
                    velocity: Vec3::new(
                        angle.cos() * 2.8,
                        2.6 + (i % 3) as f32 * 0.8,
                        angle.sin() * 2.8,
                    ),
                    gravity: -0.5,
                    age: 0.0,
                    lifetime: 0.72 + (i % 3) as f32 * 0.14,
                    start_scale: 0.44,
                    end_scale: 1.55,
                },
            ));
        }
    }
}

/// Continuously emits voxel flames and black smoke from burning tanks (active engine/fuel fire).
fn emit_active_tank_fire_effects(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimResource>,
    assets: Res<EffectsAssets>,
    mut timer: ResMut<FireEmitTimer>,
) {
    timer.accum += time.delta_secs();
    if timer.accum < 0.14 {
        return;
    }
    timer.accum = 0.0;

    for unit in &sim.0.tanks {
        let burning = unit.damage.on_fire || (!unit.alive && unit.damage.turret_blown_off);
        if !burning {
            continue;
        }
        let deck_offset = if unit.damage.turret_blown_off {
            Vec3::new(0.0, 1.45, -0.25)
        } else {
            Vec3::new(0.0, 1.35, 1.35)
        };
        let origin = to_bevy_vec3(unit.state.position)
            + Quat::from_rotation_y(unit.state.hull_yaw) * deck_offset;
        let phase = (sim.0.tick as f32 * 0.37 + unit.id.0 as f32 * 1.9).sin();

        // Rising orange flame voxel
        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(assets.fire_mat.clone()),
            Transform::from_translation(origin + Vec3::new(phase * 0.3, 0.0, -phase * 0.25))
                .with_scale(Vec3::splat(0.45)),
            TransientParticle {
                velocity: Vec3::new(phase * 0.6, 2.8, -phase * 0.5),
                gravity: -0.5,
                age: 0.0,
                lifetime: 0.42,
                start_scale: 0.52,
                end_scale: 0.08,
            },
        ));

        // Billowing black smoke column
        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(assets.wreck_smoke_mat.clone()),
            Transform::from_translation(origin + Vec3::Y * 0.4).with_scale(Vec3::splat(0.55)),
            TransientParticle {
                velocity: Vec3::new(phase * 0.9, 3.6, 0.4),
                gravity: -0.4,
                age: 0.0,
                lifetime: 1.55,
                start_scale: 0.55,
                end_scale: 2.35,
            },
        ));
    }
}

/// Emits lightweight biome-matched track dust plumes or river-fording water splashes behind fast tanks.
fn emit_track_dust_and_water_effects(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimResource>,
    assets: Res<EffectsAssets>,
    mut timer: ResMut<DustEmitTimer>,
    cam_q: Query<&Transform, With<MainTankCamera>>,
) {
    timer.accum += time.delta_secs();
    if timer.accum < 0.12 {
        return;
    }
    timer.accum = 0.0;

    let Some(cam_pos) = cam_q.single().ok().map(|tf| tf.translation) else {
        return;
    };
    let map_dust_mat = match MapId::from_id(&sim.0.map.id) {
        Some(MapId::ElAlamein) => &assets.sand_dust_mat,
        Some(MapId::StalingradWinter) => &assets.snow_dust_mat,
        _ => &assets.dirt_mat,
    };

    for unit in &sim.0.tanks {
        if !unit.alive || unit.state.forward_speed.abs() < 3.4 {
            continue;
        }
        let pos = to_bevy_vec3(unit.state.position);
        if pos.distance_squared(cam_pos) > MAX_TRACK_DUST_DIST_SQ {
            continue;
        }
        let in_water = unit.state.is_on_water(&sim.0.voxels);
        let mat = if in_water {
            assets.water_spray_mat.clone()
        } else {
            map_dust_mat.clone()
        };
        let rot = Quat::from_rotation_y(unit.state.hull_yaw);
        let rear_z = if unit.state.forward_speed >= 0.0 {
            unit.state.config.hull_half_length * 0.85
        } else {
            -unit.state.config.hull_half_length * 0.85
        };
        let track_x = (unit.state.config.hull_half_width - 0.15).max(1.0);
        let speed_scale = (unit.state.forward_speed.abs() / 12.0).clamp(0.45, 1.25);

        for side in [-1.0f32, 1.0f32] {
            let spawn_p = pos + rot * Vec3::new(side * track_x, 0.25, rear_z);
            let out_v = rot * Vec3::new(side * 0.9, if in_water { 2.4 } else { 1.3 }, rear_z.signum() * 1.4);
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(spawn_p).with_scale(Vec3::splat(0.26 * speed_scale)),
                TransientParticle {
                    velocity: out_v * speed_scale,
                    gravity: if in_water { 6.5 } else { 0.4 },
                    age: 0.0,
                    lifetime: if in_water { 0.42 } else { 0.62 },
                    start_scale: 0.28 * speed_scale,
                    end_scale: (if in_water { 0.55 } else { 0.95 }) * speed_scale,
                },
            ));
        }
    }
}

fn tick_transient_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_q: Query<(Entity, &mut Transform, &mut TransientParticle)>,
    mut light_q: Query<(Entity, &mut PointLight, &mut TransientLight)>,
) {
    let dt = time.delta_secs();

    for (entity, mut tf, mut p) in &mut particle_q {
        p.age += dt;
        if p.age >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        p.velocity.y -= p.gravity * dt;
        tf.translation += p.velocity * dt;
        tf.translation.y = tf.translation.y.max(-4.0);

        let t = (p.age / p.lifetime).clamp(0.0, 1.0);
        let s = p.start_scale.lerp(p.end_scale, t);
        tf.scale = Vec3::splat(s.max(0.01));
    }

    for (entity, mut light, mut tl) in &mut light_q {
        tl.age += dt;
        if tl.age >= tl.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (tl.age / tl.lifetime).clamp(0.0, 1.0);
        light.intensity = tl.start_intensity * (1.0 - t);
    }
}
