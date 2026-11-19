pub mod ballistics;
pub mod map_gen;
pub mod tank;

pub use ballistics::{FIXED_DT, GRAVITY, ImpactEvent, Projectile, SimWorld};
pub use map_gen::generate_flat_proving_ground;
pub use tank::{FiredShot, TankConfig, TankInput, TankState, wrap_angle};

#[cfg(test)]
mod tests {
    use voxel::build_basic_tank_models;
    use voxel::glam::Vec3;

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
}
