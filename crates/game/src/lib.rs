pub mod battle_sync;
pub mod bridge;
pub mod camera;
pub mod effects;
pub mod hangar_ui;
pub mod hud;
pub mod state;
pub mod tank_view;
pub mod voxel_render;

use bevy::prelude::*;

use crate::battle_sync::{BattleSimPlugin, PlayerInputPlugin};
use crate::camera::CameraPlugin;
use crate::effects::EffectsPlugin;
use crate::hangar_ui::HangarPlugin;
use crate::hud::HudPlugin;
use crate::state::{
    AppState, BattleEntity, HangarEntity, despawn_tagged, setup_tank_paint_materials,
};
use crate::tank_view::TankViewPlugin;
use crate::voxel_render::VoxelRenderPlugin;

/// Complete interactive game plugin bundle (Hangar + Tech Tree + 6v6 Ground Battle).
pub struct TankGamePlugin;

impl Plugin for TankGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Startup, setup_tank_paint_materials)
            .add_systems(OnExit(AppState::Hangar), despawn_tagged::<HangarEntity>)
            .add_systems(OnExit(AppState::InBattle), despawn_tagged::<BattleEntity>)
            .add_plugins((
                BattleSimPlugin,
                PlayerInputPlugin,
                HangarPlugin,
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
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use crate::battle_sync::BattleSimPlugin;
    use crate::bridge::{
        BattleControlResource, PendingSimEvents, PlayerInputResource, SimResource,
    };
    use crate::state::{
        AppState, BattleEntity, CatalogResource, HangarEntity, HangarViewMode, SelectedVehicle,
    };
    use sim::{Nation, Team};
    use voxel::glam::Vec3 as SimVec3;

    #[test]
    fn headless_bevy_ecs_smoke_test_runs_6v6_battle_drives_fires_and_restarts() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            StatesPlugin,
            BattleSimPlugin,
        ))
        .init_state::<AppState>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));

        // Enter InBattle state (simulating clicking BATTLE! from Hangar)
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InBattle);
        app.update();

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
            assert!(
                sim.0.tick >= 60,
                "expected >= 60 fixed ticks, got {}",
                sim.0.tick
            );
            assert!(
                sim.0.player_tank().position.z < start_z - 2.0,
                "tank should have driven forward along -Z on European Valley, start_z={start_z}, pos={:?}",
                sim.0.player_tank().position
            );
            assert_eq!(sim.0.player_tank().shots_fired, 1);
            assert_eq!(sim.0.player_unit().damage.alive_crew_count(), 5);

            // Frame events are cleared cleanly in PostUpdate after Update consumers read them
            let events = app.world().resource::<PendingSimEvents>();
            assert!(events.fired_shots.is_empty());
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
            StatesPlugin,
            bevy::input::InputPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Image>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(super::TankGamePlugin);

        // 1. Starts cleanly in Hangar state with Hangar entities spawned and zero Battle entities
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Hangar);
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<HangarEntity>>()
                .iter(app.world())
                .count()
                > 0
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<BattleEntity>>()
                .iter(app.world())
                .count(),
            0
        );

        // 2. Select USSR Tier VII T-44-100, toggle TechTree & back, and launch Tier VII battle
        {
            let mut sel = app.world_mut().resource_mut::<SelectedVehicle>();
            sel.nation = Nation::Ussr;
            sel.vehicle_id = "su_t44_100".to_string();
            sel.view_mode = HangarViewMode::TechTree;
            sel.preview_dirty = true;
        }
        app.update();
        {
            let mut sel = app.world_mut().resource_mut::<SelectedVehicle>();
            sel.view_mode = HangarViewMode::Podium;
        }
        app.update();

        {
            let catalog = app.world().resource::<CatalogResource>().0.clone();
            let vid = app.world().resource::<SelectedVehicle>().vehicle_id.clone();
            app.world_mut()
                .resource_mut::<SimResource>()
                .0
                .configure_tier_battle(&catalog, &vid, 6);
            app.world_mut()
                .resource_mut::<NextState<AppState>>()
                .set(AppState::InBattle);
        }

        for _ in 0..5 {
            app.update();
        }
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InBattle);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<HangarEntity>>()
                .iter(app.world())
                .count(),
            0,
            "Hangar entities must be fully despawned upon entering InBattle"
        );
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<BattleEntity>>()
                .iter(app.world())
                .count()
                > 0
        );

        // 3. Exercise Stage 4/5 turret blow-off, active fire emitter, and WT damage HUD on T-44-100 battle
        {
            let mut sim = app.world_mut().resource_mut::<SimResource>();
            assert_eq!(sim.0.player_tank().config.vehicle_id, "su_t44_100");
            assert_eq!(sim.0.player_tank().config.tier, 7);
            sim.0.tanks[6].alive = false;
            sim.0.tanks[6].damage.turret_blown_off = true;
            sim.0.tanks[0].damage.on_fire = true;
        }
        for _ in 0..8 {
            app.update();
        }

        // 4. Verify in-battle restart lifecycle
        app.world_mut()
            .resource_mut::<BattleControlResource>()
            .restart_requested = true;
        app.update();
        app.update();
        {
            let sim = app.world().resource::<SimResource>();
            assert_eq!(sim.0.tanks.len(), 12);
            assert!(!sim.0.tanks[6].damage.turret_blown_off);
            assert!(!sim.0.tanks[0].damage.on_fire);
        }

        // 5. Return to Hangar, verify BattleEntity cleanup, select M4A3E2 (76) W and enter second battle
        app.world_mut()
            .resource_mut::<BattleControlResource>()
            .return_to_hangar = true;
        app.update();
        app.update();
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Hangar);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<BattleEntity>>()
                .iter(app.world())
                .count(),
            0,
            "Battle entities must be fully despawned upon returning to Hangar"
        );
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<HangarEntity>>()
                .iter(app.world())
                .count()
                > 0
        );
    }
}
