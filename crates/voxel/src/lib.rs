pub mod chunk;
pub mod material;
pub mod mesher;
pub mod model;
pub mod raycast;

pub use chunk::{CHUNK_SIZE, CHUNK_SIZE_I32, TANK_VOXEL_SIZE, VoxelChunk, VoxelWorld, WORLD_VOXEL_SIZE};
pub use glam;
pub use material::{MaterialRegistry, VoxelId, VoxelMaterial};
pub use mesher::{VoxelMeshData, mesh_voxel_model, mesh_world_chunk};
pub use model::{TankVoxelModels, VoxelModel, build_basic_tank_models};
pub use raycast::{VoxelHit, raycast_world};

#[cfg(test)]
mod tests {
    use glam::{IVec3, UVec3, Vec3};

    use super::*;

    #[test]
    fn chunk_set_get_and_cross_chunk_dirty_tracking() {
        let mut world = VoxelWorld::default();
        // Place solid voxel at x=0 (left edge of chunk 0) and x=-1 (right edge of chunk -1)
        world.set_voxel(IVec3::new(0, 0, 0), VoxelId::GRASS);
        world.set_voxel(IVec3::new(-1, 0, 0), VoxelId::DIRT);
        assert_eq!(world.get_voxel(IVec3::new(0, 0, 0)), VoxelId::GRASS);
        assert_eq!(world.get_voxel(IVec3::new(-1, 0, 0)), VoxelId::DIRT);
        assert_eq!(world.chunk_count(), 2);

        let dirty = world.take_dirty_chunks(16);
        assert_eq!(dirty.len(), 2);

        // Editing border voxel at x=0 must also dirty neighboring chunk (-1, 0, 0)
        world.set_voxel(IVec3::new(0, 0, 0), VoxelId::AIR);
        let dirty_after = world.take_dirty_chunks(16);
        assert!(dirty_after.contains(&IVec3::ZERO));
        assert!(dirty_after.contains(&IVec3::new(-1, 0, 0)));
    }

    #[test]
    fn greedy_mesher_merges_uniform_slab_and_has_outward_winding() {
        let mut world = VoxelWorld::default();
        // Use GLASS_OPTIC which has color_jitter = 0 so the entire top merges into 1 quad
        world.fill_box(IVec3::new(0, 0, 0), IVec3::new(7, 0, 7), VoxelId::GLASS_OPTIC);
        let mesh = mesh_world_chunk(&world, IVec3::ZERO);

        // A uniform 8x1x8 box has 6 sides -> 6 quads -> exactly 12 triangles!
        assert_eq!(mesh.triangle_count(), 12);

        // Verify every triangle has right-handed CCW winding matching its stored outward normal
        for tri in mesh.indices.as_chunks::<3>().0 {
            let p0 = Vec3::from_array(mesh.positions[tri[0] as usize]);
            let p1 = Vec3::from_array(mesh.positions[tri[1] as usize]);
            let p2 = Vec3::from_array(mesh.positions[tri[2] as usize]);
            let stored_n = Vec3::from_array(mesh.normals[tri[0] as usize]);
            let geo_n = (p1 - p0).cross(p2 - p0);
            assert!(
                geo_n.dot(stored_n) > 0.0,
                "Inverted winding! geo_n={geo_n:?}, stored_n={stored_n:?}"
            );
        }
    }

    #[test]
    fn greedy_mesher_culls_hidden_faces_across_chunk_boundaries() {
        let mut world = VoxelWorld::default();
        // Two adjacent voxels across chunk x=0 / x=-1 boundary
        world.set_voxel(IVec3::new(-1, 0, 0), VoxelId::GLASS_OPTIC);
        world.set_voxel(IVec3::new(0, 0, 0), VoxelId::GLASS_OPTIC);

        let mesh_a = mesh_world_chunk(&world, IVec3::new(-1, 0, 0));
        let mesh_b = mesh_world_chunk(&world, IVec3::ZERO);

        // Each voxel should have 5 visible faces (10 triangles), not 6 (12 triangles)
        assert_eq!(mesh_a.triangle_count(), 10);
        assert_eq!(mesh_b.triangle_count(), 10);
    }

    #[test]
    fn voxel_ao_darkens_concave_corner() {
        let mut world = VoxelWorld::default();
        // Flat floor + wall rising next to (1,0,1) creates occlusion on its top face
        world.fill_box(IVec3::new(0, 0, 0), IVec3::new(2, 0, 2), VoxelId::GLASS_OPTIC);
        world.set_voxel(IVec3::new(0, 1, 1), VoxelId::GLASS_OPTIC);
        let mesh = mesh_world_chunk(&world, IVec3::ZERO);

        let mut min_top_brightness = 1.0f32;
        let mut max_top_brightness = 0.0f32;
        for (n, c) in mesh.normals.iter().zip(mesh.colors.iter()) {
            if n[1] > 0.5 {
                let lum = c[0] + c[1] + c[2];
                min_top_brightness = min_top_brightness.min(lum);
                max_top_brightness = max_top_brightness.max(lum);
            }
        }
        assert!(
            min_top_brightness < max_top_brightness,
            "Expected vertex AO to darken concave edge"
        );
    }

