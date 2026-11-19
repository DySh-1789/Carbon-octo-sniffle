use glam::{IVec3, Vec3};

use crate::chunk::{CHUNK_SIZE, CHUNK_SIZE_I32, VoxelWorld};
use crate::material::{MaterialRegistry, VoxelId};
use crate::model::VoxelModel;

/// Indexed triangle mesh produced by the voxel greedy mesher.
#[derive(Clone, Debug, Default)]
pub struct VoxelMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl VoxelMeshData {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Face descriptors with right-handed bases satisfying `u_axis × v_axis == normal`.
#[derive(Copy, Clone)]
struct FaceSpec {
    normal: IVec3,
    u_axis: IVec3,
    v_axis: IVec3,
    /// Offset along normal from voxel min corner (`1` for positive faces, `0` for negative faces).
    normal_offset: i32,
    /// Subtle directional shading factor to emphasize voxel geometry.
    face_shade: f32,
}

const FACES: [FaceSpec; 6] = [
    // +X: (+Y) x (+Z) = +X
    FaceSpec {
        normal: IVec3::X,
        u_axis: IVec3::Y,
        v_axis: IVec3::Z,
        normal_offset: 1,
        face_shade: 0.92,
    },
    // -X: (+Z) x (+Y) = -X
    FaceSpec {
        normal: IVec3::NEG_X,
        u_axis: IVec3::Z,
        v_axis: IVec3::Y,
        normal_offset: 0,
        face_shade: 0.88,
    },
    // +Y: (+Z) x (+X) = +Y
    FaceSpec {
        normal: IVec3::Y,
        u_axis: IVec3::Z,
        v_axis: IVec3::X,
        normal_offset: 1,
        face_shade: 1.00,
    },
    // -Y: (+X) x (+Z) = -Y
    FaceSpec {
        normal: IVec3::NEG_Y,
        u_axis: IVec3::X,
        v_axis: IVec3::Z,
        normal_offset: 0,
        face_shade: 0.72,
    },
    // +Z: (+X) x (+Y) = +Z
    FaceSpec {
        normal: IVec3::Z,
        u_axis: IVec3::X,
        v_axis: IVec3::Y,
        normal_offset: 1,
        face_shade: 0.90,
    },
    // -Z: (+Y) x (+X) = -Z
    FaceSpec {
        normal: IVec3::NEG_Z,
        u_axis: IVec3::Y,
        v_axis: IVec3::X,
        normal_offset: 0,
        face_shade: 0.85,
    },
];

/// Vertex AO brightness table indexed by occlusion level `0..=3` (0 = occluded, 3 = open).
const AO_CURVE: [f32; 4] = [0.52, 0.70, 0.86, 1.0];

#[derive(Copy, Clone, PartialEq, Eq)]
struct MaskCell {
    id: VoxelId,
    shade_bucket: u8,
    ao: [u8; 4],
}

#[inline]
fn vertex_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (side1 as u8 + side2 as u8 + corner as u8)
    }
}

#[inline]
fn compute_face_ao<F>(pos: IVec3, face: &FaceSpec, is_solid: &F) -> [u8; 4]
where
    F: Fn(IVec3) -> bool,
{
    let base = pos + face.normal;
    // Corner order matches quad corners: c00 (-u,-v), c10 (+u,-v), c11 (+u,+v), c01 (-u,+v)
    const SIGNS: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];
    let mut ao = [3u8; 4];
    for (i, &(su, sv)) in SIGNS.iter().enumerate() {
        let s1 = is_solid(base + face.u_axis * su);
        let s2 = is_solid(base + face.v_axis * sv);
        let c = is_solid(base + face.u_axis * su + face.v_axis * sv);
        ao[i] = vertex_ao(s1, s2, c);
    }
    ao
}

