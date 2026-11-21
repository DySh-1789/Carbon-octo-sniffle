use voxel::glam::{IVec3, Vec2, Vec3};
use voxel::{WORLD_VOXEL_SIZE, VoxelId, VoxelWorld};

/// Tactical capture / key map zone (rendered on minimap in Stage 2, capturable in Stage 3).
#[derive(Clone, Debug)]
pub struct CaptureZoneDef {
    pub label: char,
    pub center: Vec3,
    pub radius: f32,
}

/// Classification of curated tactical positions for map design & future Stage 3 AI navigation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TacticalPositionKind {
    HullDownRidge,
    VillageAmbush,
    FlankOverwatch,
    BridgeChokepoint,
}

/// Named tactical anchor embedded in `MapMetadata`.
#[derive(Clone, Debug)]
pub struct TacticalPosition {
    pub name: String,
    pub position: Vec3,
    pub kind: TacticalPositionKind,
}

/// Pre-rendered top-down RGBA8 minimap texture generated directly from `VoxelWorld`.
#[derive(Clone, Debug)]
pub struct MinimapData {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// Sky, distance fog, and sunlight color/intensity profile tailored per map biome.
#[derive(Copy, Clone, Debug)]
pub struct MapAtmosphere {
    pub sky_rgb: [f32; 3],
    pub fog_rgb: [f32; 3],
    pub sun_rgb: [f32; 3],
    pub sun_illuminance: f32,
    pub sun_dir: [f32; 3],
    pub ambient_rgb: [f32; 3],
    pub ambient_brightness: f32,
    pub fog_start: f32,
    pub fog_end: f32,
}

impl Default for MapAtmosphere {
    fn default() -> Self {
        Self::european_valley()
    }
}

impl MapAtmosphere {
    pub const fn european_valley() -> Self {
        Self {
            sky_rgb: [0.62, 0.76, 0.88],
            fog_rgb: [0.66, 0.78, 0.88],
            sun_rgb: [1.0, 0.96, 0.88],
            sun_illuminance: 15_500.0,
            sun_dir: [110.0, 155.0, 85.0],
            ambient_rgb: [0.84, 0.89, 0.96],
            ambient_brightness: 450.0,
            fog_start: 125.0,
            fog_end: 370.0,
        }
    }

    pub const fn el_alamein() -> Self {
        Self {
            sky_rgb: [0.86, 0.79, 0.64],
            fog_rgb: [0.86, 0.77, 0.60],
            sun_rgb: [1.0, 0.94, 0.78],
            sun_illuminance: 18_500.0,
            sun_dir: [120.0, 170.0, 60.0],
            ambient_rgb: [0.96, 0.88, 0.72],
            ambient_brightness: 540.0,
            fog_start: 115.0,
            fog_end: 350.0,
        }
    }

    pub const fn stalingrad_winter() -> Self {
        Self {
            sky_rgb: [0.72, 0.77, 0.83],
            fog_rgb: [0.74, 0.78, 0.83],
            sun_rgb: [0.92, 0.95, 1.0],
            sun_illuminance: 12_500.0,
            sun_dir: [-95.0, 125.0, 90.0],
            ambient_rgb: [0.82, 0.88, 0.95],
            ambient_brightness: 520.0,
            fog_start: 95.0,
            fog_end: 320.0,
        }
    }

    pub const fn karelia_ridge() -> Self {
        Self {
            sky_rgb: [0.54, 0.68, 0.80],
            fog_rgb: [0.60, 0.72, 0.80],
            sun_rgb: [0.98, 0.95, 0.88],
            sun_illuminance: 14_200.0,
            sun_dir: [95.0, 130.0, -90.0],
            ambient_rgb: [0.76, 0.84, 0.92],
            ambient_brightness: 440.0,
            fog_start: 110.0,
            fog_end: 345.0,
        }
    }

    pub const fn normandy_bocage() -> Self {
        Self {
            sky_rgb: [0.66, 0.74, 0.80],
            fog_rgb: [0.70, 0.76, 0.80],
            sun_rgb: [0.96, 0.95, 0.90],
            sun_illuminance: 14_000.0,
            sun_dir: [-100.0, 145.0, 80.0],
            ambient_rgb: [0.82, 0.86, 0.88],
            ambient_brightness: 470.0,
            fog_start: 105.0,
            fog_end: 335.0,
        }
    }
}

/// Full tactical metadata for a playable battlefield.
#[derive(Clone, Debug)]
pub struct MapMetadata {
    pub id: String,
    pub name: String,
    pub size_m: f32,
    pub bounds_min_xz: Vec2,
    pub bounds_max_xz: Vec2,
    pub spawn_allies: Vec<Vec3>,
    pub spawn_axis: Vec<Vec3>,
    pub capture_zones: Vec<CaptureZoneDef>,
    pub tactical_positions: Vec<TacticalPosition>,
    pub minimap: MinimapData,
    pub atmosphere: MapAtmosphere,
}

impl MapMetadata {
    /// Converts a world `(x, z)` position into normalized minimap UV coordinates in `[0, 1]`,
    /// where `u = 0` is West (`min_x`), `u = 1` is East (`max_x`),
    /// `v = 0` is North (`min_z`, top of minimap), and `v = 1` is South (`max_z`, bottom).
    pub fn world_to_minimap_uv(&self, world_pos: Vec3) -> Vec2 {
        let span = (self.bounds_max_xz - self.bounds_min_xz).max(Vec2::splat(1.0));
        let u = ((world_pos.x - self.bounds_min_xz.x) / span.x).clamp(0.0, 1.0);
        let v = ((world_pos.z - self.bounds_min_xz.y) / span.y).clamp(0.0, 1.0);
        Vec2::new(u, v)
    }
}

/// Generates a flat Stage 1 proving ground (`256m × 256m` when `half_extent_m = 128`).
///
/// Kept intact for deterministic physics and ballistics unit tests.
pub fn generate_flat_proving_ground(half_extent_m: i32) -> VoxelWorld {
    let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);
    let half_vox = (half_extent_m as f32 / WORLD_VOXEL_SIZE).round() as i32;

    let min_x = -half_vox;
    let max_x = half_vox - 1;
    let min_z = -half_vox;
    let max_z = half_vox - 1;

    world.bounds_min = IVec3::new(min_x, -3, min_z);
    world.bounds_max = IVec3::new(max_x, 0, max_z);

