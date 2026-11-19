use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};

use crate::material::{MaterialRegistry, VoxelId};

/// Edge length of a cubic world chunk in voxels (`32 × 32 × 32`).
pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// World voxel resolution in meters (`0.5 m` → `16 m` per chunk).
pub const WORLD_VOXEL_SIZE: f32 = 0.5;

/// Detailed tank voxel resolution in meters (`0.2 m`, Teardown-inspired scale).
pub const TANK_VOXEL_SIZE: f32 = 0.2;

/// Single `32 × 32 × 32` chunk of world voxels.
#[derive(Clone)]
pub struct VoxelChunk {
    voxels: Box<[VoxelId; CHUNK_VOLUME]>,
    non_air_count: u32,
}

impl Default for VoxelChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelChunk {
    pub fn new() -> Self {
        Self {
            voxels: Box::new([VoxelId::AIR; CHUNK_VOLUME]),
            non_air_count: 0,
        }
    }

    #[inline]
    pub const fn index(x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> VoxelId {
        self.voxels[Self::index(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, id: VoxelId) -> bool {
        let idx = Self::index(x, y, z);
        let prev = self.voxels[idx];
        if prev == id {
            return false;
        }
        if prev.is_air() && !id.is_air() {
            self.non_air_count += 1;
        } else if !prev.is_air() && id.is_air() {
            self.non_air_count -= 1;
        }
        self.voxels[idx] = id;
        true
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.non_air_count == 0
    }
}

/// Chunked 3D voxel world with automatic dirty-chunk tracking for incremental remeshing.
pub struct VoxelWorld {
    pub voxel_size: f32,
    pub materials: MaterialRegistry,
    chunks: HashMap<IVec3, VoxelChunk>,
    dirty_chunks: HashSet<IVec3>,
    /// Inclusive voxel coordinate bounds of the playable map.
    pub bounds_min: IVec3,
    pub bounds_max: IVec3,
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self::new(WORLD_VOXEL_SIZE)
    }
}

impl VoxelWorld {
    pub fn new(voxel_size: f32) -> Self {
        Self {
            voxel_size,
            materials: MaterialRegistry::new(),
            chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            bounds_min: IVec3::ZERO,
            bounds_max: IVec3::ZERO,
        }
    }

    #[inline]
    pub fn voxel_to_chunk_and_local(pos: IVec3) -> (IVec3, (usize, usize, usize)) {
        let cx = pos.x.div_euclid(CHUNK_SIZE_I32);
        let cy = pos.y.div_euclid(CHUNK_SIZE_I32);
        let cz = pos.z.div_euclid(CHUNK_SIZE_I32);
        let lx = pos.x.rem_euclid(CHUNK_SIZE_I32) as usize;
        let ly = pos.y.rem_euclid(CHUNK_SIZE_I32) as usize;
        let lz = pos.z.rem_euclid(CHUNK_SIZE_I32) as usize;
        (IVec3::new(cx, cy, cz), (lx, ly, lz))
    }

    #[inline]
    pub fn world_to_voxel(&self, world_pos: Vec3) -> IVec3 {
        let inv = 1.0 / self.voxel_size;
        IVec3::new(
            (world_pos.x * inv).floor() as i32,
            (world_pos.y * inv).floor() as i32,
            (world_pos.z * inv).floor() as i32,
        )
    }

    #[inline]
    pub fn voxel_to_world_min(&self, pos: IVec3) -> Vec3 {
        pos.as_vec3() * self.voxel_size
    }

    #[inline]
    pub fn voxel_to_world_center(&self, pos: IVec3) -> Vec3 {
        (pos.as_vec3() + Vec3::splat(0.5)) * self.voxel_size
    }

    #[inline]
    pub fn chunk_world_origin(&self, chunk_coord: IVec3) -> Vec3 {
        (chunk_coord * CHUNK_SIZE_I32).as_vec3() * self.voxel_size
    }

    /// World-space horizontal bounds `(min_x, max_x, min_z, max_z)`.
    pub fn playable_bounds_xz(&self) -> (f32, f32, f32, f32) {
        (
            self.bounds_min.x as f32 * self.voxel_size,
            (self.bounds_max.x + 1) as f32 * self.voxel_size,
            self.bounds_min.z as f32 * self.voxel_size,
            (self.bounds_max.z + 1) as f32 * self.voxel_size,
        )
    }

    #[inline]
    pub fn get_voxel(&self, pos: IVec3) -> VoxelId {
        let (chunk_coord, (lx, ly, lz)) = Self::voxel_to_chunk_and_local(pos);
        self.chunks
            .get(&chunk_coord)
            .map_or(VoxelId::AIR, |c| c.get(lx, ly, lz))
    }

    #[inline]
    pub fn is_solid(&self, pos: IVec3) -> bool {
        self.materials.is_solid(self.get_voxel(pos))
    }

    /// Sets a voxel and marks affected chunks (including neighbors when touching chunk borders)
    /// as dirty so greedy meshes & cross-chunk AO stay seamless.
    pub fn set_voxel(&mut self, pos: IVec3, id: VoxelId) {
        let (chunk_coord, (lx, ly, lz)) = Self::voxel_to_chunk_and_local(pos);
        if id.is_air() && !self.chunks.contains_key(&chunk_coord) {
            return;
        }
        let chunk = self.chunks.entry(chunk_coord).or_default();
        if !chunk.set(lx, ly, lz, id) {
            return;
        }
        let remove_empty = chunk.is_empty();
        if remove_empty {
            self.chunks.remove(&chunk_coord);
        }

        self.dirty_chunks.insert(chunk_coord);

        // Neighbor chunks need remeshing if an edited voxel lies on a shared boundary
        let mark_neighbor = |dirty: &mut HashSet<IVec3>, chunks: &HashMap<IVec3, VoxelChunk>, c: IVec3| {
            if chunks.contains_key(&c) {
                dirty.insert(c);
            }
        };
        if lx == 0 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord - IVec3::X);
        } else if lx == CHUNK_SIZE - 1 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord + IVec3::X);
        }
        if ly == 0 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord - IVec3::Y);
        } else if ly == CHUNK_SIZE - 1 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord + IVec3::Y);
        }
        if lz == 0 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord - IVec3::Z);
        } else if lz == CHUNK_SIZE - 1 {
            mark_neighbor(&mut self.dirty_chunks, &self.chunks, chunk_coord + IVec3::Z);
        }
    }

    /// Fast bulk fill without per-voxel dirty neighbor overhead; marks all touched chunks dirty
    /// at the end.
    pub fn fill_box(&mut self, min: IVec3, max: IVec3, id: VoxelId) {
        let mut touched = HashSet::new();
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                for x in min.x..=max.x {
                    let pos = IVec3::new(x, y, z);
                    let (chunk_coord, (lx, ly, lz)) = Self::voxel_to_chunk_and_local(pos);
                    if id.is_air() && !self.chunks.contains_key(&chunk_coord) {
                        continue;
                    }
                    let chunk = self.chunks.entry(chunk_coord).or_default();
                    if chunk.set(lx, ly, lz, id) {
                        touched.insert(chunk_coord);
                    }
                }
            }
        }
        for c in touched {
            if self.chunks.get(&c).is_some_and(VoxelChunk::is_empty) {
                self.chunks.remove(&c);
            }
            self.dirty_chunks.insert(c);
        }
    }

    /// Fast unchecked voxel write used during batch map generation.
    /// Caller must invoke [`Self::mark_all_dirty`] once generation finishes.
    #[inline]
    pub fn set_voxel_raw(&mut self, pos: IVec3, id: VoxelId) {
        let (chunk_coord, (lx, ly, lz)) = Self::voxel_to_chunk_and_local(pos);
        if id.is_air() && !self.chunks.contains_key(&chunk_coord) {
            return;
        }
        let chunk = self.chunks.entry(chunk_coord).or_default();
        chunk.set(lx, ly, lz, id);
    }

    /// Fast vertical column fill `(vx, y_min..=y_max, vz)` for terrain generation.
    /// Caller must invoke [`Self::mark_all_dirty`] once generation finishes.
    pub fn fill_column_raw(&mut self, vx: i32, vz: i32, y_min: i32, y_max: i32, id: VoxelId) {
        if y_min > y_max {
            return;
        }
        let cx = vx.div_euclid(CHUNK_SIZE_I32);
        let cz = vz.div_euclid(CHUNK_SIZE_I32);
        let lx = vx.rem_euclid(CHUNK_SIZE_I32) as usize;
        let lz = vz.rem_euclid(CHUNK_SIZE_I32) as usize;

        let mut y = y_min;
        while y <= y_max {
            let cy = y.div_euclid(CHUNK_SIZE_I32);
            let chunk_coord = IVec3::new(cx, cy, cz);
            let chunk_top = ((cy + 1) * CHUNK_SIZE_I32 - 1).min(y_max);
            if id.is_air() && !self.chunks.contains_key(&chunk_coord) {
                y = chunk_top + 1;
                continue;
            }
            let chunk = self.chunks.entry(chunk_coord).or_default();
            for vy in y..=chunk_top {
                let ly = vy.rem_euclid(CHUNK_SIZE_I32) as usize;
                chunk.set(lx, ly, lz, id);
            }
            y = chunk_top + 1;
        }
    }

    pub fn chunk(&self, coord: IVec3) -> Option<&VoxelChunk> {
        self.chunks.get(&coord)
    }

    pub fn chunk_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.chunks.keys().copied()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Pops up to `limit` dirty chunk coordinates for budgeted per-frame remeshing.
    pub fn take_dirty_chunks(&mut self, limit: usize) -> Vec<IVec3> {
        let mut out = Vec::with_capacity(limit.min(self.dirty_chunks.len()));
        for coord in self.dirty_chunks.iter().copied().take(limit) {
            out.push(coord);
        }
        for coord in &out {
            self.dirty_chunks.remove(coord);
        }
        out
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_chunks.extend(self.chunks.keys().copied());
    }

    /// Returns `(vy, material)` of the highest solid voxel in column `(vx, vz)`.
    pub fn top_solid_voxel_at(&self, vx: i32, vz: i32) -> Option<(i32, VoxelId)> {
        for vy in (self.bounds_min.y..=self.bounds_max.y).rev() {
            let id = self.get_voxel(IVec3::new(vx, vy, vz));
            if self.materials.is_solid(id) {
                return Some((vy, id));
            }
        }
        None
    }

    /// Finds the world-space Y coordinate of the top face of the highest solid voxel at `(x, z)`
    /// scanning downward from `bounds_max.y` to `bounds_min.y`.
    pub fn surface_height_at(&self, world_x: f32, world_z: f32) -> Option<f32> {
        let vx = (world_x / self.voxel_size).floor() as i32;
        let vz = (world_z / self.voxel_size).floor() as i32;
        self.top_solid_voxel_at(vx, vz)
            .map(|(vy, _)| (vy + 1) as f32 * self.voxel_size)
    }

    /// Finds the nearest driveable ground surface around `ref_y` within `[ref_y - max_drop, ref_y + max_step_up]`.
    ///
    /// Requires 2 open air voxels (`1.0 m` at standard world scale) directly above the surface so
    /// interiors of walls, overhanging tree canopies, and roofs are ignored when sampling ground height.
    pub fn ground_height_near(
        &self,
        world_x: f32,
        world_z: f32,
        ref_y: f32,
        max_step_up: f32,
        max_drop: f32,
    ) -> Option<f32> {
        let vx = (world_x / self.voxel_size).floor() as i32;
        let vz = (world_z / self.voxel_size).floor() as i32;
        let top_vy = (((ref_y + max_step_up) / self.voxel_size).floor() as i32 - 1)
            .min(self.bounds_max.y);
        let bottom_vy = ((((ref_y - max_drop) / self.voxel_size).ceil() as i32) - 1)
            .max(self.bounds_min.y);
        if top_vy < bottom_vy {
            return None;
        }
        for vy in (bottom_vy..=top_vy).rev() {
            if self.is_solid(IVec3::new(vx, vy, vz))
                && !self.is_solid(IVec3::new(vx, vy + 1, vz))
                && !self.is_solid(IVec3::new(vx, vy + 2, vz))
            {
                return Some((vy + 1) as f32 * self.voxel_size);
            }
        }
        None
    }
}
