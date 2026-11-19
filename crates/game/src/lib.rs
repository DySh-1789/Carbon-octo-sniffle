pub mod battle_sync;
pub mod bridge;
pub mod camera;
pub mod effects;
pub mod hud;
pub mod tank_view;
pub mod voxel_render;

use bevy::prelude::*;

use crate::battle_sync::{BattleSimPlugin, PlayerInputPlugin};
use crate::camera::CameraPlugin;
use crate::effects::EffectsPlugin;
use crate::hud::HudPlugin;
use crate::tank_view::TankViewPlugin;
use crate::voxel_render::VoxelRenderPlugin;

/// Complete interactive game plugin bundle.
pub struct TankGamePlugin;

impl Plugin for TankGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BattleSimPlugin,
            PlayerInputPlugin,
            VoxelRenderPlugin,
            TankViewPlugin,
            CameraPlugin,
            EffectsPlugin,
            HudPlugin,
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::app::ScheduleRunnerPlugin;
    use bevy::prelude::*;
    use bevy::time::TimeUpdateStrategy;

    use crate::battle_sync::BattleSimPlugin;
    use crate::bridge::{
        BattleControlResource, PendingSimEvents, PlayerInputResource, SimResource,
    };
    use sim::Team;
    use voxel::glam::Vec3 as SimVec3;

    #[test]
    fn headless_bevy_ecs_smoke_test_runs_6v6_battle_drives_fires_and_restarts() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            BattleSimPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));

        let start_z = {
            let sim = app.world().resource::<SimResource>();
            assert_eq!(sim.0.tanks.len(), 12);
            assert_eq!(sim.0.alive_count(Team::Allies), 6);
            assert_eq!(sim.0.alive_count(Team::Axis), 6);
            sim.0.player_tank().position.z
        };

        // Command player tank to drive north along main road and fire on tick 1
        {
            let mut input = app.world_mut().resource_mut::<PlayerInputResource>();
            input.throttle = 1.0;
            input.aim_world_point = SimVec3::new(0.0, 2.0, 22.0);
            input.fire_queued = true;
        }

        // Advance 90 frames (1.5 seconds of deterministic FixedUpdate ticks)
        for _ in 0..90 {
            app.update();
        }

        {
            let sim = app.world().resource::<SimResource>();
            assert_eq!(sim.0.map.id, "european_valley");
            assert_eq!(sim.0.map.capture_zones.len(), 3);
            assert!(sim.0.tick >= 60, "expected >= 60 fixed ticks, got {}", sim.0.tick);
            assert!(
                sim.0.player_tank().position.z < start_z - 2.0,
                "tank should have driven forward along -Z on European Valley, start_z={start_z}, pos={:?}",
                sim.0.player_tank().position
            );
            assert_eq!(sim.0.player_tank().shots_fired, 1);

            let events = app.world().resource::<PendingSimEvents>();
            assert!(!events.fired_shots.is_empty());
            assert!(!events.battle_restarted);
        }

        // Trigger battle restart via BattleControlResource and verify reset
        app.world_mut()
            .resource_mut::<BattleControlResource>()
            .restart_requested = true;
        app.update();

        let sim = app.world().resource::<SimResource>();
        assert_eq!(sim.0.tick, 0);
        assert_eq!(sim.0.player_tank().shots_fired, 0);
        assert_eq!(sim.0.alive_count(Team::Allies), 6);
        assert_eq!(sim.0.alive_count(Team::Axis), 6);
    }

    #[test]
    fn full_game_plugins_initialize_and_step_without_ecs_conflicts() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            bevy::input::InputPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Image>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(super::TankGamePlugin);

        for _ in 0..8 {
            app.update();
        }

        // Verify restart lifecycle across EffectsPlugin, HudPlugin, CameraPlugin & TankViewPlugin
        app.world_mut()
            .resource_mut::<BattleControlResource>()
            .restart_requested = true;
        app.update();
        app.update();

        let sim = app.world().resource::<SimResource>();
        assert_eq!(sim.0.tanks.len(), 12);
    }
}
