use bevy::prelude::*;
use sim::{BattleRewardSummary, MapId, Nation, PlayerProfile, VehicleCatalog};

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

/// Update 0.1 Economy & Progression state (Silver Credits, Nation RP, research/ownership & battle receipt).
#[derive(Resource, Clone, Debug)]
pub struct EconomyResource {
    pub profile: PlayerProfile,
    pub last_reward: Option<BattleRewardSummary>,
    pub settled_this_battle: bool,
    pub status_notice: String,
    pub persist_to_disk: bool,
}

impl Default for EconomyResource {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

impl EconomyResource {
    pub fn new_in_memory() -> Self {
        Self {
            profile: PlayerProfile::new_starter(),
            last_reward: None,
            settled_this_battle: false,
            status_notice: "UPDATE 0.1 ECONOMY | FREE TIER I RESERVES READY".to_string(),
            persist_to_disk: false,
        }
    }

    /// Loads saved profile from `~/.carbon_armor_profile.txt` on native builds, or starts fresh on WASM.
    pub fn load_or_default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = Self::save_path()
                && let Ok(text) = std::fs::read_to_string(&path)
                && let Some(profile) = PlayerProfile::from_save_text(&text)
            {
                return Self {
                    profile,
                    last_reward: None,
                    settled_this_battle: false,
                    status_notice: "PROFILE LOADED | READY FOR BATTLE".to_string(),
                    persist_to_disk: true,
                };
            }
            let mut res = Self::new_in_memory();
            res.persist_to_disk = true;
            res
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::new_in_memory()
        }
    }

    pub fn save_to_disk(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.persist_to_disk
            && let Some(path) = Self::save_path()
        {
            let _ = std::fs::write(path, self.profile.to_save_text());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_path() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME")?;
        let mut path = std::path::PathBuf::from(home);
        path.push(".carbon_armor_profile.txt");
        Some(path)
    }
}

/// Currently selected nation, vehicle, battlefield map, and Hangar UI mode.
#[derive(Resource, Clone, Debug)]
pub struct SelectedVehicle {
    pub nation: Nation,
    pub vehicle_id: String,
    pub map_id: MapId,
    pub view_mode: HangarViewMode,
    /// Set whenever `vehicle_id` changes so the 3D Hangar showcase tank rebuilds once.
    pub preview_dirty: bool,
}

impl Default for SelectedVehicle {
    fn default() -> Self {
        Self {
            nation: Nation::Usa,
            vehicle_id: "us_m2a4".to_string(),
            map_id: MapId::EuropeanValley,
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
