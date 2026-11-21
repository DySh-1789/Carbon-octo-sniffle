use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Compact voxel material identifier (`0` is always Air).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct VoxelId(pub u16);

impl VoxelId {
    pub const AIR: Self = Self(0);
    pub const GRASS: Self = Self(1);
    pub const GRASS_DARK: Self = Self(2);
    pub const DIRT: Self = Self(3);
    pub const CONCRETE: Self = Self(4);
    pub const CONCRETE_DARK: Self = Self(5);
    pub const RANGE_MARKER: Self = Self(6);
    pub const STEEL_OLIVE: Self = Self(7);
    pub const STEEL_OLIVE_DARK: Self = Self(8);
    pub const STEEL_TRACK: Self = Self(9);
    pub const STEEL_GUN: Self = Self(10);
    pub const STEEL_ACCENT: Self = Self(11);
    pub const GLASS_OPTIC: Self = Self(12);
    pub const RUBBER_WHEEL: Self = Self(13);
    pub const ROAD_ASPHALT: Self = Self(14);
    pub const COBBLESTONE: Self = Self(15);
    pub const ROCK: Self = Self(16);
    pub const BRICK: Self = Self(17);
    pub const PLASTER_WALL: Self = Self(18);
    pub const WOOD: Self = Self(19);
    pub const ROOF_TILE: Self = Self(20);
    pub const ROOF_SLATE: Self = Self(21);
    pub const HEDGE: Self = Self(22);
    pub const WATER: Self = Self(23);
    pub const SANDBAG: Self = Self(24);
    pub const TREE_TRUNK: Self = Self(25);
    pub const LEAVES: Self = Self(26);
    pub const WINDOW_DARK: Self = Self(27);
    // High-detail Stage 5 national armor paints & vehicle detail materials (0.10 m tank scale)
    pub const ARMOR_US: Self = Self(28);
    pub const ARMOR_US_DARK: Self = Self(29);
    pub const ARMOR_DE_GREY: Self = Self(30);
    pub const ARMOR_DE_GREY_DARK: Self = Self(31);
    pub const ARMOR_DE_TAN: Self = Self(32);
    pub const ARMOR_DE_TAN_DARK: Self = Self(33);
    pub const ARMOR_SU: Self = Self(34);
    pub const ARMOR_SU_DARK: Self = Self(35);
    pub const STEEL_DARK_TRIM: Self = Self(36);
    pub const HEADLIGHT_GLASS: Self = Self(37);
    pub const EXHAUST_RUST: Self = Self(38);
    pub const WOOD_TOOL: Self = Self(39);
    pub const EMBLEM_WHITE: Self = Self(40);
    pub const EMBLEM_RED: Self = Self(41);
    pub const EMBLEM_BLACK: Self = Self(42);
    // Stage 6 biome materials (El Alamein, Stalingrad Winter, Karelia Ridge, Normandy Bocage)
    pub const SAND: Self = Self(43);
    pub const SAND_DARK: Self = Self(44);
    pub const SANDSTONE: Self = Self(45);
    pub const MUDBRICK: Self = Self(46);
    pub const SNOW: Self = Self(47);
    pub const SNOW_DARK: Self = Self(48);
    pub const ICE: Self = Self(49);
    pub const RUBBLE: Self = Self(50);
    pub const PINE_LEAVES: Self = Self(51);
    pub const GRANITE: Self = Self(52);
    pub const BUNKER_CONCRETE: Self = Self(53);

