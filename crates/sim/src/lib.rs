pub mod ballistics;
pub mod map_gen;
pub mod tank;

pub use ballistics::{FIXED_DT, GRAVITY, ImpactEvent, Projectile, SimWorld};
pub use map_gen::{
    CaptureZoneDef, MapMetadata, MinimapData, TacticalPosition, TacticalPositionKind,
    default_metadata_for_world, generate_european_valley_map, generate_flat_proving_ground,
    generate_minimap_data,
};
pub use tank::{FiredShot, TankConfig, TankInput, TankState, wrap_angle};

#[cfg(test)]
mod tests {
    use voxel::glam::{IVec3, Vec3};
    use voxel::{VoxelId, build_basic_tank_models};

    use super::*;

    fn create_test_sim() -> SimWorld {
        let voxels = generate_flat_proving_ground(128);
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        SimWorld::new(voxels, cfg)
    }

    #[test]
    fn flat_proving_ground_surface_is_zero_and_symmetric() {
        let world = generate_flat_proving_ground(128);
        assert_eq!(world.surface_height_at(0.0, 0.0), Some(0.0));
        assert_eq!(world.surface_height_at(50.0, -50.0), Some(0.0));
        assert_eq!(world.surface_height_at(-100.0, 100.0), Some(0.0));
        assert!(world.chunk_count() > 0);
    }

    #[test]
    fn tank_drives_forward_turns_and_stays_inside_arena() {
        let mut sim = create_test_sim();
        let start = sim.player_tank.position;

        // Accelerate forward (-Z) for 2 seconds
        for _ in 0..120 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(
            sim.player_tank.position.z < start.z - 8.0,
            "tank should move forward along -Z, pos={:?}",
            sim.player_tank.position
        );
        assert!(sim.player_tank.forward_speed > 5.0);
        assert!((sim.player_tank.position.y - 0.0).abs() < 1e-4);

        // Turn left for 1 second
        let yaw_before = sim.player_tank.hull_yaw;
        for _ in 0..60 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    steer: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(sim.player_tank.hull_yaw > yaw_before + 0.5);
    }

