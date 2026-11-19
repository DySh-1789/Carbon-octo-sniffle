use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use sim::{
    BattleOutcome, CrewRole, HitOutcome, MAX_TANKS, ModuleHealth, ModuleKind, START_TICKETS, Team,
};

use crate::bridge::{BattleControlResource, PendingSimEvents, SimResource, to_bevy_vec3};
use crate::camera::{CameraMode, MainTankCamera, TankCameraController};

const MINIMAP_DISPLAY_PX: f32 = 192.0;
const KILL_FEED_ROWS: usize = 5;
const KILL_FEED_TTL_SECS: f32 = 8.0;
const HIT_CAM_TTL_SECS: f32 = 5.5;

const HUD_CREW_SEATS: [CrewRole; 5] = [
    CrewRole::Driver,
    CrewRole::Gunner,
    CrewRole::Loader,
    CrewRole::Commander,
    CrewRole::RadioOperator,
];

const HUD_MODULES: [ModuleKind; 9] = [
    ModuleKind::Engine,
    ModuleKind::Transmission,
    ModuleKind::LeftTrack,
    ModuleKind::RightTrack,
    ModuleKind::FuelTank,
    ModuleKind::GunBreech,
    ModuleKind::GunBarrel,
    ModuleKind::TurretRing,
    ModuleKind::AmmoRack,
];

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
struct DamageHeaderText;

#[derive(Component)]
struct CrewSeatChip(CrewRole);

#[derive(Component)]
struct ModuleChip(ModuleKind);

#[derive(Component)]
struct HitCamPanel;

#[derive(Component)]
struct HitCamTitleText;

#[derive(Component)]
struct HitCamBodyText;

#[derive(Component)]
struct MinimapZoneBadge(usize);

#[derive(Component)]
struct MinimapUnitDot(usize);

#[derive(Component)]
struct MinimapHullHeadingPip;

#[derive(Component)]
struct MinimapGunHeadingPip;

#[derive(Component)]
struct AlliesTicketsBar;

#[derive(Component)]
struct AxisTicketsBar;

#[derive(Component)]
struct AlliesScoreText;

#[derive(Component)]
struct AxisScoreText;

#[derive(Component)]
struct ScoreboardZoneBadge(usize);

#[derive(Component)]
struct BattleSubStatusText;

#[derive(Component)]
struct KillFeedRow(usize);

#[derive(Component)]
struct OverheadMarker(usize);

#[derive(Component)]
struct BattleEndOverlay;

#[derive(Component)]
struct BattleEndTitleText;

#[derive(Component)]
struct BattleEndStatsText;

#[derive(Component)]
struct PlayAgainButton;

#[derive(Clone)]
struct KillFeedEntry {
    line: String,
    color: Color,
    age: f32,
}

#[derive(Resource, Default)]
struct KillFeedBuffer {
    entries: Vec<KillFeedEntry>,
}

#[derive(Resource, Default)]
struct HitCamState {
    title: String,
    title_color: Color,
    border_color: Color,
    body: String,
    timer: f32,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KillFeedBuffer>()
            .init_resource::<HitCamState>()
            .add_systems(
                Startup,
                (
                    setup_crosshair_and_telemetry,
                    setup_battle_scoreboard_and_feed,
                    setup_overhead_markers_and_modal,
                    setup_minimap_hud,
                ),
            )
            .add_systems(
                Update,
                (
                    update_hud_telemetry,
                    update_player_damage_diagram,
                    update_hit_cam_panel,
                    update_hud_reticles,
                    update_battle_scoreboard,
                    update_kill_feed,
                    update_overhead_markers,
                    update_hud_minimap,
                    update_battle_end_modal,
                ),
            );
    }
}