    world.fill_box(
        IVec3::new(min_x, -3, min_z),
        IVec3::new(max_x, -2, max_z),
        VoxelId::DIRT,
    );
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(max_x, -1, max_z),
        VoxelId::GRASS,
    );

    let stripe_width = 8;
    for x in min_x..=max_x {
        if x.rem_euclid(32) < stripe_width {
            world.fill_box(
                IVec3::new(x, -1, min_z),
                IVec3::new(x, -1, max_z),
                VoxelId::GRASS_DARK,
            );
        }
    }

    world.fill_box(
        IVec3::new(-16, -1, -16),
        IVec3::new(15, -1, 15),
        VoxelId::CONCRETE,
    );
    world.fill_box(
        IVec3::new(-16, -1, -16),
        IVec3::new(15, -1, -15),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(-16, -1, 14),
        IVec3::new(15, -1, 15),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(-16, -1, -16),
        IVec3::new(-15, -1, 15),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(14, -1, -16),
        IVec3::new(15, -1, 15),
        VoxelId::CONCRETE_DARK,
    );

    let lane_end_z = (min_z + 12).min(-40);
    world.fill_box(
        IVec3::new(-6, -1, lane_end_z),
        IVec3::new(5, -1, -17),
        VoxelId::CONCRETE_DARK,
    );

    for dist_m in [50, 100, 150] {
        if dist_m < half_extent_m - 6 {
            let vz = -(dist_m as f32 / WORLD_VOXEL_SIZE).round() as i32;
            world.fill_box(
                IVec3::new(-24, -1, vz),
                IVec3::new(23, -1, vz + 1),
                VoxelId::RANGE_MARKER,
            );
        }
    }

    let border = 3;
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(max_x, -1, min_z + border),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(min_x, -1, max_z - border),
        IVec3::new(max_x, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(min_x, -1, min_z),
        IVec3::new(min_x + border, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );
    world.fill_box(
        IVec3::new(max_x - border, -1, min_z),
        IVec3::new(max_x, -1, max_z),
        VoxelId::CONCRETE_DARK,
    );

    world.compact_chunks();
    world.mark_all_dirty();
    world
}

/// Builds default `MapMetadata` for an arbitrary voxel world (such as the flat proving ground).
pub fn default_metadata_for_world(world: &VoxelWorld, id: &str, name: &str) -> MapMetadata {
    let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
    let spawn_y = world.surface_height_at(0.0, 0.0).unwrap_or(0.0);
    let minimap = generate_minimap_data(world, 128);
    MapMetadata {
        id: id.to_string(),
        name: name.to_string(),
        size_m: max_x - min_x,
        bounds_min_xz: Vec2::new(min_x, min_z),
        bounds_max_xz: Vec2::new(max_x, max_z),
        spawn_allies: vec![Vec3::new(0.0, spawn_y, 0.0)],
        spawn_axis: vec![Vec3::new(0.0, spawn_y, -80.0)],
        capture_zones: Vec::new(),
        tactical_positions: Vec::new(),
        minimap,
        atmosphere: MapAtmosphere::default(),
    }
}

