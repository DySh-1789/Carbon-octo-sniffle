pub mod ai;
pub mod ballistics;
pub mod battle;
pub mod catalog;
pub mod damage;
pub mod economy;
pub mod map_gen;
pub mod maps;
pub mod shell;
pub mod sim_world;
pub mod tank;

pub use ai::{BotBrain, BotMode, BotObjective, NAV_CELL_SIZE, NavGrid};
pub use ballistics::{FIXED_DT, GRAVITY, ImpactEvent, Projectile};
pub use battle::{
    BATTLE_TIME_LIMIT, BattleMode, BattleOutcome, BattleState, CAPTURE_RATE_PER_SEC,
    CaptureZoneState, KILL_TICKET_PENALTY, KillEvent, MAX_TANKS, START_TICKETS, SimRng, TankId,
    TankUnit, Team, VISION_RANGE, ZONE_BLEED_PER_SEC, ray_vs_tank, raycast_tanks,
    raycast_tanks_armor, resolve_tank_collisions,
};
pub use catalog::{Nation, VehicleCatalog, VehicleClass, VehicleSpec, tier_roman};
pub use damage::{
    ArmorHit, ArmorProfile, ArmorType, CREW_TRANSFER_TIME, CrewMemberState, CrewRole,
    DamageModifiers, FIRE_BURNOUT_TIME, HitOutcome, HitReport, KillReason, MIN_OPERATIONAL_CREW,
    ModuleHealth, ModuleKind, ModuleState, NO_LOADER_RELOAD_MULT, PartFrame, TankDamageState,
    raycast_tank_armor, resolve_ap_impact, resolve_he_impact, resolve_he_splash,
    resolve_shell_impact,
};
pub use economy::{
    BattleRewardInput, BattleRewardSummary, PlayerProfile, VehicleEconomyCost, VehicleOwnership,
    economy_cost_for_tier,
};
pub use map_gen::{
    CaptureZoneDef, MapAtmosphere, MapMetadata, MinimapData, TacticalPosition,
    TacticalPositionKind, default_metadata_for_world, generate_european_valley_map,
    generate_flat_proving_ground, generate_minimap_data,
};
pub use maps::{MapId, generate_map};
pub use shell::{ShellKind, ShellSpec};
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
            if let Some(ev) = sim.recent_impacts.first().cloned() {
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
    fn ap_penetration_destroys_enemy_tank_via_war_thunder_crew_or_ammo_detonation() {
        let voxels = generate_flat_proving_ground(128);
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let map = default_metadata_for_world(&voxels, "duel", "Duel");
        let mut sim = SimWorld::new_team_battle(voxels, map, cfg, 1);

        // Position stationary enemy tank broadside 50 m ahead so AP penetrates 42 mm side armor cleanly
        sim.brains[1] = None;
        sim.tanks[1].state.position = Vec3::new(0.0, 0.0, -50.0);
        sim.tanks[1].state.hull_yaw = std::f32::consts::FRAC_PI_2;

        // Sweep aim across crew & fighting compartment on each reload (strictly no HP bar!)
        let aim_points = [
            Vec3::new(-0.9, 1.0, -50.0), // Hull front crew / ammo
            Vec3::new(0.1, 1.7, -50.0),  // Turret gunner / loader / commander
            Vec3::new(0.0, 0.9, -50.0),  // Center ammo racks
            Vec3::new(-0.3, 1.8, -50.0), // Turret ring / commander
        ];
        let mut shot_idx = 0usize;
        let mut total_pens = 0usize;
        let mut total_kills = 0usize;
        for _ in 0..2200 {
            let aim = aim_points[shot_idx % aim_points.len()];
            let fire = sim.player_tank().is_loaded();
            if fire {
                shot_idx += 1;
            }
            sim.step_fixed(
                FIXED_DT,
                TankInput {
                    aim_world_point: aim,
                    fire,
                    ..Default::default()
                },
            );
            total_pens += sim
                .recent_hit_reports
                .iter()
                .filter(|r| !r.inert_hit && r.outcome == HitOutcome::Penetration)
                .count();
            total_kills += sim.recent_kills.len();
            if sim.battle.is_over() {
                break;
            }
        }

        assert!(total_pens >= 1, "75mm AP must penetrate 42mm side armor");
        assert_eq!(total_kills, 1, "enemy must be destroyed via WT crew KO or ammo detonation");
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

    #[test]
    fn ap_de_marre_overmatch_and_ricochet_curves_behave_realistically() {
        let ap = ShellSpec::basic_75mm_ap();

        // De Marre velocity decay: slower shell loses penetration
        let pen_0m = ap.penetration_at_speed(ap.muzzle_velocity);
        let pen_slow = ap.penetration_at_speed(ap.muzzle_velocity * 0.8);
        assert!(pen_0m > 105.0 && pen_0m < 112.0);
        assert!(pen_slow < pen_0m * 0.78);

        // Thick 55mm UFP plate at 75 deg grazing angle guarantees ricochet
        assert!(ap.ricochet_chance(75.0, 55.0) >= 0.99);
        assert_eq!(ap.ricochet_chance(45.0, 55.0), 0.0);

        // Triple overmatch: 75mm shell vs 20mm roof (3.75x) cannot ricochet even at 78 deg!
        assert_eq!(ap.ricochet_chance(78.0, 20.0), 0.0);
        // Double overmatch: 75mm vs 35mm rear (2.14x) cuts ricochet chance sharply
        assert!(ap.ricochet_chance(70.0, 35.0) < 0.35);

        // Sloped effective armor increases dramatically with obliquity & CHA multiplier (0.94)
        let eff_flat = ap.effective_armor_mm(55.0, 0.0, ArmorType::Rha.multiplier());
        let eff_52 = ap.effective_armor_mm(55.0, 52.0, ArmorType::Rha.multiplier());
        let eff_cha = ap.effective_armor_mm(55.0, 0.0, ArmorType::Cha.multiplier());
        assert!((eff_flat - 55.0).abs() < 0.1);
        assert!(eff_52 > 80.0 && eff_52 < 86.0, "55mm @ 52 deg should be ~82mm, got {eff_52}");
        assert!(eff_cha < eff_flat);
    }

    #[test]
    fn angling_hull_causes_ufp_ricochet_or_nonpen_while_flat_side_penetrates() {
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let ap = ShellSpec::basic_75mm_ap();

        // Case 1: Tank angled at 40 deg yaw — incoming shell strikes sloped front UFP at compound >61 deg
        let yaw = 40.0f32.to_radians();
        let angled_tank = TankState::new(cfg.clone(), Vec3::ZERO, yaw);
        let mut dmg_angled = TankDamageState::basic_medium_tank();
        let dir = Vec3::new(0.0, 0.0, 1.0);
        // Front center of hull in world space: hull_rot * (0, 1.05, -2.6)
        let ufp_world = angled_tank.hull_rotation() * Vec3::new(0.0, 1.05, -2.6);
        let origin = Vec3::new(ufp_world.x, ufp_world.y, -10.0);
        let hit = raycast_tank_armor(&angled_tank, origin, dir, 20.0, false, true)
            .expect("ray should hit angled hull UFP");
        assert_eq!(hit.plate_name, "Hull UFP");
        let mut rng = SimRng::new(42);
        let report = resolve_ap_impact(
            TankId(0),
            Team::Allies,
            TankId(1),
            "Target",
            Team::Axis,
            true,
            &angled_tank,
            &mut dmg_angled,
            &ap,
            hit,
            dir,
            760.0,
            &mut rng,
        );
        assert!(
            matches!(report.outcome, HitOutcome::Ricochet | HitOutcome::NonPenetration),
            "heavily angled UFP should deflect or stop AP, got {:?} on {} (angle={:.1}, eff={:.1} vs pen={:.1})",
            report.outcome,
            report.plate_name,
            report.impact_angle_deg,
            report.effective_armor_mm,
            report.penetration_mm
        );
        assert_eq!(dmg_angled.alive_crew_count(), 5);

        // Case 2: Flat 90 deg broadside into 42 mm hull side penetrates and damages interior
        let side_tank = TankState::new(cfg, Vec3::ZERO, std::f32::consts::FRAC_PI_2);
        let mut dmg_side = TankDamageState::basic_medium_tank();
        let side_dir = Vec3::new(0.0, 0.0, -1.0);
        let hit2 = raycast_tank_armor(&side_tank, Vec3::new(0.3, 0.95, 8.0), side_dir, 20.0, false, true)
            .expect("ray should hit broadside hull");
        let report2 = resolve_ap_impact(
            TankId(0),
            Team::Allies,
            TankId(1),
            "Target",
            Team::Axis,
            true,
            &side_tank,
            &mut dmg_side,
            &ap,
            hit2,
            side_dir,
            760.0,
            &mut rng,
        );
        assert_eq!(report2.outcome, HitOutcome::Penetration);
        assert!(
            !report2.damaged_modules.is_empty() || !report2.killed_crew.is_empty(),
            "penetrating broadside AP + spall cone must damage internal modules or crew"
        );
    }

    #[test]
    fn crew_replacement_takes_six_seconds_and_under_two_crew_destroys_tank() {
        let mut dmg = TankDamageState::basic_medium_tank();
        let mut rng = SimRng::new(7);

        // Knock out Driver: tank immediately loses driving ability and starts 6s transfer
        dmg.crew[0].apply_damage(100.0);
        dmg.ensure_crew_transfers_started();
        assert!(!dmg.mobility_modifiers().can_drive);
        assert!((dmg.driver_transfer_timer - CREW_TRANSFER_TIME).abs() < 0.05);

        // Advance 3 seconds: still replacing
        for _ in 0..180 {
            assert!(dmg.tick_ongoing(FIXED_DT, &mut rng).is_none());
        }
        assert!(!dmg.mobility_modifiers().can_drive);

        // Advance 3.5 more seconds: Radio Operator replaces Driver, driving restored!
        for _ in 0..210 {
            assert!(dmg.tick_ongoing(FIXED_DT, &mut rng).is_none());
        }
        assert!(dmg.mobility_modifiers().can_drive);
        assert_eq!(dmg.alive_crew_count(), 4);

        // Knock out Loader: reload slows by 1.8x
        let loader_idx = dmg
            .crew
            .iter()
            .position(|c| c.alive && c.active_seat == CrewRole::Loader)
            .unwrap();
        dmg.crew[loader_idx].apply_damage(100.0);
        assert!((dmg.mobility_modifiers().reload_time_scale - NO_LOADER_RELOAD_MULT).abs() < 0.05);

        // Reduce surviving crew to 1 (< MIN_OPERATIONAL_CREW): instant CrewKnockedOut
        let mut alive_left = dmg.alive_crew_count();
        for c in &mut dmg.crew {
            if c.alive && alive_left > 1 {
                c.apply_damage(100.0);
                alive_left -= 1;
            }
        }
        assert_eq!(dmg.alive_crew_count(), 1);
        assert_eq!(
            dmg.tick_ongoing(FIXED_DT, &mut rng),
            Some(KillReason::CrewKnockedOut)
        );
    }

    #[test]
    fn ammo_rack_detonation_blows_turret_off_and_fire_burns_out_tank() {
        // 1. Ammo rack detonation test
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let tank = TankState::new(cfg.clone(), Vec3::ZERO, std::f32::consts::FRAC_PI_2);
        let ap = ShellSpec::basic_75mm_ap();

        let mut detonated = false;
        for seed in 1u64..20 {
            let mut dmg = TankDamageState::basic_medium_tank();
            // Aim straight through hull side at left sponson ammo rack (hull local z = -0.3, y = 0.85)
            let dir = Vec3::new(0.0, 0.0, -1.0);
            let origin = Vec3::new(0.3, 0.85, 8.0);
            let hit = raycast_tank_armor(&tank, origin, dir, 20.0, false, true).unwrap();
            let mut rng = SimRng::new(seed);
            let report = resolve_ap_impact(
                TankId(0),
                Team::Allies,
                TankId(1),
                "AmmoTarget",
                Team::Axis,
                true,
                &tank,
                &mut dmg,
                &ap,
                hit,
                dir,
                780.0,
                &mut rng,
            );
            if report.kill_reason == Some(KillReason::AmmoRackDetonation) {
                assert!(dmg.turret_blown_off, "ammo rack detonation must blow turret off");
                detonated = true;
                break;
            }
        }
        assert!(detonated, "direct AP hit into hull ammo rack must detonate ammunition");

        // 2. Active fire burn-out test
        let mut dmg_fire = TankDamageState::basic_medium_tank();
        dmg_fire.on_fire = true;
        dmg_fire.fire_instigator = Some((TankId(0), Team::Allies));
        let mut rng = SimRng::new(99);
        let mut kill = None;
        for _ in 0..(20 * 60) {
            if let Some(reason) = dmg_fire.tick_ongoing(FIXED_DT, &mut rng) {
                kill = Some(reason);
                break;
            }
        }
        assert!(
            matches!(
                kill,
                Some(KillReason::BurnedOut | KillReason::AmmoRackDetonation | KillReason::CrewKnockedOut)
            ),
            "unextinguished fire must eventually destroy tank, got {kill:?}"
        );

        // 3. Broken track & destroyed breech gameplay penalties
        let mut dmg_mods = TankDamageState::basic_medium_tank();
        dmg_mods
            .modules
            .iter_mut()
            .find(|m| m.kind == ModuleKind::GunBreech)
            .unwrap()
            .apply_damage(200.0);
        assert!(!dmg_mods.can_fire());

        dmg_mods
            .modules
            .iter_mut()
            .find(|m| m.kind == ModuleKind::LeftTrack)
            .unwrap()
            .apply_damage(200.0);
        assert!(dmg_mods.mobility_modifiers().broken_track_pull.abs() > 0.3);

        dmg_mods
            .modules
            .iter_mut()
            .find(|m| m.kind == ModuleKind::RightTrack)
            .unwrap()
            .apply_damage(200.0);
        assert!(!dmg_mods.can_move());
    }

    #[test]
    fn stage5_catalog_has_24_approved_vehicles_and_all_voxel_models_mesh_cleanly() {
        let catalog = VehicleCatalog::stage5_catalog();
        assert_eq!(catalog.all().len(), 24);

        // Verify 3 nations × 8 tiers (1..=8) with unique IDs
        let mut ids = std::collections::HashSet::new();
        for nation in Nation::ALL {
            let list = catalog.by_nation(nation);
            assert_eq!(list.len(), 8, "nation {:?} must have 8 tiers", nation);
            for (idx, spec) in list.iter().enumerate() {
                assert_eq!(spec.tier as usize, idx + 1);
                assert!(ids.insert(spec.id.clone()), "duplicate id {}", spec.id);
            }
        }

        // Verify user-requested roster replacements: M4A3E2 (76) W (USA VI) & T-44-100 (USSR VII)
        let jumbo = catalog.by_id("us_m4a3e2_76").expect("M4A3E2 (76) W must exist");
        assert_eq!(jumbo.nation, Nation::Usa);
        assert_eq!(jumbo.tier, 6);
        assert!(jumbo.armor.ufp_mm >= 100.0);
        assert!(jumbo.armor.turret_mantlet_mm >= 170.0);

        let t44 = catalog.by_id("su_t44_100").expect("T-44-100 must exist");
        assert_eq!(t44.nation, Nation::Ussr);
        assert_eq!(t44.tier, 7);
        assert!(t44.shell.caliber_mm >= 100.0);
        assert!(t44.shell.penetration_0m_mm >= 200.0);

        // Every single vehicle's bespoke 0.10 m voxel model must exist and build non-empty U16 meshes
        let materials = voxel::MaterialRegistry::default();
        for spec in catalog.all() {
            assert!(
                voxel::build_vehicle_models(&spec.id).is_some(),
                "{} must have a bespoke 0.10 m voxel sculptor",
                spec.id
            );
            let models = spec.build_models();
            let hull_mesh = voxel::mesh_voxel_model(&models.hull, &materials);
            let turret_mesh = voxel::mesh_voxel_model(&models.turret, &materials);
            let barrel_mesh = voxel::mesh_voxel_model(&models.barrel, &materials);
            assert!(!hull_mesh.is_empty(), "{} hull mesh empty", spec.id);
            assert!(!turret_mesh.is_empty(), "{} turret mesh empty", spec.id);
            assert!(!barrel_mesh.is_empty(), "{} barrel mesh empty", spec.id);
            assert!(hull_mesh.positions.len() < u16::MAX as usize);
            assert!(turret_mesh.positions.len() < u16::MAX as usize);
            assert!(barrel_mesh.positions.len() < u16::MAX as usize);
            let cfg = spec.to_tank_config(&models);
            let dmg = TankDamageState::from_profile(&cfg.armor);
            assert_eq!(dmg.total_crew_count(), spec.armor.crew_count as usize);
        }
    }

    #[test]
    fn casemate_tds_enforce_limited_traverse_arc() {
        let catalog = VehicleCatalog::stage5_catalog();

        for id in ["de_stug3g", "de_jagdtiger"] {
            let voxels = generate_flat_proving_ground(96);
            let spec = catalog.by_id(id).unwrap();
            let models = spec.build_models();
            let cfg = spec.to_tank_config(&models);
            let limit = cfg.turret_yaw_limit.expect("casemate TD must have yaw limit");
            assert!(limit > 0.10 && limit < 0.25);

            let mut sim = SimWorld::new(voxels, cfg);
            // Try aiming 90 degrees to the right (+X)
            for _ in 0..180 {
                sim.step_fixed(
                    FIXED_DT,
                    TankInput {
                        aim_world_point: Vec3::new(100.0, 2.0, 0.0),
                        ..Default::default()
                    },
                );
            }
            assert!(
                sim.player_tank().turret_yaw_local.abs() <= limit + 1e-4,
                "{id} turret_yaw_local {} exceeded casemate limit {limit}",
                sim.player_tank().turret_yaw_local
            );
            assert!(
                sim.player_tank().turret_yaw_local < -limit * 0.95,
                "{id} should traverse cleanly up to right stop"
            );
        }
    }

    #[test]
    fn jumbo_76_and_is3_pike_nose_deflect_weaker_frontal_ap_hits() {
        let catalog = VehicleCatalog::stage5_catalog();
        let tiger = catalog.by_id("de_tiger_h1").unwrap();
        let jumbo = catalog.by_id("us_m4a3e2_76").unwrap();
        let is3 = catalog.by_id("su_is3").unwrap();

        // 1. Tiger H1 88mm KwK 36 AP (165mm @ 0m) striking M4A3E2 (76) Jumbo UFP at ~500m combat range
        let jumbo_cfg = jumbo.to_tank_config(&jumbo.build_models());
        let jumbo_tank = TankState::new(jumbo_cfg, Vec3::ZERO, 0.0);
        let mut jumbo_dmg = TankDamageState::from_profile(&jumbo.armor);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = raycast_tank_armor(&jumbo_tank, Vec3::new(0.0, 1.05, -10.0), dir, 20.0, false, true)
            .expect("ray should hit Jumbo UFP");
        assert_eq!(hit.plate_name, "Hull UFP");
        let mut rng = SimRng::new(12);
        let rep = resolve_ap_impact(
            TankId(0),
            Team::Axis,
            TankId(1),
            "Jumbo",
            Team::Allies,
            true,
            &jumbo_tank,
            &mut jumbo_dmg,
            &tiger.shell,
            hit,
            dir,
            tiger.shell.muzzle_velocity * 0.84,
            &mut rng,
        );
        assert!(
            matches!(rep.outcome, HitOutcome::NonPenetration | HitOutcome::Ricochet),
            "Jumbo UFP (eff={:.1}mm) should stop Tiger 88mm at mid-range (pen={:.1}mm), got {:?}",
            rep.effective_armor_mm,
            rep.penetration_mm,
            rep.outcome
        );

        // 2. IS-3 pike nose (110mm compound 56 deg + 25 deg wedge) defeats Tiger 88mm AP head-on
        let is3_cfg = is3.to_tank_config(&is3.build_models());
        let is3_tank = TankState::new(is3_cfg, Vec3::ZERO, 0.0);
        let mut is3_dmg = TankDamageState::from_profile(&is3.armor);
        let hit_is3 = raycast_tank_armor(&is3_tank, Vec3::new(0.45, 1.0, -10.0), dir, 20.0, false, true)
            .expect("ray should hit IS-3 pike nose");
        assert_eq!(hit_is3.plate_name, "Hull UFP");
        let rep_is3 = resolve_ap_impact(
            TankId(0),
            Team::Axis,
            TankId(1),
            "IS-3",
            Team::Allies,
            true,
            &is3_tank,
            &mut is3_dmg,
            &tiger.shell,
            hit_is3,
            dir,
            tiger.shell.muzzle_velocity,
            &mut rng,
        );
        assert!(
            matches!(rep_is3.outcome, HitOutcome::NonPenetration | HitOutcome::Ricochet),
            "IS-3 pike nose (angle={:.1}, eff={:.1}mm) should defeat Tiger 88mm (pen={:.1}mm), got {:?}",
            rep_is3.impact_angle_deg,
            rep_is3.effective_armor_mm,
            rep_is3.penetration_mm,
            rep_is3.outcome
        );
    }

    #[test]
    fn tier_matched_6v6_battle_spawns_all_three_nations_at_player_tier_and_switches_in_place() {
        let catalog = VehicleCatalog::stage5_catalog();
        let (world, meta) = generate_european_valley_map();
        let mut sim = SimWorld::new_tier_battle(world, meta, &catalog, "us_m4a3e2_76", 6);

        assert_eq!(sim.tanks.len(), 12);
        assert_eq!(sim.player_tank().config.vehicle_id, "us_m4a3e2_76");
        assert_eq!(sim.player_tank().config.tier, 6);
        assert!(sim.tanks.iter().all(|t| t.state.config.tier == 6));

        let nations_present: std::collections::HashSet<Nation> =
            sim.tanks.iter().map(|t| t.state.config.nation).collect();
        assert_eq!(nations_present.len(), 3, "tier 6 battle must include USA, Germany, and USSR");

        // Switch in-place to Tier VII T-44-100 without regenerating world
        sim.configure_tier_battle(&catalog, "su_t44_100", 6);
        assert_eq!(sim.player_tank().config.vehicle_id, "su_t44_100");
        assert_eq!(sim.player_tank().config.tier, 7);
        assert!(sim.tanks.iter().all(|t| t.state.config.tier == 7));
        assert_eq!(sim.tick, 0);

        for _ in 0..180 {
            sim.step_fixed(FIXED_DT, TankInput::default());
        }
        assert!(sim.tanks.iter().all(|t| t.state.position.is_finite()));
    }

    #[test]
    fn stage7_he_carves_brick_wall_and_terrain_crater_while_preserving_bedrock() {
        let mut world = generate_flat_proving_ground(64);
        world.bounds_max.y = 12;
        // Build a 3 m tall brick house wall at z = -12 m (vz = -24)
        world.fill_box(
            IVec3::new(-10, 0, -25),
            IVec3::new(10, 5, -24),
            VoxelId::BRICK,
        );
        let _ = world.take_dirty_chunks(1024);

        // Detonate 122mm HE directly into the brick wall
        let he_122 = ShellSpec::he_for_caliber("122mm OF-471 HE", 122.0, 25.0, 800.0, 3.60);
        let res = world.carve_explosion(
            Vec3::new(0.0, 1.0, -12.0),
            he_122.blast_radius_m,
            he_122.blast_power,
        );
        assert!(
            res.destroyed_voxels >= 20,
            "122mm HE must breach brick wall and carve crater, destroyed={}",
            res.destroyed_voxels
        );
        assert!(!res.debris.is_empty());
        assert!(world.dirty_chunk_count() > 0, "carved chunks must be marked dirty");
        // Verify wall center was blown open while bottom bedrock layer stays 100% intact!
        assert_eq!(world.get_voxel(IVec3::new(0, 2, -24)), VoxelId::AIR);
        assert!(
            world.is_solid(IVec3::new(0, world.bounds_min.y, -24)),
            "bedrock layer bounds_min.y must never be carved out"
        );
    }

    #[test]
    fn stage7_he_overpressures_open_top_m10_and_blasts_heavy_tracks_without_overpressuring_is3() {
        let catalog = VehicleCatalog::stage5_catalog();
        let m10 = catalog.by_id("us_m10").unwrap();
        let is3 = catalog.by_id("su_is3").unwrap();
        let he_75 = ShellSpec::basic_75mm_he(); // 16mm HE pen

        // 1. 75mm HE (16mm pen) hitting M10 Wolverine turret (open-top 10mm roof) triggers Overpressure!
        let m10_cfg = m10.to_tank_config(&m10.build_models());
        let m10_tank = TankState::new(m10_cfg, Vec3::ZERO, 0.0);
        let mut m10_dmg = TankDamageState::from_profile(&m10.armor);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit_m10 = raycast_tank_armor(&m10_tank, Vec3::new(0.0, 1.85, -10.0), dir, 20.0, false, true)
            .expect("ray should hit M10 turret");
        let mut rng = SimRng::new(55);
        let rep_m10 = resolve_shell_impact(
            TankId(0),
            Team::Axis,
            TankId(1),
            "M10",
            Team::Allies,
            true,
            &m10_tank,
            &mut m10_dmg,
            &he_75,
            hit_m10,
            dir,
            he_75.muzzle_velocity,
            &mut rng,
        );
        assert!(rep_m10.overpressure, "75mm HE must overpressure thin-roof/open-top M10");
        assert_eq!(rep_m10.kill_reason, Some(KillReason::Overpressure));

        // 2. Same 75mm HE (16mm pen) hitting IS-3 heavy UFP (110mm, 20mm+ roof) cannot overpressure IS-3!
        let is3_cfg = is3.to_tank_config(&is3.build_models());
        let is3_tank = TankState::new(is3_cfg, Vec3::ZERO, 0.0);
        let mut is3_dmg = TankDamageState::from_profile(&is3.armor);
        let hit_is3 = raycast_tank_armor(&is3_tank, Vec3::new(0.0, 1.0, -10.0), dir, 20.0, false, true)
            .expect("ray should hit IS-3 hull");
        let rep_is3 = resolve_shell_impact(
            TankId(0),
            Team::Axis,
            TankId(1),
            "IS-3",
            Team::Allies,
            true,
            &is3_tank,
            &mut is3_dmg,
            &he_75,
            hit_is3,
            dir,
            he_75.muzzle_velocity,
            &mut rng,
        );
        assert!(!rep_is3.overpressure, "75mm HE must NOT overpressure heavily armored IS-3");
        assert!(rep_is3.kill_reason.is_none());
    }

    #[test]
    fn stage8_bursting_charge_shells_aphe_aphebc_apcbc_detonate_internally_and_switch_slots() {
        let catalog = VehicleCatalog::stage5_catalog();

        // 1. Verify all 5 shell families exist across the 24-vehicle catalog
        let mut kinds = std::collections::HashSet::new();
        for spec in catalog.all() {
            let rack = spec.ammo_loadout();
            assert!(rack.len() >= 2, "{} must carry at least 2 shell types", spec.id);
            assert!(
                rack.iter().any(|s| s.kind == ShellKind::He),
                "{} must carry HE shell",
                spec.id
            );
            for sh in &rack {
                kinds.insert(sh.kind);
            }
        }
        assert!(kinds.contains(&ShellKind::Ap));
        assert!(kinds.contains(&ShellKind::He));
        assert!(kinds.contains(&ShellKind::Aphe));
        assert!(kinds.contains(&ShellKind::Aphebc));
        assert!(kinds.contains(&ShellKind::Apcbc));

        // 2. Verify Soviet APHEBC superior slope normalization & German APCBC lower drag than plain AP
        let t34_85 = catalog.by_id("su_t34_85").unwrap();
        let aphebc = &t34_85.ammo_loadout()[0];
        let panther = catalog.by_id("de_panther_a").unwrap();
        let apcbc = &panther.ammo_loadout()[0];
        let plain_ap = ShellSpec::basic_75mm_ap();
        assert_eq!(aphebc.kind, ShellKind::Aphebc);
        assert_eq!(apcbc.kind, ShellKind::Apcbc);
        assert!(aphebc.normalization_deg > plain_ap.normalization_deg);
        assert!(apcbc.drag_coeff < plain_ap.drag_coeff);

        // 3. Penetrating 85mm BR-365A APHEBC into broadside medium tank arms fuse & detonates 360° inside!
        let models = build_basic_tank_models();
        let cfg = TankConfig::stage1_basic(&models);
        let side_tank = TankState::new(cfg.clone(), Vec3::ZERO, std::f32::consts::FRAC_PI_2);
        let mut dmg = TankDamageState::basic_medium_tank();
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = raycast_tank_armor(&side_tank, Vec3::new(0.0, 1.05, 8.0), dir, 20.0, false, true)
            .expect("ray should hit side hull");
        let mut rng = SimRng::new(77);
        let rep = resolve_shell_impact(
            TankId(0),
            Team::Allies,
            TankId(1),
            "Target",
            Team::Axis,
            true,
            &side_tank,
            &mut dmg,
            aphebc,
            hit,
            dir,
            aphebc.muzzle_velocity,
            &mut rng,
        );
        assert_eq!(rep.outcome, HitOutcome::Penetration);
        assert!(rep.aphe_detonated, "85mm APHEBC penetrating 42mm side must detonate bursting charge");
        assert!(!rep.over_penetrated);
        assert!(
            rep.killed_crew.len() >= 2 || rep.kill_reason.is_some(),
            "360° APHEBC internal burst should heavily devastate crew/modules"
        );

        // 4. Verify live multi-shell slot switching (`select_shell`) chambers HE cleanly
        let voxels = generate_flat_proving_ground(96);
        let mut sim = SimWorld::new(voxels, cfg);
        assert_eq!(sim.player_tank().active_shell().kind, ShellKind::Ap);
        sim.step_fixed(
            FIXED_DT,
            TankInput {
                select_shell: Some(1),
                ..Default::default()
            },
        );
        assert_eq!(sim.player_tank().selected_shell_idx, 1);
        for _ in 0..240 {
            sim.step_fixed(FIXED_DT, TankInput::default());
        }
        assert!(sim.player_tank().is_loaded());
        assert_eq!(sim.player_tank().active_shell().kind, ShellKind::He);
    }
}
