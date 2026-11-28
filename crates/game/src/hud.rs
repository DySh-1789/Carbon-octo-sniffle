use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use sim::{
    BattleOutcome, BattleRewardInput, CrewRole, HitOutcome, MAX_TANKS, ModuleHealth, ModuleKind,
    PenIndicator, START_TICKETS, Team, tier_roman, wrap_angle,
};

use crate::bridge::{
    BattleControlResource, PendingSimEvents, PlayerInputResource, SimResource, to_bevy_vec3,
};
use crate::camera::{CameraMode, MainTankCamera, TankCameraController};
use crate::state::{AppState, BattleEntity, EconomyResource};

const MINIMAP_DISPLAY_PX: f32 = 192.0;
const KILL_FEED_ROWS: usize = 5;
const KILL_FEED_TTL_SECS: f32 = 8.0;
const HIT_CAM_TTL_SECS: f32 = 5.5;
const COMBAT_BANNER_TTL_SECS: f32 = 2.6;
const HIT_DIR_TTL_SECS: f32 = 1.8;

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
struct TelemetryStripText;

#[derive(Component)]
struct AmmoSlotCard(usize);

#[derive(Component)]
struct AmmoSlotText(usize);

#[derive(Component)]
struct AmmoSlotStatusText;

#[derive(Component)]
struct ReloadFillBar;

#[derive(Component)]
struct FpeChip;

#[derive(Component)]
struct FpeChipText;

#[derive(Component)]
struct RepairChip;

#[derive(Component)]
struct RepairChipText;

#[derive(Component)]
struct GunReticleMarker;

#[derive(Component)]
struct BallisticDropPip;

#[derive(Component)]
struct ReticleRangeText;

#[derive(Component)]
struct SniperOverlayRoot;

/// Directional incoming hit flash bars (`0 = Front`, `1 = Right`, `2 = Rear`, `3 = Left`).
#[derive(Component)]
struct HitDirIndicator(usize);

#[derive(Component)]
struct CombatBannerRoot;

#[derive(Component)]
struct CombatBannerText;

#[derive(Component)]
struct CursorHintText;

#[derive(Component)]
struct DamageAlertBanner;

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
struct ZoneCapProgressBanner;

#[derive(Component)]
struct ZoneCapProgressFill;

#[derive(Component)]
struct ZoneCapProgressText;

#[derive(Component)]
struct TabScoreboardOverlay;

#[derive(Component)]
struct TabScoreboardAlliesText;

#[derive(Component)]
struct TabScoreboardAxisText;

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

/// Unified clickable action component for in-battle HUD buttons.
#[derive(Component, Copy, Clone, PartialEq, Eq)]
enum HudButtonAction {
    PlayAgain,
    ToHangar,
    TopHangarShortcut,
    SelectAmmo(usize),
    UseFpe,
    ToggleRepair,
}

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

/// Live combat notifications (target destroyed, FPE, repair complete) & directional incoming-hit timers.
#[derive(Resource, Default)]
struct CombatHudFeedback {
    banner_text: String,
    banner_color: Color,
    banner_timer: f32,
    /// Remaining visibility seconds for `[Front, Right, Rear, Left]` incoming hit arcs.
    hit_dir_timers: [f32; 4],
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KillFeedBuffer>()
            .init_resource::<HitCamState>()
            .init_resource::<CombatHudFeedback>()
            .add_systems(
                OnEnter(AppState::InBattle),
                (
                    reset_hud_buffers,
                    setup_crosshair_and_telemetry,
                    setup_battle_scoreboard_and_hitcam,
                    setup_overhead_markers_and_modal,
                    setup_minimap_and_kill_feed,
                ),
            )
            .add_systems(
                Update,
                (
                    update_hud_telemetry,
                    update_player_damage_diagram,
                    update_hit_cam_panel,
                    update_combat_feedback_and_tab_scoreboard,
                    update_hud_reticles,
                    update_battle_scoreboard,
                    update_kill_feed,
                    update_overhead_markers,
                    update_hud_minimap,
                    update_battle_end_modal_and_buttons,
                )
                    .run_if(in_state(AppState::InBattle)),
            );
    }
}

fn reset_hud_buffers(
    mut feed: ResMut<KillFeedBuffer>,
    mut hitcam: ResMut<HitCamState>,
    mut feedback: ResMut<CombatHudFeedback>,
    mut economy: ResMut<EconomyResource>,
) {
    feed.entries.clear();
    hitcam.timer = 0.0;
    hitcam.title.clear();
    hitcam.body.clear();
    feedback.banner_timer = 0.0;
    feedback.banner_text.clear();
    feedback.hit_dir_timers = [0.0; 4];
    economy.settled_this_battle = false;
}

