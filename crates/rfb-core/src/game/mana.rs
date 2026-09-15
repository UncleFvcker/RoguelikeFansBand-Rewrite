// SPDX-License-Identifier: MPL-2.0

use super::*;

// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
// dungeon.c regenmana/process_world_aux_hp_and_sp, defines.h TURNS_PER_TICK.
const MANA_REGENERATION_INTERVAL: u32 = 10;

fn regeneration_change(maximum: u32, percent: i64) -> i64 {
    if percent > 0 {
        i64::from(maximum) * percent / 100 + 524
    } else if percent < 0 {
        let mut loss = i64::from(maximum) * 197 + 524;
        if percent < -47674 {
            loss = loss * (-percent / 4334) / 11;
        }
        -loss
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mindcrafter() -> Game {
        let mut game = Game::new_with_build(924, "demo.build.mindcrafter").unwrap();
        game.entities.clear();
        assert_eq!(game.resources["demo.resource.mana"].maximum, 9);
        game.resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 0;
        game
    }

    #[test]
    fn fractional_mana_accumulates_and_resumes_exactly_after_save() {
        let mut game = mindcrafter();
        for cycle in 1..=28 {
            game.world_tick = cycle * 10;
            game.process_mana_regeneration(false, &mut Vec::new());
        }
        assert_eq!(game.resources["demo.resource.mana"].current, 0);
        assert_eq!(
            game.resources["demo.resource.mana"].fraction,
            (28 * 2297) << 16
        );
        let mut loaded = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        assert_eq!(loaded.state_hash(), game.state_hash());
        for candidate in [&mut game, &mut loaded] {
            candidate.world_tick = 290;
            candidate.process_mana_regeneration(false, &mut Vec::new());
            assert_eq!(candidate.resources["demo.resource.mana"].current, 1);
        }
        assert_eq!(loaded.state_hash(), game.state_hash());
        let pool = game.resources.get_mut("demo.resource.mana").unwrap();
        pool.recover(100);
        assert_eq!((pool.current, pool.fraction), (9, 0));
        let mut invalid = game.to_save();
        invalid.player.resources[0].fraction = 1;
        assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
    }

    #[test]
    fn ordinary_actions_use_world_time_and_rest_uses_the_same_accumulator() {
        let mut game = mindcrafter();
        let mut resting = mindcrafter();
        let mut events = Vec::new();
        for cycle in 1..=17 {
            // A standard action costs 100 energy: ten world ticks at speed 110.
            for (candidate, rest) in [(&mut game, false), (&mut resting, true)] {
                spend_energy(&mut candidate.player.energy_need, STANDARD_ACTION_COST);
                candidate
                    .advance_until_player_ready(
                        rest,
                        false,
                        false,
                        &mut events,
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
                assert_eq!(candidate.world_tick, cycle * 10);
            }
        }
        assert_eq!(game.resources["demo.resource.mana"].current, 0);
        assert_eq!(resting.resources["demo.resource.mana"].current, 1);
        let before = game.resources["demo.resource.mana"];
        game.world_tick += 1;
        game.process_mana_regeneration(false, &mut events);
        assert_eq!(
            game.resources["demo.resource.mana"], before,
            "no extra recovery between cycles"
        );
    }

    #[test]
    fn upkeep_uses_fractional_borrow_and_original_negative_threshold() {
        assert_eq!(regeneration_change(9, 19700), 2297);
        assert_eq!(regeneration_change(9, 39400), 4070);
        assert_eq!(regeneration_change(9, 0), 0);
        assert_eq!(regeneration_change(9, -197), -2297);
        assert_eq!(regeneration_change(9, -47674), -2297);
        assert_eq!(regeneration_change(9, -52008), -2505);
        let mut pool = ResourcePool {
            current: 1,
            maximum: 9,
            fraction: 0,
        };
        pool.apply_fixed_change(-2297, 0);
        assert_eq!(pool.current, 0);
        assert_eq!(pool.fraction, (65536 - 2297) << 16);
        pool.apply_fixed_change(-65536, 0);
        assert_eq!((pool.current, pool.fraction), (0, 0));
    }

    #[test]
    fn hunger_and_search_affect_mana_but_poison_does_not_block_it() {
        let mut game = mindcrafter();
        game.player.statuses.push(
            crate::game::monster_combat::melee_status(STATUS_POISON, 20, "test.poison").status,
        );
        assert_eq!(game.mana_recovery_per_cycle(false), 2297);
        game.searching = true;
        assert_eq!(game.mana_recovery_per_cycle(false), 4070);
        assert_eq!(
            game.mana_recovery_per_cycle(true),
            4070,
            "search and rest do not stack"
        );
        game.nutrition = 0;
        assert_eq!(game.mana_recovery_per_cycle(true), 0);
    }

    #[test]
    fn maxima_refresh_rescales_once_and_preserves_zero_mana() {
        let mut game = mindcrafter();
        game.resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 3;
        game.progress.level = 10;
        game.refresh_player_resource_maxima();
        let after = game.resources["demo.resource.mana"];
        assert_eq!(after.current, after.maximum * 33 / 100);
        game.refresh_player_resource_maxima();
        assert_eq!(game.resources["demo.resource.mana"], after);
        game.resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 0;
        game.progress.level = 11;
        game.refresh_player_resource_maxima();
        assert_eq!(game.resources["demo.resource.mana"].current, 0);
    }
}

impl Game {
    /// Signed change per world regeneration cycle, in units of 2^-16 mana.
    pub(super) fn mana_recovery_per_cycle(&self, resting: bool) -> i64 {
        let Some(pool) = self.resources.get("demo.resource.mana") else {
            return 0;
        };
        if self.player_is_rage_mage() {
            return 0;
        }
        let decay = if pool.current > pool.maximum {
            -(i64::from(pool.maximum) * 32 * 197 + 524)
        } else {
            0
        };
        if self.wilderness_blocks_regeneration() {
            return decay;
        }
        let upkeep = i64::from(self.pet_upkeep().percent)
            + if self.music.spell.is_some() || self.samurai.posture == 3 {
                100
            } else {
                0
            };
        let factor = if upkeep > 100 {
            197
        } else {
            let equipment = self
                .items
                .iter()
                .filter(|item| {
                    matches!(&item.location, ItemLocation::Equipped { slot_id }
                    if self.body_slot_type(slot_id) != Some("tool"))
                        && self
                            .item_passives(item)
                            .contains(&EquipmentPassive::Regeneration)
                })
                .count() as u64
                * 100;
            let slow = self.player_has_equipped_curse_effect(ItemCurseEffectDto::SlowRegeneration);
            let regen =
                self.player_regeneration_rate_percent() + equipment / if slow { 5 } else { 1 };
            (self.nutrition_regeneration_factor() * regen / 100
                * if resting || self.searching { 2 } else { 1 }) as i64
        };
        let mut percent = (100 - upkeep) * factor;
        // CLASS_REGEN_MANA and equipment grant the same boolean ability.
        if percent > 0
            && (self
                .casting_profile()
                .is_some_and(|p| p.resource_recovery_percent >= 200)
                || self
                    .player_equipment_passives()
                    .contains(&EquipmentPassive::ManaRegeneration))
        {
            percent *= 2;
        }
        let normal = regeneration_change(pool.maximum, percent);
        if pool.current > pool.maximum {
            decay + normal.min(0)
        } else {
            normal
        }
    }

    pub(super) fn process_mana_regeneration(
        &mut self,
        resting: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.world_tick.is_multiple_of(MANA_REGENERATION_INTERVAL) {
            return;
        }
        let change = self.mana_recovery_per_cycle(resting);
        let upkeep_percent = self.pet_upkeep().percent;
        let Some(pool) = self.resources.get_mut("demo.resource.mana") else {
            return;
        };
        let before = pool.current;
        // Excess mana decays to the normal maximum, then upkeep can drain below it.
        if before > pool.maximum && change < 0 {
            let decay = i64::from(pool.maximum) * 32 * 197 + 524;
            pool.apply_fixed_change(-decay, pool.maximum);
            pool.apply_fixed_change(change + decay, 0);
        } else {
            pool.apply_fixed_change(change, 0);
            if change > 0 && pool.current >= pool.maximum {
                pool.current = pool.maximum;
                pool.fraction = 0;
            }
        }
        if pool.current > before {
            events.push(DomainEvent::ResourceRecovered {
                resolution: ResourceRecoveryResolutionDto {
                    resource_id: "demo.resource.mana".to_owned(),
                    before,
                    after: pool.current,
                    recovered: pool.current - before,
                },
            });
        } else if pool.current < before && upkeep_percent > 100 {
            events.push(DomainEvent::PetUpkeepManaLost {
                resource_id: "demo.resource.mana".to_owned(),
                amount: before - pool.current,
                upkeep_percent,
            });
        }
    }

    pub(super) fn rest_action_mana_recovery(&self) -> u32 {
        if self.player_is_samurai()
            && self
                .samurai_ability_unavailable_reason("demo.ability.samurai-concentration")
                .is_none()
        {
            return self
                .resources
                .get("demo.resource.mana")
                .map_or(0, |p| p.maximum / 2);
        }
        if self.player_is_mindcrafter()
            && self.progress.level >= 15
            && self.pet_upkeep().controlled_pets == 0
        {
            return super::player_abilities::clear_mind_recovery_amount(self.progress.level);
        }
        0
    }

    pub(super) fn recover_resources_for_rest_action(&mut self, events: &mut Vec<DomainEvent>) {
        if self.rest_action_mana_recovery() == 0 {
            return;
        }
        if self.player_is_samurai() {
            self.samurai_concentrate();
        } else {
            // The original rest loop calls cast_clear_mind directly, without a failure roll.
            self.resolve_player_clear_mind(events);
        }
    }
}
