use bevy::prelude::*;
use sim::{FIXED_DT, SimWorld, TankConfig, generate_european_valley_map};
use voxel::build_basic_tank_models;

use crate::bridge::{BattleControlResource, PendingSimEvents, PlayerInputResource, SimResource};
use crate::tank_view::TankModelsResource;

/// Headless-compatible simulation plugin (runs with `MinimalPlugins` for automated tests).
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        let models = build_basic_tank_models();
        let tank_cfg = TankConfig::stage1_basic(&models);
        let (world, metadata) = generate_european_valley_map();
        let sim = SimWorld::new_team_battle(world, metadata, tank_cfg, 6);

        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(TankModelsResource(models))
            .insert_resource(SimResource(sim))
            .init_resource::<PlayerInputResource>()
            .init_resource::<BattleControlResource>()
            .init_resource::<PendingSimEvents>()
            .add_systems(FixedUpdate, step_simulation)
            .add_systems(PostUpdate, clear_battle_restarted_flag);
    }
}

/// Interactive keyboard/mouse input plugin for driving, firing, and restarting after battle end.
pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, gather_player_driving_and_fire_input);
    }
}

fn gather_player_driving_and_fire_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    sim: Res<SimResource>,
    mut input: ResMut<PlayerInputResource>,
    mut control: ResMut<BattleControlResource>,
) {
    let battle_over = sim.0.battle.is_over();
    let player_alive = sim.0.player_unit().alive;

    if (battle_over || !player_alive) && keys.just_pressed(KeyCode::Enter) {
        control.restart_requested = true;
    }

    if battle_over || !player_alive {
        input.throttle = 0.0;
        input.steer = 0.0;
        input.fire_queued = false;
        return;
    }

    let mut throttle = 0.0f32;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        throttle += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        throttle -= 1.0;
    }

    let mut steer = 0.0f32;
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        steer += 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        steer -= 1.0;
    }

    input.throttle = throttle;
    input.steer = steer;

    if mouse_buttons.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Space) {
        input.fire_queued = true;
    }
}

fn step_simulation(
    mut sim: ResMut<SimResource>,
    mut input: ResMut<PlayerInputResource>,
    mut control: ResMut<BattleControlResource>,
    mut events: ResMut<PendingSimEvents>,
) {
    if control.restart_requested {
        control.restart_requested = false;
        input.take_for_step();
        sim.0.restart_battle();
        events.fired_shots.clear();
        events.impacts.clear();
        events.tank_hits.clear();
        events.kills.clear();
        events.battle_restarted = true;
        return;
    }

    let snap = input.take_for_step();
    sim.0.step_fixed(FIXED_DT, snap);

    if !sim.0.recent_shots.is_empty() {
        events.fired_shots.append(&mut sim.0.recent_shots);
    }
    if !sim.0.recent_impacts.is_empty() {
        events.impacts.append(&mut sim.0.recent_impacts);
    }
    if !sim.0.recent_tank_hits.is_empty() {
        events.tank_hits.append(&mut sim.0.recent_tank_hits);
    }
    if !sim.0.recent_kills.is_empty() {
        events.kills.append(&mut sim.0.recent_kills);
    }
}

fn clear_battle_restarted_flag(mut events: ResMut<PendingSimEvents>) {
    events.battle_restarted = false;
}
