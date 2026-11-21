use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use sim::{MapId, Nation, VehicleOwnership, VehicleSpec, economy_cost_for_tier, tier_roman};
use voxel::mesh_voxel_model;

use crate::bridge::{SimResource, to_bevy_vec3};
use crate::flags::FlagTextures;
use crate::state::{
    AppState, CatalogResource, EconomyResource, HangarEntity, HangarViewMode, SelectedVehicle,
    TankPaintMaterials, nation_accent_color,
};
use crate::voxel_render::voxel_mesh_to_bevy;

#[derive(Component)]
struct HangarCamera;

#[derive(Component)]
struct HangarShowcaseTank;

/// Interactive orbit controller for inspecting the 3D voxel tank on the Hangar podium.
#[derive(Resource)]
struct HangarOrbitState {
    yaw: f32,
    pitch: f32,
    distance: f32,
    auto_rotate: bool,
}

impl Default for HangarOrbitState {
    fn default() -> Self {
        Self {
            yaw: 0.48,
            pitch: 0.21,
            distance: 8.8,
            auto_rotate: true,
        }
    }
}

/// Interactive button actions across both the Industrial Hangar and Tech Tree overlays.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HangarAction {
    EnterBattle,
    SetViewMode(HangarViewMode),
    SelectNation(Nation),
    SelectTierSlot(u8),
    PrevMap,
    NextMap,
}

/// Visual styling category for an interactive `HangarAction` node.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HangarButtonStyle {
    BattlePrimary,
    SecondaryCyan,
    RailTab,
    NationTab,
    VehicleCard,
}

/// View-mode overlay visibility marker (`Podium` vs `TechTree`).
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct HangarLayer(HangarViewMode);

/// Unified dynamic text slots so one single query refreshes the entire Hangar & Tech Tree UI.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HangarDynamicText {
    PrimaryActionLabel,
    TopVehicleBanner,
    MapSelectorLabel,
    EconomyNotice,
    WalletSummary,
    SpecsTitle,
    SpecsFirepower,
    SpecsProtection,
    SpecsMobility,
    CarouselTitle(u8),
    CarouselSub(u8),
    TechTreeHeader,
    TechCardTitle(u8),
    TechCardSub(u8),
    TechSpecTitle,
    TechSpecBody,
}

pub struct HangarPlugin;

impl Plugin for HangarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HangarOrbitState>()
            .add_systems(
                OnEnter(AppState::Hangar),
                (setup_hangar_scene, setup_hangar_ui, sync_hangar_ui_content).chain(),
            )
            .add_systems(
                Update,
                (
                    rebuild_showcase_tank_if_dirty,
                    update_hangar_orbit_camera,
                    handle_hangar_ui_interaction,
                    sync_hangar_ui_content,
                )
                    .chain()
                    .run_if(in_state(AppState::Hangar)),
            );
    }
}