/// Generates the Stage 2 full tactical map: **"European Valley"** (`384 × 384 m`, `24 × 24` chunks).
///
/// Vertical elevation is aligned strictly within `vy = 0..=31` (`cy = 0`, single vertical chunk
/// layer) to keep resident voxel memory and draw calls minimal.
pub fn generate_european_valley_map() -> (VoxelWorld, MapMetadata) {
    let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);

    let half_extent_m = 192;
    let half_vox = (half_extent_m as f32 / WORLD_VOXEL_SIZE).round() as i32; // 384 voxels -> 24 chunks
    let min_x = -half_vox;
    let max_x = half_vox - 1;
    let min_z = -half_vox;
    let max_z = half_vox - 1;
    let min_y = 0;
    let max_y = 31;

    world.bounds_min = IVec3::new(min_x, min_y, min_z);
    world.bounds_max = IVec3::new(max_x, max_y, max_z);

    // Water surface sits cleanly at vy = 4 (top face Y = 2.5 m)
    const WATER_VY: i32 = 4;

    // 1. Base terrain elevation, valley stream, roads, and square paving
    for vz in min_z..=max_z {
        let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
        for vx in min_x..=max_x {
            let wx = (vx as f32 + 0.5) * WORLD_VOXEL_SIZE;
            let h_m = european_valley_height_m(wx, wz);
            let top_vy = ((h_m / WORLD_VOXEL_SIZE).round() as i32 - 1).clamp(min_y + 1, max_y - 2);

            if top_vy < WATER_VY {
                // Uniform rock bedrock beneath shallow dirt streambed + water surface
                world.fill_column_raw(vx, vz, min_y, (top_vy - 1).max(min_y), VoxelId::ROCK);
                if top_vy > min_y {
                    world.set_voxel_raw(IVec3::new(vx, top_vy, vz), VoxelId::DIRT);
                }
                world.fill_column_raw(vx, vz, top_vy + 1, WATER_VY, VoxelId::WATER);
            } else {
                // Uniform rock bedrock beneath 1 dirt subsoil layer and surface cover
                let subsoil_top = (top_vy - 1).max(min_y);
                let rock_top = (top_vy - 2).max(min_y);
                if rock_top >= min_y {
                    world.fill_column_raw(vx, vz, min_y, rock_top, VoxelId::ROCK);
                }
                if subsoil_top > rock_top {
                    world.fill_column_raw(vx, vz, rock_top + 1, subsoil_top, VoxelId::DIRT);
                }

                let surface_mat = select_surface_material(wx, wz, h_m);
                world.set_voxel_raw(IVec3::new(vx, top_vy, vz), surface_mat);
            }
        }
    }

    // 2. Stone Arch Bridge carrying the central north-south road over the valley stream
    build_stone_bridge(&mut world, WATER_VY);

    // 3. Central Village buildings (Zone B), Church bell tower, and outlying farmsteads
    let houses = [
        // Central village — West side (leaves wide 10-14m main & side avenues clear of all roads)
        HouseSpec::new(-26.0, 45.0, 11.0, 9.0, 2, VoxelId::PLASTER_WALL, VoxelId::ROOF_TILE, DoorSide::East),
        HouseSpec::new(-44.0, 26.0, 10.0, 12.0, 1, VoxelId::BRICK, VoxelId::ROOF_SLATE, DoorSide::East),
        HouseSpec::new(-26.0, 27.0, 9.0, 9.0, 2, VoxelId::PLASTER_WALL, VoxelId::ROOF_SLATE, DoorSide::East),
        HouseSpec::new(-26.0, -4.0, 10.0, 9.0, 1, VoxelId::BRICK, VoxelId::ROOF_TILE, DoorSide::South),
        HouseSpec::new(-46.0, -8.0, 11.0, 9.0, 1, VoxelId::PLASTER_WALL, VoxelId::ROOF_TILE, DoorSide::East),
        // Central village — East side
        HouseSpec::new(27.0, 44.0, 11.0, 9.0, 2, VoxelId::BRICK, VoxelId::ROOF_TILE, DoorSide::West),
        HouseSpec::new(45.0, 30.0, 10.0, 11.0, 1, VoxelId::PLASTER_WALL, VoxelId::ROOF_SLATE, DoorSide::West),
        HouseSpec::new(48.0, 4.0, 10.0, 9.0, 2, VoxelId::PLASTER_WALL, VoxelId::ROOF_TILE, DoorSide::West),
        HouseSpec::new(28.0, -15.0, 10.0, 8.0, 1, VoxelId::BRICK, VoxelId::ROOF_SLATE, DoorSide::South),
        // North bank bridgehead cottages (tactical ambush cover facing the bridge)
        HouseSpec::new(-23.0, -76.0, 10.0, 9.0, 1, VoxelId::PLASTER_WALL, VoxelId::ROOF_TILE, DoorSide::East),
        HouseSpec::new(24.0, -78.0, 11.0, 9.0, 2, VoxelId::BRICK, VoxelId::ROOF_SLATE, DoorSide::West),
        // Western farmstead barn overlooking Zone A flank
        HouseSpec::new(-86.0, 34.0, 13.0, 9.0, 1, VoxelId::WOOD, VoxelId::ROOF_TILE, DoorSide::North),
        // Eastern hillside stone cottage overlooking approach to Zone C
        HouseSpec::new(82.0, 56.0, 10.0, 8.0, 1, VoxelId::PLASTER_WALL, VoxelId::ROOF_TILE, DoorSide::West),
    ];

    for house in &houses {
        build_house(&mut world, house);
    }

    // Iconic village church with bell tower overlooking the north-east side of the town square
    build_village_church(&mut world, 28.0, 4.0);

    // 4. Tactical low cover: stone garden walls, sandbag nests, Normandy hedgerows, granite boulders
    // Town square & village corner low walls (1.5 m high — protects lower hull, clear turret shots)
    build_low_wall(&mut world, -17.0, 32.0, -17.0, 46.0, 3, VoxelId::COBBLESTONE);
    build_low_wall(&mut world, 17.0, 32.0, 17.0, 46.0, 3, VoxelId::COBBLESTONE);
    build_low_wall(&mut world, -36.0, 10.0, -22.0, 10.0, 3, VoxelId::COBBLESTONE);
    build_low_wall(&mut world, 22.0, 32.0, 36.0, 32.0, 3, VoxelId::COBBLESTONE);

    // Sandbag defensive emplacements (1.0 m high) at key tactical firing positions
    build_low_wall(&mut world, -13.0, -24.0, -8.0, -24.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, 8.0, -24.0, 13.0, -24.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, -13.0, -60.0, -8.0, -60.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, 8.0, -60.0, 13.0, -60.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, -10.0, 12.0, -6.0, 12.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, 6.0, 32.0, 10.0, 32.0, 2, VoxelId::SANDBAG);
    // Zone A & Zone C sandbag cover arcs
    build_low_wall(&mut world, -118.0, -16.0, -108.0, -16.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, -118.0, 2.0, -108.0, 2.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, 100.0, -2.0, 114.0, -2.0, 2, VoxelId::SANDBAG);
    build_low_wall(&mut world, 100.0, 18.0, 114.0, 18.0, 2, VoxelId::SANDBAG);

    // West flank Normandy-style hedgerows framing flanking lanes (with wide tank gaps!)
    build_low_wall(&mut world, -138.0, 56.0, -108.0, 56.0, 3, VoxelId::HEDGE);
    build_low_wall(&mut world, -86.0, 62.0, -62.0, 62.0, 3, VoxelId::HEDGE);
    build_low_wall(&mut world, -142.0, -56.0, -112.0, -56.0, 3, VoxelId::HEDGE);
    build_low_wall(&mut world, -84.0, -66.0, -58.0, -66.0, 3, VoxelId::HEDGE);
    build_low_wall(&mut world, 64.0, 74.0, 96.0, 74.0, 3, VoxelId::HEDGE);
    build_low_wall(&mut world, 64.0, -72.0, 96.0, -72.0, 3, VoxelId::HEDGE);

    // Granite rock outcrops on East Hill (Zone C), West Ridge, and valley overlooks
    let boulders: &[(f32, f32, f32, f32)] = &[
        (124.0, 14.0, 3.5, 3.0),
        (120.0, -10.0, 3.0, 2.5),
        (94.0, 16.0, 2.5, 2.0),
        (132.0, -28.0, 4.0, 3.2),
        (78.0, -20.0, 2.8, 2.2),
        (-128.0, -22.0, 3.2, 2.6),
        (-130.0, 16.0, 3.4, 2.4),
        (-74.0, -24.0, 2.6, 2.0),
        (-56.0, 94.0, 3.2, 2.5),
        (58.0, 96.0, 3.4, 2.5),
        (-58.0, -102.0, 3.4, 2.6),
        (60.0, -104.0, 3.2, 2.6),
    ];
    for &(bx, bz, rad, height) in boulders {
        build_boulder(&mut world, bx, bz, rad, height);
    }

    // 5. Flush tactical zone perimeter rings on the ground (Zones A, B, C)
    let zone_defs_xz = [
        ('A', -108.0f32, -8.0f32, 16.0f32),
        ('B', 0.0f32, 22.0f32, 16.0f32),
        ('C', 108.0f32, 8.0f32, 16.0f32),
    ];
    for &(_, zx, zz, radius) in &zone_defs_xz {
        paint_zone_ring(&mut world, zx, zz, radius);
    }

    // 6. Voxel trees distributed naturally along flanks, orchards, and ridgelines
    // (explicitly excludes roads, stream, buildings, capture zones, and spawn pads)
    place_valley_trees(&mut world, &houses, &zone_defs_xz);

    world.compact_chunks();
    world.mark_all_dirty();

    // 7. Build tactical MapMetadata with accurate ground heights
    let sample_spawn = |x: f32, z: f32| -> Vec3 {
        let y = world.surface_height_at(x, z).unwrap_or(0.0);
        Vec3::new(x, y, z)
    };

    let spawn_allies = vec![
        sample_spawn(0.0, 148.0),
        sample_spawn(-18.0, 152.0),
        sample_spawn(18.0, 152.0),
        sample_spawn(-38.0, 146.0),
        sample_spawn(38.0, 146.0),
        sample_spawn(0.0, 162.0),
    ];
    let spawn_axis = vec![
        sample_spawn(0.0, -148.0),
        sample_spawn(-18.0, -152.0),
        sample_spawn(18.0, -152.0),
        sample_spawn(-38.0, -146.0),
        sample_spawn(38.0, -146.0),
        sample_spawn(0.0, -162.0),
    ];

    let capture_zones = zone_defs_xz
        .iter()
        .map(|&(label, x, z, radius)| CaptureZoneDef {
            label,
            center: sample_spawn(x, z),
            radius,
        })
        .collect();

    let tactical_positions = vec![
        TacticalPosition {
            name: "West Hull-Down Ridge".to_string(),
            position: sample_spawn(-112.0, -8.0),
            kind: TacticalPositionKind::HullDownRidge,
        },
        TacticalPosition {
            name: "South-West Flank Crest".to_string(),
            position: sample_spawn(-98.0, 44.0),
            kind: TacticalPositionKind::HullDownRidge,
        },
        TacticalPosition {
            name: "North-West Flank Crest".to_string(),
            position: sample_spawn(-98.0, -68.0),
            kind: TacticalPositionKind::HullDownRidge,
        },
        TacticalPosition {
            name: "Central Town Square".to_string(),
            position: sample_spawn(0.0, 22.0),
            kind: TacticalPositionKind::VillageAmbush,
        },
        TacticalPosition {
            name: "Stone Arch Bridge".to_string(),
            position: sample_spawn(0.0, -42.0),
            kind: TacticalPositionKind::BridgeChokepoint,
        },
        TacticalPosition {
            name: "East Overwatch Hill".to_string(),
            position: sample_spawn(108.0, 8.0),
            kind: TacticalPositionKind::FlankOverwatch,
        },
    ];

    let (b_min_x, b_max_x, b_min_z, b_max_z) = world.playable_bounds_xz();
    let minimap = generate_minimap_data(&world, 192);

    let metadata = MapMetadata {
        id: "european_valley".to_string(),
        name: "European Valley".to_string(),
        size_m: b_max_x - b_min_x,
        bounds_min_xz: Vec2::new(b_min_x, b_min_z),
        bounds_max_xz: Vec2::new(b_max_x, b_max_z),
        spawn_allies,
        spawn_axis,
        capture_zones,
        tactical_positions,
        minimap,
        atmosphere: MapAtmosphere::european_valley(),
    };

    (world, metadata)
}

