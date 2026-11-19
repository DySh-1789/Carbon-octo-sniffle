//! Stage 3 AI: compact navigation grid, bounded A* pathfinding, and the tactical bot brain
//! (objective selection, path following, LOS target acquisition, ballistic lead aiming).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use voxel::glam::Vec3;
use voxel::{VoxelId, VoxelWorld, raycast_world};

use crate::ballistics::GRAVITY;
use crate::battle::{
    CaptureZoneState, MAX_TANKS, SimRng, TankId, TankUnit, VISION_RANGE, ray_vs_tank,
};
use crate::tank::{TankInput, TankState, wrap_angle};

/// Navigation cell size in meters (`384 m` map → `96 x 96` cells ≈ 46 KiB total).
pub const NAV_CELL_SIZE: f32 = 4.0;
/// Maximum terrain height change between neighbouring cells a tank may traverse.
const MAX_CELL_STEP_M: f32 = 1.25;
/// Hard cap on A* node expansions so a single query always stays well under a millisecond.
const ASTAR_MAX_POPS: usize = 6000;

const COST_BLOCKED: u8 = 0;
const COST_ROAD: u8 = 8;
const COST_NORMAL: u8 = 10;
const COST_WATER: u8 = 12;
const COST_NEAR_OBSTACLE: u8 = 8;

/// Compact 2D walkability grid built once per map from the voxel terrain.
#[derive(Clone, Debug)]
pub struct NavGrid {
    pub cell_size: f32,
    pub origin_x: f32,
    pub origin_z: f32,
    pub width: usize,
    pub depth: usize,
    /// Traversal cost in tenths (`0` = blocked).
    cost: Vec<u8>,
    /// Mean ground height of each cell in meters.
    height: Vec<f32>,
}

