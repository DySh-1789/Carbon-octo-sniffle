use std::collections::HashSet;

use crate::battle::{BattleOutcome, Team};
use crate::catalog::{Nation, VehicleCatalog};

/// Research, purchase, and post-battle repair costs for a vehicle tier (Update 0.1 Economy).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VehicleEconomyCost {
    pub rp_cost: u32,
    pub cr_cost: i64,
    pub repair_cr: i64,
}

/// Deterministic tier-scaled economy costs (Tier I reserves are completely free so players can never soft-lock).
pub const fn economy_cost_for_tier(tier: u8) -> VehicleEconomyCost {
    match tier {
        1 => VehicleEconomyCost {
            rp_cost: 0,
            cr_cost: 0,
            repair_cr: 0,
        },
        2 => VehicleEconomyCost {
            rp_cost: 450,
            cr_cost: 1_800,
            repair_cr: 220,
        },
        3 => VehicleEconomyCost {
            rp_cost: 1_100,
            cr_cost: 4_500,
            repair_cr: 450,
        },
        4 => VehicleEconomyCost {
            rp_cost: 2_200,
            cr_cost: 9_500,
            repair_cr: 750,
        },
        5 => VehicleEconomyCost {
            rp_cost: 3_800,
            cr_cost: 17_000,
            repair_cr: 1_100,
        },
        6 => VehicleEconomyCost {
            rp_cost: 5_800,
            cr_cost: 27_000,
            repair_cr: 1_500,
        },
        7 => VehicleEconomyCost {
            rp_cost: 8_500,
            cr_cost: 40_000,
            repair_cr: 2_000,
        },
        _ => VehicleEconomyCost {
            rp_cost: 12_000,
            cr_cost: 58_000,
            repair_cr: 2_600,
        },
    }
}

/// Progression ownership state of a vehicle inside a nation's 8-tier tech tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VehicleOwnership {
    /// Prerequisite lower tier is not yet owned.
    Locked,
    /// Prerequisite lower tier is owned; ready to unlock with Nation RP.
    Researchable,
    /// Researched with RP; ready to purchase with Silver Credits (CR).
    Researched,
    /// Owned in garage and ready for combat.
    Owned,
}

impl VehicleOwnership {
    pub const fn badge_label(self) -> &'static str {
        match self {
            Self::Locked => "LOCKED",
            Self::Researchable => "RESEARCH",
            Self::Researched => "BUY",
            Self::Owned => "OWNED",
        }
    }
}

/// Input performance snapshot passed to `PlayerProfile::settle_battle` when a battle concludes.
#[derive(Clone, Debug)]
pub struct BattleRewardInput {
    pub nation: Nation,
    pub tier: u8,
    pub outcome: BattleOutcome,
    pub player_survived: bool,
    pub kills: u32,
    pub penetrations: u32,
    pub allied_zones_held: u32,
}

/// Itemized post-battle economy receipt displayed on the Battle End modal & Hangar status bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleRewardSummary {
    pub nation: Nation,
    pub tier: u8,
    pub victory: bool,
    pub base_cr: i64,
    pub combat_cr: i64,
    pub gross_cr: i64,
    pub repair_cr: i64,
    pub net_cr: i64,
    pub rp_earned: u32,
    pub new_credits: i64,
    pub new_nation_rp: u32,
}

/// Persistent player progression & economy profile (Update 0.1).
#[derive(Clone, Debug)]
pub struct PlayerProfile {
    /// Global Silver Credits (`CR`) shared across all nations.
    pub credits: i64,
    /// Research Points (`RP`) indexed by `[USA, Germany, USSR]`.
    pub nation_rp: [u32; 3],
    /// Vehicles owned in garage (always includes Tier I reserves `us_m2a4`, `de_pz2c`, `su_bt5`).
    pub owned_ids: HashSet<String>,
    /// Vehicles whose tech-tree research has been completed.
    pub researched_ids: HashSet<String>,
    pub battles_played: u32,
    pub victories: u32,
    pub total_kills: u32,
    pub total_penetrations: u32,
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self::new_starter()
    }
}

impl PlayerProfile {
    /// Starter profile with `2,500 CR` and all three free Tier I reserve tanks owned.
    pub fn new_starter() -> Self {
        let starters = ["us_m2a4", "de_pz2c", "su_bt5"];
        let mut owned_ids = HashSet::new();
        let mut researched_ids = HashSet::new();
        for id in starters {
            owned_ids.insert(id.to_string());
            researched_ids.insert(id.to_string());
        }
        Self {
            credits: 2_500,
            nation_rp: [0, 0, 0],
            owned_ids,
            researched_ids,
            battles_played: 0,
            victories: 0,
            total_kills: 0,
            total_penetrations: 0,
        }
    }