fn setup_crosshair_and_telemetry(mut commands: Commands, sim: Res<SimResource>) {
    let player_cfg = &sim.0.player_tank().config;

    // 1. Center camera crosshair
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
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
            BattleEntity,
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

    // 3. Trailing actual gun-bore aiming indicator (dynamically tinted by War Thunder PenIndicator)
    commands.spawn((
        BattleEntity,
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

    // 3b. Dynamic gravity/drag ballistic drop pip (shown in Sniper optic)
    commands.spawn((
        BattleEntity,
        BallisticDropPip,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(10.0),
            height: Val::Px(10.0),
            display: Display::None,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(1.0, 0.84, 0.24, 0.95)),
        BackgroundColor(Color::srgba(1.0, 0.65, 0.12, 0.35)),
    ));

    // 3c. Compact live rangefinder & armor penetration readout beneath screen center
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|center| {
            center.spawn((
                ReticleRangeText,
                Node {
                    margin: UiRect::top(Val::Px(64.0)),
                    padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.06, 0.06, 0.55)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.95, 0.90)),
            ));
        });

    // 3d. Directional incoming hit arcs (Front, Right, Rear, Left around center screen)
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|center| {
            center
                .spawn(Node {
                    position_type: PositionType::Relative,
                    width: Val::Px(240.0),
                    height: Val::Px(240.0),
                    ..default()
                })
                .with_children(|ring| {
                    let bars: [(usize, Node); 4] = [
                        (
                            0, // Front (top)
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                left: Val::Px(70.0),
                                width: Val::Px(100.0),
                                height: Val::Px(7.0),
                                display: Display::None,
                                ..default()
                            },
                        ),
                        (
                            1, // Right
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(0.0),
                                top: Val::Px(70.0),
                                width: Val::Px(7.0),
                                height: Val::Px(100.0),
                                display: Display::None,
                                ..default()
                            },
                        ),
                        (
                            2, // Rear (bottom)
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(0.0),
                                left: Val::Px(70.0),
                                width: Val::Px(100.0),
                                height: Val::Px(7.0),
                                display: Display::None,
                                ..default()
                            },
                        ),
                        (
                            3, // Left
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(70.0),
                                width: Val::Px(7.0),
                                height: Val::Px(100.0),
                                display: Display::None,
                                ..default()
                            },
                        ),
                    ];
                    for (idx, node) in bars {
                        ring.spawn((
                            HitDirIndicator(idx),
                            node,
                            BackgroundColor(Color::srgba(1.0, 0.22, 0.16, 0.88)),
                        ));
                    }
                });
        });

    // 3e. Center-lower combat event notification banner (TARGET DESTROYED / FPE / REPAIR COMPLETE)
    commands
        .spawn((
            BattleEntity,
            CombatBannerRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(112.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                CombatBannerText,
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BorderColor::all(Color::srgba(1.0, 0.78, 0.30, 0.85)),
                BackgroundColor(Color::srgba(0.04, 0.06, 0.07, 0.86)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.5),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.88, 0.40)),
            ));
        });

    // 4. Top-left compact vehicle & Hangar return bar
    let map_title = sim.0.map.name.to_uppercase();
    let vehicle_header = format!(
        "[{}] TIER {} {} | {}",
        player_cfg.nation.short_tag(),
        tier_roman(player_cfg.tier),
        player_cfg.name.to_uppercase(),
        map_title,
    );
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|col| {
            col.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.48, 0.54, 0.50, 0.40)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.72)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(vehicle_header),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.84, 0.42)),
                ));
                panel.spawn((
                    Text::new("WASD: Drive | Q/E: Cruise | 1-3: Ammo | 6: FPE | F: Repair | Shift: Optic | Tab: Roster"),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.82, 0.86, 0.83)),
                ));
                panel.spawn((
                    CursorHintText,
                    Text::new("Click to lock aim | Esc: Unlock cursor"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.70, 0.28)),
                ));
            });

            col.spawn((
                HudButtonAction::TopHangarShortcut,
                Node {
                    align_self: AlignSelf::FlexStart,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.68, 0.72, 0.76, 0.55)),
                BackgroundColor(Color::srgba(0.10, 0.14, 0.18, 0.82)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("< HANGAR (Esc -> Click)"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.92, 0.96)),
                ));
            });
        });

    // 5. Bottom-Left War Thunder Ground Battles Damage Schematic + Compact Telemetry Strip
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                bottom: Val::Px(12.0),
                width: Val::Px(292.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            },
        ))
        .with_children(|stack| {
            // Dynamic damage warning banner above schematic (hidden when nominal)
            stack.spawn((
                DamageAlertBanner,
                Node {
                    display: Display::None,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.98, 0.40, 0.22)),
                BackgroundColor(Color::srgba(0.28, 0.08, 0.06, 0.88)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.5),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.62)),
            ));

            // Translucent X-Ray Module & Crew Card (strictly NO HP bar!)
            stack
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.55, 0.60, 0.56, 0.45)),
                    BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.78)),
                ))
                .with_children(|diag| {
                    diag.spawn((
                        DamageHeaderText,
                        Text::new("CREW 5/5 | COMBAT READY"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.94, 0.78)),
                    ));

                    // Crew seats row
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
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        }
                    });

                    // Modules status grid
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
                                BorderColor::all(Color::srgba(0.50, 0.54, 0.50, 0.72)),
                                BackgroundColor(Color::srgba(0.18, 0.21, 0.20, 0.86)),
                                Text::new(kind.short_label()),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.86, 0.88, 0.85)),
                            ));
                        }
                    });
                });

            // Compact War Thunder single-line bottom strip (GEAR | SPD | SLOPE | TUR | VIEW)
            stack
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.45, 0.50, 0.48, 0.38)),
                    BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.78)),
                ))
                .with_children(|strip| {
                    strip.spawn((
                        TelemetryStripText,
                        Text::new("GEAR N | SPD 0 km/h | SLOPE +0 deg | TUR +0 deg"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.95, 0.90)),
                    ));
                });
        });

    // 6. Bottom-Center War Thunder Multi-Slot Ammo Action Bar ([1] / [2] / [3] + reload gauge)
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|center| {
            center
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.50, 0.56, 0.52, 0.45)),
                    BackgroundColor(Color::srgba(0.05, 0.07, 0.08, 0.84)),
                ))
                .with_children(|bar| {
                    // Horizontal row of up to 3 selectable shell slots
                    bar.spawn(Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Stretch,
                        ..default()
                    })
                    .with_children(|row| {
                        for slot_idx in 0..3usize {
                            let shell_opt = player_cfg.ammo_rack.get(slot_idx);
                            let label = if let Some(sh) = shell_opt {
                                format!(
                                    "[{}] {}\n{} | {:.0}mm",
                                    slot_idx + 1,
                                    sh.kind.short_label(),
                                    sh.name,
                                    if sh.is_he() {
                                        sh.he_pen_mm
                                    } else {
                                        sh.penetration_0m_mm
                                    }
                                )
                            } else {
                                String::new()
                            };
                            row.spawn((
                                AmmoSlotCard(slot_idx),
                                HudButtonAction::SelectAmmo(slot_idx),
                                Node {
                                    min_width: Val::Px(112.0),
                                    padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                                    border: UiRect::all(Val::Px(1.5)),
                                    display: if shell_opt.is_some() {
                                        Display::Flex
                                    } else {
                                        Display::None
                                    },
                                    ..default()
                                },
                                BorderColor::all(if slot_idx == 0 {
                                    Color::srgb(0.95, 0.78, 0.30)
                                } else {
                                    Color::srgba(0.42, 0.48, 0.46, 0.65)
                                }),
                                BackgroundColor(if slot_idx == 0 {
                                    Color::srgba(0.18, 0.16, 0.08, 0.92)
                                } else {
                                    Color::srgba(0.08, 0.10, 0.11, 0.88)
                                }),
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    AmmoSlotText(slot_idx),
                                    Text::new(label),
                                    TextFont {
                                        font_size: FontSize::Px(10.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.96, 0.94, 0.86)),
                                ));
                            });
                        }

                        // [6] Fire Prevention Equipment (FPE) chip
                        row.spawn((
                            FpeChip,
                            HudButtonAction::UseFpe,
                            Node {
                                min_width: Val::Px(76.0),
                                padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.42, 0.56, 0.62, 0.70)),
                            BackgroundColor(Color::srgba(0.07, 0.11, 0.14, 0.88)),
                        ))
                        .with_children(|chip| {
                            chip.spawn((
                                FpeChipText,
                                Text::new("[6] FPE\nx2 READY"),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.76, 0.90, 0.96)),
                            ));
                        });

                        // [F] Field Repair Kit chip
                        row.spawn((
                            RepairChip,
                            HudButtonAction::ToggleRepair,
                            Node {
                                min_width: Val::Px(82.0),
                                padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.42, 0.50, 0.46, 0.65)),
                            BackgroundColor(Color::srgba(0.07, 0.10, 0.10, 0.88)),
                        ))
                        .with_children(|chip| {
                            chip.spawn((
                                RepairChipText,
                                Text::new("[F] REPAIR\nINTACT"),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.72, 0.78, 0.76)),
                            ));
                        });
                    });

                    // Shared reload bar + READY / RELOAD status row
                    bar.spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|status_row| {
                        status_row
                            .spawn((
                                Node {
                                    flex_grow: 1.0,
                                    height: Val::Px(6.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.20, 0.22, 0.22, 0.92)),
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

                        status_row.spawn((
                            AmmoSlotStatusText,
                            Text::new("READY"),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.32, 0.94, 0.42)),
                        ));
                    });
                });
        });
}

