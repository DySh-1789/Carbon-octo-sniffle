use std::collections::HashMap;

use bevy::prelude::*;

use crate::bridge::{PendingSimEvents, SimResource, to_bevy_vec3};
use crate::camera::MainTankCamera;

/// Maximum camera distance for spawning dynamic transient `PointLight`s (keeps WebGL2/WGPU light clusters lean).
const MAX_DYNAMIC_LIGHT_DIST_SQ: f32 = 95.0 * 95.0;

#[derive(Resource)]
struct EffectsAssets {
    tracer_mesh: Handle<Mesh>,
    tracer_mat: Handle<StandardMaterial>,
    puff_mesh: Handle<Mesh>,
    flash_mat: Handle<StandardMaterial>,
    smoke_mat: Handle<StandardMaterial>,
    wreck_smoke_mat: Handle<StandardMaterial>,
    dirt_mat: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct TracerEntities {
    map: HashMap<u64, Entity>,
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
            .add_systems(Startup, setup_effects_assets)
            .add_systems(
                Update,
                (
                    clear_effects_on_restart,
                    sync_projectile_tracers,
                    spawn_shot_and_impact_effects,
                    tick_transient_effects,
                )
                    .chain(),
            );
    }
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
    let puff_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let flash_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.18),
        emissive: LinearRgba::new(10.0, 4.5, 0.8, 1.0),
        unlit: true,
        ..default()
    });
    let smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.60, 0.56),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wreck_smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.21, 0.20),
        perceptual_roughness: 1.0,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.29, 0.20),
        perceptual_roughness: 0.95,
        ..default()
    });

    commands.insert_resource(EffectsAssets {
        tracer_mesh,
        tracer_mat,
        puff_mesh,
        flash_mat,
        smoke_mat,
        wreck_smoke_mat,
        dirt_mat,
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
            let entity = commands
                .spawn((
                    Mesh3d(assets.tracer_mesh.clone()),
                    MeshMaterial3d(assets.tracer_mat.clone()),
                    Transform::from_translation(pos).with_rotation(rot),
                ))
                .id();
            tracers.map.insert(proj.id, entity);
        }
    }
}