    #[inline]
    pub const fn nation_index(nation: Nation) -> usize {
        match nation {
            Nation::Usa => 0,
            Nation::Germany => 1,
            Nation::Ussr => 2,
        }
    }

    #[inline]
    pub fn rp_for(&self, nation: Nation) -> u32 {
        self.nation_rp[Self::nation_index(nation)]
    }

    #[inline]
    pub fn add_rp(&mut self, nation: Nation, amount: u32) {
        let idx = Self::nation_index(nation);
        self.nation_rp[idx] = self.nation_rp[idx].saturating_add(amount);
    }

    #[inline]
    pub fn is_owned(&self, vehicle_id: &str) -> bool {
        self.owned_ids.contains(vehicle_id)
    }

    /// Computes whether a vehicle is `Owned`, `Researched`, `Researchable`, or `Locked`.
    pub fn ownership_state(
        &self,
        catalog: &VehicleCatalog,
        vehicle_id: &str,
    ) -> VehicleOwnership {
        if self.owned_ids.contains(vehicle_id) {
            return VehicleOwnership::Owned;
        }
        if self.researched_ids.contains(vehicle_id) {
            return VehicleOwnership::Researched;
        }
        let Some(spec) = catalog.by_id(vehicle_id) else {
            return VehicleOwnership::Locked;
        };
        if spec.tier <= 1 {
            return VehicleOwnership::Owned;
        }
        if let Some(prev) = catalog.by_nation_and_tier(spec.nation, spec.tier - 1)
            && self.owned_ids.contains(&prev.id)
        {
            VehicleOwnership::Researchable
        } else {
            VehicleOwnership::Locked
        }
    }

    pub fn can_research(&self, catalog: &VehicleCatalog, vehicle_id: &str) -> bool {
        let Some(spec) = catalog.by_id(vehicle_id) else {
            return false;
        };
        let cost = economy_cost_for_tier(spec.tier);
        self.ownership_state(catalog, vehicle_id) == VehicleOwnership::Researchable
            && self.rp_for(spec.nation) >= cost.rp_cost
    }

    pub fn can_buy(&self, catalog: &VehicleCatalog, vehicle_id: &str) -> bool {
        let Some(spec) = catalog.by_id(vehicle_id) else {
            return false;
        };
        let cost = economy_cost_for_tier(spec.tier);
        self.ownership_state(catalog, vehicle_id) == VehicleOwnership::Researched
            && self.credits >= cost.cr_cost
    }

