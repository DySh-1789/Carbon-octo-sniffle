pub mod ai;
pub mod ballistics;
pub mod battle;
pub mod map_gen;
pub mod sim_world;
pub mod tank;

pub use ai::{BotBrain, BotMode, BotObjective, NAV_CELL_SIZE, NavGrid};
pub use ballistics::{FIXED_DT, GRAVITY, ImpactEvent, Projectile};
pub use battle::{
    BATTLE_TIME_LIMIT, BattleMode, BattleOutcome, BattleState, CAPTURE_RATE_PER_SEC,
    CaptureZoneState, KILL_TICKET_PENALTY, KillEvent, MAX_TANKS, STAGE3_HITS_TO_DESTROY,
    START_TICKETS, TankHitEvent, TankId, TankUnit, Team, VISION_RANGE, ZONE_BLEED_PER_SEC,
    ray_vs_tank, raycast_tanks, resolve_tank_collisions,
};
pub use map_gen::{
    CaptureZoneDef, MapMetadata, MinimapData, TacticalPosition, TacticalPositionKind,
    default_metadata_for_world, generate_european_valley_map, generate_flat_proving_ground,
    generate_minimap_data,
};
pub use sim_world::SimWorld;
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
        let start = sim.player_tank().position;

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
            sim.player_tank().position.z < start.z - 8.0,
            "tank should move forward along -Z, pos={:?}",
            sim.player_tank().position
        );
        assert!(sim.player_tank().forward_speed > 5.0);
        assert!((sim.player_tank().position.y - 0.0).abs() < 1e-4);

        // Turn left for 1 second
        let yaw_before = sim.player_tank().hull_yaw;
        for _ in 0..60 {
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    steer: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(sim.player_tank().hull_yaw > yaw_before + 0.5);
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
            (sim.player_tank().turret_yaw_local + std::f32::consts::FRAC_PI_2).abs() < 0.05,
            "turret_yaw_local={}",
            sim.player_tank().turret_yaw_local
        );
        assert!(
            (sim.player_tank().gun_pitch_local - sim.player_tank().config.gun_max_pitch).abs() < 1e-3,
            "gun_pitch_local={}",
            sim.player_tank().gun_pitch_local
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
        assert_eq!(sim.player_tank().shots_fired, 1);
        assert_eq!(sim.projectiles.len(), 1);
        assert!(!sim.player_tank().is_loaded());

        // Immediate second fire must be blocked by reload timer
        sim.step_fixed(
            FIXED_DT,
            TankInput {
                aim_world_point: aim,
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(sim.player_tank().shots_fired, 1);

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
        let start_z = sim.player_tank().position.z;
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
            sim.player_tank().position.z < start_z - 15.0,
            "player should drive cleanly north from spawn along main road, start_z={start_z}, now={}",
            sim.player_tank().position.z
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
            max_pitch_seen = max_pitch_seen.max(sim.player_tank().hull_pitch);
        }

        assert!(
            sim.player_tank().position.y > 1.0,
            "tank must climb the voxel ramp, y={}",
            sim.player_tank().position.y
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
            sim.player_tank().position.z > -9.5,
            "tank must be stopped by brick building wall, pos={:?}",
            sim.player_tank().position
        );
        assert!(
            sim.player_tank().forward_speed.abs() < 0.5,
            "impact with perpendicular wall should halt forward speed, speed={}",
            sim.player_tank().forward_speed
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

        let probe_z = sim.player_tank().position.z - 2.65 - 0.1;
        let front_h = sim.voxels.surface_height_at(0.0, probe_z);
        let front_top = sim.voxels.top_solid_voxel_at(0, (probe_z * 2.0).floor() as i32);
        assert!(
            sim.player_tank().position.z < -65.0,
            "tank should cleanly traverse village square and stone arch bridge onto north bank, stopped at {:?}, front_z={probe_z}, front_h={front_h:?}, front_top={front_top:?}",
            sim.player_tank().position
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

    #[test]
    fn nav_grid_is_compact_and_paths_reach_all_zones_and_cross_bridge() {
        let (world, meta) = generate_european_valley_map();
        let nav = NavGrid::build(&world);
        assert!(
            nav.memory_bytes() < 64 * 1024,
            "NavGrid must stay under 64 KiB, got {} B",
            nav.memory_bytes()
        );
        assert!(nav.walkable_count() > 2500);

        for spawn in meta.spawn_allies.iter().take(3) {
            for zone in &meta.capture_zones {
                let path = nav.find_path(*spawn, zone.center);
                assert!(
                    !path.is_empty(),
                    "Allies spawn {spawn:?} failed to path to Zone {}",
                    zone.label
                );
                let end = *path.last().unwrap();
                let dist_xz = ((end.x - zone.center.x).powi(2) + (end.z - zone.center.z).powi(2)).sqrt();
                assert!(
                    dist_xz < zone.radius,
                    "path from {spawn:?} to Zone {} ended too far away ({dist_xz:.1} m)",
                    zone.label
                );
            }
        }

        // Cross-map path across valley stream / bridge to Axis spawn
        let south = meta.spawn_allies[0];
        let north = meta.spawn_axis[0];
        let cross = nav.find_path(south, north);
        assert!(!cross.is_empty());
        let end = *cross.last().unwrap();
        assert!(
            ((end.x - north.x).powi(2) + (end.z - north.z).powi(2)).sqrt() < 12.0,
            "expected cross-river A* path to reach North spawn {north:?}, ended at {end:?}"
        );
    }

    #[test]
    fn tank_vs_tank_collision_separates_living_tanks_and_treats_wrecks_as_cover() {
        let voxels = generate_flat_proving_ground(96);
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);

        let mut a = TankUnit::new(
            TankId(0),
            "Alpha",
            Team::Allies,
            true,
            TankState::new(cfg.clone(), Vec3::new(0.0, 0.0, 10.0), 0.0),
        );
        let mut b = TankUnit::new(
            TankId(1),
            "Beta",
            Team::Axis,
            false,
            TankState::new(cfg, Vec3::new(0.0, 0.0, -10.0), std::f32::consts::PI),
        );
        a.state.forward_speed = 12.0;
        b.state.forward_speed = 12.0;

        let mut tanks = vec![a, b];
        for _ in 0..150 {
            let dummy = TankInput {
                throttle: 1.0,
                ..Default::default()
            };
            tanks[0].state.step(FIXED_DT, dummy, &voxels);
            tanks[1].state.step(FIXED_DT, dummy, &voxels);
            resolve_tank_collisions(&mut tanks, &voxels);
        }

        let sep_z = (tanks[0].state.position.z - tanks[1].state.position.z).abs();
        assert!(
            sep_z >= 5.2,
            "head-on colliding tanks must stay physically separated, sep_z={sep_z:.2}"
        );

        // Wreck test: destroy Beta and drive Alpha harder into it — wreck remains stationary cover
        tanks[1].alive = false;
        tanks[1].state.forward_speed = 0.0;
        let wreck_pos = tanks[1].state.position;
        for _ in 0..60 {
            tanks[0].state.step(
                FIXED_DT,
                TankInput {
                    throttle: 1.0,
                    ..Default::default()
                },
                &voxels,
            );
            resolve_tank_collisions(&mut tanks, &voxels);
        }
        assert!(
            tanks[1].state.position.distance(wreck_pos) < 1e-4,
            "wrecked tank must act as immobile physical cover"
        );
        assert!(
            (tanks[0].state.position.z - wreck_pos.z).abs() >= 5.2,
            "living tank must not clip through wreck"
        );
    }

    #[test]
    fn direct_hits_destroy_enemy_tank_in_three_shots_and_emit_kill_events() {
        let voxels = generate_flat_proving_ground(128);
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let map = default_metadata_for_world(&voxels, "duel", "Duel");
        let mut sim = SimWorld::new_team_battle(voxels, map, cfg, 1);

        // Disable Axis bot brain steering/movement and position target directly 50 m ahead
        sim.brains[1] = None;
        sim.tanks[1].state.position = Vec3::new(0.0, 0.0, -50.0);
        sim.tanks[1].state.hull_yaw = std::f32::consts::PI;

        let target_aim = sim.tanks[1].aim_center();
        let mut total_hits = 0usize;
        let mut total_kills = 0usize;
        for _ in 0..1000 {
            let fire = sim.player_tank().is_loaded();
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    aim_world_point: target_aim,
                    fire,
                    ..Default::default()
                },
            );
            total_hits += sim.recent_tank_hits.iter().filter(|h| h.damaged).count();
            total_kills += sim.recent_kills.len();
            if sim.battle.is_over() {
                break;
            }
        }

        assert_eq!(total_hits, STAGE3_HITS_TO_DESTROY as usize);
        assert_eq!(total_kills, 1);
        assert!(!sim.tanks[1].alive);
        assert_eq!(sim.player_unit().kills, 1);
        assert_eq!(sim.battle.outcome, BattleOutcome::Victory(Team::Allies));
        assert!(sim.battle.axis_tickets <= START_TICKETS - KILL_TICKET_PENALTY + 0.01);
    }

    #[test]
    fn capture_zones_transfer_ownership_bleed_tickets_and_resolve_victory() {
        let (world, meta) = generate_european_valley_map();
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let mut sim = SimWorld::new_team_battle(world, meta, cfg, 2);

        // Disable bots and station Allies unit #1 inside Zone B, keep enemy away
        sim.brains.fill(None);
        let zone_b = sim.battle.zones[1].center;
        sim.tanks[1].state.position = zone_b;
        sim.tanks[1].state.settle_on_terrain(&sim.voxels);
        // Move Axis tanks far to their north corner so nobody contests B
        for t in sim.tanks.iter_mut().filter(|t| t.team == Team::Axis) {
            t.state.position = Vec3::new(150.0, 4.0, -150.0);
        }

        // Advance 25 simulated seconds: single tank captures at 0.05/s => 20s to full capture
        for _ in 0..(25 * 60) {
            sim.step_fixed(FIXED_DT, TankInput::default());
        }

        assert_eq!(sim.battle.zones[1].owner, Some(Team::Allies));
        assert!(
            sim.battle.axis_tickets < START_TICKETS - 5.0,
            "owning more zones must bleed Axis tickets, got {}",
            sim.battle.axis_tickets
        );
        assert_eq!(sim.battle.allies_tickets, START_TICKETS);
    }

    #[test]
    fn six_v_six_european_valley_team_battle_runs_cleanly_and_restarts_in_place() {
        let (world, meta) = generate_european_valley_map();
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let mut sim = SimWorld::new_team_battle(world, meta, cfg, 6);

        assert_eq!(sim.tanks.len(), 12);
        assert_eq!(sim.alive_count(Team::Allies), 6);
        assert_eq!(sim.alive_count(Team::Axis), 6);

        for unit in &sim.tanks {
            let colls = unit.state.count_hull_collisions(
                &sim.voxels,
                unit.state.position.x,
                unit.state.position.z,
                unit.state.hull_yaw,
            );
            assert_eq!(
                colls, 0,
                "unit {} spawned inside obstacle at {:?}",
                unit.name, unit.state.position
            );
        }

        let mut total_bot_shots = 0usize;
        let mut zone_progress_seen = false;
        // Run 40 simulated seconds of autonomous 6v6 combat (player idle, 11 bots fighting)
        for _ in 0..(40 * 60) {
            sim.step_fixed(FIXED_DT, TankInput::default());
            total_bot_shots += sim.recent_shots.len();
            if sim.battle.zones.iter().any(|z| z.progress.abs() > 0.05) {
                zone_progress_seen = true;
            }
        }

        // Bots should leave spawn, maneuver toward capture zones, spot & shoot at enemies
        let allies_moved = sim
            .tanks
            .iter()
            .filter(|t| !t.is_player && t.team == Team::Allies && t.state.position.z < 110.0)
            .count();
        let axis_moved = sim
            .tanks
            .iter()
            .filter(|t| t.team == Team::Axis && t.state.position.z > -110.0)
            .count();
        assert!(allies_moved >= 4, "allied bots should advance north, allies_moved={allies_moved}");
        assert!(axis_moved >= 4, "axis bots should advance south, axis_moved={axis_moved}");
        assert!(
            total_bot_shots >= 10,
            "bots should acquire LOS and exchange cannon fire, shots={total_bot_shots}"
        );
        assert!(zone_progress_seen, "bots should reach and begin capturing A/B/C zones");

        for unit in &sim.tanks {
            assert!(unit.state.position.is_finite());
            assert!(unit.state.position.x.abs() <= 192.0);
            assert!(unit.state.position.z.abs() <= 192.0);
        }

        // Verify in-place restart restores full 6v6 state cleanly
        sim.restart_battle();
        assert_eq!(sim.tick, 0);
        assert_eq!(sim.tanks.len(), 12);
        assert_eq!(sim.alive_count(Team::Allies), 6);
        assert_eq!(sim.alive_count(Team::Axis), 6);
        assert_eq!(sim.battle.outcome, BattleOutcome::Ongoing);
        assert_eq!(sim.battle.allies_tickets, START_TICKETS);
        assert_eq!(sim.battle.axis_tickets, START_TICKETS);
        assert!(sim.battle.zones.iter().all(|z| z.owner.is_none() && z.progress == 0.0));
    }
}
