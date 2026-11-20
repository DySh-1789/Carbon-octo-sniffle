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

/// Shared PBR material palette for live armored vehicles + charred wrecks.
/// National armor paints, emblems, headlights, tracks, and tools are baked directly into
/// vertex colors at `0.10 m` voxel resolution so live tanks share a single neutral PBR material!
#[derive(Resource)]
pub struct TankPaintMaterials {
    pub live_armor: Handle<StandardMaterial>,
    pub wreck_charred: Handle<StandardMaterial>,
}

impl TankPaintMaterials {
    pub fn material_for(&self, _nation: Nation, _tier: u8) -> Handle<StandardMaterial> {
        self.live_armor.clone()
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

/// Spawns the 2 shared PBR tank materials (`live_armor` + `wreck_charred`) once on Startup.
pub fn setup_tank_paint_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let live_armor = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.62,
        metallic: 0.24,
        reflectance: 0.36,
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
        live_armor,
        wreck_charred,
    });
}

/// Generic cleanup system for despawning all entities tagged with `HangarEntity` or `BattleEntity`.
pub fn despawn_tagged<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}