impl NavGrid {
    /// Builds the grid by scanning every voxel column. A cell is blocked if it contains a vertical
    /// step of `>= 2` voxels (walls, houses, boulders, trees, parapets) or hugs the map border.
    pub fn build(world: &VoxelWorld) -> Self {
        let (min_x, max_x, min_z, max_z) = world.playable_bounds_xz();
        let cs = NAV_CELL_SIZE;
        let width = ((max_x - min_x) / cs).floor().max(1.0) as usize;
        let depth = ((max_z - min_z) / cs).floor().max(1.0) as usize;
        let vs = world.voxel_size;
        let per = (cs / vs).round().max(1.0) as usize;
        let n = per * per;
        let step_block = 2.0 * vs - 1e-3;
        let border = 5.0;

        let mut cost = vec![COST_BLOCKED; width * depth];
        let mut height = vec![0.0f32; width * depth];
        let mut hbuf = vec![0.0f32; n];

        for cz in 0..depth {
            for cx in 0..width {
                let x0 = min_x + cx as f32 * cs;
                let z0 = min_z + cz as f32 * cs;
                let center_x = x0 + cs * 0.5;
                let center_z = z0 + cs * 0.5;
                if center_x - min_x < border
                    || max_x - center_x < border
                    || center_z - min_z < border
                    || max_z - center_z < border
                {
                    continue;
                }

                let vx0 = (x0 / vs).floor() as i32;
                let vz0 = (z0 / vs).floor() as i32;
                let mut valid = true;
                let mut water = 0usize;
                let mut road = 0usize;
                let mut sum = 0.0f32;
                'cols: for j in 0..per {
                    for i in 0..per {
                        match world.top_solid_voxel_at(vx0 + i as i32, vz0 + j as i32) {
                            Some((vy, id)) => {
                                let h = (vy + 1) as f32 * vs;
                                hbuf[j * per + i] = h;
                                sum += h;
                                if id == VoxelId::WATER {
                                    water += 1;
                                } else if id == VoxelId::ROAD_ASPHALT || id == VoxelId::COBBLESTONE
                                {
                                    road += 1;
                                }
                            }
                            None => {
                                valid = false;
                                break 'cols;
                            }
                        }
                    }
                }
                if !valid {
                    continue;
                }

                let mut blocked = false;
                'step: for j in 0..per {
                    for i in 0..per {
                        let h = hbuf[j * per + i];
                        if i + 1 < per && (hbuf[j * per + i + 1] - h).abs() >= step_block {
                            blocked = true;
                            break 'step;
                        }
                        if j + 1 < per && (hbuf[(j + 1) * per + i] - h).abs() >= step_block {
                            blocked = true;
                            break 'step;
                        }
                    }
                }
                if blocked {
                    continue;
                }

                let idx = cz * width + cx;
                height[idx] = sum / n as f32;
                cost[idx] = if water * 10 > n * 3 {
                    COST_WATER
                } else if road * 2 > n {
                    COST_ROAD
                } else {
                    COST_NORMAL
                };
            }
        }

        // Penalize cells hugging obstacles so paths prefer open lanes and street centers
        let blocked: Vec<bool> = cost.iter().map(|&c| c == COST_BLOCKED).collect();
        for cz in 0..depth {
            for cx in 0..width {
                let idx = cz * width + cx;
                if blocked[idx] {
                    continue;
                }
                let mut near = false;
                for dz in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = cx as i32 + dx;
                        let nz = cz as i32 + dz;
                        if nx < 0 || nz < 0 || nx >= width as i32 || nz >= depth as i32 {
                            continue;
                        }
                        if blocked[nz as usize * width + nx as usize] {
                            near = true;
                        }
                    }
                }
                if near {
                    cost[idx] = cost[idx].saturating_add(COST_NEAR_OBSTACLE);
                }
            }
        }

        Self {
            cell_size: cs,
            origin_x: min_x,
            origin_z: min_z,
            width,
            depth,
            cost,
            height,
        }
    }

    /// Approximate heap memory used by the grid.
    pub fn memory_bytes(&self) -> usize {
        self.cost.len() + self.height.len() * std::mem::size_of::<f32>()
    }

    pub fn walkable_count(&self) -> usize {
        self.cost.iter().filter(|&&c| c != COST_BLOCKED).count()
    }

    #[inline]
    fn idx(&self, cx: usize, cz: usize) -> usize {
        cz * self.width + cx
    }

    /// Grid cell containing a world position (if inside the grid).
    pub fn cell_of(&self, p: Vec3) -> Option<(usize, usize)> {
        let fx = ((p.x - self.origin_x) / self.cell_size).floor();
        let fz = ((p.z - self.origin_z) / self.cell_size).floor();
        if fx < 0.0 || fz < 0.0 || fx >= self.width as f32 || fz >= self.depth as f32 {
            return None;
        }
        Some((fx as usize, fz as usize))
    }

    #[inline]
    pub fn is_walkable(&self, cx: usize, cz: usize) -> bool {
        cx < self.width && cz < self.depth && self.cost[self.idx(cx, cz)] != COST_BLOCKED
    }

    /// True if a world position lies on a walkable cell.
    pub fn is_walkable_at(&self, p: Vec3) -> bool {
        self.cell_of(p).is_some_and(|(cx, cz)| self.is_walkable(cx, cz))
    }

    /// World-space center of a cell at its mean ground height.
    pub fn cell_center(&self, cx: usize, cz: usize) -> Vec3 {
        Vec3::new(
            self.origin_x + (cx as f32 + 0.5) * self.cell_size,
            self.height[self.idx(cx, cz)],
            self.origin_z + (cz as f32 + 0.5) * self.cell_size,
        )
    }

    #[inline]
    fn can_step(&self, a: usize, b: usize) -> bool {
        self.cost[a] != COST_BLOCKED
            && self.cost[b] != COST_BLOCKED
            && (self.height[a] - self.height[b]).abs() <= MAX_CELL_STEP_M
    }

    /// Nearest walkable cell around `p` (spiral search up to 8 cells away).
    pub fn nearest_walkable(&self, p: Vec3) -> Option<(usize, usize)> {
        let fx = ((p.x - self.origin_x) / self.cell_size).floor() as i32;
        let fz = ((p.z - self.origin_z) / self.cell_size).floor() as i32;
        let mut best: Option<(i32, (usize, usize))> = None;
        for r in 0..=8i32 {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let cx = fx + dx;
                    let cz = fz + dz;
                    if cx < 0 || cz < 0 {
                        continue;
                    }
                    let (ux, uz) = (cx as usize, cz as usize);
                    if self.is_walkable(ux, uz) {
                        let d2 = dx * dx + dz * dz;
                        if best.is_none_or(|b| d2 < b.0) {
                            best = Some((d2, (ux, uz)));
                        }
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|b| b.1)
    }

    /// True if a 4 m wide straight corridor between two points stays on walkable, gently sloped cells.
    pub fn corridor_clear(&self, a: Vec3, b: Vec3) -> bool {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-3 {
            return self.is_walkable_at(a);
        }
        let (ux, uz) = (dx / len, dz / len);
        let (px, pz) = (-uz * 1.8, ux * 1.8);
        let steps = (len / 1.0).ceil() as usize;
        let mut prev_idx: Option<usize> = None;
        for s in 0..=steps {
            let t = (s as f32 / steps as f32) * len;
            let cx = a.x + ux * t;
            let cz = a.z + uz * t;
            for off in [0.0f32, 1.0, -1.0] {
                let p = Vec3::new(cx + px * off, 0.0, cz + pz * off);
                let Some((gx, gz)) = self.cell_of(p) else {
                    return false;
                };
                if !self.is_walkable(gx, gz) {
                    return false;
                }
                if off == 0.0 {
                    let idx = self.idx(gx, gz);
                    if let Some(pi) = prev_idx
                        && pi != idx
                        && !self.can_step(pi, idx)
                    {
                        return false;
                    }
                    prev_idx = Some(idx);
                }
            }
        }
        true
    }

    /// Bounded 8-connected A* from `start` to `goal`. Returns smoothed world waypoints (excluding
    /// the start). If the goal is unreachable within the node budget, returns a path to the
    /// explored cell closest to the goal.
    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Vec<Vec3> {
        let Some((sx, sz)) = self.nearest_walkable(start) else {
            return Vec::new();
        };
        let Some((gx, gz)) = self.nearest_walkable(goal) else {
            return Vec::new();
        };
        let start_idx = self.idx(sx, sz);
        let goal_idx = self.idx(gx, gz);
        let n = self.width * self.depth;

        let heuristic = |idx: usize| -> f32 {
            let x = (idx % self.width) as i32;
            let z = (idx / self.width) as i32;
            let dx = (x - gx as i32).abs() as f32;
            let dz = (z - gz as i32).abs() as f32;
            let (mn, mx) = if dx < dz { (dx, dz) } else { (dz, dx) };
            (mx - mn + mn * std::f32::consts::SQRT_2) * (COST_ROAD as f32 / 10.0)
        };

        let mut g = vec![f32::INFINITY; n];
        let mut came = vec![u32::MAX; n];
        let mut closed = vec![false; n];
        let mut open: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
        g[start_idx] = 0.0;
        open.push(Reverse(((heuristic(start_idx) * 100.0) as u32, start_idx as u32)));

        let mut best_idx = start_idx;
        let mut best_h = heuristic(start_idx);
        let mut pops = 0usize;
        let mut reached = false;

        const NEIGH: [(i32, i32, f32); 8] = [
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 1, std::f32::consts::SQRT_2),
            (1, -1, std::f32::consts::SQRT_2),
            (-1, 1, std::f32::consts::SQRT_2),
            (-1, -1, std::f32::consts::SQRT_2),
        ];

        while let Some(Reverse((_, cur))) = open.pop() {
            let cur = cur as usize;
            if closed[cur] {
                continue;
            }
            closed[cur] = true;
            pops += 1;
            if cur == goal_idx {
                reached = true;
                break;
            }
            let h = heuristic(cur);
            if h < best_h {
                best_h = h;
                best_idx = cur;
            }
            if pops >= ASTAR_MAX_POPS {
                break;
            }
            let cx = (cur % self.width) as i32;
            let cz = (cur / self.width) as i32;
            for &(dx, dz, len) in &NEIGH {
                let nx = cx + dx;
                let nz = cz + dz;
                if nx < 0 || nz < 0 || nx >= self.width as i32 || nz >= self.depth as i32 {
                    continue;
                }
                let nidx = nz as usize * self.width + nx as usize;
                if closed[nidx] || !self.can_step(cur, nidx) {
                    continue;
                }
                if dx != 0 && dz != 0 {
                    // No diagonal corner cutting past obstacles
                    let a = cz as usize * self.width + nx as usize;
                    let b = nz as usize * self.width + cx as usize;
                    if !self.can_step(cur, a) || !self.can_step(cur, b) {
                        continue;
                    }
                }
                let step = len * self.cost[nidx] as f32 / 10.0;
                let ng = g[cur] + step;
                if ng < g[nidx] {
                    g[nidx] = ng;
                    came[nidx] = cur as u32;
                    let f = ng + heuristic(nidx);
                    open.push(Reverse(((f * 100.0) as u32, nidx as u32)));
                }
            }
        }

        let end_idx = if reached { goal_idx } else { best_idx };
        let mut cells = Vec::new();
        let mut cur = end_idx;
        while cur != start_idx && cur != u32::MAX as usize {
            cells.push(cur);
            cur = came[cur] as usize;
        }
        cells.reverse();

        let mut raw: Vec<Vec3> = cells
            .iter()
            .map(|&i| self.cell_center(i % self.width, i / self.width))
            .collect();
        if reached && self.is_walkable_at(goal) {
            if let Some(last) = raw.last_mut() {
                *last = goal;
            } else {
                raw.push(goal);
            }
        }
        self.smooth_path(start, raw)
    }

    /// String-pulls a raw cell path using corridor visibility checks.
    fn smooth_path(&self, start: Vec3, raw: Vec<Vec3>) -> Vec<Vec3> {
        if raw.len() <= 2 {
            return raw;
        }
        let mut out = Vec::with_capacity(raw.len() / 2 + 1);
        let mut anchor = start;
        let mut i = 0usize;
        while i < raw.len() {
            let mut furthest = i;
            let max_look = (i + 12).min(raw.len() - 1);
            let mut j = max_look;
            while j > i {
                if self.corridor_clear(anchor, raw[j]) {
                    furthest = j;
                    break;
                }
                j -= 1;
            }
            out.push(raw[furthest]);
            anchor = raw[furthest];
            i = furthest + 1;
        }
        out
    }
}

