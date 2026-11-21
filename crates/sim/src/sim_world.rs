use std::f32::consts::PI;

use voxel::glam::Vec3;
use voxel::{VoxelWorld, raycast_world};

use crate::ai::{BotBrain, BotContext, BotObjective, NavGrid};
use crate::ballistics::{ImpactEvent, Projectile};
use crate::battle::{
    BattleMode, BattleState, KILL_TICKET_PENALTY, KillEvent, MAX_TANKS, SimRng, TankId, TankUnit,
    Team, VISION_RANGE, raycast_tanks, raycast_tanks_armor, resolve_tank_collisions,
};
use crate::catalog::VehicleCatalog;
use crate::damage::{ArmorHit, HitOutcome, HitReport, KillReason};
use crate::map_gen::{MapMetadata, default_metadata_for_world};
use crate::tank::{FiredShot, TankConfig, TankInput, TankState};

/// Spotting / line-of-sight refresh interval in fixed ticks (6 Hz).
const VISION_INTERVAL: u64 = 10;

const ALLIES_CALLSIGNS: [&str; 8] = [
    "Player", "Hawk", "Viper", "Bishop", "Ranger", "Mako", "Falcon", "Cobra",
];
const AXIS_CALLSIGNS: [&str; 8] = [
    "Adler", "Wolf", "Falke", "Sturm", "Blitz", "Greif", "Kobra", "Eber",
];

/// Complete headless simulation state.
pub struct SimWorld {
    pub voxels: VoxelWorld,
    pub map: MapMetadata,
    /// All tanks in the battle; `tanks[0]` is always the player.
    pub tanks: Vec<TankUnit>,
    /// AI controllers parallel to `tanks` (`None` for the player).
    pub brains: Vec<Option<BotBrain>>,
    pub battle: BattleState,
    /// Navigation grid (team battles only).
    pub nav: Option<NavGrid>,
    /// Bit `j` of `visibility[i]` is set when tank `i` has line of sight to tank `j`.
    pub visibility: [u16; MAX_TANKS],
    pub projectiles: Vec<Projectile>,
    pub recent_impacts: Vec<ImpactEvent>,
    pub recent_shots: Vec<FiredShot>,
    pub recent_hit_reports: Vec<HitReport>,
    pub recent_kills: Vec<KillEvent>,
    pub tick: u64,
    base_config: TankConfig,
    /// Optional per-slot vehicle configurations for Stage 5 multi-vehicle tier battles.
    allies_lineup: Vec<TankConfig>,
    axis_lineup: Vec<TankConfig>,
    team_size: usize,
    next_projectile_id: u64,
    rng: SimRng,
}

impl SimWorld {
    /// Creates a solo simulation on an arbitrary `VoxelWorld` (spawning at origin, used by unit tests).
    pub fn new(voxels: VoxelWorld, tank_config: TankConfig) -> Self {
        let map = default_metadata_for_world(&voxels, "proving_ground", "Proving Ground");
        Self::with_map(voxels, map, tank_config)
    }

    /// Creates a solo simulation (player tank only) from a generated `(VoxelWorld, MapMetadata)`
    /// pair, spawning the player at the primary allied spawn point.
    pub fn with_map(voxels: VoxelWorld, map: MapMetadata, tank_config: TankConfig) -> Self {
        Self::build(voxels, map, tank_config, BattleMode::Solo, 1)
    }

    /// Creates a full team battle (`per_team` vs `per_team`, player + AI allies vs AI axis).
    pub fn new_team_battle(
        voxels: VoxelWorld,
        map: MapMetadata,
        tank_config: TankConfig,
        per_team: usize,
    ) -> Self {
        let per_team = per_team.clamp(1, MAX_TANKS / 2);
        Self::build(voxels, map, tank_config, BattleMode::TeamBattle, per_team)
    }

    /// Creates a Stage 5 tier-matched `per_team` vs `per_team` battle with the player driving
    /// `player_vehicle_id` and bots driving balanced vehicles of the exact same tier across all
    /// three nations (USA, Germany, USSR).
    pub fn new_tier_battle(
        voxels: VoxelWorld,
        map: MapMetadata,
        catalog: &VehicleCatalog,
        player_vehicle_id: &str,
        per_team: usize,
    ) -> Self {
        let player_spec = catalog
            .by_id(player_vehicle_id)
            .unwrap_or_else(|| catalog.default_vehicle());
        let player_cfg = player_spec.to_tank_config(&player_spec.build_models());
        let per_team = per_team.clamp(1, MAX_TANKS / 2);
        let mut sim = Self::build(voxels, map, player_cfg, BattleMode::TeamBattle, per_team);
        sim.configure_tier_battle(catalog, &player_spec.id, per_team);
        sim
    }