    /// Spends Nation RP to research a `Researchable` vehicle.
    pub fn research_vehicle(
        &mut self,
        catalog: &VehicleCatalog,
        vehicle_id: &str,
    ) -> Result<u32, &'static str> {
        let Some(spec) = catalog.by_id(vehicle_id) else {
            return Err("UNKNOWN VEHICLE");
        };
        match self.ownership_state(catalog, vehicle_id) {
            VehicleOwnership::Owned => return Err("ALREADY OWNED"),
            VehicleOwnership::Researched => return Err("ALREADY RESEARCHED"),
            VehicleOwnership::Locked => return Err("OWN PREVIOUS TIER FIRST"),
            VehicleOwnership::Researchable => {}
        }
        let cost = economy_cost_for_tier(spec.tier).rp_cost;
        let idx = Self::nation_index(spec.nation);
        if self.nation_rp[idx] < cost {
            return Err("INSUFFICIENT RP");
        }
        self.nation_rp[idx] -= cost;
        self.researched_ids.insert(spec.id.clone());
        Ok(cost)
    }

    /// Spends Silver Credits (`CR`) to purchase a `Researched` vehicle into the player's garage.
    pub fn buy_vehicle(
        &mut self,
        catalog: &VehicleCatalog,
        vehicle_id: &str,
    ) -> Result<i64, &'static str> {
        let Some(spec) = catalog.by_id(vehicle_id) else {
            return Err("UNKNOWN VEHICLE");
        };
        match self.ownership_state(catalog, vehicle_id) {
            VehicleOwnership::Owned => return Err("ALREADY OWNED"),
            VehicleOwnership::Locked => return Err("OWN PREVIOUS TIER FIRST"),
            VehicleOwnership::Researchable => return Err("RESEARCH VEHICLE FIRST"),
            VehicleOwnership::Researched => {}
        }
        let cost = economy_cost_for_tier(spec.tier).cr_cost;
        if self.credits < cost {
            return Err("INSUFFICIENT CR");
        }
        self.credits -= cost;
        self.owned_ids.insert(spec.id.clone());
        Ok(cost)
    }

    /// Settles end-of-battle Silver Credits, Nation RP, and vehicle repair cost into the profile.
    pub fn settle_battle(&mut self, input: &BattleRewardInput) -> BattleRewardSummary {
        let victory = input.outcome == BattleOutcome::Victory(Team::Allies);
        let tier_mult = 1.0 + (input.tier.saturating_sub(1) as f32) * 0.22;
        let win_mult = if victory { 1.5 } else { 1.0 };

        let base_cr = (600.0 * tier_mult * win_mult).round() as i64;
        let combat_raw_cr = (input.kills as f32) * 850.0
            + (input.penetrations as f32) * 220.0
            + (input.allied_zones_held as f32) * 300.0;
        let combat_cr = (combat_raw_cr * tier_mult * win_mult).round() as i64;
        let gross_cr = base_cr + combat_cr;

        let base_rp = 180.0
            + (input.kills as f32) * 260.0
            + (input.penetrations as f32) * 65.0
            + (input.allied_zones_held as f32) * 90.0;
        let rp_earned = (base_rp * tier_mult * win_mult).round().max(0.0) as u32;

        // Free repair if tank survived OR if driving a Tier I reserve
        let repair_cr = if input.player_survived {
            0
        } else {
            economy_cost_for_tier(input.tier).repair_cr
        };

        let net_cr = gross_cr - repair_cr;
        self.credits = (self.credits + net_cr).max(0);
        self.add_rp(input.nation, rp_earned);
        self.battles_played = self.battles_played.saturating_add(1);
        if victory {
            self.victories = self.victories.saturating_add(1);
        }
        self.total_kills = self.total_kills.saturating_add(input.kills);
        self.total_penetrations = self.total_penetrations.saturating_add(input.penetrations);

        BattleRewardSummary {
            nation: input.nation,
            tier: input.tier,
            victory,
            base_cr,
            combat_cr,
            gross_cr,
            repair_cr,
            net_cr,
            rp_earned,
            new_credits: self.credits,
            new_nation_rp: self.rp_for(input.nation),
        }
    }

    /// Serializes profile to a deterministic pure-Rust `key=value` text format (zero external deps).
    pub fn to_save_text(&self) -> String {
        let mut owned: Vec<&str> = self.owned_ids.iter().map(String::as_str).collect();
        owned.sort_unstable();
        let mut researched: Vec<&str> = self.researched_ids.iter().map(String::as_str).collect();
        researched.sort_unstable();
        format!(
            "version=1\ncredits={}\nrp_usa={}\nrp_germany={}\nrp_ussr={}\nbattles={}\nvictories={}\nkills={}\npens={}\nowned={}\nresearched={}\n",
            self.credits,
            self.nation_rp[0],
            self.nation_rp[1],
            self.nation_rp[2],
            self.battles_played,
            self.victories,
            self.total_kills,
            self.total_penetrations,
            owned.join(","),
            researched.join(","),
        )
    }

    /// Parses a saved profile from `to_save_text`, always guaranteeing the 3 Tier I reserves remain owned.
    pub fn from_save_text(text: &str) -> Option<Self> {
        let mut profile = Self::new_starter();
        let mut saw_version = false;
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once('=')?;
            match k {
                "version" => {
                    if v != "1" {
                        return None;
                    }
                    saw_version = true;
                }
                "credits" => profile.credits = v.parse::<i64>().ok()?.max(0),
                "rp_usa" => profile.nation_rp[0] = v.parse::<u32>().ok()?,
                "rp_germany" => profile.nation_rp[1] = v.parse::<u32>().ok()?,
                "rp_ussr" => profile.nation_rp[2] = v.parse::<u32>().ok()?,
                "battles" => profile.battles_played = v.parse::<u32>().ok()?,
                "victories" => profile.victories = v.parse::<u32>().ok()?,
                "kills" => profile.total_kills = v.parse::<u32>().ok()?,
                "pens" => profile.total_penetrations = v.parse::<u32>().ok()?,
                "owned" => {
                    for id in v.split(',').filter(|s| !s.is_empty()) {
                        profile.owned_ids.insert(id.to_string());
                        profile.researched_ids.insert(id.to_string());
                    }
                }
                "researched" => {
                    for id in v.split(',').filter(|s| !s.is_empty()) {
                        profile.researched_ids.insert(id.to_string());
                    }
                }
                _ => {}
            }
        }
        saw_version.then_some(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_profile_owns_free_tier1_reserves_and_enforces_tree_order() {
        let catalog = VehicleCatalog::stage5_catalog();
        let mut profile = PlayerProfile::new_starter();

        assert_eq!(profile.credits, 2_500);
        for starter in ["us_m2a4", "de_pz2c", "su_bt5"] {
            assert_eq!(
                profile.ownership_state(&catalog, starter),
                VehicleOwnership::Owned
            );
        }
        // Tier II is immediately researchable, Tier III is locked until Tier II is owned
        assert_eq!(
            profile.ownership_state(&catalog, "us_m3_stuart"),
            VehicleOwnership::Researchable
        );
        assert_eq!(
            profile.ownership_state(&catalog, "us_m4a1"),
            VehicleOwnership::Locked
        );
        assert!(profile.research_vehicle(&catalog, "us_m4a1").is_err());
        assert!(profile.research_vehicle(&catalog, "us_m3_stuart").is_err()); // 0 RP

        // Earn battle rewards in Tier I M2A4 (Victory, 2 kills, 4 pens, 2 zones)
        let reward = profile.settle_battle(&BattleRewardInput {
            nation: Nation::Usa,
            tier: 1,
            outcome: BattleOutcome::Victory(Team::Allies),
            player_survived: false, // Tier I reserve repair is FREE (0 CR)
            kills: 2,
            penetrations: 4,
            allied_zones_held: 2,
        });
        assert!(reward.victory);
        assert_eq!(reward.repair_cr, 0, "Tier I reserves must have 0 CR repair");
        assert!(reward.net_cr > 3_000);
        assert!(
            reward.rp_earned >= 1_000,
            "1 strong Tier I victory should unlock Tier II research (450 RP), got {}",
            reward.rp_earned
        );
        assert_eq!(profile.battles_played, 1);
        assert_eq!(profile.victories, 1);

        // Now research & buy Tier II M3 Stuart -> Tier III M4A1 becomes Researchable!
        let spent_rp = profile.research_vehicle(&catalog, "us_m3_stuart").unwrap();
        assert_eq!(spent_rp, 450);
        assert_eq!(
            profile.ownership_state(&catalog, "us_m3_stuart"),
            VehicleOwnership::Researched
        );
        let spent_cr = profile.buy_vehicle(&catalog, "us_m3_stuart").unwrap();
        assert_eq!(spent_cr, 1_800);
        assert_eq!(
            profile.ownership_state(&catalog, "us_m3_stuart"),
            VehicleOwnership::Owned
        );
        assert_eq!(
            profile.ownership_state(&catalog, "us_m4a1"),
            VehicleOwnership::Researchable
        );
    }

    #[test]
    fn high_tier_repair_deducts_on_destruction_and_save_text_roundtrips() {
        let mut profile = PlayerProfile::new_starter();
        profile.credits = 100;
        let loss = profile.settle_battle(&BattleRewardInput {
            nation: Nation::Germany,
            tier: 8,
            outcome: BattleOutcome::Victory(Team::Axis),
            player_survived: false,
            kills: 0,
            penetrations: 0,
            allied_zones_held: 0,
        });
        assert_eq!(loss.repair_cr, 2_600);
        assert!(loss.net_cr < 0);
        assert_eq!(
            profile.credits, 0,
            "Credits must never go negative after high-tier repair"
        );
        assert!(profile.rp_for(Nation::Germany) > 0);

        // Save/load round-trip
        profile.credits = 14_350;
        profile.owned_ids.insert("de_tiger_h1".to_string());
        profile.researched_ids.insert("de_tiger2_h".to_string());
        let saved = profile.to_save_text();
        let restored = PlayerProfile::from_save_text(&saved).expect("valid profile save");
        assert_eq!(restored.credits, 14_350);
        assert_eq!(restored.nation_rp, profile.nation_rp);
        assert!(restored.is_owned("de_tiger_h1"));
        assert!(restored.researched_ids.contains("de_tiger2_h"));
        assert!(restored.is_owned("us_m2a4"));
    }
}