fn setup_battle_scoreboard_and_hitcam(mut commands: Commands, sim: Res<SimResource>) {
    // Top-center War Thunder ticket bar with A/B/C capture zone badges stacked right beneath it
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|wrapper| {
            // Slim blue vs red horizontal ticket bar
            wrapper
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.58, 0.62, 0.58, 0.42)),
                    BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.80)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        AlliesScoreText,
                        Text::new("6 | 800"),
                        TextFont {
                            font_size: FontSize::Px(12.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.42, 0.74, 1.0)),
                    ));

                    bar.spawn((
                        Node {
                            width: Val::Px(128.0),
                            height: Val::Px(9.0),
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.14, 0.18, 0.24, 0.90)),
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

                    bar.spawn((
                        Node {
                            width: Val::Px(2.0),
                            height: Val::Px(14.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.88, 0.90, 0.88, 0.65)),
                    ));

                    bar.spawn((
                        Node {
                            width: Val::Px(128.0),
                            height: Val::Px(9.0),
                            justify_content: JustifyContent::FlexStart,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.24, 0.14, 0.14, 0.90)),
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
                            font_size: FontSize::Px(12.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.98, 0.44, 0.40)),
                    ));
                });

            // A / B / C capture badges centered directly underneath ticket bar (WT Ground Battles layout)
            wrapper
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    for (idx, zone) in sim.0.battle.zones.iter().enumerate() {
                        row.spawn((
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
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.13, 0.90)),
                        ))
                        .with_children(|badge| {
                            badge.spawn((
                                Text::new(zone.label.to_string()),
                                TextFont {
                                    font_size: FontSize::Px(12.5),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });

            wrapper.spawn((
                BattleSubStatusText,
                Text::new("00:00 | KILLS 0 | PENS 0"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.93, 0.89)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.62)),
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    ..default()
                },
            ));

            // Live zone capture progress bar (visible whenever player sits inside A / B / C)
            wrapper
                .spawn((
                    ZoneCapProgressBanner,
                    Node {
                        display: Display::None,
                        width: Val::Px(236.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                        margin: UiRect::top(Val::Px(3.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(4.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.85, 0.80, 0.34, 0.85)),
                    BackgroundColor(Color::srgba(0.04, 0.06, 0.07, 0.86)),
                ))
                .with_children(|cap| {
                    cap.spawn((
                        ZoneCapProgressText,
                        Text::new("CAPTURING ZONE B — 0%"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    cap.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.18, 0.20, 0.22, 0.92)),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            ZoneCapProgressFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.28, 0.68, 1.0)),
                        ));
                    });
                });
        });

    // Top-Right War Thunder Hit-Cam Damage Report Panel (cleanly separated from bottom-right Kill Feed)
    commands
        .spawn((
            BattleEntity,
            HitCamPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(12.0),
                display: Display::None,
                width: Val::Px(292.0),
                padding: UiRect::all(Val::Px(9.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.35, 0.90, 0.45)),
            BackgroundColor(Color::srgba(0.04, 0.06, 0.06, 0.88)),
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

    // Hold [Tab] Tactical 6v6 Battle Roster Overlay
    commands
        .spawn((
            BattleEntity,
            TabScoreboardOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.50)),
        ))
        .with_children(|bg| {
            bg.spawn((
                Node {
                    width: Val::Px(640.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.76, 0.72, 0.44, 0.78)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.09, 0.94)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("BATTLE ROSTER  (HOLD TAB)"),
                    TextFont {
                        font_size: FontSize::Px(14.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.84, 0.42)),
                ));
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|cols| {
                        cols.spawn((
                            Node {
                                flex_grow: 1.0,
                                flex_basis: Val::Px(0.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(5.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.32, 0.64, 0.96, 0.55)),
                            BackgroundColor(Color::srgba(0.06, 0.10, 0.16, 0.85)),
                        ))
                        .with_children(|col| {
                            col.spawn((
                                Text::new("ALLIES"),
                                TextFont {
                                    font_size: FontSize::Px(12.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.45, 0.78, 1.0)),
                            ));
                            col.spawn((
                                TabScoreboardAlliesText,
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.90, 0.94, 0.96)),
                            ));
                        });

                        cols.spawn((
                            Node {
                                flex_grow: 1.0,
                                flex_basis: Val::Px(0.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(5.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.96, 0.36, 0.32, 0.55)),
                            BackgroundColor(Color::srgba(0.16, 0.07, 0.08, 0.85)),
                        ))
                        .with_children(|col| {
                            col.spawn((
                                Text::new("AXIS"),
                                TextFont {
                                    font_size: FontSize::Px(12.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.98, 0.48, 0.42)),
                            ));
                            col.spawn((
                                TabScoreboardAxisText,
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.90, 0.90)),
                            ));
                        });
                    });
            });
        });
}

fn setup_overhead_markers_and_modal(mut commands: Commands) {
    // Pooled 3D-projected overhead tank tags (strictly NO HP bar!)
    for i in 0..MAX_TANKS {
        commands.spawn((
            BattleEntity,
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
                font_size: FontSize::Px(10.5),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }

    // Centered End-of-Battle overlay modal with PLAY AGAIN & TO HANGAR buttons
    commands
        .spawn((
            BattleEntity,
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
            BackgroundColor(Color::srgba(0.02, 0.03, 0.03, 0.58)),
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        padding: UiRect::all(Val::Px(22.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.88, 0.80, 0.42, 0.85)),
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.09, 0.95)),
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
                            font_size: FontSize::Px(13.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.90, 0.92, 0.88)),
                    ));

                    card.spawn(Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            HudButtonAction::PlayAgain,
                            Node {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgb(1.0, 0.64, 0.34)),
                            BackgroundColor(Color::srgb(0.83, 0.29, 0.11)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("PLAY AGAIN (Enter)"),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                        row.spawn((
                            HudButtonAction::ToHangar,
                            Node {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.36, 0.80, 0.98)),
                            BackgroundColor(Color::srgb(0.11, 0.40, 0.56)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("TO HANGAR (H)"),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });
                });
        });
}