    /// Reconfigures this simulation for `map_id`, regenerating the `VoxelWorld` and `NavGrid`
    /// only when switching to a different map, then sets up a tier-matched `per_team` vs `per_team` battle.
    pub fn configure_battle_on_map(
        &mut self,
        map_id: crate::maps::MapId,
        catalog: &VehicleCatalog,
        player_vehicle_id: &str,
        per_team: usize,
    ) {
        if self.map.id != map_id.id() {
            let (voxels, map) = crate::maps::generate_map(map_id);
            self.nav = Some(NavGrid::build(&voxels));
            self.voxels = voxels;
            self.map = map;
        }
        self.configure_tier_battle(catalog, player_vehicle_id, per_team);
    }

    /// Reconfigures this simulation in place for a new player vehicle and tier-matched team battle,
    /// reusing the existing `VoxelWorld`, `MapMetadata`, and `NavGrid` with zero world regeneration.
    pub fn configure_tier_battle(
        &mut self,
        catalog: &VehicleCatalog,
        player_vehicle_id: &str,
        per_team: usize,
    ) {
        let per_team = per_team.clamp(1, MAX_TANKS / 2);
        let player_spec = catalog
            .by_id(player_vehicle_id)
            .unwrap_or_else(|| catalog.default_vehicle());
        let tier_specs = catalog.by_tier(player_spec.tier);

        // Build each distinct tier vehicle's TankConfig once (3 vehicles per tier = tiny CPU/RAM)
        let mut tier_configs: Vec<TankConfig> = tier_specs
            .iter()
            .map(|spec| spec.to_tank_config(&spec.build_models()))
            .collect();
        if tier_configs.is_empty() {
            tier_configs.push(player_spec.to_tank_config(&player_spec.build_models()));
        }

        let player_pos = tier_configs
            .iter()
            .position(|c| c.vehicle_id == player_spec.id)
            .unwrap_or(0);
        let player_cfg = tier_configs[player_pos].clone();

        // Rotate enemy lineup so non-player nations appear first while still showcasing all 3 nations
        let n_tier = tier_configs.len();
        let mut allies = Vec::with_capacity(per_team);
        let mut axis = Vec::with_capacity(per_team);
        for slot in 0..per_team {
            if slot == 0 {
                allies.push(player_cfg.clone());
            } else {
                let idx = (player_pos + slot) % n_tier;
                allies.push(tier_configs[idx].clone());
            }
            let enemy_idx = (player_pos + 1 + slot) % n_tier;
            axis.push(tier_configs[enemy_idx].clone());
        }

        self.base_config = player_cfg;
        self.allies_lineup = allies;
        self.axis_lineup = axis;
        self.team_size = per_team;
        self.battle.mode = BattleMode::TeamBattle;
        if self.nav.is_none() {
            self.nav = Some(NavGrid::build(&self.voxels));
        }
        self.restart_battle();
    }

    fn build(
        voxels: VoxelWorld,
        map: MapMetadata,
        tank_config: TankConfig,
        mode: BattleMode,
        team_size: usize,
    ) -> Self {
        let nav = (mode == BattleMode::TeamBattle).then(|| NavGrid::build(&voxels));
        let battle = BattleState::new(mode, &map);
        let mut sim = Self {
            voxels,
            map,
            tanks: Vec::new(),
            brains: Vec::new(),
            battle,
            nav,
            visibility: [0; MAX_TANKS],
            projectiles: Vec::new(),
            recent_impacts: Vec::new(),
            recent_shots: Vec::new(),
            recent_hit_reports: Vec::new(),
            recent_kills: Vec::new(),
            tick: 0,
            base_config: tank_config,
            allies_lineup: Vec::new(),
            axis_lineup: Vec::new(),
            team_size,
            next_projectile_id: 1,
            rng: SimRng::new(0x5EED_CA4B_0A57_u64),
        };
        sim.spawn_tanks();
        sim
    }

