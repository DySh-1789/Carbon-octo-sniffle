use bevy::prelude::*;

use crate::bridge::{SimResource, to_bevy_vec3};
use crate::camera::{CameraMode, MainTankCamera, TankCameraController};

#[derive(Component)]
struct TelemetryText;

#[derive(Component)]
struct ReloadFillBar;

#[derive(Component)]
struct GunReticleMarker;

#[derive(Component)]
struct SniperOverlayRoot;

#[derive(Component)]
struct CursorHintText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, (update_hud_telemetry, update_hud_reticles));
    }
}

fn setup_hud(mut commands: Commands) {
    // 1. Center camera crosshair (always centered on screen)
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|root| {
            // Horizontal crosshair bar
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(22.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            ));
            // Vertical crosshair bar
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(2.0),
                    height: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            ));
        });

    // 2. Sniper gunner optic reticle overlay (shown only in Sniper mode)
    commands
        .spawn((
            SniperOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|root| {
            // Long horizontal stadia line
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(340.0),
                    height: Val::Px(1.5),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            ));
            // Vertical ballistic drop stem
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(1.5),
                    height: Val::Px(220.0),
                    top: Val::Percent(50.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            ));
            // Ballistic range ticks (200m, 400m, 600m visual reference bars)
            for (offset_px, width_px) in [(24.0, 38.0), (54.0, 28.0), (92.0, 20.0), (138.0, 14.0)] {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(width_px),
                        height: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(offset_px * 2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.95, 0.85, 0.25, 0.9)),
                ));
            }
        });

    // 3. Trailing actual gun-bore aiming indicator (projected from 3D gun ray)
    commands.spawn((
        GunReticleMarker,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(18.0),
            height: Val::Px(18.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.25, 0.95, 0.35, 0.9)),
        BackgroundColor(Color::NONE),
    ));

    // 4. Top-left compact controls help
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(14.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.72)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("STAGE 1 | PROVING GROUND"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.82, 0.40)),
            ));
            panel.spawn((
                Text::new("WASD: Drive | Mouse: Aim | LMB/Space: Fire | Shift: Sniper | C: FreeLook | Scroll: Zoom | Esc: Cursor"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.88, 0.84)),
            ));
            panel.spawn((
                CursorHintText,
                Text::new("Click window to lock mouse aiming"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.68, 0.25)),
            ));
        });

    // 5. Bottom-left War Thunder-style tank telemetry & reload panel
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(260.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.78)),
        ))
        .with_children(|panel| {
            panel.spawn((
                TelemetryText,
                Text::new("SPD   0 km/h | GEAR N\nGUN   READY\nVIEW  3RD PERSON"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.95, 0.90)),
            ));

            // Reload progress bar background
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.22, 0.24, 0.22, 0.9)),
                ))
                .with_children(|bar_bg| {
                    bar_bg.spawn((
                        ReloadFillBar,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.28, 0.88, 0.36)),
                    ));
                });
        });
}

#[allow(clippy::type_complexity)]
fn update_hud_telemetry(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TelemetryText>>,
        Query<&mut Text, With<CursorHintText>>,
    )>,
    mut reload_bar_q: Query<(&mut Node, &mut BackgroundColor), With<ReloadFillBar>>,
) {
    let tank = &sim.0.player_tank;

    if let Ok(mut text) = texts.p0().single_mut() {
        let reload_str = if tank.is_loaded() {
            "READY".to_string()
        } else {
            format!("RELOAD {:.1}s", tank.reload_remaining)
        };
        let view_str = match cam_ctrl.mode {
            CameraMode::ThirdPerson if cam_ctrl.free_look => "FREE LOOK",
            CameraMode::ThirdPerson => "3RD PERSON",
            CameraMode::Sniper => "GUNNER OPTIC",
        };
        let turret_deg = tank.turret_yaw_local.to_degrees();
        let elev_deg = tank.gun_pitch_local.to_degrees();
        text.0 = format!(
            "SPD {:>3.0} km/h | GEAR {}\nTUR {:>+4.0} deg | ELV {:>+4.1} deg\nGUN {} | SHOTS {}\nVIEW {}",
            tank.speed_kmh().abs(),
            tank.gear_label(),
            turret_deg,
            elev_deg,
            reload_str,
            tank.shots_fired,
            view_str,
        );
    }

    if let Ok(mut hint) = texts.p1().single_mut() {
        hint.0 = if cam_ctrl.cursor_locked {
            "Mouse locked (press Esc to unlock cursor)".to_string()
        } else {
            "Click window to lock mouse aiming".to_string()
        };
    }

    if let Ok((mut node, mut bg)) = reload_bar_q.single_mut() {
        node.width = Val::Percent(tank.reload_progress() * 100.0);
        bg.0 = if tank.is_loaded() {
            Color::srgb(0.28, 0.88, 0.36)
        } else {
            Color::srgb(0.92, 0.62, 0.16)
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_hud_reticles(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainTankCamera>>,
    mut reticle_nodes: ParamSet<(
        Query<(&mut Node, &mut BorderColor), With<GunReticleMarker>>,
        Query<&mut Node, With<SniperOverlayRoot>>,
    )>,
) {
    let tank = &sim.0.player_tank;

    if let Ok(mut overlay) = reticle_nodes.p1().single_mut() {
        overlay.display = if cam_ctrl.mode == CameraMode::Sniper {
            Display::Flex
        } else {
            Display::None
        };
    }

    // Project actual gun barrel aim point to screen space for the trailing bore indicator
    if let (Ok((camera, cam_gtf)), Ok((mut reticle_node, mut reticle_border))) =
        (camera_q.single(), reticle_nodes.p0().single_mut())
    {
        let gun_target = to_bevy_vec3(tank.actual_gun_aim_point(&sim.0.voxels, 400.0));
        if let Ok(viewport_pos) = camera.world_to_viewport(cam_gtf, gun_target) {
            reticle_node.display = Display::Flex;
            reticle_node.left = Val::Px(viewport_pos.x - 9.0);
            reticle_node.top = Val::Px(viewport_pos.y - 9.0);
            *reticle_border = BorderColor::all(if tank.is_loaded() {
                Color::srgba(0.25, 0.95, 0.35, 0.9)
            } else {
                Color::srgba(0.95, 0.62, 0.18, 0.9)
            });
        } else {
            reticle_node.display = Display::None;
        }
    }
}
