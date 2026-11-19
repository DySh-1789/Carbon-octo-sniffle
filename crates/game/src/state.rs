use bevy::prelude::*;
use sim::{Nation, VehicleCatalog};

/// Top-level application state: Industrial Hangar / Tech Tree vs live 6v6 Ground Battle.
#[derive(States, Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Hangar,
    InBattle,
}

/// Active sub-view inside the Hangar screen.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum HangarViewMode {
    /// WoT-style dark industrial 3D podium with bottom vehicle carousel & right TTX panel.
    #[default]
    Podium,
    /// War Thunder Mobile-style nation flag tabs & 8-tier horizontal tech tree overlay.
    TechTree,
}

/// Complete Stage 5 vehicle catalog (24 vehicles across USA, Germany, USSR × 8 tiers).
#[derive(Resource)]
pub struct CatalogResource(pub VehicleCatalog);

impl Default for CatalogResource {
    fn default() -> Self {
        Self(VehicleCatalog::stage5_catalog())
    }
}

/// Currently selected nation, vehicle, and Hangar UI mode.
#[derive(Resource, Clone, Debug)]
pub struct SelectedVehicle {
    pub nation: Nation,
    pub vehicle_id: String,
    pub view_mode: HangarViewMode,
    /// Set whenever `vehicle_id` changes so the 3D Hangar showcase tank rebuilds once.
    pub preview_dirty: bool,
}

impl Default for SelectedVehicle {
    fn default() -> Self {
        Self {
            nation: Nation::Usa,
            vehicle_id: "us_m4a1".to_string(),
            view_mode: HangarViewMode::Podium,
            preview_dirty: true,
        }
    }
}

/// Marker for all entities spawned exclusively for the Hangar 3D bay & UI (despawned on exit).
#[derive(Component)]
pub struct HangarEntity;

/// Marker for all entities spawned exclusively for a live Battle (despawned on exit).
#[derive(Component)]
pub struct BattleEntity;

/// Shared PBR material palette for authentic WWII national armor paints + charred wrecks.
/// Keeps total tank material count fixed at 5 handles across Hangar and 6v6 Battle.
#[derive(Resource)]
pub struct TankPaintMaterials {
    pub us_olive: Handle<StandardMaterial>,
    pub de_panzer_grey: Handle<StandardMaterial>,
    pub de_dunkelgelb: Handle<StandardMaterial>,
    pub su_4bo_green: Handle<StandardMaterial>,
    pub wreck_charred: Handle<StandardMaterial>,
}

impl TankPaintMaterials {
    pub fn material_for(&self, nation: Nation, tier: u8) -> Handle<StandardMaterial> {
        match nation {
            Nation::Usa => self.us_olive.clone(),
            Nation::Germany => {
                if tier <= 3 {
                    self.de_panzer_grey.clone()
                } else {
                    self.de_dunkelgelb.clone()
                }
            }
            Nation::Ussr => self.su_4bo_green.clone(),
        }
    }
}

/// Accent UI color per nation (used on borders, headers, and tech tree cards).
pub fn nation_accent_color(nation: Nation) -> Color {
    match nation {
        Nation::Usa => Color::srgb(0.42, 0.70, 0.96),
        Nation::Germany => Color::srgb(0.90, 0.72, 0.34),
        Nation::Ussr => Color::srgb(0.92, 0.36, 0.32),
    }
}

/// Spawns the 5 shared national & wreck PBR tank materials once on Startup.
pub fn setup_tank_paint_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let us_olive = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.96, 1.00, 0.84),
        perceptual_roughness: 0.66,
        metallic: 0.24,
        reflectance: 0.34,
        ..default()
    });
    let de_panzer_grey = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.68, 0.74, 0.86),
        perceptual_roughness: 0.60,
        metallic: 0.32,
        reflectance: 0.38,
        ..default()
    });
    let de_dunkelgelb = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(1.48, 1.24, 0.78),
        perceptual_roughness: 0.64,
        metallic: 0.26,
        reflectance: 0.35,
        ..default()
    });
    let su_4bo_green = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.76, 1.12, 0.66),
        perceptual_roughness: 0.68,
        metallic: 0.22,
        reflectance: 0.32,
        ..default()
    });
    let wreck_charred = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.20, 0.19, 0.19),
        perceptual_roughness: 0.94,
        metallic: 0.12,
        reflectance: 0.15,
        ..default()
    });

    commands.insert_resource(TankPaintMaterials {
        us_olive,
        de_panzer_grey,
        de_dunkelgelb,
        su_4bo_green,
        wreck_charred,
    });
}

/// Generic cleanup system for despawning all entities tagged with `HangarEntity` or `BattleEntity`.
pub fn despawn_tagged<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}