fn setup_crosshair_and_telemetry(mut commands: Commands, sim: Res<SimResource>) {
    // 1. Center camera crosshair
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

    // 2. Sniper gunner optic reticle overlay
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

    // 3. Trailing actual gun-bore aiming indicator
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
                left: Val::Px(14.0),
                top: Val::Px(12.0),
                max_width: Val::Px(348.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.74)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(format!("STAGE 4 | {map_title} - WT DAMAGE & 75MM AP")),
                TextFont {
                    font_size: FontSize::Px(12.5),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.82, 0.40)),
            ));
            panel.spawn((
                Text::new("WASD: Drive | LMB/Space: Fire | Shift: Optic | C: FreeLook | Esc: Cursor"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.87, 0.83)),
            ));
            panel.spawn((
                CursorHintText,
                Text::new("Click window to lock mouse aiming"),
                TextFont {
                    font_size: FontSize::Px(11.5),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.68, 0.25)),
            ));
        });

    // 5. Bottom-left stack: War Thunder Crew/Module Schematic + Telemetry & Reload
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            width: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|stack| {
            // 5a. War Thunder Crew & Internal Module Damage Schematic (strictly no HP bar!)
            stack
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(9.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.55, 0.60, 0.54, 0.50)),
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.07, 0.82)),
                ))
                .with_children(|diag| {
                    diag.spawn((
                        DamageHeaderText,
                        Text::new("CREW 5/5 | MODULES NOMINAL"),
                        TextFont {
                            font_size: FontSize::Px(11.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.94, 0.78)),
                    ));

                    // Crew seat chips row
                    diag.spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(4.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|row| {
                        for seat in HUD_CREW_SEATS {
                            row.spawn((
                                CrewSeatChip(seat),
                                Node {
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.32, 0.74, 0.40)),
                                BackgroundColor(Color::srgba(0.14, 0.34, 0.18, 0.85)),
                                Text::new(seat.short_label()),
                                TextFont {
                                    font_size: FontSize::Px(10.5),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        }
                    });

                    // Modules status chips (2 compact rows)
                    diag.spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(4.0),
                        row_gap: Val::Px(4.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|grid| {
                        for kind in HUD_MODULES {
                            grid.spawn((
                                ModuleChip(kind),
                                Node {
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgba(0.50, 0.54, 0.50, 0.75)),
                                BackgroundColor(Color::srgba(0.20, 0.23, 0.21, 0.88)),
                                Text::new(kind.short_label()),
                                TextFont {
                                    font_size: FontSize::Px(10.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.86, 0.88, 0.85)),
                            ));
                        }
                    });
                });

            // 5b. Telemetry & Cannon Reload Panel
            stack
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.80)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        TelemetryText,
                        Text::new("SPD   0 km/h | GEAR N\nSHELL 75mm M72 AP\nGUN   READY"),
                        TextFont {
                            font_size: FontSize::Px(12.5),
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
        });
}

fn setup_battle_scoreboard_and_feed(mut commands: Commands, sim: Res<SimResource>) {
    // Top-center War Thunder ticket & capture zone bar
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|wrapper| {
            wrapper
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.65, 0.68, 0.60, 0.45)),
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.07, 0.82)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        AlliesScoreText,
                        Text::new("6 | 800"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.42, 0.72, 1.0)),
                    ));

                    bar.spawn((
                        Node {
                            width: Val::Px(104.0),
                            height: Val::Px(10.0),
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.16, 0.20, 0.25, 0.9)),
                    ))
                    .with_children(|bg| {
                        bg.spawn((
                            AlliesTicketsBar,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.24, 0.58, 0.96)),
                        ));
                    });

                    for (idx, zone) in sim.0.battle.zones.iter().enumerate() {
                        bar.spawn((
                            ScoreboardZoneBadge(idx),
                            Node {
                                width: Val::Px(24.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.85, 0.82, 0.36)),
                            BackgroundColor(Color::srgba(0.14, 0.16, 0.14, 0.9)),
                        ))
                        .with_children(|badge| {
                            badge.spawn((
                                Text::new(zone.label.to_string()),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }

                    bar.spawn((
                        Node {
                            width: Val::Px(104.0),
                            height: Val::Px(10.0),
                            justify_content: JustifyContent::FlexStart,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.26, 0.16, 0.16, 0.9)),
                    ))
                    .with_children(|bg| {
                        bg.spawn((
                            AxisTicketsBar,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.92, 0.28, 0.26)),
                        ));
                    });

                    bar.spawn((
                        AxisScoreText,
                        Text::new("800 | 6"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.98, 0.44, 0.40)),
                    ));
                });

            wrapper.spawn((
                BattleSubStatusText,
                Text::new("00:00 | KILLS 0 | PENS 0"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.94, 0.90)),
                BackgroundColor(Color::srgba(0.06, 0.08, 0.07, 0.65)),
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    ..default()
                },
            ));
        });

    // Top-right stack: Kill Feed + War Thunder Hit-Cam Damage Report Panel
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|col| {
            // Kill feed rows
            for row in 0..KILL_FEED_ROWS {
                col.spawn((
                    KillFeedRow(row),
                    Node {
                        display: Display::None,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.07, 0.78)),
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }

            // Hit-Cam / Shell Impact Report Panel (shown for 5.5s on player shots)
            col.spawn((
                HitCamPanel,
                Node {
                    display: Display::None,
                    width: Val::Px(286.0),
                    padding: UiRect::all(Val::Px(9.0)),
                    margin: UiRect::top(Val::Px(6.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.35, 0.90, 0.45)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.06, 0.88)),
            ))
            .with_children(|card| {
                card.spawn((
                    HitCamTitleText,
                    Text::new("PENETRATION"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.35, 0.95, 0.45)),
                ));
                card.spawn((
                    HitCamBodyText,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.90, 0.93, 0.88)),
                ));
            });
        });
}

