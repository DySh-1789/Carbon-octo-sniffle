use bevy::prelude::*;
use sim::{FIXED_DT, SimWorld, TankConfig, generate_european_valley_map};
use voxel::build_basic_tank_models;

use crate::bridge::{PendingSimEvents, PlayerInputResource, SimResource};
use crate::tank_view::TankModelsResource;

/// Headless-compatible simulation plugin (runs with `MinimalPlugins` for automated tests).
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        let models = build_basic_tank_models();
        let tank_cfg = TankConfig::stage1_basic(&models);
        let (world, metadata) = generate_european_valley_map();
        let sim = SimWorld::with_map(world, metadata, tank_cfg);

        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(TankModelsResource(models))
            .insert_resource(SimResource(sim))
            .init_resource::<PlayerInputResource>()
            .init_resource::<PendingSimEvents>()
            .add_systems(FixedUpdate, step_simulation);
    }
}

/// Interactive keyboard/mouse input plugin for driving and firing.
pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, gather_player_driving_and_fire_input);
    }
}

fn gather_player_driving_and_fire_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut input: ResMut<PlayerInputResource>,
) {
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
    mut events: ResMut<PendingSimEvents>,
) {
    let snap = input.take_for_step();
    sim.0.step_fixed(FIXED_DT, snap);

    if let Some(shot) = sim.0.last_shot.take() {
        events.fired_shots.push(shot);
    }
    if !sim.0.recent_impacts.is_empty() {
        events.impacts.append(&mut sim.0.recent_impacts);
    }
}