    /// Instantly restarts the battle in place, reusing the voxel world, map metadata, and nav grid
    /// (no world regeneration and no extra memory).
    pub fn restart_battle(&mut self) {
        self.battle = BattleState::new(self.battle.mode, &self.map);
        self.projectiles.clear();
        self.recent_impacts.clear();
        self.recent_shots.clear();
        self.recent_hit_reports.clear();
        self.recent_kills.clear();
        self.visibility = [0; MAX_TANKS];
        self.tick = 0;
        self.spawn_tanks();
    }

    fn spawn_point(&self, team: Team, slot: usize) -> Vec3 {
        let spawns = match team {
            Team::Allies => &self.map.spawn_allies,
            Team::Axis => &self.map.spawn_axis,
        };
        if spawns.is_empty() {
            let z = if team == Team::Allies { 0.0 } else { -80.0 };
            let x = slot as f32 * 8.0;
            let y = self.voxels.surface_height_at(x, z).unwrap_or(0.0);
            return Vec3::new(x, y, z);
        }
        let base = spawns[slot % spawns.len()];
        let row = (slot / spawns.len()) as f32;
        let x = base.x + row * 9.0;
        let y = self.voxels.surface_height_at(x, base.z).unwrap_or(base.y);
        Vec3::new(x, y, base.z)
    }

    fn config_for_slot(&self, team: Team, slot: usize) -> TankConfig {
        let lineup = match team {
            Team::Allies => &self.allies_lineup,
            Team::Axis => &self.axis_lineup,
        };
        if let Some(cfg) = lineup.get(slot) {
            cfg.clone()
        } else {
            self.base_config.clone()
        }
    }

    fn spawn_tanks(&mut self) {
        self.tanks.clear();
        self.brains.clear();
        let zone_count = self.battle.zones.len();
        let teams: &[Team] = if self.battle.mode == BattleMode::TeamBattle {
            &[Team::Allies, Team::Axis]
        } else {
            &[Team::Allies]
        };

        for &team in teams {
            for slot in 0..self.team_size {
                let id = TankId(self.tanks.len() as u32);
                let is_player = team == Team::Allies && slot == 0;
                let yaw = if team == Team::Allies { 0.0 } else { PI };
                let pos = self.spawn_point(team, slot);
                let cfg = self.config_for_slot(team, slot);
                let vehicle_label = cfg.name.clone();
                let mut state = TankState::new(cfg, pos, yaw);
                state.settle_on_terrain(&self.voxels);
                let callsign = match team {
                    Team::Allies => ALLIES_CALLSIGNS[slot % ALLIES_CALLSIGNS.len()],
                    Team::Axis => AXIS_CALLSIGNS[slot % AXIS_CALLSIGNS.len()],
                };
                let display_name = if self.allies_lineup.is_empty() {
                    callsign.to_string()
                } else if is_player {
                    format!("Player ({vehicle_label})")
                } else {
                    format!("{callsign} ({vehicle_label})")
                };
                let unit = TankUnit::new(id, &display_name, team, is_player, state);
                let brain = (!is_player).then(|| {
                    let objective = if zone_count >= 3 {
                        let lane = if pos.x < -10.0 {
                            0
                        } else if pos.x > 10.0 {
                            zone_count - 1
                        } else {
                            zone_count / 2
                        };
                        BotObjective::Zone(lane)
                    } else if zone_count > 0 {
                        BotObjective::Zone(slot % zone_count)
                    } else {
                        BotObjective::Hunt
                    };
                    BotBrain::new(&unit, objective)
                });
                self.tanks.push(unit);
                self.brains.push(brain);
            }
        }
    }

    /// The player's tank state.
    #[inline]
    pub fn player_tank(&self) -> &TankState {
        &self.tanks[0].state
    }

    /// Mutable access to the player's tank state.
    #[inline]
    pub fn player_tank_mut(&mut self) -> &mut TankState {
        &mut self.tanks[0].state
    }

    /// The player's battle unit.
    #[inline]
    pub fn player_unit(&self) -> &TankUnit {
        &self.tanks[0]
    }

    pub fn alive_count(&self, team: Team) -> usize {
        self.tanks.iter().filter(|t| t.alive && t.team == team).count()
    }

    pub fn total_count(&self, team: Team) -> usize {
        self.tanks.iter().filter(|t| t.team == team).count()
    }