/// Centerline Z coordinate (in meters) of the winding valley stream at world X.
#[inline]
fn stream_center_z(wx: f32) -> f32 {
    -42.0 + 13.0 * (wx * 0.022).sin() + 4.5 * (wx * 0.048 + 0.7).cos()
}

/// Smooth Gaussian hill helper.
#[inline]
fn gaussian_hill(wx: f32, wz: f32, cx: f32, cz: f32, radius: f32, height: f32) -> f32 {
    let dx = wx - cx;
    let dz = wz - cz;
    let r2 = radius * radius;
    height * (-((dx * dx + dz * dz) / (2.0 * r2))).exp()
}

/// Smoothstep interpolation in `[0, 1]`.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Continuous deterministic terrain height (in meters) for European Valley.
fn european_valley_height_m(wx: f32, wz: f32) -> f32 {
    // 1. Base valley elevation + smooth multi-octave rolling relief
    let mut h = 2.0
        + 1.65 * (wx * 0.018).sin() * (wz * 0.016).cos()
        + 0.85 * (wx * 0.034 + 1.1).cos() * (wz * 0.029 - 0.6).sin()
        + 0.35 * (wx * 0.062 - wz * 0.051).sin();

    // 2. Purposeful tactical hills & hull-down crests
    // West flank ridge (Zone A) + south/north hull-down shoulders
    h += gaussian_hill(wx, wz, -112.0, -8.0, 44.0, 5.4);
    h += gaussian_hill(wx, wz, -104.0, 46.0, 34.0, 3.2);
    h += gaussian_hill(wx, wz, -104.0, -70.0, 34.0, 3.2);

    // East overwatch hill (Zone C)
    h += gaussian_hill(wx, wz, 112.0, 8.0, 46.0, 7.2);
    h += gaussian_hill(wx, wz, 118.0, -68.0, 36.0, 3.6);

    // South Allies & North Axis spawn overlooks
    h += gaussian_hill(wx, wz, 0.0, 152.0, 54.0, 4.2);
    h += gaussian_hill(wx, wz, 0.0, -152.0, 54.0, 4.4);

    // 3. Gently flatten the central village terrace around (0, +22) to Y = 2.0 m
    let village_dist = (wx * wx + (wz - 22.0) * (wz - 22.0)).sqrt();
    let village_blend = 1.0 - smoothstep(42.0, 68.0, village_dist);
    h = h * (1.0 - village_blend) + 2.0 * village_blend;

    // Flatten a clean combat plateau on East Hilltop (Zone C) and West Ridge (Zone A)
    let zone_c_dist = ((wx - 108.0).powi(2) + (wz - 8.0).powi(2)).sqrt();
    let zone_c_flat = 1.0 - smoothstep(14.0, 28.0, zone_c_dist);
    h = h * (1.0 - zone_c_flat) + 8.5 * zone_c_flat;

    let zone_a_dist = ((wx + 108.0).powi(2) + (wz + 8.0).powi(2)).sqrt();
    let zone_a_flat = 1.0 - smoothstep(14.0, 28.0, zone_a_dist);
    h = h * (1.0 - zone_a_flat) + 6.5 * zone_a_flat;

    // 4. Carve the shallow winding valley stream across Z ≈ -42 m
    let d_stream = (wz - stream_center_z(wx)).abs();
    // Fords at X ≈ -96 and X ≈ +96 have extra-gentle shallow banks
    let near_ford = (wx + 96.0).abs() < 14.0 || (wx - 96.0).abs() < 14.0;
    let stream_half_w = if near_ford { 6.5 } else { 5.5 };
    let bank_outer = if near_ford { 24.0 } else { 19.0 };
    let stream_cut = 1.0 - smoothstep(stream_half_w, bank_outer, d_stream);
    // Stream bed dips to -1.1 m (so water top at -0.5 m is ~0.6 m deep visually, smoothly fordable)
    let bed_y = if near_ford { -0.85 } else { -1.15 };
    h = h * (1.0 - stream_cut) + bed_y * stream_cut;

    // 5. Natural perimeter valley walls near edges (|x| or |z| > 164 m)
    let edge = wx.abs().max(wz.abs());
    let rim = smoothstep(162.0, 190.0, edge);
    h += rim * 5.2;

    // Shift +3.0 m upward so bedrock sits at vy = 0 (Y = 0.0 m) and entire map fits in cy = 0
    (h + 3.0).clamp(1.2, 15.0)
}