/// High-level objective of a bot.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BotObjective {
    /// Advance to, capture, and hold capture zone `index`.
    Zone(usize),
    /// All zones secured: push toward the nearest enemy.
    Hunt,
}

/// Current behaviour mode (useful for debugging / HUD).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BotMode {
    Advance,
    Hold,
    Engage,
    Cover,
    Unstick,
}

/// Read-only world snapshot handed to every bot each tick.
pub struct BotContext<'a> {
    pub tanks: &'a [TankUnit],
    pub zones: &'a [CaptureZoneState],
    pub voxels: &'a VoxelWorld,
    pub nav: &'a NavGrid,
    /// Bit `j` of `visibility[i]` is set when tank `i` has line of sight to tank `j`.
    pub visibility: &'a [u16; MAX_TANKS],
    pub tick: u64,
}

/// Tactical finite-state bot controller producing `TankInput` for one AI tank.
#[derive(Clone, Debug)]
pub struct BotBrain {
    pub objective: BotObjective,
    pub mode: BotMode,
    pub target: Option<TankId>,
    path: Vec<Vec3>,
    path_idx: usize,
    path_goal: Vec3,
    replan_timer: f32,
    park_offset: Vec3,
    aim_jitter: Vec3,
    /// `1.0` = freshly acquired / moving (wide dispersion), `0.0` = fully aimed.
    aim_settle: f32,
    stuck_timer: f32,
    unstick_timer: f32,
    unstick_steer: f32,
    cover_timer: f32,
    last_damage_events: u32,
    think_phase: u64,
}