fn setup_minimap_and_kill_feed(
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

    // Bottom-Right Stack: Kill Feed stacked directly ABOVE the tactical minimap (WT Ground Battles layout)
    commands
        .spawn((
            BattleEntity,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                bottom: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|col| {
            // Kill feed column sitting directly above minimap
            col.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(3.0),
                ..default()
            })
            .with_children(|feed_col| {
                for row in 0..KILL_FEED_ROWS {
                    feed_col.spawn((
                        KillFeedRow(row),
                        Node {
                            display: Display::None,
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.78)),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                }
            });

            // 192x192 Tactical Minimap frame
            col.spawn((
                Node {
                    padding: UiRect::all(Val::Px(7.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.62, 0.66, 0.60, 0.52)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.07, 0.84)),
            ))
            .with_children(|frame| {
                frame.spawn((
                    Text::new(map_header),
                    TextFont {
                        font_size: FontSize::Px(10.5),
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
        });
}

#[allow(clippy::type_complexity)]
fn update_hud_telemetry(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    player_input: Res<PlayerInputResource>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TelemetryStripText>>,
        Query<(&mut Text, &mut TextColor), With<CursorHintText>>,
        Query<(&AmmoSlotText, &mut Text, &mut TextColor)>,
        Query<(&mut Text, &mut TextColor), With<AmmoSlotStatusText>>,
        Query<(&mut Text, &mut TextColor), With<FpeChipText>>,
        Query<(&mut Text, &mut TextColor), With<RepairChipText>>,
    )>,
    mut boxes: ParamSet<(
        Query<(&mut Node, &mut BackgroundColor), With<ReloadFillBar>>,
        Query<(&AmmoSlotCard, &mut Node, &mut BorderColor, &mut BackgroundColor)>,
        Query<(&mut BorderColor, &mut BackgroundColor), With<FpeChip>>,
        Query<(&mut BorderColor, &mut BackgroundColor), With<RepairChip>>,
    )>,
) {
    let player = sim.0.player_unit();
    let tank = &player.state;
    let dmg = &player.damage;
    let mods = dmg.mobility_modifiers();

    if let Ok(mut strip) = texts.p0().single_mut() {
        if !player.alive {
            strip.0 = "DESTROYED | Q/E: Spectate | Enter: Restart | H: Hangar".to_string();
        } else {
            let view_str = match cam_ctrl.mode {
                CameraMode::ThirdPerson if cam_ctrl.free_look => "FREE",
                CameraMode::ThirdPerson => "3RD",
                CameraMode::Sniper => "OPTIC",
            };
            let cruise_tag = match player_input.cruise_gear {
                -1 => " [CR]",
                1 => " [CC1]",
                2 => " [CC2]",
                _ => "",
            };
            let ford_tag = if tank.is_on_water(&sim.0.voxels) {
                " [FORD]"
            } else {
                ""
            };
            strip.0 = format!(
                "GEAR {}{} | {:>2.0} km/h{} | SLP {:>+3.0} | TUR {:>+4.0} | {}",
                tank.gear_label(),
                cruise_tag,
                tank.speed_kmh().abs(),
                ford_tag,
                tank.hull_pitch.to_degrees(),
                tank.turret_yaw_local.to_degrees(),
                view_str,
            );
        }
    }

    if let Ok((mut hint, mut color)) = texts.p1().single_mut() {
        if dmg.on_fire && player.alive {
            hint.0 = if dmg.fpe_charges > 0 {
                format!("!! ENGINE / FUEL FIRE — PRESS [6] FOR FPE (x{}) !!", dmg.fpe_charges)
            } else {
                "!! ENGINE / FUEL FIRE — NO FPE LEFT !!".to_string()
            };
            color.0 = Color::srgb(1.0, 0.35, 0.20);
        } else if dmg.repairing && player.alive {
            hint.0 = format!("REPAIRING MODULES {:.1}s | F: Cancel", dmg.repair_remaining);
            color.0 = Color::srgb(0.42, 0.90, 1.0);
        } else if cam_ctrl.cursor_locked {
            hint.0 = "Aim locked | Esc: Unlock cursor / Hangar".to_string();
            color.0 = Color::srgb(0.68, 0.88, 0.72);
        } else {
            hint.0 = "Click to lock aim | Esc: Unlock cursor".to_string();
            color.0 = Color::srgb(0.95, 0.70, 0.28);
        }
    }

    let rack = &tank.config.ammo_rack;
    for (slot, mut text, mut color) in &mut texts.p2() {
        if let Some(sh) = rack.get(slot.0) {
            let pen_val = if sh.is_he() {
                sh.he_pen_mm
            } else {
                sh.penetration_0m_mm
            };
            let loaded_mark = if tank.loaded_shell_idx == slot.0 && tank.is_loaded() {
                " *"
            } else if tank.selected_shell_idx == slot.0 && !tank.is_loaded() {
                " >"
            } else {
                ""
            };
            text.0 = format!(
                "[{}] {}{}\n{} | {:.0}mm",
                slot.0 + 1,
                sh.kind.short_label(),
                loaded_mark,
                sh.name,
                pen_val,
            );
            color.0 = if tank.selected_shell_idx == slot.0 {
                if sh.is_he() {
                    Color::srgb(1.0, 0.78, 0.42)
                } else {
                    Color::srgb(1.0, 0.96, 0.78)
                }
            } else {
                Color::srgb(0.74, 0.78, 0.76)
            };
        }
    }

    for (card, mut node, mut border, mut bg) in &mut boxes.p1() {
        if let Some(sh) = rack.get(card.0) {
            node.display = Display::Flex;
            let selected = tank.selected_shell_idx == card.0;
            let loaded = tank.loaded_shell_idx == card.0 && tank.is_loaded();
            if selected {
                *border = BorderColor::all(if sh.is_he() {
                    Color::srgb(1.0, 0.56, 0.22)
                } else {
                    Color::srgb(0.96, 0.80, 0.30)
                });
                bg.0 = if loaded {
                    Color::srgba(0.20, 0.18, 0.08, 0.94)
                } else {
                    Color::srgba(0.16, 0.14, 0.08, 0.92)
                };
            } else {
                *border = BorderColor::all(Color::srgba(0.38, 0.44, 0.42, 0.62));
                bg.0 = Color::srgba(0.07, 0.09, 0.10, 0.88);
            }
        } else {
            node.display = Display::None;
        }
    }

    // [6] FPE chip status & urgent highlight when on fire
    if let Ok((mut text, mut tc)) = texts.p4().single_mut() {
        if dmg.on_fire && dmg.fpe_charges > 0 && player.alive {
            text.0 = format!("[6] FPE x{}\nEXTINGUISH!", dmg.fpe_charges);
            tc.0 = Color::srgb(1.0, 0.92, 0.72);
        } else if dmg.fpe_charges == 0 {
            text.0 = "[6] FPE\nEMPTY".to_string();
            tc.0 = Color::srgb(0.55, 0.58, 0.58);
        } else if dmg.fpe_cooldown > 0.0 {
            text.0 = format!("[6] FPE x{}\nCD {:.1}s", dmg.fpe_charges, dmg.fpe_cooldown);
            tc.0 = Color::srgb(0.76, 0.84, 0.88);
        } else {
            text.0 = format!("[6] FPE\nx{} READY", dmg.fpe_charges);
            tc.0 = Color::srgb(0.76, 0.90, 0.96);
        }
    }
    if let Ok((mut border, mut bg)) = boxes.p2().single_mut() {
        if dmg.on_fire && dmg.fpe_charges > 0 && player.alive {
            *border = BorderColor::all(Color::srgb(1.0, 0.42, 0.16));
            bg.0 = Color::srgba(0.52, 0.12, 0.06, 0.95);
        } else if dmg.fpe_charges == 0 {
            *border = BorderColor::all(Color::srgba(0.30, 0.34, 0.36, 0.50));
            bg.0 = Color::srgba(0.06, 0.07, 0.08, 0.82);
        } else {
            *border = BorderColor::all(Color::srgba(0.42, 0.56, 0.62, 0.70));
            bg.0 = Color::srgba(0.07, 0.11, 0.14, 0.88);
        }
    }

    // [F] Field Repair Kit chip status
    let needs_repair = dmg.needs_repair();
    if let Ok((mut text, mut tc)) = texts.p5().single_mut() {
        if dmg.repairing && player.alive {
            text.0 = format!("[F] REPAIR\n{:.1}s", dmg.repair_remaining);
            tc.0 = Color::srgb(0.72, 0.96, 1.0);
        } else if needs_repair && player.alive {
            text.0 = if tank.forward_speed.abs() > 3.0 {
                "[F] REPAIR\nSTOP FIRST".to_string()
            } else {
                "[F] REPAIR\nAVAILABLE".to_string()
            };
            tc.0 = Color::srgb(1.0, 0.90, 0.54);
        } else {
            text.0 = "[F] REPAIR\nINTACT".to_string();
            tc.0 = Color::srgb(0.64, 0.70, 0.68);
        }
    }
    if let Ok((mut border, mut bg)) = boxes.p3().single_mut() {
        if dmg.repairing && player.alive {
            *border = BorderColor::all(Color::srgb(0.32, 0.84, 0.98));
            bg.0 = Color::srgba(0.08, 0.24, 0.32, 0.94);
        } else if needs_repair && player.alive {
            *border = BorderColor::all(Color::srgb(0.96, 0.74, 0.22));
            bg.0 = Color::srgba(0.24, 0.18, 0.06, 0.92);
        } else {
            *border = BorderColor::all(Color::srgba(0.36, 0.42, 0.40, 0.55));
            bg.0 = Color::srgba(0.07, 0.09, 0.09, 0.86);
        }
    }

    if let Ok((mut status, mut col)) = texts.p3().single_mut() {
        if !player.alive {
            status.0 = "OUT".to_string();
            col.0 = Color::srgb(0.95, 0.32, 0.28);
        } else if !mods.can_fire {
            status.0 = if !dmg.has_active_seat(CrewRole::Gunner) {
                "NO GUNNER".to_string()
            } else {
                "DISABLED".to_string()
            };
            col.0 = Color::srgb(0.98, 0.36, 0.30);
        } else if tank.is_loaded() {
            status.0 = format!("READY [{}]", tank.active_shell().kind.short_label());
            col.0 = Color::srgb(0.32, 0.94, 0.42);
        } else {
            status.0 = format!(
                "{:.1}s -> {}",
                tank.reload_remaining * mods.reload_time_scale,
                tank.selected_shell().kind.short_label()
            );
            col.0 = Color::srgb(0.98, 0.74, 0.24);
        }
    }

    if let Ok((mut node, mut bg)) = boxes.p0().single_mut() {
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
        Query<(&mut Node, &mut Text), With<DamageAlertBanner>>,
        Query<(&mut Text, &mut TextColor), With<DamageHeaderText>>,
        Query<(&CrewSeatChip, &mut Node, &mut BackgroundColor, &mut BorderColor, &mut TextColor)>,
        Query<(&ModuleChip, &mut BackgroundColor, &mut BorderColor, &mut TextColor)>,
    )>,
) {
    let player = sim.0.player_unit();
    let dmg = &player.damage;
    let alive_crew = dmg.alive_crew_count();
    let total_crew = dmg.total_crew_count();

    // Dynamic floating damage warning banner above schematic
    if let Ok((mut banner_node, mut banner_text)) = q.p0().single_mut() {
        let alert = if !player.alive {
            None
        } else if dmg.on_fire {
            Some(if dmg.fpe_charges > 0 {
                "!! FIRE — PRESS [6] FOR FPE !!"
            } else {
                "!! FIRE IN ENGINE / FUEL COMPARTMENT !!"
            })
        } else if dmg.repairing {
            Some("FIELD REPAIR IN PROGRESS — PRESS [F] TO CANCEL")
        } else if dmg.module_worst_health(ModuleKind::GunBreech) == ModuleHealth::Destroyed
            || dmg.module_worst_health(ModuleKind::GunBarrel) == ModuleHealth::Destroyed
        {
            Some("! MAIN GUN DISABLED — STOP & PRESS [F] TO REPAIR !")
        } else if dmg.module_worst_health(ModuleKind::LeftTrack) == ModuleHealth::Destroyed
            || dmg.module_worst_health(ModuleKind::RightTrack) == ModuleHealth::Destroyed
        {
            Some("! TRACK BROKEN — PRESS [F] TO REPAIR !")
        } else if dmg.module_worst_health(ModuleKind::Engine) == ModuleHealth::Destroyed
            || dmg.module_worst_health(ModuleKind::Transmission) == ModuleHealth::Destroyed
        {
            Some("! MOBILITY DISABLED — PRESS [F] TO REPAIR !")
        } else {
            None
        };
        if let Some(msg) = alert {
            banner_node.display = Display::Flex;
            banner_text.0 = msg.to_string();
        } else {
            banner_node.display = Display::None;
        }
    }

    if let Ok((mut header, mut color)) = q.p1().single_mut() {
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
        } else if dmg.repairing {
            header.0 = format!(
                "CREW {alive_crew}/{total_crew} | REPAIRING {:.1}s",
                dmg.repair_remaining
            );
            color.0 = Color::srgb(0.40, 0.90, 1.0);
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

    for (chip, mut node, mut bg, mut border, mut tc) in &mut q.p2() {
        // Hide seats not present on 3-crew / 4-crew tanks (e.g. RadioOperator on IS-2 / IS-3 / BT-5)
        let seat_exists = dmg.crew.iter().any(|c| c.role == chip.0);
        if !seat_exists {
            node.display = Display::None;
            continue;
        }
        node.display = Display::Flex;

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

    for (chip, mut bg, mut border, mut tc) in &mut q.p3() {
        match dmg.module_worst_health(chip.0) {
            ModuleHealth::Intact => {
                bg.0 = Color::srgba(0.18, 0.21, 0.20, 0.86);
                *border = BorderColor::all(Color::srgba(0.50, 0.54, 0.50, 0.72));
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
        } else if rep.overpressure {
            ("OVERPRESSURE!".to_string(), Color::srgb(1.0, 0.54, 0.22))
        } else if rep.aphe_detonated {
            (
                format!("{} BURST PENETRATION", rep.shell_kind.short_label()),
                Color::srgb(0.32, 0.96, 0.44),
            )
        } else {
            match rep.outcome {
                HitOutcome::Penetration => (
                    format!("PENETRATION [{}]", rep.shell_kind.short_label()),
                    Color::srgb(0.32, 0.95, 0.42),
                ),
                HitOutcome::ExternalModuleHit => (
                    format!("BLAST / MODULE [{}]", rep.shell_kind.short_label()),
                    Color::srgb(0.96, 0.82, 0.30),
                ),
                HitOutcome::Ricochet => ("RICOCHET".to_string(), Color::srgb(0.98, 0.68, 0.26)),
                HitOutcome::NonPenetration => (
                    "NON-PENETRATION".to_string(),
                    Color::srgb(0.76, 0.78, 0.76),
                ),
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
            "Angle {:.0} deg | Eff {:.0}mm vs {:.0}mm {}",
            rep.impact_angle_deg,
            rep.effective_armor_mm,
            rep.penetration_mm,
            rep.shell_kind.short_label(),
        ));
        if rep.overpressure {
            lines.push("HE OVERPRESSURE WAVE!".to_string());
        } else if rep.aphe_detonated {
            lines.push("INTERNAL BURSTING CHARGE DETONATED!".to_string());
        } else if rep.over_penetrated {
            lines.push("OVER-PENETRATION (FUSE NOT ARMED)".to_string());
        }
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_combat_feedback_and_tab_scoreboard(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    pending: Res<PendingSimEvents>,
    mut feedback: ResMut<CombatHudFeedback>,
    mut nodes: ParamSet<(
        Query<(&HitDirIndicator, &mut Node, &mut BackgroundColor)>,
        Query<&mut Node, With<CombatBannerRoot>>,
        Query<&mut Node, With<TabScoreboardOverlay>>,
    )>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor, &mut BorderColor), With<CombatBannerText>>,
        Query<&mut Text, With<TabScoreboardAlliesText>>,
        Query<&mut Text, With<TabScoreboardAxisText>>,
    )>,
) {
    if pending.battle_restarted {
        feedback.banner_timer = 0.0;
        feedback.hit_dir_timers = [0.0; 4];
    }

    let dt = time.delta_secs();
    feedback.banner_timer = (feedback.banner_timer - dt).max(0.0);
    for t in &mut feedback.hit_dir_timers {
        *t = (*t - dt).max(0.0);
    }

    let player = sim.0.player_unit();
    let player_id = player.id;

    // 1. Directional incoming hit arcs + combat event banners
    for rep in &pending.hit_reports {
        if rep.inert_hit {
            continue;
        }
        if rep.shooter_id == player_id {
            if let Some(reason) = rep.kill_reason {
                feedback.banner_text = format!("TARGET DESTROYED!  [{}]", reason.short_label());
                feedback.banner_color = Color::srgb(1.0, 0.42, 0.28);
                feedback.banner_timer = COMBAT_BANNER_TTL_SECS;
            } else if rep.overpressure {
                feedback.banner_text = "OVERPRESSURE HIT!".to_string();
                feedback.banner_color = Color::srgb(1.0, 0.62, 0.22);
                feedback.banner_timer = COMBAT_BANNER_TTL_SECS;
            } else if !rep.killed_crew.is_empty() || rep.started_fire {
                feedback.banner_text = if rep.started_fire {
                    "CRITICAL HIT!  [FIRE IGNITED]".to_string()
                } else {
                    "CRITICAL HIT!".to_string()
                };
                feedback.banner_color = Color::srgb(0.42, 0.96, 0.48);
                feedback.banner_timer = 2.0;
            }
        } else if rep.target_id == player_id {
            // Compute incoming direction relative to camera yaw (`0=Front, 1=Right, 2=Rear, 3=Left`)
            let shooter_pos = sim
                .0
                .tanks
                .get(rep.shooter_id.index())
                .map(|s| s.state.position)
                .unwrap_or(rep.position + rep.normal * 10.0);
            let to_attacker = shooter_pos - player.state.position;
            let attacker_yaw = (-to_attacker.x).atan2(-to_attacker.z);
            let rel = wrap_angle(attacker_yaw - cam_ctrl.yaw);
            // `rel == 0` is directly ahead; positive `rel` is to the left, negative `rel` is to the right
            let quad = if rel.abs() <= std::f32::consts::FRAC_PI_4 {
                0 // Front
            } else if rel.abs() >= 3.0 * std::f32::consts::FRAC_PI_4 {
                2 // Rear
            } else if rel < 0.0 {
                1 // Right
            } else {
                3 // Left
            };
            feedback.hit_dir_timers[quad] = HIT_DIR_TTL_SECS;

            if rep.outcome == HitOutcome::Ricochet && feedback.banner_timer <= 0.1 {
                feedback.banner_text = format!("ARMOR RICOCHET!  ({})", rep.plate_name);
                feedback.banner_color = Color::srgb(0.55, 0.88, 1.0);
                feedback.banner_timer = 1.6;
            }
        }
    }

    for &tid in &pending.fpe_uses {
        if tid == player_id {
            feedback.banner_text = "FIRE EXTINGUISHED!  (FPE)".to_string();
            feedback.banner_color = Color::srgb(0.45, 0.90, 1.0);
            feedback.banner_timer = 2.2;
        }
    }
    for &tid in &pending.repairs_completed {
        if tid == player_id {
            feedback.banner_text = "FIELD REPAIR COMPLETE!".to_string();
            feedback.banner_color = Color::srgb(0.38, 0.96, 0.56);
            feedback.banner_timer = 2.2;
        }
    }

    // Update directional hit indicator bars
    for (dir_ind, mut node, mut bg) in &mut nodes.p0() {
        let rem = feedback.hit_dir_timers.get(dir_ind.0).copied().unwrap_or(0.0);
        if rem > 0.0 && player.alive {
            node.display = Display::Flex;
            let alpha = (rem / HIT_DIR_TTL_SECS).clamp(0.0, 1.0) * 0.90;
            bg.0 = Color::srgba(1.0, 0.22, 0.16, alpha);
        } else {
            node.display = Display::None;
        }
    }

    // Update center-lower combat callout banner
    if let Ok(mut root) = nodes.p1().single_mut() {
        root.display = if feedback.banner_timer > 0.0 && !sim.0.battle.is_over() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if feedback.banner_timer > 0.0
        && let Ok((mut t, mut tc, mut bc)) = texts.p0().single_mut()
    {
        t.0 = feedback.banner_text.clone();
        tc.0 = feedback.banner_color;
        *bc = BorderColor::all(feedback.banner_color);
    }

    // 2. Hold [Tab] 6v6 tactical roster overlay
    let show_tab = keys.pressed(KeyCode::Tab) && !sim.0.battle.is_over();
    if let Ok(mut overlay) = nodes.p2().single_mut() {
        overlay.display = if show_tab {
            Display::Flex
        } else {
            Display::None
        };
    }
    if show_tab {
        let format_team = |team: Team| -> String {
            let mut lines = Vec::with_capacity(7);
            lines.push("STATUS  UNIT                    K   P".to_string());
            for u in sim.0.tanks.iter().filter(|u| u.team == team) {
                let status = if u.alive { "ALIVE" } else { " KO  " };
                let mark = if u.is_player { ">" } else { " " };
                lines.push(format!(
                    "[{}]{}{:<22} {:>2}  {:>2}",
                    status,
                    mark,
                    u.name.chars().take(22).collect::<String>(),
                    u.kills,
                    u.hits_landed,
                ));
            }
            lines.join("\n")
        };
        if let Ok(mut t) = texts.p1().single_mut() {
            t.0 = format_team(Team::Allies);
        }
        if let Ok(mut t) = texts.p2().single_mut() {
            t.0 = format_team(Team::Axis);
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
        Query<(&mut Node, &mut BorderColor), With<BallisticDropPip>>,
        Query<(&mut Node, &mut Text, &mut TextColor), With<ReticleRangeText>>,
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

    if !player_unit.alive {
        if let Ok((mut reticle_node, _)) = reticle_nodes.p0().single_mut() {
            reticle_node.display = Display::None;
        }
        if let Ok((mut pip_node, _)) = reticle_nodes.p2().single_mut() {
            pip_node.display = Display::None;
        }
        if let Ok((mut rng_node, _, _)) = reticle_nodes.p3().single_mut() {
            rng_node.display = Display::None;
        }
        return;
    }

    let Ok((camera, cam_gtf)) = camera_q.single() else {
        return;
    };

    let telemetry = sim.0.evaluate_aim_telemetry(0, 420.0);
    let pen_color = telemetry.assessment.as_ref().map(|a| match a.indicator {
        PenIndicator::Likely => Color::srgba(0.24, 0.98, 0.36, 0.96),
        PenIndicator::Marginal => Color::srgba(1.0, 0.84, 0.22, 0.96),
        PenIndicator::Unlikely => Color::srgba(1.0, 0.28, 0.24, 0.96),
    });

    if let Ok((mut reticle_node, mut reticle_border)) = reticle_nodes.p0().single_mut() {
        if let Ok(viewport_pos) =
            camera.world_to_viewport(cam_gtf, to_bevy_vec3(telemetry.bore_point))
        {
            reticle_node.display = Display::Flex;
            reticle_node.left = Val::Px(viewport_pos.x - 9.0);
            reticle_node.top = Val::Px(viewport_pos.y - 9.0);
            *reticle_border = BorderColor::all(if !can_fire {
                Color::srgba(0.95, 0.25, 0.22, 0.92)
            } else if !tank.is_loaded() {
                Color::srgba(0.95, 0.62, 0.18, 0.92)
            } else if let Some(col) = pen_color {
                col
            } else {
                Color::srgba(0.25, 0.95, 0.35, 0.90)
            });
        } else {
            reticle_node.display = Display::None;
        }
    }

    // Dynamic ballistic drop pip in Sniper optic (accounts for gravity + drag of active shell)
    if let Ok((mut pip_node, mut pip_border)) = reticle_nodes.p2().single_mut() {
        if cam_ctrl.mode == CameraMode::Sniper
            && let Ok(vp) =
                camera.world_to_viewport(cam_gtf, to_bevy_vec3(telemetry.ballistic_impact_point))
        {
            pip_node.display = Display::Flex;
            pip_node.left = Val::Px(vp.x - 5.0);
            pip_node.top = Val::Px(vp.y - 5.0);
            *pip_border = BorderColor::all(
                pen_color.unwrap_or(Color::srgba(1.0, 0.84, 0.24, 0.95)),
            );
        } else {
            pip_node.display = Display::None;
        }
    }

    // Live rangefinder & armor penetration readout
    if let Ok((mut rng_node, mut rng_text, mut rng_color)) = reticle_nodes.p3().single_mut() {
        if let Some(a) = &telemetry.assessment {
            rng_node.display = Display::Flex;
            let verdict = match a.indicator {
                PenIndicator::Likely => "PEN",
                PenIndicator::Marginal => "RISK",
                PenIndicator::Unlikely => "NO PEN",
            };
            rng_text.0 = format!(
                "{:.0}m | {} {} ({:.0} vs {:.0}mm {})",
                telemetry.distance_m,
                verdict,
                a.plate_name,
                a.penetration_mm,
                a.effective_armor_mm,
                tank.active_shell().kind.short_label(),
            );
            rng_color.0 = pen_color.unwrap_or(Color::WHITE);
        } else if cam_ctrl.mode == CameraMode::Sniper || telemetry.distance_m < 415.0 {
            rng_node.display = Display::Flex;
            rng_text.0 = format!(
                "{:.0}m | {}",
                telemetry.distance_m,
                tank.active_shell().kind.short_label()
            );
            rng_color.0 = Color::srgb(0.86, 0.90, 0.86);
        } else {
            rng_node.display = Display::None;
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_battle_scoreboard(
    sim: Res<SimResource>,
    cam_ctrl: Res<TankCameraController>,
    mut bars: ParamSet<(
        Query<&mut Node, With<AlliesTicketsBar>>,
        Query<&mut Node, With<AxisTicketsBar>>,
        Query<(&mut Node, &mut BorderColor), With<ZoneCapProgressBanner>>,
        Query<(&mut Node, &mut BackgroundColor), With<ZoneCapProgressFill>>,
        Query<(&ScoreboardZoneBadge, &mut BorderColor, &mut BackgroundColor)>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<AlliesScoreText>>,
        Query<&mut Text, With<AxisScoreText>>,
        Query<(&mut Text, &mut TextColor), With<BattleSubStatusText>>,
        Query<(&mut Text, &mut TextColor), With<ZoneCapProgressText>>,
    )>,
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

    for (badge, mut border, mut bg) in &mut bars.p4() {
        if let Some(zone) = battle.zones.get(badge.0) {
            let (fill, rim) = zone_colors(zone.owner, zone.contested);
            bg.0 = fill;
            *border = BorderColor::all(rim);
        }
    }

    let player = sim.0.player_unit();
    if let Ok((mut text, mut color)) = texts.p2().single_mut() {
        let secs = battle.battle_time as u32;
        if !player.alive && !battle.is_over() {
            let spec_label = if cam_ctrl.spectate_tank_idx != 0
                && let Some(ally) = sim.0.tanks.get(cam_ctrl.spectate_tank_idx)
            {
                format!("SPECTATING: {} (Q/E)", ally.name)
            } else {
                "DESTROYED (Q/E: Spectate)".to_string()
            };
            text.0 = format!(
                "{:02}:{:02} | {} | Enter: Restart | H: Hangar",
                secs / 60,
                secs % 60,
                spec_label,
            );
            color.0 = Color::srgb(0.98, 0.50, 0.40);
        } else {
            text.0 = format!(
                "{:02}:{:02} | TIER {} BATTLE | KILLS {} | PENS {}",
                secs / 60,
                secs % 60,
                tier_roman(player.state.config.tier),
                player.kills,
                player.hits_landed,
            );
            color.0 = Color::srgb(0.92, 0.94, 0.90);
        }
    }

    // Live capture progress bar when player is inside a capture zone
    let player_zone = player
        .alive
        .then(|| {
            battle
                .zones
                .iter()
                .find(|z| z.contains_xz(player.state.position))
        })
        .flatten();

    if let Some(zone) = player_zone {
        let pct = (zone.progress.abs() * 100.0).clamp(0.0, 100.0);
        let (label, accent) = if zone.contested {
            (
                format!("ZONE {} CONTESTED  ({}v{})", zone.label, zone.allies_inside, zone.axis_inside),
                Color::srgb(1.0, 0.78, 0.24),
            )
        } else if zone.secured_by(Team::Allies) {
            (
                format!("ZONE {} SECURED BY ALLIES", zone.label),
                Color::srgb(0.38, 0.78, 1.0),
            )
        } else if zone.progress < 0.0 {
            (
                format!("NEUTRALIZING ZONE {} — {:.0}%", zone.label, pct),
                Color::srgb(0.98, 0.44, 0.36),
            )
        } else {
            (
                format!("CAPTURING ZONE {} — {:.0}%", zone.label, pct),
                Color::srgb(0.38, 0.78, 1.0),
            )
        };
        if let Ok((mut node, mut border)) = bars.p2().single_mut() {
            node.display = Display::Flex;
            *border = BorderColor::all(accent);
        }
        if let Ok((mut fill, mut bg)) = bars.p3().single_mut() {
            fill.width = Val::Percent(pct);
            bg.0 = accent;
        }
        if let Ok((mut txt, mut col)) = texts.p3().single_mut() {
            txt.0 = label;
            col.0 = accent;
        }
    } else if let Ok((mut node, _)) = bars.p2().single_mut() {
        node.display = Display::None;
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
                "{} > {} ({})",
                k.killer_name,
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
        node.left = Val::Px(screen.x - 42.0);
        node.top = Val::Px(screen.y - 16.0);

        let fire_tag = if unit.damage.on_fire { " [FIRE]" } else { "" };
        if unit.team == Team::Allies {
            *border = BorderColor::all(Color::srgba(0.35, 0.72, 1.0, 0.85));
            color.0 = Color::srgb(0.62, 0.86, 1.0);
            text.0 = format!("{} {:.0}m{}", unit.state.config.name, dist, fire_tag);
        } else {
            *border = BorderColor::all(Color::srgba(0.95, 0.32, 0.28, 0.90));
            color.0 = Color::srgb(1.0, 0.54, 0.48);
            text.0 = format!("{} {:.0}m{}", unit.state.config.name, dist, fire_tag);
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_battle_end_modal_and_buttons(
    sim: Res<SimResource>,
    pending: Res<PendingSimEvents>,
    cam_ctrl: Res<TankCameraController>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut economy: ResMut<EconomyResource>,
    mut control: ResMut<BattleControlResource>,
    mut player_input: ResMut<PlayerInputResource>,
    mut overlay_q: Query<&mut Node, With<BattleEndOverlay>>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<BattleEndTitleText>>,
        Query<&mut Text, With<BattleEndStatsText>>,
    )>,
    mut btn_q: Query<(
        &HudButtonAction,
        &ComputedNode,
        &UiGlobalTransform,
        &mut BackgroundColor,
    )>,
) {
    if pending.battle_restarted {
        economy.settled_this_battle = false;
    }

    let battle = &sim.0.battle;
    let battle_over = battle.is_over();

    if battle_over && !economy.settled_this_battle {
        let player = sim.0.player_unit();
        let allied_zones = battle.owned_zone_count(Team::Allies) as u32;
        let summary = economy.profile.settle_battle(&BattleRewardInput {
            nation: player.state.config.nation,
            tier: player.state.config.tier,
            outcome: battle.outcome,
            player_survived: player.alive,
            kills: player.kills,
            penetrations: player.hits_landed,
            allied_zones_held: allied_zones,
        });
        let sign = if summary.net_cr >= 0 { "+" } else { "" };
        economy.status_notice = format!(
            "LAST BATTLE: {}{} CR | +{} {} RP",
            sign,
            summary.net_cr,
            summary.rp_earned,
            summary.nation.short_tag(),
        );
        economy.last_reward = Some(summary);
        economy.settled_this_battle = true;
        economy.save_to_disk();
    }

    if let Ok(mut overlay) = overlay_q.single_mut() {
        overlay.display = if battle_over {
            Display::Flex
        } else {
            Display::None
        };
    }

    if battle_over {
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
            let econ_block = if let Some(r) = &economy.last_reward {
                let sign = if r.net_cr >= 0 { "+" } else { "" };
                format!(
                    "\n--- ECONOMY (UPDATE 0.1) ---\nGROSS: +{} CR  |  REPAIR: -{} CR\nNET: {}{} CR  |  +{} {} RP\nWALLET: {} CR  |  {} {} RP",
                    r.gross_cr,
                    r.repair_cr,
                    sign,
                    r.net_cr,
                    r.rp_earned,
                    r.nation.short_tag(),
                    r.new_credits,
                    r.new_nation_rp,
                    r.nation.short_tag(),
                )
            } else {
                String::new()
            };
            stats.0 = format!(
                "{}  (TIER {})\nALLIES {:.0}  -  {:.0} AXIS\nBATTLE TIME {:02}:{:02}\nYOUR KILLS: {}  |  PENETRATIONS: {}{}",
                player.state.config.name,
                tier_roman(player.state.config.tier),
                battle.allies_tickets.ceil(),
                battle.axis_tickets.ceil(),
                secs / 60,
                secs % 60,
                player.kills,
                player.hits_landed,
                econ_block,
            );
        }
    }

    let cursor_phys = window_q
        .single()
        .ok()
        .and_then(|w| w.physical_cursor_position());
    let clicked = mouse_buttons.just_pressed(MouseButton::Left);
    let player_alive = sim.0.player_unit().alive;

    for (action, computed, ui_gtf, mut bg) in &mut btn_q {
        let interactive = match action {
            HudButtonAction::PlayAgain | HudButtonAction::ToHangar => battle_over,
            HudButtonAction::TopHangarShortcut => !cam_ctrl.cursor_locked,
            HudButtonAction::SelectAmmo(_)
            | HudButtonAction::UseFpe
            | HudButtonAction::ToggleRepair => !battle_over && player_alive && !cam_ctrl.cursor_locked,
        };
        let visible_size = computed.size().x > 1.0 && computed.size().y > 1.0;
        let hovered = interactive
            && visible_size
            && cursor_phys.is_some_and(|p| computed.contains_point(*ui_gtf, p));
        if hovered && clicked {
            match *action {
                HudButtonAction::PlayAgain => control.restart_requested = true,
                HudButtonAction::ToHangar | HudButtonAction::TopHangarShortcut => {
                    control.return_to_hangar = true;
                }
                HudButtonAction::SelectAmmo(slot) => {
                    player_input.select_shell = Some(slot);
                }
                HudButtonAction::UseFpe => {
                    player_input.use_fpe_queued = true;
                }
                HudButtonAction::ToggleRepair => {
                    player_input.toggle_repair_queued = true;
                    player_input.cruise_gear = 0;
                }
            }
        }
        match action {
            HudButtonAction::PlayAgain => {
                bg.0 = if hovered {
                    Color::srgb(0.95, 0.37, 0.16)
                } else {
                    Color::srgb(0.83, 0.29, 0.11)
                };
            }
            HudButtonAction::ToHangar => {
                bg.0 = if hovered {
                    Color::srgb(0.16, 0.52, 0.70)
                } else {
                    Color::srgb(0.11, 0.40, 0.56)
                };
            }
            HudButtonAction::TopHangarShortcut => {
                bg.0 = if hovered {
                    Color::srgba(0.18, 0.26, 0.34, 0.92)
                } else {
                    Color::srgba(0.10, 0.14, 0.18, 0.82)
                };
            }
            // Ammo, FPE, and Repair chips are dynamically styled by `update_hud_telemetry`
            HudButtonAction::SelectAmmo(_)
            | HudButtonAction::UseFpe
            | HudButtonAction::ToggleRepair => {}
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