/// Returns whether `(wx, wz)` lies on the main asphalt road, cobblestone town square, or dirt flank road.
fn road_material_at(wx: f32, wz: f32) -> Option<VoxelId> {
    // Central cobblestone town square
    if wx.abs() <= 16.5 && (10.0..=34.0).contains(&wz) {
        return Some(VoxelId::COBBLESTONE);
    }

    // Main north-south asphalt highway (X ≈ 0) connecting both spawns through the village & bridge
    let main_road_x = 1.8 * (wz * 0.02).sin() * smoothstep(55.0, 110.0, wz.abs());
    if (wx - main_road_x).abs() <= 4.4 && wz.abs() <= 166.0 {
        // Inside village it transitions neatly to cobblestone
        if (-14.0..=48.0).contains(&wz) {
            return Some(VoxelId::COBBLESTONE);
        }
        return Some(VoxelId::ROAD_ASPHALT);
    }

    // Cross-valley E-W road connecting Zone A (-108, -8) <-> Town Square (0, +22) <-> Zone C (+108, +8)
    if (-116.0..=116.0).contains(&wx) {
        let target_z = if wx < 0.0 {
            let t = (-wx / 108.0).clamp(0.0, 1.0);
            22.0 * (1.0 - t) + (-8.0) * t
        } else {
            let t = (wx / 108.0).clamp(0.0, 1.0);
            22.0 * (1.0 - t) + 8.0 * t
        };
        if (wz - target_z).abs() <= 3.3 {
            if wx.abs() <= 54.0 {
                return Some(VoxelId::COBBLESTONE);
            }
            return Some(VoxelId::DIRT);
        }
    }

    // West flank dirt road connecting South Spawn -> Zone A -> West Ford -> North Spawn
    if (-152.0..=152.0).contains(&wz) {
        let west_road_x = -104.0 + 18.0 * (wz / 152.0).powi(2) * 3.8;
        if (wx - west_road_x).abs() <= 3.2 {
            return Some(VoxelId::DIRT);
        }
        let east_road_x = 104.0 - 18.0 * (wz / 152.0).powi(2) * 3.8;
        if (wx - east_road_x).abs() <= 3.2 {
            return Some(VoxelId::DIRT);
        }
    }

    None
}

fn select_surface_material(wx: f32, wz: f32, height_m: f32) -> VoxelId {
    if let Some(road) = road_material_at(wx, wz) {
        return road;
    }

    // Muddy/gravel riverbanks close to the stream
    let d_stream = (wz - stream_center_z(wx)).abs();
    if d_stream < 10.5 && height_m < 3.75 {
        return VoxelId::DIRT;
    }

    // Rocky upper slopes & valley rim
    if height_m > 12.8 {
        return VoxelId::ROCK;
    }

    // Subtle meadow patches (mottled green fields)
    let patch = (wx * 0.055).sin() * (wz * 0.055).cos();
    if patch > 0.22 {
        VoxelId::GRASS_DARK
    } else {
        VoxelId::GRASS
    }
}

/// Builds a sturdy stone arch bridge carrying the main N-S highway across the valley stream,
/// smoothly graded between the actual north and south bank elevations.
fn build_stone_bridge(world: &mut VoxelWorld, water_vy: i32) {
    let center_z = stream_center_z(0.0); // ≈ -38.5 m
    let half_len_m = 16.5f32;
    let z_min_m = center_z - half_len_m;
    let z_max_m = center_z + half_len_m;

    // Sample actual bank heights just beyond each bridge abutment so approaches join seamlessly
    let north_h = european_valley_height_m(0.0, z_min_m - 0.75);
    let south_h = european_valley_height_m(0.0, z_max_m + 0.75);

    let vx_min = (-5.5f32 / WORLD_VOXEL_SIZE).floor() as i32; // -11
    let vx_max = (5.5f32 / WORLD_VOXEL_SIZE).floor() as i32; // +10
    let vz_min = (z_min_m / WORLD_VOXEL_SIZE).floor() as i32;
    let vz_max = (z_max_m / WORLD_VOXEL_SIZE).floor() as i32;

    for vz in vz_min..=vz_max {
        let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
        let dist_from_center = (wz - center_z).abs();
        let t = ((wz - z_min_m) / (z_max_m - z_min_m)).clamp(0.0, 1.0);
        let arch_crown = 0.35 * (t * std::f32::consts::PI).sin();
        let deck_h = north_h * (1.0 - t) + south_h * t + arch_crown;
        let deck_vy = (deck_h / WORLD_VOXEL_SIZE).round() as i32 - 1;

        for vx in vx_min..=vx_max {
            let is_parapet = vx == vx_min || vx == vx_max;

            // Clear air above bridge deck so roadway is completely open
            for vy in (deck_vy + 1)..=(deck_vy + 12) {
                world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
            }

            // Road deck surface
            let deck_mat = if is_parapet && dist_from_center <= 14.5 {
                VoxelId::COBBLESTONE
            } else {
                VoxelId::ROAD_ASPHALT
            };
            world.set_voxel_raw(IVec3::new(vx, deck_vy, vz), deck_mat);
            // Stone deck underside slab
            world.set_voxel_raw(IVec3::new(vx, deck_vy - 1, vz), VoxelId::COBBLESTONE);

            // Solid bridge abutments at ends + intermediate piers; open water arch in middle
            let is_abutment = dist_from_center >= 9.5;
            let is_pier = (dist_from_center - 4.5).abs() < 1.0 && (is_parapet || vx.abs() <= 1);
            if is_abutment || is_pier {
                let pier_bot = (water_vy - 3).max(world.bounds_min.y);
                world.fill_column_raw(vx, vz, pier_bot, deck_vy - 2, VoxelId::COBBLESTONE);
            } else {
                // Keep arch channel clear above water
                for vy in (water_vy + 1)..=(deck_vy - 2) {
                    world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
                }
            }

            // Low protective stone parapets (1.0 m tall) along the main bridge span
            if is_parapet && dist_from_center <= 14.5 {
                world.set_voxel_raw(IVec3::new(vx, deck_vy + 1, vz), VoxelId::COBBLESTONE);
                world.set_voxel_raw(IVec3::new(vx, deck_vy + 2, vz), VoxelId::COBBLESTONE);
            }
        }
    }
}

#[derive(Copy, Clone)]
enum DoorSide {
    North,
    South,
    East,
    West,
}

struct HouseSpec {
    cx: f32,
    cz: f32,
    width_m: f32,
    depth_m: f32,
    stories: u8,
    wall_mat: VoxelId,
    roof_mat: VoxelId,
    door_side: DoorSide,
}

