use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use voxel::glam::IVec3;
use voxel::{VoxelMeshData, mesh_world_chunk};

use crate::bridge::{SimResource, to_bevy_vec3};
use crate::state::{AppState, BattleEntity};

/// Number of chunks nearest to spawn meshed immediately on battle entry so the visible area is solid on Frame 1.
const INITIAL_NEAR_CHUNKS: usize = 36;

/// Max dirty chunks meshed per frame during streaming & runtime updates.
/// Keeping batches modest lets `RenderAssetUsages::RENDER_WORLD` release CPU mesh memory between
/// frames and avoids huge WASM/wgpu Frame-0 staging buffer spikes.
const STREAM_CHUNKS_PER_FRAME: usize = 16;

/// Tracks spawned Bevy entities corresponding to meshed voxel chunks.
#[derive(Resource, Default)]
pub struct ChunkMeshEntities {
    pub map: HashMap<IVec3, Entity>,
}

/// Shared PBR material for world terrain voxels (multiplies with baked vertex AO & colors).
#[derive(Resource)]
pub struct WorldVoxelMaterial(pub Handle<StandardMaterial>);

#[derive(Component)]
pub struct VoxelChunkMesh {
    pub coord: IVec3,
}

/// Converts a greedy-meshed `VoxelMeshData` into a Bevy `Mesh` by value (zero vector cloning)
/// and packs triangle indices as `U16` whenever vertex count fits (`<= 65,535`), halving index RAM.
pub fn voxel_mesh_to_bevy(data: VoxelMeshData) -> Mesh {
    let use_u16 = data.positions.len() <= u16::MAX as usize;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colors);
    if use_u16 {
        let indices_u16: Vec<u16> = data.indices.into_iter().map(|i| i as u16).collect();
        mesh.insert_indices(Indices::U16(indices_u16));
    } else {
        mesh.insert_indices(Indices::U32(data.indices));
    }
    mesh
}

pub struct VoxelRenderPlugin;

impl Plugin for VoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMeshEntities>()
            .add_systems(Startup, setup_world_material)
            .add_systems(OnEnter(AppState::InBattle), build_initial_world_chunks)
            .add_systems(
                Update,
                sync_dirty_world_chunks.run_if(in_state(AppState::InBattle)),
            )
            .add_systems(OnExit(AppState::InBattle), clear_chunk_registry);
    }
}

fn setup_world_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.90,
        metallic: 0.04,
        reflectance: 0.2,
        ..default()
    });
    commands.insert_resource(WorldVoxelMaterial(mat));
}

fn build_initial_world_chunks(
    mut commands: Commands,
    mut sim: ResMut<SimResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WorldVoxelMaterial>,
    mut registry: ResMut<ChunkMeshEntities>,
) {
    registry.map.clear();
    sim.0.voxels.mark_all_dirty();
    let focus = sim.0.player_tank().position;
    let dirty = sim
        .0
        .voxels
        .take_dirty_chunks_nearest(focus, INITIAL_NEAR_CHUNKS);
    rebuild_chunks(&mut commands, &sim.0, &mut meshes, &material.0, &mut registry, &dirty);
}

fn clear_chunk_registry(mut registry: ResMut<ChunkMeshEntities>) {
    // Entities themselves are despawned by the centralized `despawn_tagged::<BattleEntity>` cleanup.
    registry.map.clear();
}

fn sync_dirty_world_chunks(
    mut commands: Commands,
    mut sim: ResMut<SimResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WorldVoxelMaterial>,
    mut registry: ResMut<ChunkMeshEntities>,
) {
    if sim.0.voxels.dirty_chunk_count() == 0 {
        return;
    }
    let focus = sim.0.player_tank().position;
    let dirty = sim
        .0
        .voxels
        .take_dirty_chunks_nearest(focus, STREAM_CHUNKS_PER_FRAME);
    rebuild_chunks(&mut commands, &sim.0, &mut meshes, &material.0, &mut registry, &dirty);
}

fn rebuild_chunks(
    commands: &mut Commands,
    sim: &sim::SimWorld,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    registry: &mut ChunkMeshEntities,
    coords: &[IVec3],
) {
    for &coord in coords {
        if let Some(old_entity) = registry.map.remove(&coord) {
            commands.entity(old_entity).despawn();
        }
        let mesh_data = mesh_world_chunk(&sim.voxels, coord);
        // Never spawn empty meshes (avoids unnecessary draw calls and Bevy empty-mesh errors)
        if mesh_data.is_empty() {
            continue;
        }
        let mesh_handle = meshes.add(voxel_mesh_to_bevy(mesh_data));
        let origin = to_bevy_vec3(sim.voxels.chunk_world_origin(coord));
        let entity = commands
            .spawn((
                BattleEntity,
                VoxelChunkMesh { coord },
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(origin),
            ))
            .id();
        registry.map.insert(coord, entity);
    }
}