/// Generic greedy mesher over a bounded integer box `[0..dims.x, 0..dims.y, 0..dims.z]`.
///
/// Vertex positions are emitted in local meters:
/// `(local_voxel_coord - pivot) * voxel_size`.
#[allow(clippy::too_many_arguments)]
fn greedy_mesh_volume<FG, FS, FC, FY>(
    dims: IVec3,
    voxel_origin: IVec3,
    pivot: Vec3,
    voxel_size: f32,
    materials: &MaterialRegistry,
    get_voxel: FG,
    is_solid: FS,
    cull_face: FC,
    is_y_layer_air: FY,
) -> VoxelMeshData
where
    FG: Fn(IVec3) -> VoxelId,
    FS: Fn(IVec3) -> bool,
    FC: Fn(IVec3, IVec3) -> bool,
    FY: Fn(i32) -> bool,
{
    let mut mesh = VoxelMeshData::default();
    let max_mask_len = (dims.x * dims.y).max(dims.y * dims.z).max(dims.x * dims.z) as usize;
    let mut mask: Vec<Option<MaskCell>> = vec![None; max_mask_len];

    for face in &FACES {
        let n_idx = axis_index(face.normal);
        let u_idx = axis_index(face.u_axis);
        let v_idx = axis_index(face.v_axis);

        let n_len = dims[n_idx];
        let u_len = dims[u_idx] as usize;
        let v_len = dims[v_idx] as usize;

        for slice in 0..n_len {
            // Fast skip when sweeping horizontal Y-slices that are completely air
            if n_idx == 1 && is_y_layer_air(slice) {
                continue;
            }

            let mut any_visible = false;
            // Fill 2D face mask for this slice
            for v in 0..v_len {
                if v_idx == 1 && is_y_layer_air(v as i32) {
                    let row = v * u_len;
                    mask[row..row + u_len].fill(None);
                    continue;
                }
                for u in 0..u_len {
                    if u_idx == 1 && is_y_layer_air(u as i32) {
                        mask[u + v * u_len] = None;
                        continue;
                    }
                    let mut local = IVec3::ZERO;
                    local[n_idx] = slice;
                    local[u_idx] = u as i32;
                    local[v_idx] = v as i32;
                    let sample_pos = voxel_origin + local;

                    let id = get_voxel(sample_pos);
                    if !materials.is_solid(id) {
                        mask[u + v * u_len] = None;
                        continue;
                    }
                    // Cull faces hidden behind neighboring solids or outside playable world shell
                    if cull_face(sample_pos, face.normal) || is_solid(sample_pos + face.normal) {
                        mask[u + v * u_len] = None;
                        continue;
                    }
                    let shade_bucket = materials.shade_bucket(id, sample_pos);
                    let ao = compute_face_ao(sample_pos, face, &is_solid);
                    mask[u + v * u_len] = Some(MaskCell {
                        id,
                        shade_bucket,
                        ao,
                    });
                    any_visible = true;
                }
            }

            if !any_visible {
                continue;
            }

            // Greedy merge rectangles inside the 2D mask
            for v in 0..v_len {
                let mut u = 0;
                while u < u_len {
                    let Some(cell) = mask[u + v * u_len] else {
                        u += 1;
                        continue;
                    };

                    // Expand width along u
                    let mut w = 1usize;
                    while u + w < u_len && mask[(u + w) + v * u_len] == Some(cell) {
                        w += 1;
                    }

                    // Expand height along v
                    let mut h = 1usize;
                    'grow_h: while v + h < v_len {
                        for k in 0..w {
                            if mask[(u + k) + (v + h) * u_len] != Some(cell) {
                                break 'grow_h;
                            }
                        }
                        h += 1;
                    }

                    // Clear consumed cells
                    for dv in 0..h {
                        for du in 0..w {
                            mask[(u + du) + (v + dv) * u_len] = None;
                        }
                    }

                    // Emit merged quad
                    let mut base = IVec3::ZERO;
                    base[n_idx] = slice + face.normal_offset;
                    base[u_idx] = u as i32;
                    base[v_idx] = v as i32;

                    let du_vec = face.u_axis.as_vec3() * w as f32;
                    let dv_vec = face.v_axis.as_vec3() * h as f32;
                    let p00 = base.as_vec3();

                    let corners = [p00, p00 + du_vec, p00 + du_vec + dv_vec, p00 + dv_vec];
                    let base_color = materials.color_for_bucket(cell.id, cell.shade_bucket);
                    let normal = face.normal.as_vec3().to_array();
                    let base_vert = mesh.positions.len() as u32;

                    for (ci, corner) in corners.iter().enumerate() {
                        let world_p = (*corner - pivot) * voxel_size;
                        mesh.positions.push(world_p.to_array());
                        mesh.normals.push(normal);

                        let shade = face.face_shade * AO_CURVE[cell.ao[ci] as usize];
                        mesh.colors.push([
                            (base_color[0] * shade).clamp(0.0, 1.0),
                            (base_color[1] * shade).clamp(0.0, 1.0),
                            (base_color[2] * shade).clamp(0.0, 1.0),
                            base_color[3],
                        ]);
                    }

                    // Flip quad diagonal when needed to avoid AO interpolation anisotropy
                    if cell.ao[0] + cell.ao[2] > cell.ao[1] + cell.ao[3] {
                        mesh.indices.extend_from_slice(&[
                            base_vert,
                            base_vert + 1,
                            base_vert + 2,
                            base_vert,
                            base_vert + 2,
                            base_vert + 3,
                        ]);
                    } else {
                        mesh.indices.extend_from_slice(&[
                            base_vert + 1,
                            base_vert + 2,
                            base_vert + 3,
                            base_vert + 1,
                            base_vert + 3,
                            base_vert,
                        ]);
                    }

                    u += w;
                }
            }
        }
    }

    mesh.positions.shrink_to_fit();
    mesh.normals.shrink_to_fit();
    mesh.colors.shrink_to_fit();
    mesh.indices.shrink_to_fit();
    mesh
}