impl HouseSpec {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        cx: f32,
        cz: f32,
        width_m: f32,
        depth_m: f32,
        stories: u8,
        wall_mat: VoxelId,
        roof_mat: VoxelId,
        door_side: DoorSide,
    ) -> Self {
        Self {
            cx,
            cz,
            width_m,
            depth_m,
            stories,
            wall_mat,
            roof_mat,
            door_side,
        }
    }
}

/// Builds a detailed hollow 1- or 2-story European voxel house with stone foundation, brick corners,
/// recessed dark windows, doorway, and pitched gable roof.
fn build_house(world: &mut VoxelWorld, spec: &HouseSpec) {
    let half_w = (spec.width_m * 0.5 / WORLD_VOXEL_SIZE).round() as i32;
    let half_d = (spec.depth_m * 0.5 / WORLD_VOXEL_SIZE).round() as i32;
    let cvx = (spec.cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (spec.cz / WORLD_VOXEL_SIZE).round() as i32;

    let min_x = cvx - half_w;
    let max_x = cvx + half_w;
    let min_z = cvz - half_d;
    let max_z = cvz + half_d;

    // Sample ground at footprint center for flush foundation
    let base_h = european_valley_height_m(spec.cx, spec.cz);
    let floor_vy = (base_h / WORLD_VOXEL_SIZE).round() as i32 - 1;
    let wall_h = if spec.stories >= 2 { 12 } else { 7 }; // 6.0 m or 3.5 m
    let top_wall_vy = (floor_vy + wall_h).min(world.bounds_max.y - 4);
    let foundation_bot = (floor_vy - 5).max(world.bounds_min.y);

    // 1. Foundation plinth + floor slab + clear interior air
    for vz in min_z..=max_z {
        for vx in min_x..=max_x {
            world.fill_column_raw(vx, vz, foundation_bot, floor_vy, VoxelId::COBBLESTONE);
            for vy in (floor_vy + 1)..=top_wall_vy {
                world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
            }
        }
    }

    // 2. Perimeter walls with brick quoins, recessed dark windows, and doorway
    for vy in (floor_vy + 1)..=top_wall_vy {
        let rel_y = vy - floor_vy;
        let is_window_row = rel_y == 3 || rel_y == 4 || (spec.stories >= 2 && (rel_y == 9 || rel_y == 10));
        let is_trim_row = rel_y == 1 || (spec.stories >= 2 && rel_y == 7);

        for vz in min_z..=max_z {
            for vx in min_x..=max_x {
                let on_x_edge = vx == min_x || vx == max_x;
                let on_z_edge = vz == min_z || vz == max_z;
                if !on_x_edge && !on_z_edge {
                    continue;
                }

                let is_corner = on_x_edge && on_z_edge;

                // Doorway cutout on specified facade (width 2 voxels = 1.0 m, height 4 voxels = 2.0 m)
                let is_door = rel_y <= 4
                    && match spec.door_side {
                        DoorSide::North => vz == min_z && (vx - cvx).abs() <= 1,
                        DoorSide::South => vz == max_z && (vx - cvx).abs() <= 1,
                        DoorSide::West => vx == min_x && (vz - cvz).abs() <= 1,
                        DoorSide::East => vx == max_x && (vz - cvz).abs() <= 1,
                    };
                if is_door {
                    world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::WOOD);
                    continue;
                }

                // Regular European facade window rhythm
                let along = if on_z_edge { (vx - min_x).abs() } else { (vz - min_z).abs() };
                let span = if on_z_edge { max_x - min_x } else { max_z - min_z };
                let is_window = is_window_row
                    && !is_corner
                    && along >= 3
                    && along <= span - 3
                    && (along % 5 == 3 || along % 5 == 4);

                let mat = if is_corner || is_trim_row {
                    if spec.wall_mat == VoxelId::BRICK {
                        VoxelId::COBBLESTONE
                    } else {
                        VoxelId::BRICK
                    }
                } else if is_window {
                    VoxelId::WINDOW_DARK
                } else {
                    spec.wall_mat
                };

                world.set_voxel_raw(IVec3::new(vx, vy, vz), mat);
            }
        }
    }

    // 3. Pitched gable roof with 1-voxel overhang eaves along the shorter span
    let gable_along_x = (max_x - min_x) <= (max_z - min_z);
    let half_span = if gable_along_x { half_w } else { half_d };
    let roof_layers = (half_span / 2 + 2).clamp(3, 6);

    for step in 0..roof_layers {
        let vy = top_wall_vy + 1 + step;
        if vy > world.bounds_max.y {
            break;
        }
        let inset = step - 1; // step 0 overhangs by 1 voxel!
        if gable_along_x {
            let rx_min = min_x + inset;
            let rx_max = max_x - inset;
            if rx_min > rx_max {
                break;
            }
            for vz in (min_z - 1)..=(max_z + 1) {
                let is_gable_end = vz == min_z || vz == max_z;
                for vx in rx_min..=rx_max {
                    let is_roof_shell =
                        vx == rx_min || vx == rx_max || step == roof_layers - 1 || vz < min_z || vz > max_z;
                    if is_roof_shell {
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), spec.roof_mat);
                    } else if is_gable_end {
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), spec.wall_mat);
                    }
                }
            }
        } else {
            let rz_min = min_z + inset;
            let rz_max = max_z - inset;
            if rz_min > rz_max {
                break;
            }
            for vx in (min_x - 1)..=(max_x + 1) {
                let is_gable_end = vx == min_x || vx == max_x;
                for vz in rz_min..=rz_max {
                    let is_roof_shell =
                        vz == rz_min || vz == rz_max || step == roof_layers - 1 || vx < min_x || vx > max_x;
                    if is_roof_shell {
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), spec.roof_mat);
                    } else if is_gable_end {
                        world.set_voxel_raw(IVec3::new(vx, vy, vz), spec.wall_mat);
                    }
                }
            }
        }
    }
}

