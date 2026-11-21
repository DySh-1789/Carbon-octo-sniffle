use bevy::prelude::*;
use sim::{FIXED_DT, SimWorld, VehicleCatalog, generate_european_valley_map};

use crate::bridge::{BattleControlResource, PendingSimEvents, PlayerInputResource, SimResource};
use crate::state::{AppState, CatalogResource, SelectedVehicle};

/// Headless-compatible simulation plugin (runs with `MinimalPlugins` + `StatesPlugin` for automated tests).
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        let catalog = VehicleCatalog::stage5_catalog();
        let selected = SelectedVehicle::default();
        let (world, metadata) = generate_european_valley_map();
        let sim = SimWorld::new_tier_battle(world, metadata, &catalog, &selected.vehicle_id, 6);

        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(CatalogResource(catalog))
            .insert_resource(selected)
            .insert_resource(SimResource(sim))
            .init_resource::<PlayerInputResource>()
            .init_resource::<BattleControlResource>()
            .init_resource::<PendingSimEvents>()
            .add_systems(
                FixedUpdate,
                step_simulation.run_if(in_state(AppState::InBattle)),
            )
            .add_systems(
                Update,
                handle_return_to_hangar.run_if(in_state(AppState::InBattle)),
            )
            .add_systems(
                PostUpdate,
                clear_pending_sim_events.run_if(in_state(AppState::InBattle)),
            );
    }
}

/// Interactive keyboard/mouse input plugin for driving, firing, restarting, and returning to Hangar.
pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            gather_player_driving_and_fire_input.run_if(in_state(AppState::InBattle)),
        );
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
    if (battle_over || !player_alive) && keys.just_pressed(KeyCode::KeyH) {
        control.return_to_hangar = true;
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

    if keys.just_pressed(KeyCode::Digit1) {
        input.select_shell = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) {
        input.select_shell = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) {
        input.select_shell = Some(2);
    }

    if mouse_buttons.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Space) {
        input.fire_queued = true;
    }
}

fn handle_return_to_hangar(
    mut control: ResMut<BattleControlResource>,
    mut events: ResMut<PendingSimEvents>,
    mut selected: ResMut<SelectedVehicle>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if control.return_to_hangar {
        control.return_to_hangar = false;
        control.restart_requested = false;
        events.clear_frame_events();
        selected.preview_dirty = true;
        next_state.set(AppState::Hangar);
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
        events.clear_frame_events();
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
    if !sim.0.recent_hit_reports.is_empty() {
        events.hit_reports.append(&mut sim.0.recent_hit_reports);
    }
    if !sim.0.recent_kills.is_empty() {
        events.kills.append(&mut sim.0.recent_kills);
    }
}

fn clear_pending_sim_events(mut events: ResMut<PendingSimEvents>) {
    events.clear_frame_events();
}