#[inline]
fn axis_index(v: IVec3) -> usize {
    if v.x != 0 {
        0
    } else if v.y != 0 {
        1
    } else {
        2
    }
}

/// Builds a chunk-local greedy mesh for `chunk_coord` in `world`.
///
/// The returned vertex coordinates are relative to `world.chunk_world_origin(chunk_coord)`.
pub fn mesh_world_chunk(world: &VoxelWorld, chunk_coord: IVec3) -> VoxelMeshData {
    let Some(chunk) = world.chunk(chunk_coord) else {
        return VoxelMeshData::default();
    };
    let voxel_origin = chunk_coord * CHUNK_SIZE_I32;
    let has_bounds = world.bounds_min != world.bounds_max;
    let bmin = world.bounds_min;
    let bmax = world.bounds_max;

    // Precompute which local Y-slices in this chunk are completely Air
    let mut air_y = [false; CHUNK_SIZE];
    for (ly, slot) in air_y.iter_mut().enumerate() {
        *slot = chunk.uniform_slice(ly).is_some_and(VoxelId::is_air);
    }

    greedy_mesh_volume(
        IVec3::splat(CHUNK_SIZE as i32),
        voxel_origin,
        Vec3::ZERO,
        world.voxel_size,
        &world.materials,
        |pos| {
            let local = pos - voxel_origin;
            chunk.get(local.x as usize, local.y as usize, local.z as usize)
        },
        |pos| world.is_solid(pos),
        |pos, normal| {
            if !has_bounds {
                return false;
            }
            // Never emit unseen underside of the world bedrock or outer perimeter skirt faces
            (normal.y == -1 && pos.y == bmin.y)
                || (normal.x == -1 && pos.x == bmin.x)
                || (normal.x == 1 && pos.x == bmax.x)
                || (normal.z == -1 && pos.z == bmin.z)
                || (normal.z == 1 && pos.z == bmax.z)
        },
        |ly| air_y[ly as usize],
    )
}

/// Builds a greedy mesh for a fine-scale `VoxelModel` (such as a tank hull, turret, or gun).
pub fn mesh_voxel_model(model: &VoxelModel, materials: &MaterialRegistry) -> VoxelMeshData {
    greedy_mesh_volume(
        model.size.as_ivec3(),
        IVec3::ZERO,
        model.pivot,
        model.voxel_size,
        materials,
        |pos| model.get(pos),
        |pos| materials.is_solid(model.get(pos)),
        |_, _| false,
        |_| false,
    )
}