    #[inline]
    pub const fn is_air(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_water(self) -> bool {
        self.0 == Self::WATER.0
    }
}

/// Physical and visual metadata for a voxel material.
#[derive(Clone, Debug)]
pub struct VoxelMaterial {
    pub name: &'static str,
    /// Linear-ish sRGB color `[r, g, b, a]` in `0.0..=1.0`.
    pub base_color: [f32; 4],
    pub is_solid: bool,
    /// Structural durability used for destruction in later stages.
    pub durability: f32,
    /// Resistance factor against blast waves (Stage 7).
    pub blast_resistance: f32,
    /// Equivalent rolled homogeneous armor in mm for through-wall ballistics.
    pub armor_equiv_mm: f32,
    /// Subtle quantized shade amplitude for per-voxel visual breakup.
    pub color_jitter: f32,
}

/// Registry mapping `VoxelId` values to their material properties.
#[derive(Clone, Debug)]
pub struct MaterialRegistry {
    materials: Vec<VoxelMaterial>,
}

impl Default for MaterialRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialRegistry {
    pub fn new() -> Self {
        let materials = vec![
            // 0: AIR
            VoxelMaterial {
                name: "Air",
                base_color: [0.0, 0.0, 0.0, 0.0],
                is_solid: false,
                durability: 0.0,
                blast_resistance: 0.0,
                armor_equiv_mm: 0.0,
                color_jitter: 0.0,
            },
            // 1: GRASS
            VoxelMaterial {
                name: "Grass",
                base_color: [0.30, 0.44, 0.22, 1.0],
                is_solid: true,
                durability: 25.0,
                blast_resistance: 1.0,
                armor_equiv_mm: 4.0,
                color_jitter: 0.025,
            },
            // 2: GRASS_DARK
            VoxelMaterial {
                name: "GrassDark",
                base_color: [0.24, 0.37, 0.18, 1.0],
                is_solid: true,
                durability: 25.0,
                blast_resistance: 1.0,
                armor_equiv_mm: 4.0,
                color_jitter: 0.022,
            },
            // 3: DIRT
            VoxelMaterial {
                name: "Dirt",
                base_color: [0.36, 0.27, 0.19, 1.0],
                is_solid: true,
                durability: 35.0,
                blast_resistance: 1.4,
                armor_equiv_mm: 8.0,
                color_jitter: 0.022,
            },
            // 4: CONCRETE
            VoxelMaterial {
                name: "Concrete",
                base_color: [0.60, 0.60, 0.57, 1.0],
                is_solid: true,
                durability: 140.0,
                blast_resistance: 4.5,
                armor_equiv_mm: 25.0,
                color_jitter: 0.0,
            },
            // 5: CONCRETE_DARK
            VoxelMaterial {
                name: "ConcreteDark",
                base_color: [0.40, 0.41, 0.40, 1.0],
                is_solid: true,
                durability: 160.0,
                blast_resistance: 5.0,
                armor_equiv_mm: 30.0,
                color_jitter: 0.0,
            },
            // 6: RANGE_MARKER
            VoxelMaterial {
                name: "RangeMarker",
                base_color: [0.85, 0.72, 0.24, 1.0],
                is_solid: true,
                durability: 120.0,
                blast_resistance: 4.0,
                armor_equiv_mm: 20.0,
                color_jitter: 0.0,
            },
            // 7: STEEL_OLIVE
            VoxelMaterial {
                name: "SteelOlive",
                base_color: [0.28, 0.34, 0.22, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 8: STEEL_OLIVE_DARK
            VoxelMaterial {
                name: "SteelOliveDark",
                base_color: [0.20, 0.25, 0.16, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 9: STEEL_TRACK
            VoxelMaterial {
                name: "SteelTrack",
                base_color: [0.16, 0.16, 0.15, 1.0],
                is_solid: true,
                durability: 400.0,
                blast_resistance: 15.0,
                armor_equiv_mm: 30.0,
                color_jitter: 0.0,
            },
            // 10: STEEL_GUN
            VoxelMaterial {
                name: "SteelGun",
                base_color: [0.24, 0.28, 0.21, 1.0],
                is_solid: true,
                durability: 450.0,
                blast_resistance: 18.0,
                armor_equiv_mm: 60.0,
                color_jitter: 0.0,
            },
            // 11: STEEL_ACCENT
            VoxelMaterial {
                name: "SteelAccent",
                base_color: [0.46, 0.44, 0.36, 1.0],
                is_solid: true,
                durability: 400.0,
                blast_resistance: 16.0,
                armor_equiv_mm: 40.0,
                color_jitter: 0.0,
            },
            // 12: GLASS_OPTIC
            VoxelMaterial {
                name: "GlassOptic",
                base_color: [0.18, 0.42, 0.56, 1.0],
                is_solid: true,
                durability: 60.0,
                blast_resistance: 2.0,
                armor_equiv_mm: 10.0,
                color_jitter: 0.0,
            },
            // 13: RUBBER_WHEEL
            VoxelMaterial {
                name: "RubberWheel",
                base_color: [0.11, 0.11, 0.11, 1.0],
                is_solid: true,
                durability: 200.0,
                blast_resistance: 8.0,
                armor_equiv_mm: 15.0,
                color_jitter: 0.0,
            },
            // 14: ROAD_ASPHALT
            VoxelMaterial {
                name: "RoadAsphalt",
                base_color: [0.24, 0.25, 0.26, 1.0],
                is_solid: true,
                durability: 130.0,
                blast_resistance: 4.0,
                armor_equiv_mm: 20.0,
                color_jitter: 0.0,
            },
            // 15: COBBLESTONE
            VoxelMaterial {
                name: "Cobblestone",
                base_color: [0.50, 0.48, 0.44, 1.0],
                is_solid: true,
                durability: 115.0,
                blast_resistance: 3.8,
                armor_equiv_mm: 18.0,
                color_jitter: 0.0,
            },
            // 16: ROCK
            VoxelMaterial {
                name: "Rock",
                base_color: [0.44, 0.45, 0.44, 1.0],
                is_solid: true,
                durability: 220.0,
                blast_resistance: 8.0,
                armor_equiv_mm: 45.0,
                color_jitter: 0.03,
            },
            // 17: BRICK
            VoxelMaterial {
                name: "Brick",
                base_color: [0.56, 0.25, 0.19, 1.0],
                is_solid: true,
                durability: 100.0,
                blast_resistance: 3.2,
                armor_equiv_mm: 22.0,
                color_jitter: 0.0,
            },
            // 18: PLASTER_WALL
            VoxelMaterial {
                name: "PlasterWall",
                base_color: [0.77, 0.73, 0.63, 1.0],
                is_solid: true,
                durability: 85.0,
                blast_resistance: 2.6,
                armor_equiv_mm: 16.0,
                color_jitter: 0.0,
            },
            // 19: WOOD
            VoxelMaterial {
                name: "Wood",
                base_color: [0.39, 0.27, 0.16, 1.0],
                is_solid: true,
                durability: 45.0,
                blast_resistance: 1.2,
                armor_equiv_mm: 6.0,
                color_jitter: 0.0,
            },
            // 20: ROOF_TILE
            VoxelMaterial {
                name: "RoofTile",
                base_color: [0.50, 0.22, 0.15, 1.0],
                is_solid: true,
                durability: 60.0,
                blast_resistance: 1.5,
                armor_equiv_mm: 8.0,
                color_jitter: 0.0,
            },
            // 21: ROOF_SLATE
            VoxelMaterial {
                name: "RoofSlate",
                base_color: [0.29, 0.33, 0.39, 1.0],
                is_solid: true,
                durability: 65.0,
                blast_resistance: 1.6,
                armor_equiv_mm: 9.0,
                color_jitter: 0.0,
            },
            // 22: HEDGE
            VoxelMaterial {
                name: "Hedge",
                base_color: [0.18, 0.31, 0.14, 1.0],
                is_solid: true,
                durability: 20.0,
                blast_resistance: 0.6,
                armor_equiv_mm: 2.0,
                color_jitter: 0.0,
            },
            // 23: WATER (shallow fordable riverbed surface)
            VoxelMaterial {
                name: "Water",
                base_color: [0.18, 0.42, 0.56, 1.0],
                is_solid: true,
                durability: 999.0,
                blast_resistance: 99.0,
                armor_equiv_mm: 1.0,
                color_jitter: 0.0,
            },
            // 24: SANDBAG
            VoxelMaterial {
                name: "Sandbag",
                base_color: [0.64, 0.58, 0.44, 1.0],
                is_solid: true,
                durability: 70.0,
                blast_resistance: 2.4,
                armor_equiv_mm: 12.0,
                color_jitter: 0.0,
            },
            // 25: TREE_TRUNK
            VoxelMaterial {
                name: "TreeTrunk",
                base_color: [0.31, 0.22, 0.14, 1.0],
                is_solid: true,
                durability: 55.0,
                blast_resistance: 1.4,
                armor_equiv_mm: 8.0,
                color_jitter: 0.0,
            },
            // 26: LEAVES
            VoxelMaterial {
                name: "Leaves",
                base_color: [0.21, 0.38, 0.17, 1.0],
                is_solid: true,
                durability: 12.0,
                blast_resistance: 0.4,
                armor_equiv_mm: 1.0,
                color_jitter: 0.0,
            },
            // 27: WINDOW_DARK
            VoxelMaterial {
                name: "WindowDark",
                base_color: [0.13, 0.16, 0.20, 1.0],
                is_solid: true,
                durability: 25.0,
                blast_resistance: 0.5,
                armor_equiv_mm: 2.0,
                color_jitter: 0.0,
            },
            // 28: ARMOR_US (WWII US Olive Drab No. 9)
            VoxelMaterial {
                name: "ArmorUS",
                base_color: [0.31, 0.34, 0.22, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 29: ARMOR_US_DARK (US cast/shaded Olive Drab)
            VoxelMaterial {
                name: "ArmorUSDdark",
                base_color: [0.22, 0.25, 0.16, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 30: ARMOR_DE_GREY (Wehrmacht RAL 7021 Dunkelgrau / Panzergrau)
            VoxelMaterial {
                name: "ArmorDEGrey",
                base_color: [0.27, 0.30, 0.34, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 31: ARMOR_DE_GREY_DARK (Shaded Panzergrau)
            VoxelMaterial {
                name: "ArmorDEGreyDark",
                base_color: [0.19, 0.21, 0.25, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 32: ARMOR_DE_TAN (Wehrmacht RAL 7028 Dunkelgelb)
            VoxelMaterial {
                name: "ArmorDETan",
                base_color: [0.58, 0.50, 0.32, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 33: ARMOR_DE_TAN_DARK (Shaded Dunkelgelb / camo trim)
            VoxelMaterial {
                name: "ArmorDETanDark",
                base_color: [0.42, 0.36, 0.22, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 34: ARMOR_SU (Soviet 4BO Protective Green)
            VoxelMaterial {
                name: "ArmorSU",
                base_color: [0.24, 0.36, 0.19, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 35: ARMOR_SU_DARK (Shaded 4BO Green)
            VoxelMaterial {
                name: "ArmorSUDark",
                base_color: [0.16, 0.25, 0.13, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 36: STEEL_DARK_TRIM (Gunmetal MG barrels, vision slits, tow hooks, grilles)
            VoxelMaterial {
                name: "SteelDarkTrim",
                base_color: [0.13, 0.14, 0.15, 1.0],
                is_solid: true,
                durability: 400.0,
                blast_resistance: 15.0,
                armor_equiv_mm: 40.0,
                color_jitter: 0.0,
            },
            // 37: HEADLIGHT_GLASS (Warm bright headlamp lens)
            VoxelMaterial {
                name: "HeadlightGlass",
                base_color: [0.96, 0.92, 0.72, 1.0],
                is_solid: true,
                durability: 40.0,
                blast_resistance: 1.0,
                armor_equiv_mm: 4.0,
                color_jitter: 0.0,
            },
            // 38: EXHAUST_RUST (Burnt iron exhaust pipes & spare track links)
            VoxelMaterial {
                name: "ExhaustRust",
                base_color: [0.36, 0.20, 0.14, 1.0],
                is_solid: true,
                durability: 200.0,
                blast_resistance: 6.0,
                armor_equiv_mm: 18.0,
                color_jitter: 0.0,
            },
            // 39: WOOD_TOOL (Soviet unditching log & pioneer tool handles)
            VoxelMaterial {
                name: "WoodTool",
                base_color: [0.46, 0.32, 0.18, 1.0],
                is_solid: true,
                durability: 60.0,
                blast_resistance: 2.0,
                armor_equiv_mm: 8.0,
                color_jitter: 0.0,
            },
            // 40: EMBLEM_WHITE (Allied star / Balkenkreuz white border / turret stripe)
            VoxelMaterial {
                name: "EmblemWhite",
                base_color: [0.92, 0.92, 0.88, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 41: EMBLEM_RED (Soviet red star / rear tail lamps)
            VoxelMaterial {
                name: "EmblemRed",
                base_color: [0.80, 0.14, 0.14, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 42: EMBLEM_BLACK (Balkenkreuz core / bore shadow / recessed engine vents)
            VoxelMaterial {
                name: "EmblemBlack",
                base_color: [0.07, 0.07, 0.08, 1.0],
                is_solid: true,
                durability: 500.0,
                blast_resistance: 20.0,
                armor_equiv_mm: 80.0,
                color_jitter: 0.0,
            },
            // 43: SAND (North African desert dunes)
            VoxelMaterial {
                name: "Sand",
                base_color: [0.78, 0.67, 0.46, 1.0],
                is_solid: true,
                durability: 20.0,
                blast_resistance: 0.9,
                armor_equiv_mm: 4.0,
                color_jitter: 0.022,
            },
            // 44: SAND_DARK (Packed desert trail / dune shadow)
            VoxelMaterial {
                name: "SandDark",
                base_color: [0.65, 0.53, 0.35, 1.0],
                is_solid: true,
                durability: 24.0,
                blast_resistance: 1.0,
                armor_equiv_mm: 5.0,
                color_jitter: 0.02,
            },
            // 45: SANDSTONE (Desert wadi rock & cliffs)
            VoxelMaterial {
                name: "Sandstone",
                base_color: [0.62, 0.48, 0.32, 1.0],
                is_solid: true,
                durability: 175.0,
                blast_resistance: 6.5,
                armor_equiv_mm: 35.0,
                color_jitter: 0.025,
            },
            // 46: MUDBRICK (North African adobe village walls)
            VoxelMaterial {
                name: "Mudbrick",
                base_color: [0.74, 0.60, 0.42, 1.0],
                is_solid: true,
                durability: 80.0,
                blast_resistance: 2.4,
                armor_equiv_mm: 14.0,
                color_jitter: 0.0,
            },
            // 47: SNOW (Winter snowpack)
            VoxelMaterial {
                name: "Snow",
                base_color: [0.86, 0.89, 0.93, 1.0],
                is_solid: true,
                durability: 18.0,
                blast_resistance: 0.8,
                armor_equiv_mm: 3.0,
                color_jitter: 0.018,
            },
            // 48: SNOW_DARK (Churning tank-tracked muddy winter snow)
            VoxelMaterial {
                name: "SnowDark",
                base_color: [0.68, 0.72, 0.76, 1.0],
                is_solid: true,
                durability: 22.0,
                blast_resistance: 1.0,
                armor_equiv_mm: 5.0,
                color_jitter: 0.02,
            },
            // 49: ICE (Frozen river sheet)
            VoxelMaterial {
                name: "Ice",
                base_color: [0.62, 0.78, 0.88, 1.0],
                is_solid: true,
                durability: 45.0,
                blast_resistance: 1.5,
                armor_equiv_mm: 6.0,
                color_jitter: 0.0,
            },
            // 50: RUBBLE (Bombed city brick & concrete debris)
            VoxelMaterial {
                name: "Rubble",
                base_color: [0.43, 0.37, 0.34, 1.0],
                is_solid: true,
                durability: 110.0,
                blast_resistance: 3.5,
                armor_equiv_mm: 22.0,
                color_jitter: 0.025,
            },
            // 51: PINE_LEAVES (Dark Nordic spruce/pine foliage)
            VoxelMaterial {
                name: "PineLeaves",
                base_color: [0.14, 0.27, 0.16, 1.0],
                is_solid: true,
                durability: 12.0,
                blast_resistance: 0.4,
                armor_equiv_mm: 1.0,
                color_jitter: 0.0,
            },
            // 52: GRANITE (Nordic Karelia grey-blue granite ridges & cliffs)
            VoxelMaterial {
                name: "Granite",
                base_color: [0.40, 0.43, 0.47, 1.0],
                is_solid: true,
                durability: 260.0,
                blast_resistance: 10.0,
                armor_equiv_mm: 55.0,
                color_jitter: 0.025,
            },
            // 53: BUNKER_CONCRETE (Atlantic Wall reinforced steel-concrete bunker)
            VoxelMaterial {
                name: "BunkerConcrete",
                base_color: [0.49, 0.50, 0.47, 1.0],
                is_solid: true,
                durability: 360.0,
                blast_resistance: 16.0,
                armor_equiv_mm: 95.0,
                color_jitter: 0.0,
            },
        ];

        Self { materials }
    }

    #[inline]
    pub fn get(&self, id: VoxelId) -> &VoxelMaterial {
        self.materials
            .get(id.0 as usize)
            .unwrap_or(&self.materials[0])
    }

    #[inline]
    pub fn is_solid(&self, id: VoxelId) -> bool {
        !id.is_air() && self.get(id).is_solid
    }

    /// Coarse 2-shade macro-patch bucket (`8 m × 8 m` cells) so greedy meshing merges large
    /// runs of terrain into minimal quads while preserving subtle organic ground breakup.
    #[inline]
    pub fn shade_bucket(&self, id: VoxelId, pos: IVec3) -> u8 {
        if self.get(id).color_jitter <= 0.0 {
            return 0;
        }
        let hx = (pos.x >> 4).wrapping_mul(374_761_393);
        let hy = (pos.y >> 3).wrapping_mul(668_265_263);
        let hz = (pos.z >> 4).wrapping_mul(2_147_483_647);
        let mut h = (hx ^ hy ^ hz) as u32;
        h ^= h >> 13;
        h = h.wrapping_mul(1_274_126_177);
        ((h >> 16) & 1) as u8
    }

    /// Computes final RGBA color for a voxel given its shade bucket (`0..2`).
    #[inline]
    pub fn color_for_bucket(&self, id: VoxelId, bucket: u8) -> [f32; 4] {
        let mat = self.get(id);
        if mat.color_jitter <= 0.0 {
            return mat.base_color;
        }
        let offset = if bucket == 0 {
            -mat.color_jitter
        } else {
            mat.color_jitter
        };
        [
            (mat.base_color[0] + offset).clamp(0.0, 1.0),
            (mat.base_color[1] + offset).clamp(0.0, 1.0),
            (mat.base_color[2] + offset).clamp(0.0, 1.0),
            mat.base_color[3],
        ]
    }
}