fn setup_hangar_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut orbit: ResMut<HangarOrbitState>,
    mut selected: ResMut<SelectedVehicle>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // Always unlock cursor on Hangar entry
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }

    orbit.yaw = 0.48;
    orbit.pitch = 0.21;
    orbit.distance = 8.8;
    orbit.auto_rotate = true;
    selected.preview_dirty = true;

    // Dark industrial steel hangar mood (inspired by WoT 2.0 bay)
    commands.insert_resource(ClearColor(Color::srgb(0.055, 0.068, 0.085)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.56, 0.64, 0.76),
        brightness: 280.0,
        affects_lightmapped_meshes: true,
    });

    // Overhead warm tungsten key light illuminating the showcase tank
    commands.spawn((
        HangarEntity,
        DirectionalLight {
            illuminance: 13_500.0,
            shadow_maps_enabled: false,
            color: Color::srgb(1.0, 0.92, 0.78),
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 10.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));

    // Cool steel rim light from rear-left + warm industrial fill from front-right
    commands.spawn((
        HangarEntity,
        PointLight {
            color: Color::srgb(0.45, 0.68, 1.0),
            intensity: 480_000.0,
            range: 28.0,
            ..default()
        },
        Transform::from_xyz(-8.5, 5.5, -7.5),
    ));
    commands.spawn((
        HangarEntity,
        PointLight {
            color: Color::srgb(1.0, 0.72, 0.42),
            intensity: 340_000.0,
            range: 24.0,
            ..default()
        },
        Transform::from_xyz(7.5, 4.2, 7.0),
    ));
    // Soft lower front-fill key light illuminating frontal glacis, driver visor, bow MG, headlights & mantlet
    commands.spawn((
        HangarEntity,
        PointLight {
            color: Color::srgb(0.98, 0.94, 0.88),
            intensity: 280_000.0,
            range: 22.0,
            ..default()
        },
        Transform::from_xyz(-2.0, 2.2, 6.8),
    ));

    // Hangar bay architecture: dark concrete floor, circular steel turntable podium, glowing amber ring & I-beam pillars
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.13, 0.15),
        perceptual_roughness: 0.84,
        metallic: 0.18,
        ..default()
    });
    let podium_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.17, 0.19, 0.22),
        perceptual_roughness: 0.56,
        metallic: 0.48,
        ..default()
    });
    let rim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.46, 0.16),
        emissive: LinearRgba::new(2.2, 0.85, 0.18, 1.0),
        unlit: true,
        ..default()
    });
    let steel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.15, 0.18),
        perceptual_roughness: 0.72,
        metallic: 0.52,
        ..default()
    });

    // Main floor slab
    commands.spawn((
        HangarEntity,
        Mesh3d(meshes.add(Cuboid::new(48.0, 0.2, 48.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    // Circular armored vehicle turntable podium (top surface at Y = 0.24)
    commands.spawn((
        HangarEntity,
        Mesh3d(meshes.add(Cylinder::new(4.9, 0.24))),
        MeshMaterial3d(podium_mat),
        Transform::from_xyz(0.0, 0.12, 0.0),
    ));
    // Subtle industrial orange hazard ring under podium rim
    commands.spawn((
        HangarEntity,
        Mesh3d(meshes.add(Cylinder::new(5.06, 0.06))),
        MeshMaterial3d(rim_mat),
        Transform::from_xyz(0.0, 0.04, 0.0),
    ));

    // Industrial hangar structural steel columns & rear bay wall framing the backdrop
    let pillar_mesh = meshes.add(Cuboid::new(0.85, 11.0, 0.85));
    for &(px, pz) in &[
        (-11.0, -10.0),
        (-4.5, -12.5),
        (4.5, -12.5),
        (11.0, -10.0),
        (-13.0, 0.0),
        (13.0, 0.0),
        (-11.0, 10.0),
        (11.0, 10.0),
    ] {
        commands.spawn((
            HangarEntity,
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(steel_mat.clone()),
            Transform::from_xyz(px, 5.5, pz),
        ));
    }
    // Overhead steel gantry beam
    commands.spawn((
        HangarEntity,
        Mesh3d(meshes.add(Cuboid::new(26.0, 0.75, 0.75))),
        MeshMaterial3d(steel_mat.clone()),
        Transform::from_xyz(0.0, 9.6, -11.5),
    ));
    commands.spawn((
        HangarEntity,
        Mesh3d(meshes.add(Cuboid::new(34.0, 12.0, 0.6))),
        MeshMaterial3d(steel_mat),
        Transform::from_xyz(0.0, 6.0, -15.5),
    ));

    // Hangar 3D Orbit Camera
    commands.spawn((
        HangarEntity,
        HangarCamera,
        Camera3d::default(),
        Msaa::Off,
        Projection::Perspective(PerspectiveProjection {
            fov: 46.0f32.to_radians(),
            near: 0.15,
            far: 140.0,
            ..default()
        }),
        Transform::from_xyz(3.97, 2.98, 7.63).looking_at(Vec3::new(0.0, 1.15, 0.0), Vec3::Y),
    ));
}

/// Rebuilds the 3D voxel tank sitting on the Hangar podium whenever the user selects another vehicle.
fn rebuild_showcase_tank_if_dirty(
    mut commands: Commands,
    catalog: Res<CatalogResource>,
    sim: Res<SimResource>,
    paints: Res<TankPaintMaterials>,
    mut selected: ResMut<SelectedVehicle>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing_q: Query<Entity, With<HangarShowcaseTank>>,
) {
    if !selected.preview_dirty {
        return;
    }
    selected.preview_dirty = false;

    for entity in &existing_q {
        commands.entity(entity).despawn();
    }

    let spec = catalog
        .0
        .by_id(&selected.vehicle_id)
        .unwrap_or_else(|| catalog.0.default_vehicle());
    let models = spec.build_models();
    let hull_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.hull,
        &sim.0.voxels.materials,
    )));
    let turret_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.turret,
        &sim.0.voxels.materials,
    )));
    let barrel_mesh = meshes.add(voxel_mesh_to_bevy(mesh_voxel_model(
        &models.barrel,
        &sim.0.voxels.materials,
    )));
    let mat = paints.material_for(spec.nation, spec.tier);

    // Slight heroic turret turn (for turreted tanks) toward the camera so the silhouette pops on the podium
    let turret_pose_yaw = if spec.turret_yaw_limit_deg.is_some() {
        0.0
    } else {
        0.16
    };

    commands
        .spawn((
            HangarEntity,
            HangarShowcaseTank,
            // Tank local forward is -Z; rotate by PI - 0.35 so front glacis, visor, headlights & mantlet face the front-3/4 camera
            Transform::from_xyz(0.0, 0.24, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI - 0.35)),
            Visibility::default(),
        ))
        .with_children(|hull| {
            hull.spawn((
                Mesh3d(hull_mesh),
                MeshMaterial3d(mat.clone()),
                Transform::IDENTITY,
            ));
            hull.spawn((
                Transform::from_translation(to_bevy_vec3(models.turret_mount_offset))
                    .with_rotation(Quat::from_rotation_y(turret_pose_yaw)),
                Visibility::default(),
            ))
            .with_children(|turret| {
                turret.spawn((
                    Mesh3d(turret_mesh),
                    MeshMaterial3d(mat.clone()),
                    Transform::IDENTITY,
                ));
                turret.spawn((
                    Mesh3d(barrel_mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(to_bevy_vec3(models.gun_mount_offset)),
                ));
            });
        });
}

fn update_hangar_orbit_camera(
    time: Res<Time>,
    selected: Res<SelectedVehicle>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut orbit: ResMut<HangarOrbitState>,
    mut cam_q: Query<&mut Transform, With<HangarCamera>>,
) {
    let Ok(mut tf) = cam_q.single_mut() else {
        return;
    };

    let dragging = selected.view_mode == HangarViewMode::Podium
        && (mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Left));
    if dragging && mouse_motion.delta.length_squared() > 0.01 {
        orbit.auto_rotate = false;
        orbit.yaw -= mouse_motion.delta.x * 0.0065;
        orbit.pitch = (orbit.pitch + mouse_motion.delta.y * 0.005).clamp(0.08, 0.85);
    } else if orbit.auto_rotate {
        orbit.yaw += time.delta_secs() * 0.18;
    }

    if selected.view_mode == HangarViewMode::Podium && mouse_scroll.delta.y.abs() > 0.01 {
        orbit.distance = (orbit.distance - mouse_scroll.delta.y * 0.65).clamp(6.2, 16.5);
    }

    // Offset pivot slightly downward so the tank frames nicely between top BATTLE button & bottom carousel
    let pivot = Vec3::new(0.0, 1.15, 0.0);
    let cos_p = orbit.pitch.cos();
    let offset = Vec3::new(
        orbit.yaw.sin() * cos_p * orbit.distance,
        orbit.pitch.sin() * orbit.distance,
        orbit.yaw.cos() * cos_p * orbit.distance,
    );
    tf.translation = pivot + offset;
    tf.look_at(pivot, Vec3::Y);
}