    #[test]
    fn turret_slews_toward_target_and_respects_elevation_limits() {
        let mut sim = create_test_sim();
        // Aim to the right (+X) and high up (+Y)
        let aim_right_high = Vec3::new(80.0, 120.0, 0.0);
        for _ in 0..240 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    aim_world_point: aim_right_high,
                    ..Default::default()
                },
            );
        }
        // Right is -PI/2 yaw; gun pitch should clamp cleanly at gun_max_pitch
        assert!(
            (sim.player_tank.turret_yaw_local + std::f32::consts::FRAC_PI_2).abs() < 0.05,
            "turret_yaw_local={}",
            sim.player_tank.turret_yaw_local
        );
        assert!(
            (sim.player_tank.gun_pitch_local - sim.player_tank.config.gun_max_pitch).abs() < 1e-3,
            "gun_pitch_local={}",
            sim.player_tank.gun_pitch_local
        );
    }

    #[test]
    fn firing_respects_reload_and_shell_impacts_flat_ground() {
        let mut sim = create_test_sim();
        // Aim slightly downward toward a point 40m ahead on the ground
        let aim = Vec3::new(0.0, 0.0, -40.0);
        for _ in 0..60 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    aim_world_point: aim,
                    ..Default::default()
                },
            );
        }

        // Fire first shot
        sim.step_fixed(
            FIXED_DT,
            TankInput {
                aim_world_point: aim,
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(sim.player_tank.shots_fired, 1);
        assert_eq!(sim.projectiles.len(), 1);
        assert!(!sim.player_tank.is_loaded());

        // Immediate second fire must be blocked by reload timer
        sim.step_fixed(
            FIXED_DT,
            TankInput {
                aim_world_point: aim,
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(sim.player_tank.shots_fired, 1);

        // Advance simulation until shell hits flat ground (~40m / 780m/s ≈ 0.05s)
        let mut impact = None;
        for _ in 0..120 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    aim_world_point: aim,
                    ..Default::default()
                },
            );
            if let Some(ev) = sim.recent_impacts.first().copied() {
                impact = Some(ev);
                break;
            }
        }
        let hit = impact.expect("shell must impact the flat ground");
        assert!(hit.position.y.abs() < 0.05, "impact={:?}", hit.position);
        assert!(
            (hit.position.z - (-40.0)).abs() < 3.0,
            "expected impact near z=-40, got {:?}",
            hit.position
        );
        assert!(sim.projectiles.is_empty());
    }

    #[test]
    fn european_valley_generates_hills_buildings_bridge_and_minimap() {
        let (world, meta) = generate_european_valley_map();
        assert_eq!(meta.id, "european_valley");
        assert!((meta.size_m - 384.0).abs() < 0.1);
        assert_eq!(meta.capture_zones.len(), 3);
        assert!(meta.spawn_allies.len() >= 4);
        assert!(meta.spawn_axis.len() >= 4);
        assert!(meta.tactical_positions.len() >= 5);

        // Minimap pixel buffer matches size^2 * 4 RGBA bytes
        assert_eq!(
            meta.minimap.rgba.len(),
            (meta.minimap.size * meta.minimap.size * 4) as usize
        );

        // Verify genuine elevation contrast: East Hill (Zone C) is notably higher than Village Square (Zone B)
        let h_village = world.surface_height_at(0.0, 22.0).unwrap();
        let h_east_hill = world.surface_height_at(108.0, 8.0).unwrap();
        let h_west_ridge = world.surface_height_at(-108.0, -8.0).unwrap();
        assert!(
            h_east_hill > h_village + 4.5,
            "east hill ({h_east_hill}) should tower above village ({h_village})"
        );
        assert!(
            h_west_ridge > h_village + 3.0,
            "west ridge ({h_west_ridge}) should sit above village ({h_village})"
        );

        // Verify tall bell-tower church landmark exists overlooking town square
        let h_church = world.surface_height_at(23.5, 4.0).unwrap();
        assert!(
            h_church > h_village + 10.0,
            "church bell tower ({h_church}) should rise well above ground ({h_village})"
        );

        // Verify player can spawn cleanly and drive along the south approach road without snagging
        let models = build_basic_tank_models();
        let mut sim = SimWorld::with_map(world, meta, TankConfig::stage1_basic(&models));
        let start_z = sim.player_tank.position.z;
        for _ in 0..180 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(
            sim.player_tank.position.z < start_z - 15.0,
            "player should drive cleanly north from spawn along main road, start_z={start_z}, now={}",
            sim.player_tank.position.z
        );
    }

    #[test]
    fn six_point_suspension_tilts_hull_on_voxel_ramp_and_slows_uphill() {
        let mut world = generate_flat_proving_ground(64);
        world.bounds_max.y = 16;

        // Build a gentle climbable voxel ramp along -Z (steps up 0.5m every 3.0m ≈ 9.5 deg)
        for step in 0..8 {
            let z_max_vox = -14 - step * 6;
            let z_min_vox = z_max_vox - 5;
            world.fill_box(
                IVec3::new(-16, 0, z_min_vox),
                IVec3::new(16, step, z_max_vox),
                VoxelId::DIRT,
            );
        }

        let models = build_basic_tank_models();
        let mut sim = SimWorld::new(world, TankConfig::stage1_basic(&models));

        let mut max_pitch_seen = 0.0f32;
        for _ in 0..240 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
            );
            max_pitch_seen = max_pitch_seen.max(sim.player_tank.hull_pitch);
        }

        assert!(
            sim.player_tank.position.y > 1.0,
            "tank must climb the voxel ramp, y={}",
            sim.player_tank.position.y
        );
        assert!(
            max_pitch_seen > 0.08,
            "6-point suspension must tilt hull nose up on ramp, max_pitch={max_pitch_seen}"
        );
    }

    #[test]
    fn tank_collides_with_brick_building_wall_and_does_not_pass_through() {
        let mut world = generate_flat_proving_ground(64);
        world.bounds_max.y = 12;

        // Place a solid 3m-tall brick wall directly across the forward path at z = -10m (vz = -20)
        world.fill_box(
            IVec3::new(-16, 0, -22),
            IVec3::new(16, 5, -20),
            VoxelId::BRICK,
        );

        let models = build_basic_tank_models();
        let mut sim = SimWorld::new(world, TankConfig::stage1_basic(&models));

        for _ in 0..240 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
            );
        }

        // Wall front face is at z = -9.5m (vz = -20 spans [-10.0, -9.5]); hull front is 2.65m ahead
        assert!(
            sim.player_tank.position.z > -9.5,
            "tank must be stopped by brick building wall, pos={:?}",
            sim.player_tank.position
        );
        assert!(
            sim.player_tank.forward_speed.abs() < 0.5,
            "impact with perpendicular wall should halt forward speed, speed={}",
            sim.player_tank.forward_speed
        );
    }

    #[test]
    fn tank_drives_full_highway_through_village_and_across_stone_bridge() {
        let (world, meta) = generate_european_valley_map();
        let models = build_basic_tank_models();
        let mut sim = SimWorld::with_map(world, meta, TankConfig::stage1_basic(&models));

        // Drive straight north along the main highway (X = 0) for 22 simulated seconds:
        // South Spawn (Z = +148) -> Village Square (Z = +22) -> Stone Arch Bridge (Z = -38.5) -> North Bank (Z < -65)
        for _ in 0..(22 * 60) {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
            );
        }

        let probe_z = sim.player_tank.position.z - 2.65 - 0.1;
        let front_h = sim.voxels.surface_height_at(0.0, probe_z);
        let front_top = sim.voxels.top_solid_voxel_at(0, (probe_z * 2.0).floor() as i32);
        assert!(
            sim.player_tank.position.z < -65.0,
            "tank should cleanly traverse village square and stone arch bridge onto north bank, stopped at {:?}, front_z={probe_z}, front_h={front_h:?}, front_top={front_top:?}",
            sim.player_tank.position
        );
        // Verify bridge deck stays dry above water (Y > 0.5 at bridge center)
        let bridge_y = sim.voxels.surface_height_at(0.0, -38.5).unwrap();
        assert!(
            bridge_y >= 1.4,
            "stone arch bridge deck must rise well above stream water, got Y={bridge_y}"
        );
    }

    #[test]
    fn memory_footprint_stays_within_strict_budget() {
        let (world, meta) = generate_european_valley_map();
        let chunk_count = world.chunk_count();
        let voxel_bytes = world.memory_bytes();

        let mut mesh_count = 0usize;
        let mut total_verts = 0usize;
        let mut total_indices = 0usize;
        let mut max_chunk_verts = 0usize;
        for coord in world.chunk_coords() {
            let m = voxel::mesh_world_chunk(&world, coord);
            if !m.is_empty() {
                mesh_count += 1;
                total_verts += m.positions.len();
                total_indices += m.indices.len();
                max_chunk_verts = max_chunk_verts.max(m.positions.len());
            }
        }
        // Vertex attributes: pos(12) + normal(12) + color(16) = 40 B; U16 indices = 2 B
        let mesh_bytes = total_verts * 40 + total_indices * 2;
        let voxel_mb = voxel_bytes as f64 / (1024.0 * 1024.0);
        let mesh_mb = mesh_bytes as f64 / (1024.0 * 1024.0);
        eprintln!(
            "MEMORY PROFILE: chunks={chunk_count}, voxel_ram={voxel_mb:.2} MB, meshes={mesh_count}, verts={total_verts}, tris={}, max_chunk_verts={max_chunk_verts}, gpu_mesh_buf={mesh_mb:.2} MB, minimap={:.2} KB",
            total_indices / 3,
            meta.minimap.rgba.len() as f64 / 1024.0,
        );

        // Strict regression guards so future stages never bloat RAM
        let stray: Vec<_> = world
            .chunk_coords()
            .filter(|c| c.y != 0 || !(-12..=11).contains(&c.x) || !(-12..=11).contains(&c.z))
            .collect();
        assert!(
            chunk_count <= 576,
            "European Valley should fit inside a single 24x24 vertical chunk layer, got {chunk_count}, stray={stray:?}"
        );
        assert!(
            voxel_mb < 12.0,
            "sparse voxel world RAM must stay under 12 MB, got {voxel_mb:.2} MB"
        );
        assert!(
            max_chunk_verts <= u16::MAX as usize,
            "every chunk mesh must fit inside U16 indices, got {max_chunk_verts} verts"
        );
        assert!(
            mesh_mb < 25.0,
            "total world GPU mesh buffer budget must stay under 25 MB, got {mesh_mb:.2} MB"
        );
    }
}