    /// Raycast against both voxel terrain and tank armor hitboxes; returns the closest hit point.
    pub fn raycast_scene(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
        ignore: Option<TankId>,
    ) -> Option<Vec3> {
        let dir = dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let world_hit = raycast_world(&self.voxels, origin, dir, max_dist);
        let limit = world_hit.map_or(max_dist, |h| h.distance);
        if let Some((_, d, _)) = raycast_tanks(&self.tanks, origin, dir, limit, ignore) {
            return Some(origin + dir * d);
        }
        world_hit.map(|h| h.point)
    }

    /// Advances the deterministic simulation by `dt` seconds.
    pub fn step_fixed(&mut self, dt: f32, player_input: TankInput) {
        self.tick += 1;
        self.recent_impacts.clear();
        self.recent_shots.clear();
        self.recent_hit_reports.clear();
        self.recent_kills.clear();

        let team_battle = self.battle.mode == BattleMode::TeamBattle;
        let over = self.battle.is_over();
        let n = self.tanks.len();

        // 1. Spotting decay & periodic line-of-sight refresh
        for t in &mut self.tanks {
            if !t.alive {
                t.spot_timer = 0.0;
                t.spotted = false;
            } else {
                t.spot_timer = (t.spot_timer - dt).max(0.0);
                t.spotted = t.spot_timer > 0.0;
            }
        }
        if team_battle && !over && self.tick.is_multiple_of(VISION_INTERVAL) {
            self.update_visibility();
        }

        // 2. Ongoing internal crew replacement (6s) and fire burn-out ticks
        let mut ongoing_kills: Vec<(usize, TankId, Team, KillReason)> = Vec::new();
        if !over {
            for i in 0..n {
                if !self.tanks[i].alive {
                    continue;
                }
                if let Some(reason) = self.tanks[i].damage.tick_ongoing(dt, &mut self.rng) {
                    let (killer_id, killer_team) = self.tanks[i]
                        .damage
                        .fire_instigator
                        .unwrap_or((self.tanks[i].id, self.tanks[i].team.opposite()));
                    ongoing_kills.push((i, killer_id, killer_team, reason));
                }
            }
        }
        for (victim_idx, killer_id, killer_team, reason) in ongoing_kills {
            self.finalize_tank_destruction(victim_idx, killer_id, killer_team, reason);
        }

        // 3. Gather control inputs (player + AI bots)
        let mut inputs = [TankInput::default(); MAX_TANKS];
        for (i, input) in inputs.iter_mut().enumerate().take(n) {
            let st = &self.tanks[i].state;
            *input = TankInput {
                aim_world_point: st.muzzle_world_position() + st.gun_forward() * 100.0,
                ..Default::default()
            };
        }
        if !over {
            if self.tanks[0].alive {
                inputs[0] = player_input;
            }
            if let Some(nav) = self.nav.as_ref() {
                let ctx = BotContext {
                    tanks: &self.tanks,
                    zones: &self.battle.zones,
                    voxels: &self.voxels,
                    nav,
                    visibility: &self.visibility,
                    tick: self.tick,
                };
                for (i, brain) in self.brains.iter_mut().enumerate() {
                    if let Some(brain) = brain
                        && ctx.tanks[i].alive
                    {
                        inputs[i] = brain.update(i, &ctx, dt, &mut self.rng);
                    }
                }
            }
        }

        // 4. Step every living tank with active module/crew damage modifiers & spawn AP shells
        for (i, input) in inputs.iter().enumerate().take(n) {
            if !self.tanks[i].alive {
                continue;
            }
            let mods = self.tanks[i].damage.mobility_modifiers();
            if let Some(shot) =
                self.tanks[i]
                    .state
                    .step_with_modifiers(dt, *input, &self.voxels, mods, &mut self.rng)
            {
                let id = self.next_projectile_id;
                self.next_projectile_id += 1;
                self.projectiles.push(Projectile {
                    id,
                    owner: self.tanks[i].id,
                    owner_team: self.tanks[i].team,
                    shell: shot.shell.clone(),
                    bounced: false,
                    position: shot.muzzle_position,
                    prev_position: shot.muzzle_position,
                    velocity: shot.velocity,
                    drag_coeff: shot.drag_coeff,
                    age: 0.0,
                    max_lifetime: 8.0,
                });
                self.recent_shots.push(shot);
            }
        }

        // 5. Tank-vs-tank collisions (wrecks act as immovable cover)
        if n > 1 {
            resolve_tank_collisions(&mut self.tanks, &self.voxels);
        }

        // 6. Ballistics with swept terrain + full Stage 4 armor/spall/ricochet resolution
        self.step_projectiles(dt);

        // 7. Capture zones, tickets, victory
        self.battle.update(dt, &self.tanks);
    }

