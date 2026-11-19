use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::bridge::{SimResource, to_bevy_vec3};
use crate::camera::{CameraMode, MainTankCamera, TankCameraController};

const MINIMAP_DISPLAY_PX: f32 = 192.0;

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

#[derive(Component)]
struct MinimapPlayerMarker;

#[derive(Component)]
struct MinimapHullHeadingPip;

#[derive(Component)]
struct MinimapGunHeadingPip;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_hud, setup_minimap_hud))
            .add_systems(
                Update,
                (update_hud_telemetry, update_hud_reticles, update_hud_minimap),
            );
    }
}

fn setup_hud(mut commands: Commands, sim: Res<SimResource>) {
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
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(22.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            ));
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
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(340.0),
                    height: Val::Px(1.5),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            ));
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

    // 4. Top-left compact stage & controls header
    let map_title = sim.0.map.name.to_uppercase();
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
            BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.74)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(format!("STAGE 2 | {map_title}")),
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
                width: Val::Px(286.0),
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
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.95, 0.90)),
            ));

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

fn setup_minimap_hud(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut images: ResMut<Assets<Image>>,
) {
    let minimap = &sim.0.map.minimap;
    let image = Image::new(
        Extent3d {
            width: minimap.size,
            height: minimap.size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        minimap.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let minimap_handle = images.add(image);

    let map_header = format!(
        "{} | {:.0}m",
        sim.0.map.name.to_uppercase(),
        sim.0.map.size_m
    );

    // Bottom-right tactical minimap frame
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.65, 0.68, 0.60, 0.55)),
            BackgroundColor(Color::srgba(0.06, 0.08, 0.07, 0.84)),
        ))
        .with_children(|frame| {
            frame.spawn((
                Text::new(map_header),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.84, 0.58)),
            ));

            frame
                .spawn((
                    Node {
                        width: Val::Px(MINIMAP_DISPLAY_PX),
                        height: Val::Px(MINIMAP_DISPLAY_PX),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.85, 0.88, 0.82, 0.45)),
                    ImageNode::new(minimap_handle),
                ))
                .with_children(|map_box| {
                    // Tactical capture zone markers (A / B / C)
                    for zone in &sim.0.map.capture_zones {
                        let uv = sim.0.map.world_to_minimap_uv(zone.center);
                        let px = (uv.x * MINIMAP_DISPLAY_PX - 9.0)
                            .clamp(0.0, MINIMAP_DISPLAY_PX - 18.0);
                        let py = (uv.y * MINIMAP_DISPLAY_PX - 9.0)
                            .clamp(0.0, MINIMAP_DISPLAY_PX - 18.0);

                        map_box
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(px),
                                    top: Val::Px(py),
                                    width: Val::Px(18.0),
                                    height: Val::Px(18.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgba(0.95, 0.85, 0.28, 0.95)),
                                BackgroundColor(Color::srgba(0.08, 0.10, 0.09, 0.78)),
                            ))
                            .with_children(|badge| {
                                badge.spawn((
                                    Text::new(zone.label.to_string()),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.98, 0.90, 0.42)),
                                ));
                            });
                    }

                    // Gun / turret direction indicator pip (gold)
                    map_box.spawn((
                        MinimapGunHeadingPip,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(4.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 0.88, 0.25)),
                    ));

                    // Hull forward direction pip (lime)
                    map_box.spawn((
                        MinimapHullHeadingPip,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(4.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.75, 1.0, 0.65)),
                    ));

                    // Player tank hull center dot (bright green with crisp dark border)
                    map_box.spawn((
                        MinimapPlayerMarker,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgb(0.04, 0.08, 0.04)),
                        BackgroundColor(Color::srgb(0.22, 0.96, 0.34)),
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
        let slope_deg = tank.hull_pitch.to_degrees();
        let ford_tag = if tank.is_on_water(&sim.0.voxels) {
            " [FORD]"
        } else {
            ""
        };
        text.0 = format!(
            "SPD {:>3.0} km/h | GEAR {}{}\nALT {:>+4.1}m | SLOPE {:>+4.1} deg\nTUR {:>+4.0} deg | ELV {:>+4.1} deg\nGUN {} | VIEW {}",
            tank.speed_kmh().abs(),
            tank.gear_label(),
            ford_tag,
            tank.position.y,
            slope_deg,
            turret_deg,
            elev_deg,
            reload_str,
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

#[allow(clippy::type_complexity)]
fn update_hud_minimap(
    sim: Res<SimResource>,
    mut markers: ParamSet<(
        Query<&mut Node, With<MinimapPlayerMarker>>,
        Query<&mut Node, With<MinimapHullHeadingPip>>,
        Query<&mut Node, With<MinimapGunHeadingPip>>,
    )>,
) {
    let tank = &sim.0.player_tank;
    let uv = sim.0.map.world_to_minimap_uv(tank.position);
    let cx = uv.x * MINIMAP_DISPLAY_PX;
    let cy = uv.y * MINIMAP_DISPLAY_PX;

    if let Ok(mut player_node) = markers.p0().single_mut() {
        player_node.left = Val::Px((cx - 4.0).clamp(0.0, MINIMAP_DISPLAY_PX - 8.0));
        player_node.top = Val::Px((cy - 4.0).clamp(0.0, MINIMAP_DISPLAY_PX - 8.0));
    }

    let hull_fwd = tank.hull_forward();
    let hull_dir = Vec2::new(hull_fwd.x, hull_fwd.z).normalize_or_zero();
    if let Ok(mut hull_pip) = markers.p1().single_mut() {
        let hx = (cx + hull_dir.x * 7.5 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0);
        let hy = (cy + hull_dir.y * 7.5 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0);
        hull_pip.left = Val::Px(hx);
        hull_pip.top = Val::Px(hy);
    }

    let gun_fwd = tank.gun_forward();
    let gun_dir = Vec2::new(gun_fwd.x, gun_fwd.z).normalize_or_zero();
    if let Ok(mut gun_pip) = markers.p2().single_mut() {
        let gx = (cx + gun_dir.x * 12.0 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0);
        let gy = (cy + gun_dir.y * 12.0 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0);
        gun_pip.left = Val::Px(gx);
        gun_pip.top = Val::Px(gy);
    }
}
