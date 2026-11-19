//! Stage 3 team battle rules: teams, tank units, capture zones, tickets, victory conditions,
//! tank-vs-tank collision, and projectile-vs-tank hit geometry.
//!
//! NOTE (Stage 3 stub): tanks are knocked out after [`STAGE3_HITS_TO_DESTROY`] direct hits.
//! This temporary counter is fully replaced by the War Thunder-style module/crew damage model
//! (no HP bar) in Stage 4.

use voxel::VoxelWorld;
use voxel::glam::{Quat, Vec3};

use crate::map_gen::MapMetadata;
use crate::tank::TankState;

/// Hard cap on tanks per battle (fits the `u16` visibility bitmasks).
pub const MAX_TANKS: usize = 16;
/// Stage 3 temporary knockout counter (replaced by WT damage model in Stage 4).
pub const STAGE3_HITS_TO_DESTROY: u8 = 3;
/// Starting tickets for each team.
pub const START_TICKETS: f32 = 800.0;
/// Tickets removed from a team each time one of its tanks is destroyed.
pub const KILL_TICKET_PENALTY: f32 = 40.0;
/// Ticket bleed per second for each zone of advantage held by the enemy.
pub const ZONE_BLEED_PER_SEC: f32 = 2.5;
/// Base capture progress per second for a single tank (`1.0` = full capture from neutral).
pub const CAPTURE_RATE_PER_SEC: f32 = 0.05;
/// Battle time limit in seconds (team with more tickets wins afterwards).
pub const BATTLE_TIME_LIMIT: f32 = 900.0;
/// Maximum visual spotting / engagement distance in meters.
pub const VISION_RANGE: f32 = 210.0;

/// Hull hit box (hull-local, meters) matching the 16x8x30 voxel hull at 0.2 m.
const HULL_BOX_MIN: Vec3 = Vec3::new(-1.6, 0.1, -3.0);
const HULL_BOX_MAX: Vec3 = Vec3::new(1.6, 1.6, 3.0);
/// Turret hit box (turret-local, meters) matching the 10x6x14 voxel turret.
const TURRET_BOX_MIN: Vec3 = Vec3::new(-1.0, 0.0, -1.4);
const TURRET_BOX_MAX: Vec3 = Vec3::new(1.0, 1.2, 1.4);
/// Two-circle horizontal footprint used for tank-vs-tank collision.
const FOOTPRINT_OFFSET: f32 = 1.25;
const FOOTPRINT_RADIUS: f32 = 1.6;

/// Stable tank identifier (index into `SimWorld::tanks`; player is always `TankId(0)`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TankId(pub u32);

impl TankId {
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Allies,
    Axis,
}

impl Team {
    #[inline]
    pub fn opposite(self) -> Self {
        match self {
            Team::Allies => Team::Axis,
            Team::Axis => Team::Allies,
        }
    }

    /// Capture progress direction (`+1` Allies, `-1` Axis).
    #[inline]
    pub fn sign(self) -> f32 {
        match self {
            Team::Allies => 1.0,
            Team::Axis => -1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Team::Allies => "ALLIES",
            Team::Axis => "AXIS",
        }
    }
}

/// A tank taking part in the battle (player or AI bot).
#[derive(Clone, Debug)]
pub struct TankUnit {
    pub id: TankId,
    pub name: String,
    pub team: Team,
    pub is_player: bool,
    pub state: TankState,
    pub alive: bool,
    /// Stage 3 stub knockout counter (see module docs).
    pub hits_remaining: u8,
    pub kills: u32,
    pub hits_landed: u32,
    /// True while recently spotted by at least one living enemy.
    pub spotted: bool,
    /// Persistence countdown in seconds after last enemy line of sight.
    pub spot_timer: f32,
}

impl TankUnit {
    pub fn new(id: TankId, name: &str, team: Team, is_player: bool, state: TankState) -> Self {
        Self {
            id,
            name: name.to_string(),
            team,
            is_player,
            state,
            alive: true,
            hits_remaining: STAGE3_HITS_TO_DESTROY,
            kills: 0,
            hits_landed: 0,
            spotted: false,
            spot_timer: 0.0,
        }
    }

    /// Center-of-mass aim point (upper hull / turret ring).
    #[inline]
    pub fn aim_center(&self) -> Vec3 {
        self.state.position + self.state.hull_rotation() * Vec3::new(0.0, 1.35, 0.0)
    }

    /// Commander eye point used for line-of-sight spotting.
    #[inline]
    pub fn eye_point(&self) -> Vec3 {
        self.state.turret_world_position() + Vec3::new(0.0, 1.3, 0.0)
    }