const THINK_INTERVAL: u64 = 20;
const ENGAGE_HOLD_RANGE: f32 = 85.0;

impl BotBrain {
    pub fn new(unit: &TankUnit, initial_objective: BotObjective) -> Self {
        let k = unit.id.0 as f32;
        let angle = k * 2.399_963;
        let radius = 4.0 + (unit.id.0 % 3) as f32 * 3.0;
        Self {
            objective: initial_objective,
            mode: BotMode::Advance,
            target: None,
            path: Vec::new(),
            path_idx: 0,
            path_goal: Vec3::splat(f32::MAX),
            replan_timer: 0.0,
            park_offset: Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
            aim_jitter: Vec3::ZERO,
            aim_settle: 1.0,
            stuck_timer: 0.0,
            unstick_timer: 0.0,
            unstick_steer: 1.0,
            cover_timer: 0.0,
            last_damage_events: unit.damage.damage_events_received,
            think_phase: (unit.id.0 as u64 * 7) % THINK_INTERVAL,
        }
    }

    /// Current path waypoints (for debugging / tests).
    pub fn path(&self) -> &[Vec3] {
        &self.path
    }

    /// Runs one fixed tick of bot decision making and returns its control input.
    pub fn update(&mut self, me_idx: usize, ctx: &BotContext, dt: f32, rng: &mut SimRng) -> TankInput {
        let me = &ctx.tanks[me_idx];
        let st = &me.state;
        let pos = st.position;
        let think = (ctx.tick + self.think_phase).is_multiple_of(THINK_INTERVAL);

        // React to incoming hits: back off into cover while reloading (if still mobile)
        if me.damage.damage_events_received > self.last_damage_events {
            self.last_damage_events = me.damage.damage_events_received;
            self.aim_settle = self.aim_settle.max(0.5);
            if me.damage.can_move() && (st.reload_remaining > 1.5 || !me.damage.can_fire()) {
                self.cover_timer = 1.6;
            }
        }

        if think {
            self.choose_objective(me, ctx);
            self.choose_target(me_idx, ctx, rng);
        }

        // Movement goal & path planning
        let goal = self.goal_point(me, ctx);
        self.replan_timer -= dt;
        let path_done = self.path_idx >= self.path.len();
        if think
            && (self.path_goal.distance_squared(goal) > 64.0
                || self.replan_timer <= 0.0
                || (path_done && horiz_dist(pos, goal) > 6.0))
        {
            self.path = ctx.nav.find_path(pos, goal);
            self.path_idx = 0;
            self.path_goal = goal;
            self.replan_timer = 6.0;
        }

        // Target visibility & ballistic aiming
        let target = self
            .target
            .and_then(|id| ctx.tanks.get(id.index()))
            .filter(|t| t.alive);
        let target_visible =
            target.is_some_and(|t| ctx.visibility[me_idx] & (1u16 << t.id.index()) != 0);

        // Aim-in: dispersion shrinks while stationary, grows while driving
        let speed_ratio = (st.forward_speed.abs() / st.config.max_forward_speed).clamp(0.0, 1.0);
        self.aim_settle = (self.aim_settle - dt / 2.2).max(speed_ratio * 0.7).clamp(0.0, 1.0);

        let mut fire = false;
        let mut target_dist = f32::MAX;
        let aim_point = if let (Some(t), true) = (target, target_visible) {
            let gun = st.gun_world_position();
            let center = t.aim_center();
            let dist = gun.distance(center);
            target_dist = dist;
            let v0 = st.config.muzzle_velocity.max(1.0);
            let mut tof = dist / v0;
            tof *= 1.0 + 0.5 * st.config.shell_drag * tof;
            let lead = center + t.velocity() * tof;
            let drop = 0.5 * GRAVITY * tof * tof;
            let spread = dist * (0.015 + 0.025 * self.aim_settle);
            let aim = lead + Vec3::Y * drop + self.aim_jitter * spread;

            let muzzle = st.muzzle_world_position();
            let bore = st.gun_forward();
            let desired = (aim - gun).normalize_or_zero();
            let aligned = bore.dot(desired) > 1.5f32.to_radians().cos();
            let muzzle_dist = (muzzle.distance(center) - 2.5).max(0.5);
            let voxel_clear = raycast_world(ctx.voxels, muzzle, bore, muzzle_dist).is_none();
            if st.is_loaded()
                && aligned
                && dist <= VISION_RANGE + 20.0
                && voxel_clear
                && line_clear_of_friendlies(me_idx, t.id.index(), ctx, muzzle, bore, muzzle_dist)
            {
                fire = true;
                self.reroll_jitter(rng);
            }
            aim
        } else {
            self.idle_aim_point(me, ctx, goal)
        };

        // Driving
        let (throttle, steer);
        if self.unstick_timer > 0.0 {
            self.unstick_timer -= dt;
            self.mode = BotMode::Unstick;
            throttle = -0.85;
            steer = self.unstick_steer;
        } else if self.cover_timer > 0.0 {
            self.cover_timer -= dt;
            self.mode = BotMode::Cover;
            throttle = -1.0;
            steer = 0.0;
        } else {
            let dist_goal = horiz_dist(pos, goal);
            let engage_hold = target_visible
                && me.damage.can_fire()
                && target_dist < ENGAGE_HOLD_RANGE
                && st.reload_remaining < 1.2;
            if dist_goal < 3.5 || engage_hold {
                self.mode = if engage_hold {
                    BotMode::Engage
                } else {
                    BotMode::Hold
                };
                throttle = 0.0;
                if target_visible && let Some(lim) = st.config.turret_yaw_limit {
                    let dx = aim_point.x - pos.x;
                    let dz = aim_point.z - pos.z;
                    let desired_yaw = (-dx).atan2(-dz);
                    let yaw_err = wrap_angle(desired_yaw - st.hull_yaw);
                    steer = if yaw_err.abs() > lim * 0.70 {
                        (yaw_err * 3.0).clamp(-1.0, 1.0)
                    } else {
                        0.0
                    };
                } else {
                    steer = 0.0;
                }
            } else {
                self.mode = if target_visible {
                    BotMode::Engage
                } else {
                    BotMode::Advance
                };
                while self.path_idx < self.path.len()
                    && horiz_dist(pos, self.path[self.path_idx]) < 4.5
                {
                    self.path_idx += 1;
                }
                let waypoint = self.path.get(self.path_idx).copied().unwrap_or(goal);
                let (t, s) = drive_toward(st, waypoint, dist_goal);
                throttle = t;
                steer = s;
            }

            // Anti-stuck: stalled against an obstacle while trying to drive (ignore if immobilized by damage)
            if me.damage.can_move() && throttle > 0.4 && st.forward_speed.abs() < 0.6 {
                self.stuck_timer += dt;
                if self.stuck_timer > 1.4 {
                    self.stuck_timer = 0.0;
                    self.unstick_timer = 1.2;
                    self.unstick_steer = if rng.next_f32() < 0.5 { 1.0 } else { -1.0 };
                    self.replan_timer = 0.0;
                }
            } else {
                self.stuck_timer = (self.stuck_timer - dt).max(0.0);
            }
        }

        TankInput {
            throttle,
            steer,
            aim_world_point: aim_point,
            fire: fire && me.damage.can_fire(),
        }
    }

