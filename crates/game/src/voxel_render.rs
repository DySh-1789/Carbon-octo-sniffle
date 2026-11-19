use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use voxel::glam::IVec3;
use voxel::{VoxelMeshData, mesh_world_chunk};

use crate::bridge::{SimResource, to_bevy_vec3};

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

/// Converts a greedy-meshed `VoxelMeshData` into a Bevy `Mesh`.
pub fn voxel_mesh_to_bevy(data: &VoxelMeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colors.clone());
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}

pub struct VoxelRenderPlugin;

impl Plugin for VoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMeshEntities>()
            .add_systems(Startup, (setup_world_material, build_initial_world_chunks).chain())
            .add_systems(Update, sync_dirty_world_chunks);
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
    let dirty = sim.0.voxels.take_dirty_chunks(usize::MAX);
    rebuild_chunks(&mut commands, &sim.0, &mut meshes, &material.0, &mut registry, &dirty);
}

fn sync_dirty_world_chunks(
    mut commands: Commands,
    mut sim: ResMut<SimResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WorldVoxelMaterial>,
    mut registry: ResMut<ChunkMeshEntities>,
) {
    // Budgeted remeshing (max 4 chunks per frame) to prevent frame drops during explosions
    let dirty = sim.0.voxels.take_dirty_chunks(4);
    if dirty.is_empty() {
        return;
    }
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
        let mesh_handle = meshes.add(voxel_mesh_to_bevy(&mesh_data));
        let origin = to_bevy_vec3(sim.voxels.chunk_world_origin(coord));
        let entity = commands
            .spawn((
                VoxelChunkMesh { coord },
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(origin),
            ))
            .id();
        registry.map.insert(coord, entity);
    }
}