    fn update_visibility(&mut self) {
        self.visibility = [0; MAX_TANKS];
        let n = self.tanks.len().min(MAX_TANKS);
        for i in 0..n {
            for j in (i + 1)..n {
                let (a, b) = (&self.tanks[i], &self.tanks[j]);
                if !a.alive || !b.alive || a.team == b.team {
                    continue;
                }
                let ea = a.eye_point();
                let eb = b.eye_point();
                let delta = eb - ea;
                let dist = delta.length();
                if dist > VISION_RANGE || dist < 1e-3 {
                    continue;
                }
                if raycast_world(&self.voxels, ea, delta, (dist - 0.5).max(0.0)).is_none() {
                    self.visibility[i] |= 1 << j;
                    self.visibility[j] |= 1 << i;
                }
            }
        }
        for i in 0..n {
            if (0..n).any(|j| self.visibility[j] & (1 << i) != 0) {
                self.tanks[i].spot_timer = 2.0;
                self.tanks[i].spotted = true;
            }
        }
    }

    fn step_projectiles(&mut self, dt: f32) {
        let mut surviving = Vec::with_capacity(self.projectiles.len());
        let (min_x, max_x, min_z, max_z) = self.voxels.playable_bounds_xz();
        let margin = 120.0;
        let mut pending_hits: Vec<(Projectile, usize, ArmorHit, Vec3)> = Vec::new();
        let mut pending_world_blasts: Vec<(Projectile, Vec3, Vec3, voxel::VoxelId)> = Vec::new();

        for mut proj in self.projectiles.drain(..) {
            let step_vec = proj.integrate(dt);
            if proj.age > proj.max_lifetime {
                continue;
            }
            let step_dist = step_vec.length();
            if step_dist > 1e-5 {
                let dir = step_vec / step_dist;
                let world_hit = raycast_world(&self.voxels, proj.prev_position, dir, step_dist);
                let limit = world_hit.map_or(step_dist, |h| h.distance);
                if let Some((ti, armor_hit)) = raycast_tanks_armor(
                    &self.tanks,
                    proj.prev_position,
                    dir,
                    limit,
                    Some(proj.owner),
                ) {
                    pending_hits.push((proj, ti, armor_hit, dir));
                    continue;
                }
                if let Some(hit) = world_hit {
                    pending_world_blasts.push((proj, hit.point, hit.normal.as_vec3(), hit.material));
                    continue;
                }
            }

            proj.position = proj.prev_position + step_vec;

            let out_of_bounds = proj.position.y < -40.0
                || proj.position.x < min_x - margin
                || proj.position.x > max_x + margin
                || proj.position.z < min_z - margin
                || proj.position.z > max_z + margin;
            if !out_of_bounds {
                surviving.push(proj);
            }
        }
        self.projectiles = surviving;

        for (proj, pos, normal, material) in pending_world_blasts {
            let destr = self.voxels.carve_explosion(
                pos,
                proj.shell.blast_radius_m,
                proj.shell.blast_power,
            );
            let is_he = proj.shell.is_he();
            let blast_radius_m = proj.shell.blast_radius_m;
            self.recent_impacts.push(ImpactEvent {
                projectile_id: proj.id,
                position: pos,
                normal,
                material,
                impact_speed: proj.velocity.length(),
                blast_radius_m,
                is_he,
                destroyed_voxels: destr.destroyed_voxels,
                debris: destr.debris,
            });
            if is_he {
                self.apply_he_splash(&proj, pos);
            }
        }

        for (proj, ti, armor_hit, dir) in pending_hits {
            self.apply_armor_hit(&proj, ti, armor_hit, dir);
        }
    }