fn setup_overhead_markers_and_modal(mut commands: Commands) {
    // Pooled 3D-projected overhead tank tags (strictly NO HP bar!)
    for i in 0..MAX_TANKS {
        commands.spawn((
            OverheadMarker(i),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                display: Display::None,
                padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            BackgroundColor(Color::srgba(0.05, 0.07, 0.06, 0.72)),
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }

    // Centered End-of-Battle overlay modal
    commands
        .spawn((
            BattleEndOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.03, 0.56)),
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    Node {
                        width: Val::Px(360.0),
                        padding: UiRect::all(Val::Px(22.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.88, 0.80, 0.42, 0.85)),
                    BackgroundColor(Color::srgba(0.07, 0.09, 0.08, 0.94)),
                ))
                .with_children(|card| {
                    card.spawn((
                        BattleEndTitleText,
                        Text::new("VICTORY"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.45, 0.78, 1.0)),
                    ));
                    card.spawn((
                        BattleEndStatsText,
                        Text::new("ALLIES 800 - 0 AXIS"),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.90, 0.92, 0.88)),
                    ));
                    card.spawn((
                        PlayAgainButton,
                        Node {
                            padding: UiRect::axes(Val::Px(22.0), Val::Px(10.0)),
                            margin: UiRect::top(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor::all(Color::srgb(0.95, 0.85, 0.42)),
                        BackgroundColor(Color::srgb(0.18, 0.34, 0.20)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("PLAY AGAIN (Enter)"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
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
                    for (idx, zone) in sim.0.map.capture_zones.iter().enumerate() {
                        let uv = sim.0.map.world_to_minimap_uv(zone.center);
                        let px = (uv.x * MINIMAP_DISPLAY_PX - 9.0)
                            .clamp(0.0, MINIMAP_DISPLAY_PX - 18.0);
                        let py = (uv.y * MINIMAP_DISPLAY_PX - 9.0)
                            .clamp(0.0, MINIMAP_DISPLAY_PX - 18.0);

                        map_box
                            .spawn((
                                MinimapZoneBadge(idx),
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
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }

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

                    for i in (0..MAX_TANKS).rev() {
                        let is_player = i == 0;
                        let sz = if is_player { 8.0 } else { 6.0 };
                        map_box.spawn((
                            MinimapUnitDot(i),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(sz),
                                height: Val::Px(sz),
                                display: Display::None,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.04, 0.06, 0.05)),
                            BackgroundColor(Color::srgb(0.22, 0.96, 0.34)),
                        ));
                    }
                });
        });
}

#[allow(clippy::type_complexity)]
fn update_hud_telemetry(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TelemetryText>>,
        Query<(&mut Text, &mut TextColor), With<CursorHintText>>,
    )>,
    mut reload_bar_q: Query<(&mut Node, &mut BackgroundColor), With<ReloadFillBar>>,
) {
    let player = sim.0.player_unit();
    let tank = &player.state;
    let mods = player.damage.mobility_modifiers();

    if let Ok(mut text) = texts.p0().single_mut() {
        if !player.alive {
            text.0 = "VEHICLE DESTROYED\nSPECTATING BATTLE\nPress Enter to Restart".to_string();
        } else {
            let gun_str = if !mods.can_fire {
                if !player.damage.has_active_seat(CrewRole::Gunner) {
                    "NO GUNNER".to_string()
                } else {
                    "GUN DISABLED".to_string()
                }
            } else if tank.is_loaded() {
                "READY".to_string()
            } else {
                format!("RELOAD {:.1}s", tank.reload_remaining * mods.reload_time_scale)
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
            let sh = &tank.config.shell;
            text.0 = format!(
                "SPD {:>3.0} km/h | GEAR {}{}\nALT {:>+4.1}m | SLOPE {:>+4.1} deg\nTUR {:>+4.0} deg | ELV {:>+4.1} deg\nSHELL {} ({:.0}mm)\nGUN {} | {}",
                tank.speed_kmh().abs(),
                tank.gear_label(),
                ford_tag,
                tank.position.y,
                slope_deg,
                turret_deg,
                elev_deg,
                sh.name,
                sh.penetration_0m_mm,
                gun_str,
                view_str,
            );
        }
    }

    if let Ok((mut hint, mut color)) = texts.p1().single_mut() {
        if player.damage.on_fire && player.alive {
            hint.0 = "!! ENGINE / FUEL FIRE !!" .to_string();
            color.0 = Color::srgb(1.0, 0.35, 0.20);
        } else if cam_ctrl.cursor_locked {
            hint.0 = "Mouse locked (press Esc to unlock cursor)".to_string();
            color.0 = Color::srgb(0.68, 0.88, 0.72);
        } else {
            hint.0 = "Click window to lock mouse aiming".to_string();
            color.0 = Color::srgb(0.95, 0.68, 0.25);
        }
    }

    if let Ok((mut node, mut bg)) = reload_bar_q.single_mut() {
        node.width = Val::Percent(tank.reload_progress() * 100.0);
        bg.0 = if !player.alive || !mods.can_fire {
            Color::srgb(0.75, 0.22, 0.20)
        } else if tank.is_loaded() {
            Color::srgb(0.28, 0.88, 0.36)
        } else {
            Color::srgb(0.92, 0.62, 0.16)
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_player_damage_diagram(
    sim: Res<SimResource>,
    mut q: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<DamageHeaderText>>,
        Query<(&CrewSeatChip, &mut BackgroundColor, &mut BorderColor, &mut TextColor)>,
        Query<(&ModuleChip, &mut BackgroundColor, &mut BorderColor, &mut TextColor)>,
    )>,
) {
    let player = sim.0.player_unit();
    let dmg = &player.damage;
    let alive_crew = dmg.alive_crew_count();
    let total_crew = dmg.total_crew_count();

    if let Ok((mut header, mut color)) = q.p0().single_mut() {
        if !player.alive {
            header.0 = if dmg.turret_blown_off {
                "AMMO RACK DETONATED".to_string()
            } else {
                format!("CREW {alive_crew}/{total_crew} | KNOCKED OUT")
            };
            color.0 = Color::srgb(0.98, 0.34, 0.30);
        } else if dmg.on_fire {
            header.0 = format!("CREW {alive_crew}/{total_crew} | !! ON FIRE !!");
            color.0 = Color::srgb(1.0, 0.42, 0.18);
        } else if dmg.gunner_transfer_timer > 0.0 {
            header.0 = format!(
                "CREW {alive_crew}/{total_crew} | REPLACING GUNNER {:.1}s",
                dmg.gunner_transfer_timer
            );
            color.0 = Color::srgb(0.98, 0.82, 0.30);
        } else if dmg.driver_transfer_timer > 0.0 {
            header.0 = format!(
                "CREW {alive_crew}/{total_crew} | REPLACING DRIVER {:.1}s",
                dmg.driver_transfer_timer
            );
            color.0 = Color::srgb(0.98, 0.82, 0.30);
        } else {
            let status = if alive_crew == total_crew {
                "COMBAT READY"
            } else {
                "CREW REDUCED"
            };
            header.0 = format!("CREW {alive_crew}/{total_crew} | {status}");
            color.0 = if alive_crew <= 2 {
                Color::srgb(0.98, 0.45, 0.34)
            } else if alive_crew < total_crew {
                Color::srgb(0.95, 0.82, 0.34)
            } else {
                Color::srgb(0.76, 0.94, 0.78)
            };
        }
    }

    for (chip, mut bg, mut border, mut tc) in &mut q.p1() {
        let manned = dmg.has_active_seat(chip.0);
        let replacing = (chip.0 == CrewRole::Driver && dmg.driver_transfer_timer > 0.0)
            || (chip.0 == CrewRole::Gunner && dmg.gunner_transfer_timer > 0.0);
        if manned {
            bg.0 = Color::srgba(0.14, 0.36, 0.18, 0.88);
            *border = BorderColor::all(Color::srgb(0.32, 0.78, 0.42));
            tc.0 = Color::WHITE;
        } else if replacing {
            bg.0 = Color::srgba(0.48, 0.36, 0.08, 0.90);
            *border = BorderColor::all(Color::srgb(0.96, 0.78, 0.24));
            tc.0 = Color::srgb(1.0, 0.95, 0.75);
        } else {
            bg.0 = Color::srgba(0.38, 0.10, 0.10, 0.90);
            *border = BorderColor::all(Color::srgb(0.88, 0.26, 0.24));
            tc.0 = Color::srgb(0.98, 0.64, 0.62);
        }
    }

    for (chip, mut bg, mut border, mut tc) in &mut q.p2() {
        match dmg.module_worst_health(chip.0) {
            ModuleHealth::Intact => {
                bg.0 = Color::srgba(0.20, 0.23, 0.21, 0.88);
                *border = BorderColor::all(Color::srgba(0.50, 0.54, 0.50, 0.75));
                tc.0 = Color::srgb(0.84, 0.87, 0.84);
            }
            ModuleHealth::Damaged => {
                bg.0 = Color::srgba(0.54, 0.38, 0.08, 0.92);
                *border = BorderColor::all(Color::srgb(0.98, 0.78, 0.22));
                tc.0 = Color::srgb(1.0, 0.96, 0.80);
            }
            ModuleHealth::Destroyed => {
                bg.0 = Color::srgba(0.46, 0.10, 0.10, 0.94);
                *border = BorderColor::all(Color::srgb(0.95, 0.24, 0.22));
                tc.0 = Color::srgb(1.0, 0.72, 0.70);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_hit_cam_panel(
    time: Res<Time>,
    sim: Res<SimResource>,
    pending: Res<PendingSimEvents>,
    mut state: ResMut<HitCamState>,
    mut panel_q: Query<(&mut Node, &mut BorderColor), With<HitCamPanel>>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<HitCamTitleText>>,
        Query<&mut Text, With<HitCamBodyText>>,
    )>,
) {
    if pending.battle_restarted {
        state.timer = 0.0;
    }

    let player_id = sim.0.player_unit().id;
    for rep in &pending.hit_reports {
        if rep.shooter_id != player_id || rep.inert_hit {
            continue;
        }
        let (title, color) = if let Some(reason) = rep.kill_reason {
            (
                format!("DESTROYED - {}", reason.short_label()),
                Color::srgb(1.0, 0.36, 0.28),
            )
        } else {
            match rep.outcome {
                HitOutcome::Penetration => ("PENETRATION".to_string(), Color::srgb(0.32, 0.95, 0.42)),
                HitOutcome::ExternalModuleHit => {
                    ("MODULE HIT".to_string(), Color::srgb(0.96, 0.82, 0.30))
                }
                HitOutcome::Ricochet => ("RICOCHET".to_string(), Color::srgb(0.98, 0.68, 0.26)),
                HitOutcome::NonPenetration => {
                    ("NON-PENETRATION".to_string(), Color::srgb(0.76, 0.78, 0.76))
                }
            }
        };

        let mut lines = Vec::new();
        lines.push(format!(
            "{} | {} ({:.0}mm {})",
            rep.target_name,
            rep.plate_name,
            rep.nominal_armor_mm,
            rep.armor_type.label()
        ));
        lines.push(format!(
            "Angle {:.0} deg | Eff {:.0}mm vs {:.0}mm AP",
            rep.impact_angle_deg, rep.effective_armor_mm, rep.penetration_mm
        ));
        if !rep.killed_crew.is_empty() {
            let names: Vec<&str> = rep.killed_crew.iter().map(|c| c.label()).collect();
            lines.push(format!("CREW KO: {}", names.join(", ")));
        }
        if !rep.damaged_modules.is_empty() {
            let mods: Vec<String> = rep
                .damaged_modules
                .iter()
                .take(4)
                .map(|(k, h)| {
                    let tag = if *h == ModuleHealth::Destroyed {
                        "X"
                    } else {
                        "!"
                    };
                    format!("{}[{}]", k.short_label(), tag)
                })
                .collect();
            lines.push(format!("MODULES: {}", mods.join(" ")));
        }
        if rep.started_fire {
            lines.push("FIRE IGNITED!".to_string());
        }

        state.title = title;
        state.title_color = color;
        state.border_color = color;
        state.body = lines.join("\n");
        state.timer = HIT_CAM_TTL_SECS;
    }

    state.timer = (state.timer - time.delta_secs()).max(0.0);

    if let Ok((mut node, mut border)) = panel_q.single_mut() {
        if state.timer > 0.0 {
            node.display = Display::Flex;
            *border = BorderColor::all(state.border_color);
        } else {
            node.display = Display::None;
        }
    }
    if state.timer > 0.0 {
        if let Ok((mut t, mut c)) = texts.p0().single_mut() {
            t.0 = state.title.clone();
            c.0 = state.title_color;
        }
        if let Ok(mut b) = texts.p1().single_mut() {
            b.0 = state.body.clone();
        }
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
    let player_unit = sim.0.player_unit();
    let tank = &player_unit.state;
    let can_fire = player_unit.damage.can_fire();

    if let Ok(mut overlay) = reticle_nodes.p1().single_mut() {
        overlay.display = if player_unit.alive && cam_ctrl.mode == CameraMode::Sniper {
            Display::Flex
        } else {
            Display::None
        };
    }

    if let (Ok((camera, cam_gtf)), Ok((mut reticle_node, mut reticle_border))) =
        (camera_q.single(), reticle_nodes.p0().single_mut())
    {
        if !player_unit.alive {
            reticle_node.display = Display::None;
            return;
        }
        let muzzle = tank.muzzle_world_position();
        let bore = tank.gun_forward();
        let gun_target_sim = sim
            .0
            .raycast_scene(muzzle, bore, 400.0, Some(player_unit.id))
            .unwrap_or(muzzle + bore * 400.0);
        if let Ok(viewport_pos) = camera.world_to_viewport(cam_gtf, to_bevy_vec3(gun_target_sim)) {
            reticle_node.display = Display::Flex;
            reticle_node.left = Val::Px(viewport_pos.x - 9.0);
            reticle_node.top = Val::Px(viewport_pos.y - 9.0);
            *reticle_border = BorderColor::all(if !can_fire {
                Color::srgba(0.95, 0.25, 0.22, 0.9)
            } else if tank.is_loaded() {
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
fn update_battle_scoreboard(
    sim: Res<SimResource>,
    mut bars: ParamSet<(
        Query<&mut Node, With<AlliesTicketsBar>>,
        Query<&mut Node, With<AxisTicketsBar>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<AlliesScoreText>>,
        Query<&mut Text, With<AxisScoreText>>,
        Query<(&mut Text, &mut TextColor), With<BattleSubStatusText>>,
    )>,
    mut zone_badges: Query<(&ScoreboardZoneBadge, &mut BorderColor, &mut BackgroundColor)>,
) {
    let battle = &sim.0.battle;
    let allies_alive = sim.0.alive_count(Team::Allies);
    let axis_alive = sim.0.alive_count(Team::Axis);

    if let Ok(mut node) = bars.p0().single_mut() {
        node.width = Val::Percent((battle.allies_tickets / START_TICKETS * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut node) = bars.p1().single_mut() {
        node.width = Val::Percent((battle.axis_tickets / START_TICKETS * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        t.0 = format!("{allies_alive} | {:.0}", battle.allies_tickets.ceil());
    }
    if let Ok(mut t) = texts.p1().single_mut() {
        t.0 = format!("{:.0} | {axis_alive}", battle.axis_tickets.ceil());
    }

    for (badge, mut border, mut bg) in &mut zone_badges {
        if let Some(zone) = battle.zones.get(badge.0) {
            let (fill, rim) = zone_colors(zone.owner, zone.contested);
            bg.0 = fill;
            *border = BorderColor::all(rim);
        }
    }

    if let Ok((mut text, mut color)) = texts.p2().single_mut() {
        let player = sim.0.player_unit();
        let secs = battle.battle_time as u32;
        if !player.alive && !battle.is_over() {
            text.0 = format!(
                "{:02}:{:02} | TANK DESTROYED - SPECTATING | PRESS ENTER TO RESTART",
                secs / 60,
                secs % 60
            );
            color.0 = Color::srgb(0.98, 0.45, 0.38);
        } else {
            text.0 = format!(
                "{:02}:{:02} | KILLS {} | PENS {}",
                secs / 60,
                secs % 60,
                player.kills,
                player.hits_landed,
            );
            color.0 = Color::srgb(0.92, 0.94, 0.90);
        }
    }
}

fn update_kill_feed(
    time: Res<Time>,
    pending: Res<PendingSimEvents>,
    mut feed: ResMut<KillFeedBuffer>,
    mut rows: Query<(&KillFeedRow, &mut Node, &mut Text, &mut TextColor)>,
) {
    if pending.battle_restarted {
        feed.entries.clear();
    }

    let dt = time.delta_secs();
    for entry in &mut feed.entries {
        entry.age += dt;
    }
    feed.entries.retain(|e| e.age < KILL_FEED_TTL_SECS);

    for k in &pending.kills {
        let color = if k.killer_team == Team::Allies {
            Color::srgb(0.55, 0.82, 1.0)
        } else {
            Color::srgb(0.98, 0.50, 0.44)
        };
        feed.entries.push(KillFeedEntry {
            line: format!(
                "[{}] {} > [{}] {} ({})",
                k.killer_team.label(),
                k.killer_name,
                k.victim_team.label(),
                k.victim_name,
                k.reason.short_label(),
            ),
            color,
            age: 0.0,
        });
        if feed.entries.len() > KILL_FEED_ROWS {
            feed.entries.remove(0);
        }
    }

    let count = feed.entries.len();
    for (row, mut node, mut text, mut color) in &mut rows {
        if row.0 < count {
            let entry = &feed.entries[count - 1 - row.0];
            node.display = Display::Flex;
            text.0 = entry.line.clone();
            color.0 = entry.color;
        } else {
            node.display = Display::None;
        }
    }
}

fn update_overhead_markers(
    sim: Res<SimResource>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainTankCamera>>,
    mut markers: Query<(
        &OverheadMarker,
        &mut Node,
        &mut BorderColor,
        &mut Text,
        &mut TextColor,
    )>,
) {
    let Ok((camera, cam_gtf)) = camera_q.single() else {
        return;
    };
    let cam_pos = cam_gtf.translation();

    for (marker, mut node, mut border, mut text, mut color) in &mut markers {
        let Some(unit) = sim.0.tanks.get(marker.0) else {
            node.display = Display::None;
            continue;
        };
        let visible = !unit.is_player
            && unit.alive
            && (unit.team == Team::Allies || unit.spotted);
        if !visible {
            node.display = Display::None;
            continue;
        }

        let world_pos = to_bevy_vec3(unit.state.position) + Vec3::Y * 3.15;
        let dist = cam_pos.distance(world_pos);
        if dist > 340.0 {
            node.display = Display::None;
            continue;
        }

        let Ok(screen) = camera.world_to_viewport(cam_gtf, world_pos) else {
            node.display = Display::None;
            continue;
        };

        node.display = Display::Flex;
        node.left = Val::Px(screen.x - 38.0);
        node.top = Val::Px(screen.y - 16.0);

        let fire_tag = if unit.damage.on_fire { " [FIRE]" } else { "" };
        if unit.team == Team::Allies {
            *border = BorderColor::all(Color::srgba(0.35, 0.72, 1.0, 0.85));
            color.0 = Color::srgb(0.62, 0.86, 1.0);
            text.0 = format!("{} {:.0}m{}", unit.name, dist, fire_tag);
        } else {
            *border = BorderColor::all(Color::srgba(0.95, 0.32, 0.28, 0.90));
            color.0 = Color::srgb(1.0, 0.54, 0.48);
            text.0 = format!("{} {:.0}m{}", unit.name, dist, fire_tag);
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_hud_minimap(
    sim: Res<SimResource>,
    mut q: ParamSet<(
        Query<(&MinimapZoneBadge, &mut BorderColor, &mut BackgroundColor)>,
        Query<(&MinimapUnitDot, &mut Node, &mut BackgroundColor)>,
        Query<&mut Node, With<MinimapHullHeadingPip>>,
        Query<&mut Node, With<MinimapGunHeadingPip>>,
    )>,
) {
    for (badge, mut border, mut bg) in &mut q.p0() {
        if let Some(zone) = sim.0.battle.zones.get(badge.0) {
            let (fill, rim) = zone_colors(zone.owner, zone.contested);
            bg.0 = fill;
            *border = BorderColor::all(rim);
        }
    }

    for (dot, mut node, mut bg) in &mut q.p1() {
        let Some(unit) = sim.0.tanks.get(dot.0) else {
            node.display = Display::None;
            continue;
        };
        let show = unit.alive && (unit.is_player || unit.team == Team::Allies || unit.spotted);
        if !show {
            node.display = Display::None;
            continue;
        }

        let uv = sim.0.map.world_to_minimap_uv(unit.state.position);
        let sz = if unit.is_player { 8.0 } else { 6.0 };
        let half = sz * 0.5;
        node.display = Display::Flex;
        node.left = Val::Px((uv.x * MINIMAP_DISPLAY_PX - half).clamp(0.0, MINIMAP_DISPLAY_PX - sz));
        node.top = Val::Px((uv.y * MINIMAP_DISPLAY_PX - half).clamp(0.0, MINIMAP_DISPLAY_PX - sz));
        bg.0 = if unit.is_player {
            Color::srgb(0.22, 0.96, 0.34)
        } else if unit.team == Team::Allies {
            Color::srgb(0.32, 0.72, 1.0)
        } else {
            Color::srgb(0.96, 0.30, 0.28)
        };
    }

    let player = sim.0.player_unit();
    let uv = sim.0.map.world_to_minimap_uv(player.state.position);
    let cx = uv.x * MINIMAP_DISPLAY_PX;
    let cy = uv.y * MINIMAP_DISPLAY_PX;

    let hull_fwd = player.state.hull_forward();
    let hull_dir = Vec2::new(hull_fwd.x, hull_fwd.z).normalize_or_zero();
    if let Ok(mut hull_pip) = q.p2().single_mut() {
        hull_pip.display = if player.alive {
            Display::Flex
        } else {
            Display::None
        };
        hull_pip.left = Val::Px((cx + hull_dir.x * 7.5 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0));
        hull_pip.top = Val::Px((cy + hull_dir.y * 7.5 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0));
    }

    let gun_fwd = player.state.gun_forward();
    let gun_dir = Vec2::new(gun_fwd.x, gun_fwd.z).normalize_or_zero();
    if let Ok(mut gun_pip) = q.p3().single_mut() {
        gun_pip.display = if player.alive {
            Display::Flex
        } else {
            Display::None
        };
        gun_pip.left = Val::Px((cx + gun_dir.x * 12.0 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0));
        gun_pip.top = Val::Px((cy + gun_dir.y * 12.0 - 2.0).clamp(0.0, MINIMAP_DISPLAY_PX - 4.0));
    }
}

#[allow(clippy::type_complexity)]
fn update_battle_end_modal(
    sim: Res<SimResource>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut control: ResMut<BattleControlResource>,
    mut overlay_q: Query<&mut Node, With<BattleEndOverlay>>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<BattleEndTitleText>>,
        Query<&mut Text, With<BattleEndStatsText>>,
    )>,
    mut btn_q: Query<(&ComputedNode, &UiGlobalTransform, &mut BackgroundColor), With<PlayAgainButton>>,
) {
    let battle = &sim.0.battle;
    let Ok(mut overlay) = overlay_q.single_mut() else {
        return;
    };

    if !battle.is_over() {
        overlay.display = Display::None;
        return;
    }

    overlay.display = Display::Flex;

    let (title, title_color) = match battle.outcome {
        BattleOutcome::Victory(Team::Allies) => ("VICTORY", Color::srgb(0.45, 0.78, 1.0)),
        BattleOutcome::Victory(Team::Axis) => ("DEFEAT", Color::srgb(0.98, 0.36, 0.34)),
        BattleOutcome::Draw => ("DRAW", Color::srgb(0.92, 0.84, 0.42)),
        BattleOutcome::Ongoing => ("", Color::WHITE),
    };

    if let Ok((mut t, mut c)) = texts.p0().single_mut() {
        t.0 = title.to_string();
        c.0 = title_color;
    }

    if let Ok(mut stats) = texts.p1().single_mut() {
        let player = sim.0.player_unit();
        let secs = battle.battle_time as u32;
        stats.0 = format!(
            "ALLIES {:.0}  -  {:.0} AXIS\nBATTLE TIME {:02}:{:02}\nYOUR KILLS: {}  |  PENETRATIONS: {}",
            battle.allies_tickets.ceil(),
            battle.axis_tickets.ceil(),
            secs / 60,
            secs % 60,
            player.kills,
            player.hits_landed,
        );
    }

    let cursor_phys = window_q
        .single()
        .ok()
        .and_then(|w| w.physical_cursor_position());
    let clicked = mouse_buttons.just_pressed(MouseButton::Left);

    for (computed, ui_gtf, mut bg) in &mut btn_q {
        let hovered = cursor_phys.is_some_and(|p| computed.contains_point(*ui_gtf, p));
        if hovered && clicked {
            bg.0 = Color::srgb(0.26, 0.52, 0.28);
            control.restart_requested = true;
        } else if hovered {
            bg.0 = Color::srgb(0.24, 0.44, 0.26);
        } else {
            bg.0 = Color::srgb(0.18, 0.34, 0.20);
        }
    }
}

fn zone_colors(owner: Option<Team>, contested: bool) -> (Color, Color) {
    let fill = match owner {
        Some(Team::Allies) => Color::srgba(0.18, 0.46, 0.88, 0.90),
        Some(Team::Axis) => Color::srgba(0.84, 0.22, 0.22, 0.90),
        None => Color::srgba(0.14, 0.16, 0.14, 0.85),
    };
    let rim = if contested {
        Color::srgb(1.0, 0.90, 0.25)
    } else {
        match owner {
            Some(Team::Allies) => Color::srgb(0.45, 0.78, 1.0),
            Some(Team::Axis) => Color::srgb(1.0, 0.44, 0.40),
            None => Color::srgb(0.90, 0.84, 0.38),
        }
    };
    (fill, rim)
}