    /// Horizontal world velocity of the hull.
    #[inline]
    pub fn velocity(&self) -> Vec3 {
        let yaw = self.state.hull_yaw;
        Vec3::new(-yaw.sin(), 0.0, -yaw.cos()) * self.state.forward_speed
    }
}

/// Live capture state of a single zone.
#[derive(Clone, Debug)]
pub struct CaptureZoneState {
    pub label: char,
    pub center: Vec3,
    pub radius: f32,
    /// `-1.0` = fully Axis, `0.0` = neutral, `+1.0` = fully Allies.
    pub progress: f32,
    pub owner: Option<Team>,
    pub contested: bool,
    pub allies_inside: u8,
    pub axis_inside: u8,
}

impl CaptureZoneState {
    #[inline]
    pub fn contains_xz(&self, p: Vec3) -> bool {
        let dx = p.x - self.center.x;
        let dz = p.z - self.center.z;
        dx * dx + dz * dz <= self.radius * self.radius && (p.y - self.center.y).abs() < 12.0
    }

    /// True if `team` already fully owns this zone and nobody contests it.
    #[inline]
    pub fn secured_by(&self, team: Team) -> bool {
        self.owner == Some(team) && self.progress * team.sign() >= 0.999 && !self.contested
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BattleOutcome {
    Ongoing,
    Victory(Team),
    Draw,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BattleMode {
    /// Single player tank only (proving ground / unit tests).
    Solo,
    /// Full team battle with AI bots, capture zones, and tickets.
    TeamBattle,
}

#[derive(Clone, Debug)]
pub struct KillEvent {
    pub tick: u64,
    pub killer_id: TankId,
    pub killer_name: String,
    pub killer_team: Team,
    pub victim_id: TankId,
    pub victim_name: String,
    pub victim_team: Team,
}

#[derive(Copy, Clone, Debug)]
pub struct TankHitEvent {
    pub shooter_id: TankId,
    pub target_id: TankId,
    pub target_team: Team,
    pub position: Vec3,
    pub normal: Vec3,
    /// False for friendly-fire / wreck hits that deal no damage.
    pub damaged: bool,
    pub destroyed: bool,
}

/// Team battle scoreboard & objective state.
#[derive(Clone, Debug)]
pub struct BattleState {
    pub mode: BattleMode,
    pub allies_tickets: f32,
    pub axis_tickets: f32,
    pub zones: Vec<CaptureZoneState>,
    pub outcome: BattleOutcome,
    pub battle_time: f32,
    pub kill_log: Vec<KillEvent>,
}

impl BattleState {
    pub fn new(mode: BattleMode, map: &MapMetadata) -> Self {
        let zones = map
            .capture_zones
            .iter()
            .map(|z| CaptureZoneState {
                label: z.label,
                center: z.center,
                radius: z.radius,
                progress: 0.0,
                owner: None,
                contested: false,
                allies_inside: 0,
                axis_inside: 0,
            })
            .collect();
        Self {
            mode,
            allies_tickets: START_TICKETS,
            axis_tickets: START_TICKETS,
            zones,
            outcome: BattleOutcome::Ongoing,
            battle_time: 0.0,
            kill_log: Vec::new(),
        }
    }

    #[inline]
    pub fn is_over(&self) -> bool {
        self.outcome != BattleOutcome::Ongoing
    }

    pub fn tickets(&self, team: Team) -> f32 {
        match team {
            Team::Allies => self.allies_tickets,
            Team::Axis => self.axis_tickets,
        }
    }

    pub fn tickets_mut(&mut self, team: Team) -> &mut f32 {
        match team {
            Team::Allies => &mut self.allies_tickets,
            Team::Axis => &mut self.axis_tickets,
        }
    }

    pub fn owned_zone_count(&self, team: Team) -> usize {
        self.zones.iter().filter(|z| z.owner == Some(team)).count()
    }

    /// Advances capture zones, ticket bleed, and victory checks by `dt` seconds.
    pub fn update(&mut self, dt: f32, tanks: &[TankUnit]) {
        if self.mode != BattleMode::TeamBattle || self.is_over() {
            return;
        }
        self.battle_time += dt;

        // 1. Capture zones
        for zone in &mut self.zones {
            let mut allies = 0u8;
            let mut axis = 0u8;
            for t in tanks.iter().filter(|t| t.alive) {
                if zone.contains_xz(t.state.position) {
                    match t.team {
                        Team::Allies => allies += 1,
                        Team::Axis => axis += 1,
                    }
                }
            }
            zone.allies_inside = allies;
            zone.axis_inside = axis;
            zone.contested = allies > 0 && axis > 0;
            if zone.contested || (allies == 0 && axis == 0) {
                continue;
            }
            let (team, count) = if allies > 0 {
                (Team::Allies, allies)
            } else {
                (Team::Axis, axis)
            };
            let rate = CAPTURE_RATE_PER_SEC * (1.0 + 0.5 * (count.min(3) as f32 - 1.0));
            zone.progress = (zone.progress + team.sign() * rate * dt).clamp(-1.0, 1.0);
            // Enemy-owned zones are neutralized first, then captured
            if let Some(owner) = zone.owner
                && owner != team
                && zone.progress * owner.sign() <= 0.0
            {
                zone.owner = None;
            }
            if zone.progress * team.sign() >= 1.0 {
                zone.owner = Some(team);
            }
        }

        // 2. Ticket bleed from zone advantage
        let allies_zones = self.owned_zone_count(Team::Allies) as f32;
        let axis_zones = self.owned_zone_count(Team::Axis) as f32;
        let diff = allies_zones - axis_zones;
        if diff > 0.0 {
            self.axis_tickets -= ZONE_BLEED_PER_SEC * diff * dt;
        } else if diff < 0.0 {
            self.allies_tickets -= ZONE_BLEED_PER_SEC * (-diff) * dt;
        }
        self.allies_tickets = self.allies_tickets.max(0.0);
        self.axis_tickets = self.axis_tickets.max(0.0);

        // 3. Victory conditions: elimination, ticket depletion, time limit
        let allies_alive = tanks.iter().any(|t| t.alive && t.team == Team::Allies);
        let axis_alive = tanks.iter().any(|t| t.alive && t.team == Team::Axis);
        self.outcome = match (allies_alive, axis_alive) {
            (false, false) => BattleOutcome::Draw,
            (true, false) => BattleOutcome::Victory(Team::Allies),
            (false, true) => BattleOutcome::Victory(Team::Axis),
            (true, true) => {
                if self.allies_tickets <= 0.0 && self.axis_tickets <= 0.0 {
                    BattleOutcome::Draw
                } else if self.axis_tickets <= 0.0 {
                    BattleOutcome::Victory(Team::Allies)
                } else if self.allies_tickets <= 0.0 {
                    BattleOutcome::Victory(Team::Axis)
                } else if self.battle_time >= BATTLE_TIME_LIMIT {
                    if (self.allies_tickets - self.axis_tickets).abs() < 1.0 {
                        BattleOutcome::Draw
                    } else if self.allies_tickets > self.axis_tickets {
                        BattleOutcome::Victory(Team::Allies)
                    } else {
                        BattleOutcome::Victory(Team::Axis)
                    }
                } else {
                    BattleOutcome::Ongoing
                }
            }
        };
    }
}

/// Slab ray-vs-AABB test in a local frame. Returns `(t, local_normal)` of the entry point.
fn ray_vs_aabb(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3, max_t: f32) -> Option<(f32, Vec3)> {
    let mut t_min = 0.0f32;
    let mut t_max = max_t;
    let mut normal = Vec3::ZERO;
    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        if d.abs() < 1e-8 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
            continue;
        }
        let inv = 1.0 / d;
        let mut t0 = (min[axis] - o) * inv;
        let mut t1 = (max[axis] - o) * inv;
        let mut n = Vec3::ZERO;
        n[axis] = -d.signum();
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > t_min {
            t_min = t0;
            normal = n;
        }
        t_max = t_max.min(t1);
        if t_min > t_max {
            return None;
        }
    }
    if normal == Vec3::ZERO {
        // Ray starts inside the box — report an immediate hit facing back along the ray
        normal = -dir;
    }
    Some((t_min, normal))
}

fn ray_vs_oriented_box(
    origin: Vec3,
    dir: Vec3,
    max_t: f32,
    frame_pos: Vec3,
    frame_rot: Quat,
    min: Vec3,
    max: Vec3,
) -> Option<(f32, Vec3)> {
    let inv = frame_rot.inverse();
    let lo = inv * (origin - frame_pos);
    let ld = inv * dir;
    ray_vs_aabb(lo, ld, min, max, max_t).map(|(t, n)| (t, frame_rot * n))
}

/// Ray test against a tank's hull + turret hit boxes. `dir` must be normalized.
/// Returns `(distance, world_normal)` of the closest entry point within `max_dist`.
pub fn ray_vs_tank(tank: &TankState, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
    // Broad phase: bounding sphere around the tank
    let center = tank.position + Vec3::new(0.0, 1.3, 0.0);
    let to_c = center - origin;
    let along = to_c.dot(dir).clamp(0.0, max_dist);
    let closest = origin + dir * along;
    if closest.distance_squared(center) > 4.2 * 4.2 {
        return None;
    }

    let hull = ray_vs_oriented_box(
        origin,
        dir,
        max_dist,
        tank.position,
        tank.hull_rotation(),
        HULL_BOX_MIN,
        HULL_BOX_MAX,
    );
    let turret = ray_vs_oriented_box(
        origin,
        dir,
        max_dist,
        tank.turret_world_position(),
        tank.turret_world_rotation(),
        TURRET_BOX_MIN,
        TURRET_BOX_MAX,
    );
    match (hull, turret) {
        (Some(h), Some(t)) => Some(if h.0 <= t.0 { h } else { t }),
        (h, t) => h.or(t),
    }
}

/// Finds the closest tank hit along a ray, skipping `ignore` (usually the shooter).
pub fn raycast_tanks(
    tanks: &[TankUnit],
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    ignore: Option<TankId>,
) -> Option<(usize, f32, Vec3)> {
    let mut best: Option<(usize, f32, Vec3)> = None;
    for (i, t) in tanks.iter().enumerate() {
        if Some(t.id) == ignore {
            continue;
        }
        let limit = best.map_or(max_dist, |b| b.1);
        if let Some((d, n)) = ray_vs_tank(&t.state, origin, dir, limit)
            && best.is_none_or(|b| d < b.1)
        {
            best = Some((i, d, n));
        }
    }
    best
}

#[inline]
fn footprint_centers(t: &TankState) -> [(f32, f32); 2] {
    let fx = -t.hull_yaw.sin();
    let fz = -t.hull_yaw.cos();
    [
        (t.position.x + fx * FOOTPRINT_OFFSET, t.position.z + fz * FOOTPRINT_OFFSET),
        (t.position.x - fx * FOOTPRINT_OFFSET, t.position.z - fz * FOOTPRINT_OFFSET),
    ]
}

/// Separates overlapping tanks (living tanks and wrecks) using a two-circle footprint per hull.
/// Wrecks are immovable physical cover. Push-outs never shove a tank into voxel obstacles.
pub fn resolve_tank_collisions(tanks: &mut [TankUnit], world: &VoxelWorld) {
    let n = tanks.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let (left, right) = tanks.split_at_mut(j);
            let a = &mut left[i];
            let b = &mut right[0];
            if !a.alive && !b.alive {
                continue;
            }
            let dx = a.state.position.x - b.state.position.x;
            let dz = a.state.position.z - b.state.position.z;
            if dx * dx + dz * dz > 7.5 * 7.5 || (a.state.position.y - b.state.position.y).abs() > 3.5
            {
                continue;
            }

            let ca = footprint_centers(&a.state);
            let cb = footprint_centers(&b.state);
            let mut best_pen = 0.0f32;
            let mut best_n = (0.0f32, 0.0f32);
            for pa in &ca {
                for pb in &cb {
                    let ddx = pa.0 - pb.0;
                    let ddz = pa.1 - pb.1;
                    let dist = (ddx * ddx + ddz * ddz).sqrt();
                    let pen = 2.0 * FOOTPRINT_RADIUS - dist;
                    if pen > best_pen {
                        best_pen = pen;
                        best_n = if dist > 1e-4 {
                            (ddx / dist, ddz / dist)
                        } else {
                            // Perfectly stacked: separate along hull right axis deterministically
                            (a.state.hull_yaw.cos(), -a.state.hull_yaw.sin())
                        };
                    }
                }
            }
            if best_pen <= 0.0 {
                continue;
            }

            let (wa, wb) = match (a.alive, b.alive) {
                (true, true) => (0.5, 0.5),
                (true, false) => (1.0, 0.0),
                (false, true) => (0.0, 1.0),
                (false, false) => (0.0, 0.0),
            };
            let push = best_pen + 0.01;
            for (unit, w, sign) in [(&mut *a, wa, 1.0f32), (&mut *b, wb, -1.0f32)] {
                if w <= 0.0 {
                    continue;
                }
                let nx = best_n.0 * sign;
                let nz = best_n.1 * sign;
                let new_x = unit.state.position.x + nx * push * w;
                let new_z = unit.state.position.z + nz * push * w;
                let yaw = unit.state.hull_yaw;
                if unit.state.count_hull_collisions(world, new_x, new_z, yaw) == 0 {
                    unit.state.position.x = new_x;
                    unit.state.position.z = new_z;
                }
                // Damp velocity component driving into the other tank
                let fwd_x = -yaw.sin() * unit.state.forward_speed;
                let fwd_z = -yaw.cos() * unit.state.forward_speed;
                if fwd_x * nx + fwd_z * nz < 0.0 {
                    unit.state.forward_speed *= 0.55;
                }
            }
        }
    }
}

/// Deterministic xorshift64* random generator for reproducible simulations.
#[derive(Clone, Debug)]
pub struct SimRng(u64);

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in `[0, 1)`.
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform float in `[-1, 1)`.
    #[inline]
    pub fn signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}