fn spawn_shot_and_impact_effects(
    mut commands: Commands,
    mut pending: ResMut<PendingSimEvents>,
    assets: Res<EffectsAssets>,
    cam_q: Query<&Transform, With<MainTankCamera>>,
) {
    let cam_pos = cam_q.single().ok().map(|tf| tf.translation);
    let near_cam = |pos: Vec3| cam_pos.is_some_and(|c| c.distance_squared(pos) < MAX_DYNAMIC_LIGHT_DIST_SQ);

    // 1. Muzzle flash + smoke puffs on firing (player & bots)
    for shot in pending.fired_shots.drain(..) {
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

    // 2. Impact voxel dust / flash burst when shell strikes terrain or buildings
    for impact in pending.impacts.drain(..) {
        let pos = to_bevy_vec3(impact.position);
        let normal = to_bevy_vec3(impact.normal).normalize_or_zero();
        let up = if normal.length_squared() > 0.1 {
            normal
        } else {
            Vec3::Y
        };

        if near_cam(pos) {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.6, 0.2),
                    intensity: 280_000.0,
                    range: 14.0,
                    ..default()
                },
                Transform::from_translation(pos + up * 0.6),
                TransientLight {
                    age: 0.0,
                    lifetime: 0.16,
                    start_intensity: 280_000.0,
                },
            ));
        }

        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(assets.flash_mat.clone()),
            Transform::from_translation(pos + up * 0.35).with_scale(Vec3::splat(0.9)),
            TransientParticle {
                velocity: up * 2.0,
                gravity: 0.0,
                age: 0.0,
                lifetime: 0.18,
                start_scale: 0.95,
                end_scale: 0.05,
            },
        ));

        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::TAU / 8.0;
            let spread = 2.4 + (i % 3) as f32 * 1.4;
            let vel = up * (4.2 + (i % 4) as f32 * 1.8)
                + Vec3::new(angle.cos() * spread, 1.5, angle.sin() * spread);
            let is_smoke = i % 2 == 0;
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(if is_smoke {
                    assets.smoke_mat.clone()
                } else {
                    assets.dirt_mat.clone()
                }),
                Transform::from_translation(pos + up * 0.3).with_scale(Vec3::splat(0.35)),
                TransientParticle {
                    velocity: vel,
                    gravity: if is_smoke { 2.5 } else { 14.0 },
                    age: 0.0,
                    lifetime: if is_smoke { 0.85 } else { 0.65 },
                    start_scale: if is_smoke { 0.45 } else { 0.32 },
                    end_scale: if is_smoke { 1.6 } else { 0.08 },
                },
            ));
        }
    }

    // 3. Direct tank armor hits (sparks) & vehicle knockouts (fireball + billowing smoke plume)
    for hit in pending.tank_hits.drain(..) {
        let pos = to_bevy_vec3(hit.position);
        let normal = to_bevy_vec3(hit.normal).normalize_or_zero();
        let up = if normal.length_squared() > 0.1 {
            normal
        } else {
            Vec3::Y
        };

        if near_cam(pos) {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.55, 0.15),
                    intensity: if hit.destroyed { 650_000.0 } else { 320_000.0 },
                    range: if hit.destroyed { 22.0 } else { 12.0 },
                    ..default()
                },
                Transform::from_translation(pos + Vec3::Y * 0.8),
                TransientLight {
                    age: 0.0,
                    lifetime: if hit.destroyed { 0.45 } else { 0.14 },
                    start_intensity: if hit.destroyed { 650_000.0 } else { 320_000.0 },
                },
            ));
        }

        // Hit spark / blast flash
        commands.spawn((
            Mesh3d(assets.puff_mesh.clone()),
            MeshMaterial3d(assets.flash_mat.clone()),
            Transform::from_translation(pos + up * 0.25)
                .with_scale(Vec3::splat(if hit.destroyed { 1.65 } else { 0.8 })),
            TransientParticle {
                velocity: Vec3::Y * if hit.destroyed { 2.5 } else { 0.6 },
                gravity: 0.0,
                age: 0.0,
                lifetime: if hit.destroyed { 0.36 } else { 0.14 },
                start_scale: if hit.destroyed { 1.75 } else { 0.85 },
                end_scale: 0.1,
            },
        ));

        // Metallic armor sparks flying along hit normal
        for i in 0..6 {
            let angle = i as f32 * std::f32::consts::TAU / 6.0;
            let vel = up * (6.0 + (i % 3) as f32 * 2.5)
                + Vec3::new(angle.cos() * 3.5, 2.0, angle.sin() * 3.5);
            commands.spawn((
                Mesh3d(assets.puff_mesh.clone()),
                MeshMaterial3d(assets.flash_mat.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(0.18)),
                TransientParticle {
                    velocity: vel,
                    gravity: 12.0,
                    age: 0.0,
                    lifetime: 0.35 + (i % 2) as f32 * 0.12,
                    start_scale: 0.20,
                    end_scale: 0.02,
                },
            ));
        }

        // Tall dark-smoke plume on knockout
        if hit.destroyed {
            for i in 0..12 {
                let angle = i as f32 * std::f32::consts::TAU / 12.0;
                let spread = 1.1 + (i % 3) as f32 * 0.6;
                commands.spawn((
                    Mesh3d(assets.puff_mesh.clone()),
                    MeshMaterial3d(assets.wreck_smoke_mat.clone()),
                    Transform::from_translation(pos + Vec3::Y * 0.5).with_scale(Vec3::splat(0.6)),
                    TransientParticle {
                        velocity: Vec3::new(
                            angle.cos() * spread,
                            3.6 + (i % 4) as f32 * 1.4,
                            angle.sin() * spread,
                        ),
                        gravity: -0.6,
                        age: 0.0,
                        lifetime: 1.6 + (i % 5) as f32 * 0.35,
                        start_scale: 0.55,
                        end_scale: 2.8,
                    },
                ));
            }
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