    fn reroll_jitter(&mut self, rng: &mut SimRng) {
        self.aim_jitter = Vec3::new(rng.signed(), rng.signed() * 0.5, rng.signed()) * 0.7;
    }

    fn choose_objective(&mut self, me: &TankUnit, ctx: &BotContext) {
        let team = me.team;
        let current_ok = match self.objective {
            BotObjective::Zone(i) => ctx.zones.get(i).is_some_and(|z| !z.secured_by(team)),
            BotObjective::Hunt => false,
        };
        if current_ok {
            return;
        }
        let pos = me.state.position;
        let best = ctx
            .zones
            .iter()
            .enumerate()
            .filter(|(_, z)| !z.secured_by(team))
            .min_by(|a, b| {
                horiz_dist(pos, a.1.center).total_cmp(&horiz_dist(pos, b.1.center))
            })
            .map(|(i, _)| i);
        self.objective = match best {
            Some(i) => BotObjective::Zone(i),
            None => BotObjective::Hunt,
        };
    }

    fn choose_target(&mut self, me_idx: usize, ctx: &BotContext, rng: &mut SimRng) {
        let me = &ctx.tanks[me_idx];
        let pos = me.state.position;
        let mut best: Option<(f32, TankId)> = None;
        for (j, t) in ctx.tanks.iter().enumerate() {
            if !t.alive || t.team == me.team || ctx.visibility[me_idx] & (1u16 << j) == 0 {
                continue;
            }
            let d = pos.distance(t.state.position);
            if d > VISION_RANGE {
                continue;
            }
            let mut score = d;
            if Some(t.id) == self.target {
                score -= 25.0;
            }
            // Prefer finishing off critically damaged enemies with fewer surviving crew
            let crew_lost = t.damage.total_crew_count().saturating_sub(t.damage.alive_crew_count());
            score -= crew_lost as f32 * 10.0;
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, t.id));
            }
        }
        let new_target = best.map(|b| b.1);
        if new_target != self.target {
            self.target = new_target;
            self.aim_settle = 1.0;
            self.reroll_jitter(rng);
        }
    }

    fn goal_point(&self, me: &TankUnit, ctx: &BotContext) -> Vec3 {
        match self.objective {
            BotObjective::Zone(i) => ctx
                .zones
                .get(i)
                .map(|z| z.center + self.park_offset)
                .unwrap_or(me.state.position),
            BotObjective::Hunt => nearest_enemy(me, ctx.tanks)
                .map(|t| t.state.position)
                .unwrap_or(me.state.position),
        }
    }

    /// Where to point the gun when no enemy is visible: toward the nearest known enemy while
    /// holding a position, otherwise along the direction of travel.
    fn idle_aim_point(&self, me: &TankUnit, ctx: &BotContext, goal: Vec3) -> Vec3 {
        let pos = me.state.position;
        let holding = horiz_dist(pos, goal) < 6.0;
        if holding && let Some(e) = nearest_enemy(me, ctx.tanks) {
            return e.aim_center();
        }
        let toward = self.path.get(self.path_idx).copied().unwrap_or(goal);
        let mut dir = Vec3::new(toward.x - pos.x, 0.0, toward.z - pos.z);
        if dir.length_squared() < 1.0 {
            let yaw = me.state.hull_yaw;
            dir = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        }
        pos + dir.normalize() * 80.0 + Vec3::Y * 2.0
    }
}