/// Builds a landmark European village church with a tall stone bell tower and slate spire
/// sized to stay cleanly inside the primary `vy = 0..=31` chunk layer.
fn build_village_church(world: &mut VoxelWorld, cx: f32, cz: f32) {
    // Main nave
    let nave = HouseSpec::new(
        cx + 3.0,
        cz,
        14.0,
        10.0,
        2,
        VoxelId::COBBLESTONE,
        VoxelId::ROOF_SLATE,
        DoorSide::West,
    );
    build_house(world, &nave);

    // Tall square bell tower on the western entrance overlooking the town square
    let tower_cx = cx - 4.5;
    let tower_cvx = (tower_cx / WORLD_VOXEL_SIZE).round() as i32;
    let tower_cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let half = 4; // 4.5 m square tower
    let base_vy = (european_valley_height_m(tower_cx, cz) / WORLD_VOXEL_SIZE).round() as i32 - 1;
    let tower_h = 17; // 8.5 m stone shaft + 2.5 m slate spire = 11 m above village
    let foundation_bot = (base_vy - 5).max(world.bounds_min.y);

    for vz in (tower_cvz - half)..=(tower_cvz + half) {
        for vx in (tower_cvx - half)..=(tower_cvx + half) {
            world.fill_column_raw(vx, vz, foundation_bot, base_vy, VoxelId::COBBLESTONE);
            let on_edge = vx == tower_cvx - half
                || vx == tower_cvx + half
                || vz == tower_cvz - half
                || vz == tower_cvz + half;
            for vy in (base_vy + 1)..=(base_vy + tower_h).min(world.bounds_max.y) {
                let rel_y = vy - base_vy;
                if !on_edge {
                    world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::AIR);
                    continue;
                }
                // Belfry openings near top of tower
                let is_belfry = (12..=15).contains(&rel_y)
                    && ((vx - tower_cvx).abs() <= 1 || (vz - tower_cvz).abs() <= 1);
                let mat = if is_belfry {
                    VoxelId::WINDOW_DARK
                } else if rel_y == 10 || rel_y == tower_h {
                    VoxelId::PLASTER_WALL
                } else {
                    VoxelId::COBBLESTONE
                };
                world.set_voxel_raw(IVec3::new(vx, vy, vz), mat);
            }
        }
    }

    // Stepped slate spire atop the bell tower (stays strictly <= bounds_max.y = 31)
    for step in 0..=half {
        let vy = base_vy + tower_h + 1 + step;
        if vy > world.bounds_max.y {
            break;
        }
        let r = half - step;
        for vz in (tower_cvz - r)..=(tower_cvz + r) {
            for vx in (tower_cvx - r)..=(tower_cvx + r) {
                world.set_voxel_raw(IVec3::new(vx, vy, vz), VoxelId::ROOF_SLATE);
            }
        }
    }
}