    fn apply_he_splash(&mut self, proj: &Projectile, blast_center: Vec3) {
        let mut splash_kills: Vec<(usize, KillReason)> = Vec::new();
        for i in 0..self.tanks.len() {
            if self.tanks[i].id == proj.owner || !self.tanks[i].alive {
                continue;
            }
            let target_id = self.tanks[i].id;
            let target_team = self.tanks[i].team;
            let target_name = self.tanks[i].name.clone();
            let target_state = self.tanks[i].state.clone();
            if let Some(report) = crate::damage::resolve_he_splash(
                proj.owner,
                proj.owner_team,
                target_id,
                &target_name,
                target_team,
                true,
                &target_state,
                &mut self.tanks[i].damage,
                &proj.shell,
                blast_center,
                &mut self.rng,
            ) {
                if let Some(reason) = report.kill_reason {
                    splash_kills.push((i, reason));
                }
                self.recent_hit_reports.push(report);
            }
        }
        for (idx, reason) in splash_kills {
            self.finalize_tank_destruction(idx, proj.owner, proj.owner_team, reason);
        }
    }

    fn apply_armor_hit(&mut self, proj: &Projectile, target_idx: usize, hit: ArmorHit, dir: Vec3) {
        let impact_speed = proj.velocity.length();
        let target_alive = self.tanks[target_idx].alive;
        let target_id = self.tanks[target_idx].id;
        let target_team = self.tanks[target_idx].team;
        let target_name = self.tanks[target_idx].name.clone();
        let target_state = self.tanks[target_idx].state.clone();

        let report = crate::damage::resolve_shell_impact(
            proj.owner,
            proj.owner_team,
            target_id,
            &target_name,
            target_team,
            target_alive,
            &target_state,
            &mut self.tanks[target_idx].damage,
            &proj.shell,
            hit,
            dir,
            impact_speed,
            &mut self.rng,
        );

        // Spawn visible deflected ricochet tracer so bounces deflect off angled armor in 3D
        if report.outcome == HitOutcome::Ricochet
            && !proj.bounced
            && let Some(rdir) = report.ricochet_dir
        {
            let id = self.next_projectile_id;
            self.next_projectile_id += 1;
            let start = report.position + rdir * 0.15;
            self.projectiles.push(Projectile {
                id,
                owner: target_id,
                owner_team: proj.owner_team,
                shell: proj.shell.clone(),
                bounced: true,
                position: start,
                prev_position: start,
                velocity: rdir * (impact_speed * 0.45),
                drag_coeff: proj.drag_coeff * 2.0,
                age: 0.0,
                max_lifetime: 2.2,
            });
        }

        if !report.inert_hit
            && matches!(
                report.outcome,
                HitOutcome::Penetration | HitOutcome::ExternalModuleHit
            )
            && let Some(shooter) = self.tanks.get_mut(proj.owner.index())
        {
            shooter.hits_landed += 1;
        }

        let kill_reason = report.kill_reason;
        self.recent_hit_reports.push(report);

        if let Some(reason) = kill_reason {
            self.finalize_tank_destruction(target_idx, proj.owner, proj.owner_team, reason);
        }
    }

    fn finalize_tank_destruction(
        &mut self,
        victim_idx: usize,
        killer_id: TankId,
        killer_team: Team,
        reason: KillReason,
    ) {
        if !self.tanks[victim_idx].alive {
            return;
        }
        self.tanks[victim_idx].alive = false;
        self.tanks[victim_idx].state.forward_speed = 0.0;
        let victim_id = self.tanks[victim_idx].id;
        let victim_team = self.tanks[victim_idx].team;
        let victim_name = self.tanks[victim_idx].name.clone();

        if killer_id != victim_id
            && let Some(shooter) = self.tanks.get_mut(killer_id.index())
        {
            shooter.kills += 1;
        }

        if self.battle.mode == BattleMode::TeamBattle {
            let tickets = self.battle.tickets_mut(victim_team);
            *tickets = (*tickets - KILL_TICKET_PENALTY).max(0.0);
        }

        let (killer_name, actual_killer_team) = self
            .tanks
            .get(killer_id.index())
            .map(|s| (s.name.clone(), s.team))
            .unwrap_or_else(|| ("Unknown".to_string(), killer_team));
        let event = KillEvent {
            tick: self.tick,
            killer_id,
            killer_name,
            killer_team: actual_killer_team,
            victim_id,
            victim_name,
            victim_team,
            reason,
        };
        self.recent_kills.push(event.clone());
        self.battle.kill_log.push(event);
    }
}