fn nearest_enemy<'a>(me: &TankUnit, tanks: &'a [TankUnit]) -> Option<&'a TankUnit> {
    tanks
        .iter()
        .filter(|t| t.alive && t.team != me.team)
        .min_by(|a, b| {
            me.state
                .position
                .distance_squared(a.state.position)
                .total_cmp(&me.state.position.distance_squared(b.state.position))
        })
}

/// True if no friendly tank or wreck sits on the firing line (avoids shooting teammates or
/// wasting shells into burnt-out hulls).
fn line_clear_of_friendlies(
    me_idx: usize,
    target_idx: usize,
    ctx: &BotContext,
    origin: Vec3,
    dir: Vec3,
    dist: f32,
) -> bool {
    let me = &ctx.tanks[me_idx];
    ctx.tanks.iter().enumerate().all(|(j, t)| {
        if j == me_idx || j == target_idx || (t.alive && t.team != me.team) {
            return true;
        }
        ray_vs_tank(&t.state, origin, dir, dist).is_none()
    })
}

#[inline]
fn horiz_dist(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

/// Simple tracked-vehicle steering controller toward a waypoint.
fn drive_toward(st: &TankState, waypoint: Vec3, dist_goal: f32) -> (f32, f32) {
    let dx = waypoint.x - st.position.x;
    let dz = waypoint.z - st.position.z;
    let desired_yaw = (-dx).atan2(-dz);
    let err = wrap_angle(desired_yaw - st.hull_yaw);
    let steer = (err * 2.5).clamp(-1.0, 1.0);
    let mut throttle = if err.abs() > 1.2 {
        0.0
    } else if err.abs() > 0.6 {
        0.45
    } else {
        1.0
    };
    if dist_goal < 14.0 {
        throttle *= (dist_goal / 14.0).max(0.35);
    }
    (throttle, steer)
}