/// Builds a terrain-conforming low wall, sandbag line, or bocage hedgerow between `(x0, z0)` and `(x1, z1)`.
fn build_low_wall(
    world: &mut VoxelWorld,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    height_vox: i32,
    mat: VoxelId,
) {
    let dist = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt();
    let steps = (dist / (WORLD_VOXEL_SIZE * 0.7)).ceil().max(1.0) as i32;
    let thickness = if mat == VoxelId::HEDGE { 1 } else { 0 };

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let wx = x0 + (x1 - x0) * t;
        let wz = z0 + (z1 - z0) * t;
        let cvx = (wx / WORLD_VOXEL_SIZE).floor() as i32;
        let cvz = (wz / WORLD_VOXEL_SIZE).floor() as i32;
        let base_vy = (european_valley_height_m(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1;
        let top_vy = (base_vy + height_vox).min(world.bounds_max.y);

        for dz in -thickness..=thickness {
            for dx in -thickness..=thickness {
                let vx = cvx + dx;
                let vz = cvz + dz;
                world.fill_column_raw(vx, vz, base_vy + 1, top_vy, mat);
            }
        }
    }
}

/// Builds an organic granite boulder outcrop anchored into the hillside.
fn build_boulder(world: &mut VoxelWorld, cx: f32, cz: f32, radius_m: f32, height_m: f32) {
    let rv = (radius_m / WORLD_VOXEL_SIZE).ceil() as i32;
    let hv = (height_m / WORLD_VOXEL_SIZE).ceil() as i32;
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;
    let base_vy = (european_valley_height_m(cx, cz) / WORLD_VOXEL_SIZE).round() as i32 - 1;

    for dz in -rv..=rv {
        for dx in -rv..=rv {
            let nx = dx as f32 / rv.max(1) as f32;
            let nz = dz as f32 / rv.max(1) as f32;
            let r2 = nx * nx + nz * nz;
            if r2 > 1.0 {
                continue;
            }
            let local_h = ((1.0 - r2).sqrt() * hv as f32).round() as i32;
            if local_h <= 0 {
                continue;
            }
            world.fill_column_raw(
                cvx + dx,
                cvz + dz,
                (base_vy - 1).max(world.bounds_min.y),
                (base_vy + local_h).min(world.bounds_max.y),
                VoxelId::ROCK,
            );
        }
    }
}

/// Paints a flush tactical zone ring directly onto terrain voxels for visual ground orientation.
fn paint_zone_ring(world: &mut VoxelWorld, cx: f32, cz: f32, radius_m: f32) {
    let outer = radius_m + 0.6;
    let inner = radius_m - 0.6;
    let rv = (outer / WORLD_VOXEL_SIZE).ceil() as i32;
    let cvx = (cx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (cz / WORLD_VOXEL_SIZE).round() as i32;

    for dz in -rv..=rv {
        for dx in -rv..=rv {
            let vx = cvx + dx;
            let vz = cvz + dz;
            let wx = (vx as f32 + 0.5) * WORLD_VOXEL_SIZE;
            let wz = (vz as f32 + 0.5) * WORLD_VOXEL_SIZE;
            let d = ((wx - cx).powi(2) + (wz - cz).powi(2)).sqrt();
            if d >= inner && d <= outer {
                let top_vy = (european_valley_height_m(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1;
                // Only paint on open ground (never overwrite buildings/walls)
                if world.is_solid(IVec3::new(vx, top_vy, vz))
                    && !world.is_solid(IVec3::new(vx, top_vy + 1, vz))
                {
                    world.set_voxel_raw(IVec3::new(vx, top_vy, vz), VoxelId::RANGE_MARKER);
                }
            }
        }
    }
}

/// Places deterministic stylized voxel trees across valley flanks, orchards, and ridges,
/// avoiding roads, buildings, capture zones, stream channel, and spawn pads.
fn place_valley_trees(
    world: &mut VoxelWorld,
    houses: &[HouseSpec],
    zones: &[(char, f32, f32, f32)],
) {
    // Deterministic 14 m candidate grid with jitter
    let spacing = 14.0f32;
    let mut gx = -168.0f32;
    while gx <= 168.0 {
        let mut gz = -168.0f32;
        while gz <= 168.0 {
            let h = hash2d(gx as i32, gz as i32);
            // Keep open sightlines: only place trees on ~32% of eligible rural grid cells
            if (h & 0xFF) < 82 {
                let jx = (((h >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 7.0;
                let jz = (((h >> 16) & 0xFF) as f32 / 255.0 - 0.5) * 7.0;
                let tx = gx + jx;
                let tz = gz + jz;

                if is_tree_spot_clear(tx, tz, houses, zones) {
                    let pine = (h & 0x1000000) != 0;
                    build_tree(world, tx, tz, pine);
                }
            }
            gz += spacing;
        }
        gx += spacing;
    }
}

fn is_tree_spot_clear(
    wx: f32,
    wz: f32,
    houses: &[HouseSpec],
    zones: &[(char, f32, f32, f32)],
) -> bool {
    if wx.abs() > 172.0 || wz.abs() > 172.0 {
        return false;
    }
    // Keep clear of stream channel & muddy banks
    if (wz - stream_center_z(wx)).abs() < 14.0 {
        return false;
    }
    // Keep clear of central village footprint & church
    if wx.abs() < 56.0 && (-18.0..=54.0).contains(&wz) {
        return false;
    }
    // Keep clear of spawn areas
    if wx.abs() < 52.0 && (wz > 128.0 || wz < -128.0) {
        return false;
    }
    // Keep clear of roads (check 4 m buffer around roads)
    for &(ox, oz) in &[
        (0.0, 0.0),
        (4.5, 0.0),
        (-4.5, 0.0),
        (0.0, 4.5),
        (0.0, -4.5),
    ] {
        if road_material_at(wx + ox, wz + oz).is_some() {
            return false;
        }
    }
    // Keep clear of capture zones
    for &(_, zx, zz, rad) in zones {
        if ((wx - zx).powi(2) + (wz - zz).powi(2)).sqrt() < rad + 6.0 {
            return false;
        }
    }
    // Keep clear of outlying houses
    for house in houses {
        if (wx - house.cx).abs() < house.width_m * 0.5 + 6.0
            && (wz - house.cz).abs() < house.depth_m * 0.5 + 6.0
        {
            return false;
        }
    }
    true
}

/// Builds a single stylized voxel tree with trunk clearance >= 3.0 m beneath foliage canopy.
/// Uses crisp tiered rectangular canopies so greedy meshing merges each canopy into minimal quads.
fn build_tree(world: &mut VoxelWorld, wx: f32, wz: f32, is_pine: bool) {
    let cvx = (wx / WORLD_VOXEL_SIZE).round() as i32;
    let cvz = (wz / WORLD_VOXEL_SIZE).round() as i32;
    let base_vy = (european_valley_height_m(wx, wz) / WORLD_VOXEL_SIZE).round() as i32 - 1;
    // Grow trees above water and below barren rocky peaks so crowns stay strictly within cy = 0
    if !(5..=19).contains(&base_vy) {
        return;
    }

    let trunk_h = if is_pine { 8 } else { 7 }; // 3.5–4.0 m trunk
    world.fill_column_raw(cvx, cvz, base_vy + 1, base_vy + trunk_h, VoxelId::TREE_TRUNK);

    let tiers: &[(i32, i32, i32)] = if is_pine {
        // (dy_start, dy_end, half_radius) — crisp 3-tier conifer silhouette
        &[(6, 7, 2), (8, 9, 1), (10, 11, 0)]
    } else {
        // Crisp 2-tier broadleaf oak canopy
        &[(6, 8, 2), (9, 10, 1)]
    };

    for &(dy0, dy1, r) in tiers {
        for dy in dy0..=dy1 {
            let vy = base_vy + dy;
            if vy > world.bounds_max.y {
                continue;
            }
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx != 0 || dz != 0 || dy > trunk_h {
                        world.set_voxel_raw(IVec3::new(cvx + dx, vy, cvz + dz), VoxelId::LEAVES);
                    }
                }
            }
        }
    }
}

#[inline]
fn hash2d(x: i32, z: i32) -> u32 {
    let mut h = (x.wrapping_mul(374_761_393) ^ z.wrapping_mul(668_265_263)) as u32;
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Generates a crisp top-down RGBA8 tactical minimap directly from `VoxelWorld` surface materials
/// and directional hillshade relief.
pub fn generate_minimap_data(world: &VoxelWorld, size: u32) -> MinimapData {
    let size = size.max(16);
    let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
    let span_x = (max_x - min_x).max(1.0);
    let span_z = (max_z - min_z).max(1.0);

    let count = (size * size) as usize;
    let mut heights = vec![0i32; count];
    let mut mats = vec![VoxelId::AIR; count];

    for py in 0..size {
        let wz = min_z + ((py as f32 + 0.5) / size as f32) * span_z;
        let vz = (wz / world.voxel_size).floor() as i32;
        for px in 0..size {
            let wx = min_x + ((px as f32 + 0.5) / size as f32) * span_x;
            let vx = (wx / world.voxel_size).floor() as i32;
            let idx = (py * size + px) as usize;
            if let Some((vy, id)) = world.top_solid_voxel_at(vx, vz) {
                heights[idx] = vy;
                mats[idx] = id;
            }
        }
    }

    let mut rgba = vec![0u8; count * 4];
    for py in 0..size {
        for px in 0..size {
            let idx = (py * size + px) as usize;
            let id = mats[idx];
            let base = world.materials.get(id).base_color;

            // Directional NW hillshade so hills, stream valley, and buildings pop clearly
            let h_here = heights[idx];
            let h_west = heights[(py * size + px.saturating_sub(1)) as usize];
            let h_north = heights[(py.saturating_sub(1) * size + px) as usize];
            let slope = ((h_here - h_west) + (h_here - h_north)) as f32;
            let hillshade = (1.0 + slope * 0.085).clamp(0.68, 1.28);

            // Slight readability boost for key tactical layers (roads, roofs, water)
            let (mut r, mut g, mut b) = (
                base[0] * hillshade,
                base[1] * hillshade,
                base[2] * hillshade,
            );
            if id == VoxelId::WATER {
                r *= 0.85;
                g *= 1.05;
                b = (b * 1.25).min(1.0);
            } else if id == VoxelId::ROOF_TILE || id == VoxelId::ROOF_SLATE {
                r = (r * 1.15).min(1.0);
                g = (g * 1.10).min(1.0);
                b = (b * 1.10).min(1.0);
            }

            // Subtle 6x6 tactical map grid overlay
            let grid_step = (size / 6).max(1);
            let on_grid = (px > 0 && px % grid_step == 0) || (py > 0 && py % grid_step == 0);
            if on_grid {
                r = r * 0.82 + 0.14;
                g = g * 0.82 + 0.16;
                b = b * 0.82 + 0.14;
            }

            let out = idx * 4;
            rgba[out] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[out + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[out + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[out + 3] = 255;
        }
    }

    MinimapData { size, rgba }
}