fn spawn_mini_flag(
    parent: &mut ChildSpawnerCommands,
    flags: &FlagTextures,
    nation: Nation,
    width: f32,
    height: f32,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(width + 2.0),
                height: Val::Px(height + 2.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgba(0.86, 0.90, 0.96, 0.58)),
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.95)),
        ))
        .with_children(|frame| {
            frame.spawn((
                Node {
                    width: Val::Px(width),
                    height: Val::Px(height),
                    ..default()
                },
                ImageNode::new(flags.handle(nation)),
            ));
        });
}

fn setup_hangar_ui(
    mut commands: Commands,
    flags: Option<Res<FlagTextures>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Bevy 0.20 runs initial `OnEnter(AppState::Hangar)` before `Startup`, so ensure flag textures exist immediately.
    let flags = match flags {
        Some(existing) => existing.clone(),
        None => {
            let created = FlagTextures::from_images(&mut images);
            commands.insert_resource(created.clone());
            created
        }
    };
    // =========================================================================
    // LAYER 1: INDUSTRIAL HANGAR PODIUM OVERLAY (matching WoT 2.0 reference)
    // =========================================================================
    commands
        .spawn((
            HangarEntity,
            HangarLayer(HangarViewMode::Podium),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // 1a. Top-Center Burnt-Orange BATTLE! Button + Vehicle Title Banner
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    HangarAction::EnterBattle,
                    HangarButtonStyle::BattlePrimary,
                    Node {
                        width: Val::Px(248.0),
                        height: Val::Px(46.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(1.0, 0.64, 0.34)),
                    BackgroundColor(Color::srgb(0.83, 0.29, 0.11)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        HangarDynamicText::PrimaryActionLabel,
                        Text::new("BATTLE!"),
                        TextFont {
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

                top.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.65, 0.70, 0.78, 0.35)),
                    BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.78)),
                ))
                .with_children(|banner| {
                    banner.spawn((
                        HangarDynamicText::TopVehicleBanner,
                        Text::new("USA | TIER I | M2A4 | LIGHT TANK"),
                        TextFont {
                            font_size: FontSize::Px(12.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.94, 0.88)),
                    ));
                });

                // Compact 5-map battlefield selector (< MAP: EUROPEAN VALLEY >)
                top.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|map_row| {
                    map_row
                        .spawn((
                            HangarAction::PrevMap,
                            HangarButtonStyle::RailTab,
                            Node {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.42, 0.48, 0.56, 0.55)),
                            BackgroundColor(Color::srgba(0.08, 0.11, 0.15, 0.86)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("<"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    map_row
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.45, 0.62, 0.78, 0.45)),
                            BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.82)),
                        ))
                        .with_children(|lbl| {
                            lbl.spawn((
                                HangarDynamicText::MapSelectorLabel,
                                Text::new("MAP 1/5: EUROPEAN VALLEY (TEMPERATE VILLAGE)"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.68, 0.88, 0.98)),
                            ));
                        });

                    map_row
                        .spawn((
                            HangarAction::NextMap,
                            HangarButtonStyle::RailTab,
                            Node {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.42, 0.48, 0.56, 0.55)),
                            BackgroundColor(Color::srgba(0.08, 0.11, 0.15, 0.86)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(">"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });

                // Subtle Update 0.1 Economy notice banner (last battle receipt / research & purchase feedback)
                top.spawn((
                    HangarDynamicText::EconomyNotice,
                    Text::new("UPDATE 0.1 ECONOMY | FREE TIER I RESERVES READY"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.82, 0.44)),
                    BackgroundColor(Color::srgba(0.04, 0.06, 0.09, 0.72)),
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(2.0)),
                        ..default()
                    },
                ));
            });

            // 1b. Left Vertical Navigation Rail (HANGAR / TECH TREE / WALLET / NATIONS)
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    top: Val::Px(14.0),
                    width: Val::Px(178.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.42, 0.48, 0.56, 0.42)),
                BackgroundColor(Color::srgba(0.06, 0.08, 0.11, 0.86)),
            ))
            .with_children(|rail| {
                rail.spawn((
                    Text::new("CARBON ARMOR"),
                    TextFont {
                        font_size: FontSize::Px(14.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.74, 0.32)),
                ));
                rail.spawn((
                    Text::new("UPDATE 0.1 | ECONOMY"),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.56, 0.82, 0.96)),
                ));

                // Compact Update 0.1 Economy Wallet card (CR + Nation RP + Career)
                rail.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(7.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.86, 0.70, 0.30, 0.52)),
                    BackgroundColor(Color::srgba(0.09, 0.11, 0.14, 0.92)),
                ))
                .with_children(|wallet| {
                    wallet.spawn((
                        HangarDynamicText::WalletSummary,
                        Text::new("CR: 2500\nUSA RP: 0\nBTL 0 | WIN 0 | KILLS 0"),
                        TextFont {
                            font_size: FontSize::Px(10.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.86, 0.50)),
                    ));
                });

                // Navigation Tabs
                for (label, mode) in [
                    ("HANGAR", HangarViewMode::Podium),
                    ("TECH TREE", HangarViewMode::TechTree),
                ] {
                    rail.spawn((
                        HangarAction::SetViewMode(mode),
                        HangarButtonStyle::RailTab,
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                            margin: UiRect::top(Val::Px(2.0)),
                            border: UiRect::left(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgb(0.92, 0.46, 0.16)),
                        BackgroundColor(Color::srgba(0.14, 0.18, 0.24, 0.90)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                }

                // Divider label
                rail.spawn((
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                    Text::new("NATIONS"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.60, 0.66, 0.74)),
                ));

                for nation in Nation::ALL {
                    rail.spawn((
                        HangarAction::SelectNation(nation),
                        HangarButtonStyle::NationTab,
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(7.0)),
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.35, 0.42, 0.50, 0.55)),
                        BackgroundColor(Color::srgba(0.10, 0.13, 0.17, 0.88)),
                    ))
                    .with_children(|btn| {
                        spawn_mini_flag(btn, &flags, nation, 28.0, 18.0);
                        btn.spawn((
                            Text::new(nation.label()),
                            TextFont {
                                font_size: FontSize::Px(12.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.92, 0.94, 0.96)),
                        ));
                    });
                }

                rail.spawn((
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                    Text::new("Drag: Orbit 3D\nScroll: Zoom\nEnter: Battle"),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.56, 0.62, 0.70)),
                ));
            });

            // 1c. Right Translucent Vehicle Specifications (TTX) Card
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    width: Val::Px(286.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.45, 0.52, 0.60, 0.45)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.84)),
            ))
            .with_children(|spec| {
                spec.spawn((
                    HangarDynamicText::SpecsTitle,
                    Text::new("M4A1 SHERMAN"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.82, 0.42)),
                ));

                spec.spawn((
                    Text::new("FIREPOWER"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.58, 0.28)),
                ));
                spec.spawn((
                    HangarDynamicText::SpecsFirepower,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.91, 0.94)),
                ));

                spec.spawn((
                    Text::new("PROTECTION"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.46, 0.80, 0.98)),
                ));
                spec.spawn((
                    HangarDynamicText::SpecsProtection,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.91, 0.94)),
                ));

                spec.spawn((
                    Text::new("MOBILITY & CREW"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.48, 0.90, 0.56)),
                ));
                spec.spawn((
                    HangarDynamicText::SpecsMobility,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.91, 0.94)),
                ));
            });

            // 1d. Bottom Horizontal 8-Tier Vehicle Carousel (matching WoT 2.0 dock)
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    right: Val::Px(14.0),
                    bottom: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    column_gap: Val::Px(6.0),
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.42, 0.48, 0.56, 0.42)),
                BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.88)),
            ))
            .with_children(|dock| {
                for tier in 1u8..=8 {
                    dock.spawn((
                        HangarAction::SelectTierSlot(tier),
                        HangarButtonStyle::VehicleCard,
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            height: Val::Px(74.0),
                            padding: UiRect::all(Val::Px(7.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.36, 0.42, 0.50, 0.65)),
                        BackgroundColor(Color::srgba(0.10, 0.13, 0.17, 0.92)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            HangarDynamicText::CarouselTitle(tier),
                            Text::new(format!("{}  TANK", tier_roman(tier))),
                            TextFont {
                                font_size: FontSize::Px(11.5),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        card.spawn((
                            HangarDynamicText::CarouselSub(tier),
                            Text::new("MT | 75mm"),
                            TextFont {
                                font_size: FontSize::Px(10.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.76, 0.82)),
                        ));
                    });
                }
            });
        });

    // =========================================================================
    // LAYER 2: WAR THUNDER MOBILE TECH TREE OVERLAY (matching images.jpg)
    // =========================================================================
    commands
        .spawn((
            HangarEntity,
            HangarLayer(HangarViewMode::TechTree),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                column_gap: Val::Px(14.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.09, 0.90)),
        ))
        .with_children(|root| {
            // 2a. Left Vertical Nation Flag Tabs (USA / GERMANY / USSR)
            root.spawn((
                Node {
                    width: Val::Px(148.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.36, 0.46, 0.56, 0.50)),
                BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.92)),
            ))
            .with_children(|tabs| {
                tabs.spawn((
                    Text::new("TECH TREE"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.45, 0.84, 0.98)),
                ));

                for nation in Nation::ALL {
                    tabs.spawn((
                        HangarAction::SelectNation(nation),
                        HangarButtonStyle::NationTab,
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.36, 0.44, 0.52, 0.65)),
                        BackgroundColor(Color::srgba(0.10, 0.14, 0.19, 0.92)),
                    ))
                    .with_children(|btn| {
                        spawn_mini_flag(btn, &flags, nation, 64.0, 40.0);
                        btn.spawn((
                            Text::new(nation.label()),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                }

                // Return to 3D Hangar button at bottom of tab rail
                tabs.spawn((
                    HangarAction::SetViewMode(HangarViewMode::Podium),
                    HangarButtonStyle::SecondaryCyan,
                    Node {
                        width: Val::Percent(100.0),
                        margin: UiRect::top(Val::Auto),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.35, 0.78, 0.96)),
                    BackgroundColor(Color::srgb(0.11, 0.36, 0.50)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("< HANGAR"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            });

            // 2b. Center Connected 8-Tier Progression Tree (I -> IV and V -> VIII)
            root.spawn((
                Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.36, 0.46, 0.56, 0.45)),
                BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.82)),
            ))
            .with_children(|tree| {
                tree.spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|hdr| {
                    hdr.spawn((
                        HangarDynamicText::TechTreeHeader,
                        Text::new("USA ARMORED FORCES  |  TIERS I - VIII"),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.84, 0.44)),
                    ));
                    hdr.spawn((
                        HangarDynamicText::WalletSummary,
                        Text::new("CR: 2500 | USA RP: 0"),
                        TextFont {
                            font_size: FontSize::Px(11.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.86, 0.50)),
                    ));
                });

                tree.spawn((
                    HangarDynamicText::EconomyNotice,
                    Text::new("UPDATE 0.1 ECONOMY | FREE TIER I RESERVES READY"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.68, 0.88, 0.98)),
                ));

                for (row_label, tiers) in [
                    ("EARLY & MID WAR  (TIERS I - IV)", [1u8, 2, 3, 4]),
                    ("LATE WAR & HEAVY  (TIERS V - VIII)", [5u8, 6, 7, 8]),
                ] {
                    tree.spawn((
                        Text::new(row_label),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.62, 0.70, 0.80)),
                    ));

                    tree.spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (idx, tier) in tiers.iter().copied().enumerate() {
                            row.spawn((
                                HangarAction::SelectTierSlot(tier),
                                HangarButtonStyle::VehicleCard,
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: Val::Px(0.0),
                                    height: Val::Px(134.0),
                                    padding: UiRect::all(Val::Px(10.0)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::SpaceBetween,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgba(0.38, 0.48, 0.58, 0.65)),
                                BackgroundColor(Color::srgba(0.10, 0.14, 0.19, 0.94)),
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    HangarDynamicText::TechCardTitle(tier),
                                    Text::new(format!("TIER {}", tier_roman(tier))),
                                    TextFont {
                                        font_size: FontSize::Px(13.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                                card.spawn((
                                    HangarDynamicText::TechCardSub(tier),
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(10.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.74, 0.80, 0.86)),
                                ));
                            });

                            if idx < 3 {
                                row.spawn((
                                    Node {
                                        width: Val::Px(16.0),
                                        height: Val::Px(3.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.48, 0.64, 0.80, 0.65)),
                                ));
                            }
                        }
                    });
                }
            });

            // 2c. Right Spec Preview Card + VIEW IN HANGAR & PRIMARY ACTION button (matching images.jpg)
            root.spawn((
                Node {
                    width: Val::Px(296.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.38, 0.48, 0.58, 0.52)),
                BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.94)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    HangarDynamicText::TechSpecTitle,
                    Text::new("M2A4"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.82, 0.42)),
                ));

                panel.spawn((
                    HangarDynamicText::TechSpecBody,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.92, 0.96)),
                ));

                // Action buttons docked at bottom
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        margin: UiRect::top(Val::Auto),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|btns| {
                        btns.spawn((
                            HangarAction::SetViewMode(HangarViewMode::Podium),
                            HangarButtonStyle::SecondaryCyan,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.36, 0.82, 0.98)),
                            BackgroundColor(Color::srgb(0.11, 0.42, 0.58)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("VIEW IN HANGAR"),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                        btns.spawn((
                            HangarAction::EnterBattle,
                            HangarButtonStyle::BattlePrimary,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(46.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(1.0, 0.64, 0.34)),
                            BackgroundColor(Color::srgb(0.83, 0.29, 0.11)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                HangarDynamicText::PrimaryActionLabel,
                                Text::new("BATTLE!"),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_hangar_ui_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    catalog: Res<CatalogResource>,
    mut economy: ResMut<EconomyResource>,
    mut selected: ResMut<SelectedVehicle>,
    mut sim: ResMut<SimResource>,
    mut next_state: ResMut<NextState<AppState>>,
    mut btn_q: Query<(
        &HangarAction,
        &HangarButtonStyle,
        &ComputedNode,
        &UiGlobalTransform,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let cursor_phys = window_q
        .single()
        .ok()
        .and_then(|w| w.physical_cursor_position());
    let clicked = mouse_buttons.just_pressed(MouseButton::Left);

    let current_spec = catalog
        .0
        .by_id(&selected.vehicle_id)
        .unwrap_or_else(|| catalog.0.default_vehicle());
    let current_tier = current_spec.tier;
    let current_ownership = economy
        .profile
        .ownership_state(&catalog.0, &selected.vehicle_id);
    let can_research_curr = economy
        .profile
        .can_research(&catalog.0, &selected.vehicle_id);
    let can_buy_curr = economy.profile.can_buy(&catalog.0, &selected.vehicle_id);

    let mut trigger_primary = keys.just_pressed(KeyCode::Enter);

    if keys.just_pressed(KeyCode::Tab) {
        selected.view_mode = match selected.view_mode {
            HangarViewMode::Podium => HangarViewMode::TechTree,
            HangarViewMode::TechTree => HangarViewMode::Podium,
        };
    }

    let mut pending_action: Option<HangarAction> = None;

    for (action, style, computed, ui_gtf, mut bg, mut border) in &mut btn_q {
        let hovered = cursor_phys.is_some_and(|p| computed.contains_point(*ui_gtf, p));
        if hovered && clicked {
            pending_action = Some(*action);
        }

        let is_active = match *action {
            HangarAction::SetViewMode(m) => selected.view_mode == m,
            HangarAction::SelectNation(n) => selected.nation == n,
            HangarAction::SelectTierSlot(t) => current_tier == t,
            HangarAction::EnterBattle | HangarAction::PrevMap | HangarAction::NextMap => false,
        };

        let (fill, rim) = match style {
            HangarButtonStyle::BattlePrimary => match current_ownership {
                VehicleOwnership::Owned => {
                    if hovered {
                        (Color::srgb(0.95, 0.37, 0.16), Color::srgb(1.0, 0.80, 0.48))
                    } else {
                        (Color::srgb(0.83, 0.29, 0.11), Color::srgb(1.0, 0.64, 0.34))
                    }
                }
                VehicleOwnership::Researched => {
                    if can_buy_curr {
                        if hovered {
                            (Color::srgb(0.82, 0.62, 0.14), Color::srgb(1.0, 0.90, 0.48))
                        } else {
                            (Color::srgb(0.66, 0.48, 0.10), Color::srgb(0.96, 0.80, 0.34))
                        }
                    } else {
                        (Color::srgb(0.28, 0.24, 0.16), Color::srgb(0.56, 0.48, 0.30))
                    }
                }
                VehicleOwnership::Researchable => {
                    if can_research_curr {
                        if hovered {
                            (Color::srgb(0.14, 0.56, 0.76), Color::srgb(0.58, 0.94, 1.0))
                        } else {
                            (Color::srgb(0.10, 0.42, 0.60), Color::srgb(0.36, 0.82, 0.98))
                        }
                    } else {
                        (Color::srgb(0.15, 0.22, 0.28), Color::srgb(0.34, 0.48, 0.58))
                    }
                }
                VehicleOwnership::Locked => {
                    (Color::srgb(0.16, 0.18, 0.21), Color::srgb(0.36, 0.40, 0.46))
                }
            },
            HangarButtonStyle::SecondaryCyan => {
                if hovered {
                    (Color::srgb(0.16, 0.54, 0.72), Color::srgb(0.56, 0.92, 1.0))
                } else {
                    (Color::srgb(0.11, 0.42, 0.58), Color::srgb(0.36, 0.82, 0.98))
                }
            }
            HangarButtonStyle::RailTab => {
                if is_active {
                    (
                        Color::srgba(0.22, 0.27, 0.34, 0.96),
                        Color::srgb(0.96, 0.56, 0.20),
                    )
                } else if hovered {
                    (
                        Color::srgba(0.16, 0.20, 0.27, 0.92),
                        Color::srgba(0.65, 0.72, 0.82, 0.75),
                    )
                } else {
                    (
                        Color::srgba(0.10, 0.13, 0.18, 0.85),
                        Color::srgba(0.32, 0.38, 0.46, 0.45),
                    )
                }
            }
            HangarButtonStyle::NationTab => {
                let n = match *action {
                    HangarAction::SelectNation(n) => n,
                    _ => selected.nation,
                };
                if is_active {
                    (
                        Color::srgba(0.18, 0.24, 0.32, 0.96),
                        nation_accent_color(n),
                    )
                } else if hovered {
                    (
                        Color::srgba(0.14, 0.19, 0.26, 0.92),
                        Color::srgba(0.62, 0.70, 0.80, 0.75),
                    )
                } else {
                    (
                        Color::srgba(0.09, 0.12, 0.16, 0.88),
                        Color::srgba(0.32, 0.38, 0.46, 0.50),
                    )
                }
            }
            HangarButtonStyle::VehicleCard => {
                let slot_state = if let HangarAction::SelectTierSlot(t) = *action {
                    catalog
                        .0
                        .by_nation_and_tier(selected.nation, t)
                        .map_or(VehicleOwnership::Locked, |v| {
                            economy.profile.ownership_state(&catalog.0, &v.id)
                        })
                } else {
                    VehicleOwnership::Locked
                };
                if is_active {
                    (
                        Color::srgba(0.24, 0.20, 0.13, 0.96),
                        Color::srgb(0.98, 0.66, 0.22),
                    )
                } else if hovered {
                    (
                        Color::srgba(0.15, 0.21, 0.28, 0.95),
                        Color::srgb(0.48, 0.78, 0.96),
                    )
                } else {
                    match slot_state {
                        VehicleOwnership::Owned => (
                            Color::srgba(0.09, 0.16, 0.13, 0.94),
                            Color::srgba(0.30, 0.68, 0.42, 0.68),
                        ),
                        VehicleOwnership::Researched => (
                            Color::srgba(0.16, 0.14, 0.08, 0.94),
                            Color::srgba(0.86, 0.70, 0.28, 0.72),
                        ),
                        VehicleOwnership::Researchable => (
                            Color::srgba(0.08, 0.13, 0.18, 0.94),
                            Color::srgba(0.34, 0.64, 0.82, 0.68),
                        ),
                        VehicleOwnership::Locked => (
                            Color::srgba(0.08, 0.09, 0.12, 0.90),
                            Color::srgba(0.28, 0.32, 0.38, 0.48),
                        ),
                    }
                }
            }
        };

        bg.0 = fill;
        *border = BorderColor::all(rim);
    }

    if let Some(action) = pending_action {
        match action {
            HangarAction::EnterBattle => {
                trigger_primary = true;
            }
            HangarAction::SetViewMode(mode) => {
                selected.view_mode = mode;
            }
            HangarAction::SelectNation(nation) => {
                if selected.nation != nation {
                    selected.nation = nation;
                    if let Some(spec) = catalog.0.by_nation_and_tier(nation, current_tier) {
                        selected.vehicle_id = spec.id.clone();
                        selected.preview_dirty = true;
                    }
                }
            }
            HangarAction::SelectTierSlot(tier) => {
                if let Some(spec) = catalog.0.by_nation_and_tier(selected.nation, tier)
                    && selected.vehicle_id != spec.id
                {
                    selected.vehicle_id = spec.id.clone();
                    selected.preview_dirty = true;
                }
            }
            HangarAction::PrevMap => {
                selected.map_id = selected.map_id.prev();
            }
            HangarAction::NextMap => {
                selected.map_id = selected.map_id.next();
            }
        }
    }

    if trigger_primary {
        let vid = selected.vehicle_id.clone();
        let spec_name = current_spec.name.to_uppercase();
        let spec_tier = current_spec.tier;
        let spec_nation = current_spec.nation;
        let cost = economy_cost_for_tier(spec_tier);
        match economy.profile.ownership_state(&catalog.0, &vid) {
            VehicleOwnership::Owned => {
                sim.0
                    .configure_battle_on_map(selected.map_id, &catalog.0, &vid, 6);
                next_state.set(AppState::InBattle);
            }
            VehicleOwnership::Researched => match economy.profile.buy_vehicle(&catalog.0, &vid) {
                Ok(spent) => {
                    economy.status_notice =
                        format!("PURCHASED {} (-{} CR) | READY FOR BATTLE!", spec_name, spent);
                    economy.save_to_disk();
                }
                Err(_) => {
                    let missing = (cost.cr_cost - economy.profile.credits).max(0);
                    economy.status_notice =
                        format!("NEED {} MORE CR TO BUY {}", missing, spec_name);
                }
            },
            VehicleOwnership::Researchable => {
                match economy.profile.research_vehicle(&catalog.0, &vid) {
                    Ok(spent) => {
                        economy.status_notice = format!(
                            "RESEARCHED {} (-{} {} RP) | BUY FOR {} CR",
                            spec_name,
                            spent,
                            spec_nation.short_tag(),
                            cost.cr_cost,
                        );
                        economy.save_to_disk();
                    }
                    Err(_) => {
                        let missing = cost
                            .rp_cost
                            .saturating_sub(economy.profile.rp_for(spec_nation));
                        economy.status_notice = format!(
                            "NEED {} MORE {} RP TO RESEARCH {}",
                            missing,
                            spec_nation.short_tag(),
                            spec_name,
                        );
                    }
                }
            }
            VehicleOwnership::Locked => {
                economy.status_notice = format!(
                    "LOCKED: OWN {} TIER {} FIRST",
                    spec_nation.short_tag(),
                    tier_roman(spec_tier.saturating_sub(1)),
                );
            }
        }
    }
}

fn sync_hangar_ui_content(
    catalog: Res<CatalogResource>,
    economy: Res<EconomyResource>,
    selected: Res<SelectedVehicle>,
    mut layer_q: Query<(&HangarLayer, &mut Node)>,
    mut text_q: Query<(&HangarDynamicText, &mut Text, &mut TextColor)>,
) {
    for (layer, mut node) in &mut layer_q {
        node.display = if layer.0 == selected.view_mode {
            Display::Flex
        } else {
            Display::None
        };
    }

    let spec = catalog
        .0
        .by_id(&selected.vehicle_id)
        .unwrap_or_else(|| catalog.0.default_vehicle());
    let accent = nation_accent_color(spec.nation);
    let sel_ownership = economy.profile.ownership_state(&catalog.0, &spec.id);
    let sel_cost = economy_cost_for_tier(spec.tier);
    let nation_rp = economy.profile.rp_for(selected.nation);

    for (slot, mut text, mut color) in &mut text_q {
        match *slot {
            HangarDynamicText::PrimaryActionLabel => match sel_ownership {
                VehicleOwnership::Owned => {
                    text.0 = "BATTLE!".to_string();
                    color.0 = Color::WHITE;
                }
                VehicleOwnership::Researched => {
                    text.0 = format!("BUY  {} CR", sel_cost.cr_cost);
                    color.0 = if economy.profile.credits >= sel_cost.cr_cost {
                        Color::WHITE
                    } else {
                        Color::srgb(0.86, 0.76, 0.56)
                    };
                }
                VehicleOwnership::Researchable => {
                    text.0 = format!("RESEARCH  {} RP", sel_cost.rp_cost);
                    color.0 = if economy.profile.rp_for(spec.nation) >= sel_cost.rp_cost {
                        Color::WHITE
                    } else {
                        Color::srgb(0.72, 0.82, 0.90)
                    };
                }
                VehicleOwnership::Locked => {
                    text.0 = format!("LOCKED (TIER {})", tier_roman(spec.tier.saturating_sub(1)));
                    color.0 = Color::srgb(0.70, 0.74, 0.78);
                }
            },
            HangarDynamicText::TopVehicleBanner => {
                text.0 = format!(
                    "{}  |  TIER {}  |  {}  |  {}  [{}]",
                    spec.nation.label(),
                    tier_roman(spec.tier),
                    spec.name.to_uppercase(),
                    spec.class.label().to_uppercase(),
                    sel_ownership.badge_label(),
                );
                color.0 = Color::srgb(0.96, 0.94, 0.88);
            }
            HangarDynamicText::MapSelectorLabel => {
                let idx = MapId::ALL
                    .iter()
                    .position(|&m| m == selected.map_id)
                    .unwrap_or(0)
                    + 1;
                text.0 = format!(
                    "MAP {}/{}: {} ({})",
                    idx,
                    MapId::ALL.len(),
                    selected.map_id.name().to_uppercase(),
                    selected.map_id.biome_tag(),
                );
            }
            HangarDynamicText::EconomyNotice => {
                text.0 = economy.status_notice.clone();
            }
            HangarDynamicText::WalletSummary => {
                text.0 = format!(
                    "CR: {}  |  {} RP: {}\nBTL {} | WIN {} | KILLS {}",
                    economy.profile.credits,
                    selected.nation.short_tag(),
                    nation_rp,
                    economy.profile.battles_played,
                    economy.profile.victories,
                    economy.profile.total_kills,
                );
            }
            HangarDynamicText::SpecsTitle | HangarDynamicText::TechSpecTitle => {
                text.0 = format!("{}  {}", tier_roman(spec.tier), spec.name);
                color.0 = accent;
            }
            HangarDynamicText::SpecsFirepower => {
                text.0 = format_firepower(spec);
            }
            HangarDynamicText::SpecsProtection => {
                text.0 = format_protection(spec);
            }
            HangarDynamicText::SpecsMobility => {
                let econ_line = if sel_cost.repair_cr == 0 {
                    "Economy: FREE TIER I RESERVE (0 CR repair)".to_string()
                } else {
                    format!(
                        "Economy: {} RP | {} CR | Repair {} CR",
                        sel_cost.rp_cost, sel_cost.cr_cost, sel_cost.repair_cr
                    )
                };
                text.0 = format!("{}\n{}", format_mobility(spec), econ_line);
            }
            HangarDynamicText::TechTreeHeader => {
                text.0 = format!(
                    "{} ARMORED FORCES  |  TIERS I - VIII",
                    selected.nation.label()
                );
                color.0 = nation_accent_color(selected.nation);
            }
            HangarDynamicText::TechSpecBody => {
                let econ_line = if sel_cost.repair_cr == 0 {
                    "FREE RESERVE (0 RP / 0 CR / 0 CR repair)".to_string()
                } else {
                    format!(
                        "Status: {}\nResearch: {} {} RP | Buy: {} CR\nMax Repair: {} CR",
                        sel_ownership.badge_label(),
                        sel_cost.rp_cost,
                        spec.nation.short_tag(),
                        sel_cost.cr_cost,
                        sel_cost.repair_cr,
                    )
                };
                text.0 = format!(
                    "{}  |  {}\n{}\n\n-- FIREPOWER --\n{}\n\n-- PROTECTION --\n{}\n\n-- MOBILITY & CREW --\n{}",
                    spec.nation.label(),
                    spec.class.label(),
                    econ_line,
                    format_firepower(spec),
                    format_protection(spec),
                    format_mobility(spec),
                );
            }
            HangarDynamicText::CarouselTitle(tier) => {
                if let Some(v) = catalog.0.by_nation_and_tier(selected.nation, tier) {
                    text.0 = format!("{}  {}", tier_roman(tier), v.name);
                    color.0 = if v.id == spec.id {
                        Color::srgb(1.0, 0.86, 0.45)
                    } else if economy.profile.is_owned(&v.id) {
                        Color::srgb(0.84, 0.98, 0.86)
                    } else {
                        Color::srgb(0.76, 0.80, 0.84)
                    };
                }
            }
            HangarDynamicText::CarouselSub(tier) => {
                if let Some(v) = catalog.0.by_nation_and_tier(selected.nation, tier) {
                    let loadout = v.ammo_loadout();
                    let primary = loadout.first().unwrap_or(&v.shell);
                    let cost = economy_cost_for_tier(tier);
                    let status_tag = match economy.profile.ownership_state(&catalog.0, &v.id) {
                        VehicleOwnership::Owned => "OWNED".to_string(),
                        VehicleOwnership::Researched => format!("BUY {} CR", cost.cr_cost),
                        VehicleOwnership::Researchable => format!("RES {} RP", cost.rp_cost),
                        VehicleOwnership::Locked => "LOCKED".to_string(),
                    };
                    text.0 = format!(
                        "{} | {:.0}mm {} | {}",
                        v.class.short_label(),
                        primary.caliber_mm,
                        primary.kind.short_label(),
                        status_tag,
                    );
                }
            }
            HangarDynamicText::TechCardTitle(tier) => {
                if let Some(v) = catalog.0.by_nation_and_tier(selected.nation, tier) {
                    text.0 = format!(
                        "TIER {}  [{}]\n{}",
                        tier_roman(tier),
                        v.class.short_label(),
                        v.name
                    );
                    color.0 = if v.id == spec.id {
                        Color::srgb(1.0, 0.86, 0.45)
                    } else if economy.profile.is_owned(&v.id) {
                        Color::srgb(0.84, 0.98, 0.86)
                    } else {
                        Color::srgb(0.82, 0.86, 0.90)
                    };
                }
            }
            HangarDynamicText::TechCardSub(tier) => {
                if let Some(v) = catalog.0.by_nation_and_tier(selected.nation, tier) {
                    let loadout = v.ammo_loadout();
                    let primary = loadout.first().unwrap_or(&v.shell);
                    let cost = economy_cost_for_tier(tier);
                    let econ_tag = match economy.profile.ownership_state(&catalog.0, &v.id) {
                        VehicleOwnership::Owned => "OWNED [READY]".to_string(),
                        VehicleOwnership::Researched => format!("BUY: {} CR", cost.cr_cost),
                        VehicleOwnership::Researchable => {
                            format!("RESEARCH: {} RP | {} CR", cost.rp_cost, cost.cr_cost)
                        }
                        VehicleOwnership::Locked => {
                            format!("LOCKED ({} RP / {} CR)", cost.rp_cost, cost.cr_cost)
                        }
                    };
                    text.0 = format!(
                        "Gun: {}\nPen: {:.0}mm {} | Reload {:.1}s\nUFP: {:.0}mm @{:.0} deg\n{}",
                        v.gun_name,
                        primary.penetration_0m_mm,
                        primary.kind.short_label(),
                        v.reload_time,
                        v.armor.ufp_mm,
                        v.armor.ufp_slope_deg,
                        econ_tag,
                    );
                }
            }
        }
    }
}

fn format_firepower(spec: &VehicleSpec) -> String {
    let loadout = spec.ammo_loadout();
    let primary = loadout.first().unwrap_or(&spec.shell);
    // Approximate 500m penetration using De Marre velocity decay
    let tof_500 = 500.0 / primary.muzzle_velocity.max(1.0);
    let v_500 = primary.muzzle_velocity * (-primary.drag_coeff * tof_500).exp();
    let pen_500 = primary.penetration_at_speed(v_500);
    let arc = match spec.turret_yaw_limit_deg {
        Some(lim) => format!("+-{:.0} deg Casemate", lim),
        None => format!("360 deg ({:.0} deg/s)", spec.turret_turn_deg_s),
    };
    let ammo_lines: Vec<String> = loadout
        .iter()
        .enumerate()
        .map(|(i, sh)| {
            if sh.is_he() {
                format!(
                    "[{}] {} ({:.0}mm HE | {:.2}kg TNT)",
                    i + 1,
                    sh.name,
                    sh.he_pen_mm,
                    sh.explosive_mass_kg
                )
            } else if sh.has_bursting_charge() {
                format!(
                    "[{}] {} ({} {:.0}mm | {:.0}g TNT)",
                    i + 1,
                    sh.name,
                    sh.kind.short_label(),
                    sh.penetration_0m_mm,
                    sh.explosive_mass_kg * 1000.0
                )
            } else {
                format!(
                    "[{}] {} ({} {:.0}mm)",
                    i + 1,
                    sh.name,
                    sh.kind.short_label(),
                    sh.penetration_0m_mm
                )
            }
        })
        .collect();
    format!(
        "Gun: {}\nPen: {:.0}mm (0m) / {:.0}mm (500m)\nReload: {:.1}s | Dep/Elv: {:+.0}/{:+.0} deg\nTraverse: {}\nAmmo:\n{}",
        spec.gun_name,
        primary.penetration_0m_mm,
        pen_500,
        spec.reload_time,
        spec.gun_min_deg,
        spec.gun_max_deg,
        arc,
        ammo_lines.join("\n"),
    )
}

fn format_protection(spec: &VehicleSpec) -> String {
    let a = &spec.armor;
    let nose_tag = if a.pike_nose { " [PIKE NOSE]" } else { "" };
    let turret_label = if a.is_casemate { "Casemate" } else { "Turret" };
    format!(
        "Hull UFP: {:.0}mm @ {:.0} deg ({}){}\nHull Side/Rear: {:.0} / {:.0} mm\n{} Front/Mantlet: {:.0} / {:.0} mm\n{} Side/Rear: {:.0} / {:.0} mm ({})",
        a.ufp_mm,
        a.ufp_slope_deg,
        a.hull_armor_type.label(),
        nose_tag,
        a.hull_side_mm,
        a.hull_rear_mm,
        turret_label,
        a.turret_front_mm,
        a.turret_mantlet_mm,
        turret_label,
        a.turret_side_mm,
        a.turret_rear_mm,
        a.turret_armor_type.label(),
    )
}

fn format_mobility(spec: &VehicleSpec) -> String {
    format!(
        "Speed Fwd / Rev: {:.0} / {:.0} km/h\nPower: {:.0} hp ({:.1} hp/t) | {:.1} t\nCrew: {} | Hull Turn: {:.0} deg/s",
        spec.max_forward_kmh,
        spec.max_reverse_kmh,
        spec.engine_hp,
        spec.power_to_weight(),
        spec.mass_tons,
        spec.armor.crew_count,
        spec.hull_turn_deg_s,
    )
}