    #[test]
    fn raycast_hits_top_of_flat_floor_precisely() {
        let mut world = VoxelWorld::default();
        // Solid layer at vy = -1 means top surface is at world Y = 0.0
        world.fill_box(IVec3::new(-4, -1, -4), IVec3::new(4, -1, 4), VoxelId::GRASS);

        let hit = raycast_world(&world, Vec3::new(0.25, 10.0, 0.25), Vec3::NEG_Y, 50.0)
            .expect("ray must hit flat floor");
        assert_eq!(hit.normal, IVec3::Y);
        assert_eq!(hit.voxel.y, -1);
        assert!((hit.distance - 10.0).abs() < 1e-4, "distance={}", hit.distance);
        assert!(hit.point.y.abs() < 1e-4, "point={:?}", hit.point);
    }

    #[test]
    fn basic_tank_models_build_and_mesh_cleanly() {
        let registry = MaterialRegistry::new();
        let models = build_basic_tank_models();
        assert!(models.hull.non_air_count() > 500);
        assert!(models.turret.non_air_count() > 100);
        assert!(models.barrel.non_air_count() > 20);

        for m in [&models.hull, &models.turret, &models.barrel] {
            let mesh = mesh_voxel_model(m, &registry);
            assert!(!mesh.is_empty());
            for tri in mesh.indices.as_chunks::<3>().0 {
                let p0 = Vec3::from_array(mesh.positions[tri[0] as usize]);
                let p1 = Vec3::from_array(mesh.positions[tri[1] as usize]);
                let p2 = Vec3::from_array(mesh.positions[tri[2] as usize]);
                let stored_n = Vec3::from_array(mesh.normals[tri[0] as usize]);
                assert!((p1 - p0).cross(p2 - p0).dot(stored_n) > 0.0);
            }
        }

        // Empty model produces empty mesh
        let empty = VoxelModel::new(UVec3::splat(4), Vec3::ZERO, TANK_VOXEL_SIZE);
        assert!(mesh_voxel_model(&empty, &registry).is_empty());
    }

    #[test]
    fn stage2_materials_registered_and_ground_height_near_ignores_canopy_overhang() {
        let registry = MaterialRegistry::new();
        for id in [
            VoxelId::ROAD_ASPHALT,
            VoxelId::COBBLESTONE,
            VoxelId::ROCK,
            VoxelId::BRICK,
            VoxelId::PLASTER_WALL,
            VoxelId::WOOD,
            VoxelId::ROOF_TILE,
            VoxelId::ROOF_SLATE,
            VoxelId::HEDGE,
            VoxelId::WATER,
            VoxelId::SANDBAG,
            VoxelId::TREE_TRUNK,
            VoxelId::LEAVES,
            VoxelId::WINDOW_DARK,
        ] {
            let mat = registry.get(id);
            assert_ne!(mat.name, "Air", "material {:?} fell back to Air", id);
            assert!(registry.is_solid(id));
        }

        // Ground at vy = -1 (top at Y = 0.0), overhanging tree leaves at vy = 7..9 (Y = 3.5..5.0)
        let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);
        world.bounds_min = IVec3::new(-4, -2, -4);
        world.bounds_max = IVec3::new(4, 12, 4);
        world.fill_column_raw(0, 0, -2, -1, VoxelId::GRASS);
        world.fill_column_raw(0, 0, 7, 9, VoxelId::LEAVES);

        // surface_height_at sees top of canopy, whereas ground_height_near finds the road/ground below
        assert_eq!(world.surface_height_at(0.25, 0.25), Some(5.0));
        assert_eq!(
            world.ground_height_near(0.25, 0.25, 0.0, 0.75, 2.0),
            Some(0.0)
        );
    }

    #[test]
    fn sparse_chunk_compaction_and_nearest_dirty_ordering() {
        let mut world = VoxelWorld::new(WORLD_VOXEL_SIZE);
        // Fill bottom 2 slices of chunk (0,0,0) uniformly with ROCK
        for z in 0..CHUNK_SIZE_I32 {
            for x in 0..CHUNK_SIZE_I32 {
                world.fill_column_raw(x, z, 0, 1, VoxelId::ROCK);
            }
        }
        // Plus one single mixed surface voxel at y = 2
        world.set_voxel_raw(IVec3::new(5, 2, 5), VoxelId::GRASS);

        let before = world.memory_bytes();
        world.compact_chunks();
        let after = world.memory_bytes();
        assert!(
            after < before,
            "compacting uniform rock slices should reduce heap memory: before={before}, after={after}"
        );
        let chunk = world.chunk(IVec3::ZERO).unwrap();
        assert_eq!(chunk.uniform_slice(0), Some(VoxelId::ROCK));
        assert_eq!(chunk.uniform_slice(1), Some(VoxelId::ROCK));
        assert_eq!(chunk.uniform_slice(2), None);
        assert_eq!(chunk.uniform_slice(3), Some(VoxelId::AIR));

        // Verify nearest-first dirty chunk prioritization
        world.set_voxel(IVec3::new(0, 0, 0), VoxelId::DIRT); // chunk (0,0,0)
        world.set_voxel(IVec3::new(64, 0, 0), VoxelId::DIRT); // chunk (2,0,0)
        world.set_voxel(IVec3::new(160, 0, 0), VoxelId::DIRT); // chunk (5,0,0)
        let nearest = world.take_dirty_chunks_nearest(Vec3::new(8.0, 0.0, 8.0), 2);
        assert_eq!(nearest.len(), 2);
        assert!(nearest.contains(&IVec3::ZERO));
        assert!(nearest.contains(&IVec3::new(2, 0, 0)));
    }
}
