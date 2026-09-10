// SPDX-License-Identifier: MPL-2.0
use super::*;

impl Game {
    fn curse_triggers(&mut self, effect: ItemCurseEffectDto, odds: u16) -> bool {
        self.player_has_equipped_curse_effect(effect) && ego::one_in(&mut self.rng, odds)
    }

    fn choose_cursed_item(&mut self, effect: ItemCurseEffectDto) -> usize {
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| self.item_has_active_equipped_curse_effect(item, effect))
            .map(|(index, item)| {
                (
                    match &item.location {
                        ItemLocation::Equipped { slot_id } => slot_id.clone(),
                        _ => unreachable!(),
                    },
                    item.id.clone(),
                    index,
                )
            })
            .collect::<Vec<_>>();
        candidates.sort();
        let choice = if candidates.len() == 1 {
            0
        } else {
            self.rng.bounded(candidates.len() as u64) as usize
        };
        candidates[choice].2
    }

    pub(super) fn curse_teleport(
        &mut self,
        distance: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.player_has_anti_teleport() {
            return;
        }
        let positions = self.random_teleport_candidates(distance);
        if !positions.is_empty() {
            let index = if positions.len() == 1 {
                0
            } else {
                self.rng.bounded(positions.len() as u64) as usize
            };
            events.extend(self.relocate_player(positions[index], changed));
        }
    }

    fn add_curse_status(&mut self, kind: &str, duration: u32) {
        let existing = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == kind)
            .map_or(0, |status| status.remaining_ticks);
        let mut status = monster_combat::melee_status(
            kind,
            duration.saturating_add(existing),
            "equipment.curse",
        );
        status.stacking = StatusStacking::Replace;
        apply_status_application(&mut self.player.statuses, status);
    }

    pub(super) fn process_other_equipped_curses(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        use ItemCurseEffectDto::*;
        if self.curse_triggers(ByCurse, 200) {
            self.equipped_baby_curse(events);
        }
        if self.curse_triggers(Normality, 128) {
            let count = ego::randint1(&mut self.rng, 20);
            for _ in 0..count {
                self.curse_dispel_one_status();
            }
        }
        if self.player_has_equipped_curse_effect(Allergy)
            && !self.player_has_status_kind(STATUS_UNWELL)
            && ego::one_in(&mut self.rng, 888)
            && self.curse_allergy_race_eligible()
        {
            self.add_curse_status(STATUS_UNWELL, 70);
        }
        if self.curse_triggers(CrappyMutation, 1500) {
            self.gain_random_bad_mutation(events);
        }
        if self
            .selected_race_definition()
            .is_none_or(|race| race.id != "rfb-legacy.race.android")
            && self.curse_triggers(DrainExperience, 4)
        {
            let amount = u64::from(self.progress.level.div_ceil(2));
            self.progress.maximum_experience =
                self.progress.maximum_experience.saturating_sub(amount);
            self.apply_player_experience_drain(amount, "equipment.curse", events);
        }
        for (effect, power) in [(AddLightCurse, 0), (AddHeavyCurse, 1)] {
            if self.curse_triggers(effect, 2000) {
                let index = self.choose_cursed_item(effect);
                let tval = self
                    .content
                    .item(&self.items[index].kind_id)
                    .and_then(|kind| kind.rfb_base_kind)
                    .map_or(0, |kind| kind.tval);
                let new = ego::curses::get_curse(&mut self.rng, power, tval);
                if let Some(roll) = self.items[index].rolled_affixes.first_mut() {
                    roll.curse_effects.insert(new);
                } else {
                    self.items[index].intrinsic_curse_effects.insert(new);
                }
            }
        }
        for (effect, odds, category) in [
            (CallAnimal, 2500, "animal"),
            (CallDemon, 1111, "demon"),
            (CallDragon, 800, "dragon"),
        ] {
            if self.curse_triggers(effect, odds) {
                let level = self.floor_depth(&self.current_floor_id);
                if self.ty_curse_summon("equipment.curse", category, level, true, events, changed)
                    > 0
                {
                    self.choose_cursed_item(effect);
                }
            }
        }
        if self.curse_triggers(Cowardice, 1500) && !self.curse_fear_save() {
            self.add_curse_status(STATUS_FEAR, 50);
        }
        let cursed_teleport = self.items.iter().any(|item| {
            item.curse.is_some() && self.item_has_active_equipped_curse_effect(item, Teleport)
        });
        if cursed_teleport && ego::one_in(&mut self.rng, 200) {
            self.curse_teleport(40, events, changed);
        }
        if self.curse_triggers(DrainHp, 666) {
            let index = self.choose_cursed_item(DrainHp);
            let source = self.items[index].kind_id.clone();
            let damage = resolve_damage(
                DamagePacket::new(
                    i32::from(self.progress.level).saturating_mul(2).min(100),
                    DamageType::Physical,
                ),
                ResistanceLevel::Normal,
            );
            let outcome = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            events.push(DomainEvent::ItemLifeLost {
                source_kind_id: source.clone(),
                display_name_key: self.item_display_name_key(&source),
                amount: outcome.damage.applied,
                fatal: self.player_is_dead(),
            });
        }
        if self.player_has_equipped_curse_effect(DrainMana)
            && let Some(id) = self
                .casting_profile()
                .map(|profile| profile.resource_id.clone())
            && self.resources.get(&id).is_some_and(|pool| pool.current > 0)
            && ego::one_in(&mut self.rng, 666)
        {
            let index = self.choose_cursed_item(DrainMana);
            let source = self.items[index].kind_id.clone();
            let pool = self.resources.get_mut(&id).unwrap();
            let drained = pool.current.min(u32::from(self.progress.level).min(50));
            pool.current -= drained;
            events.push(DomainEvent::ItemResourceDrained {
                source_kind_id: source.clone(),
                display_name_key: self.item_display_name_key(&source),
                resource_id: id,
                drained,
            });
        }
        if self.curse_triggers(DrainPack, 333) && self.curse_drain_pack() {
            self.choose_cursed_item(DrainPack);
        }
    }

    fn curse_drain_pack(&mut self) -> bool {
        let mut pack = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.location == ItemLocation::Inventory)
            .map(|(index, item)| (item.id.clone(), index))
            .collect::<Vec<_>>();
        pack.sort();
        if pack.is_empty() {
            return false;
        }
        for _ in 0..10 {
            let choice = if pack.len() == 1 {
                0
            } else {
                self.rng.bounded(pack.len() as u64) as usize
            };
            let index = pack[choice].1;
            let kind = self.content.item(&self.items[index].kind_id).unwrap();
            if !kind.tags.iter().any(|tag| tag == "device") {
                continue;
            }
            let rod = kind.tags.iter().any(|tag| tag == "rod");
            if self.player_has_device_charge_drain_immunity()
                || self
                    .item_passives(&self.items[index])
                    .contains(&EquipmentPassive::HoldLife)
            {
                return false;
            }
            self.decrease_item_charges(index, if rod { 100 / 3 } else { 100 });
            return true;
        }
        false
    }

    fn curse_allergy_race_eligible(&self) -> bool {
        // dungeon.c literally tests (!race->flags & RACE_IS_NONLIVING).
        self.character_definitions().is_none_or(|(_, race, _, _)| {
            !race.tags.iter().any(|tag| {
                matches!(
                    tag.as_str(),
                    "nonliving" | "undead" | "construct" | "monster" | "demon"
                )
            }) && !matches!(
                race.id.as_str(),
                "demo.race.rfb-human"
                    | "rfb-legacy.race.android"
                    | "rfb-legacy.race.balrog"
                    | "rfb-legacy.race.barbarian"
                    | "rfb-legacy.race.draconian"
                    | "rfb-legacy.race.dunadan"
                    | "rfb-legacy.race.half-orc"
                    | "rfb-legacy.race.einheri"
                    | "rfb-legacy.race.golem"
                    | "rfb-legacy.race.imp"
                    | "rfb-legacy.race.skeleton"
                    | "rfb-legacy.race.spectre"
                    | "rfb-legacy.race.vampire"
                    | "rfb-legacy.race.zombie"
            )
        })
    }

    fn curse_fear_save(&mut self) -> bool {
        const CHR_SAVE: [i32; 38] = [
            0, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 19, 20, 21, 22, 23, 24, 25, 26,
            27, 28, 29, 30, 32, 34, 36, 38, 40, 42, 44, 47, 50, 55,
        ];
        let threat = self
            .entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && self.entity_is_visible_to_player(actor)
                    && super::super::projectile_geometry::has_line_of_effect(
                        self,
                        self.player.position,
                        actor.position,
                    )
            })
            .filter_map(|actor| {
                self.actor_apparent_definition(actor)
                    .filter(|kind| kind.tags.iter().any(|tag| tag == "aura-fear"))
                    .map(|kind| {
                        (kind.level
                            + if kind.tags.iter().any(|tag| tag == "unique") {
                                3
                            } else {
                                0
                            })
                            / rfb_distance(self.player.position, actor.position)
                                .saturating_sub(2)
                                .max(1)
                    })
            })
            .max()
            .unwrap_or(0)
            .max(u32::from(
                self.floor_depth(&self.current_floor_id)
                    .saturating_add(2)
                    .min(127),
            )) as u16;
        let resistance = self.effective_player_resistances().level(DamageType::Fear);
        if threat <= 1
            || resistance == ResistanceLevel::Immune
            || self.player_status_immunities().contains(STATUS_FEAR)
        {
            return true;
        }
        let mut level = i32::from(self.progress.level);
        if self
            .character_definitions()
            .is_some_and(|(_, _, _, personality)| personality.id.ends_with(".craven"))
        {
            level = (level - 5).max(1);
        }
        if self.player_has_mutation(HUMAN_INT_MUTATION_ID) {
            level = (level - 10).max(1);
        }
        let mut power = (if level <= 40 {
            level + 5
        } else {
            45 + (level - 40) * 2
        } + CHR_SAVE[usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Charisma),
        )])
        .max(1);
        let rolls = match resistance {
            ResistanceLevel::Vulnerable => {
                power = (power + 1) / 2;
                1
            }
            ResistanceLevel::Resistant => 2,
            ResistanceLevel::Strong => 3,
            _ => 1,
        };
        for _ in 0..rolls {
            if ego::randint1(&mut self.rng, threat) <= ego::randint1(&mut self.rng, power as u16) {
                return true;
            }
        }
        false
    }

    fn curse_dispel_one_status(&mut self) -> bool {
        for _ in 0..200 {
            let kinds: &[&str] = match ego::randint1(&mut self.rng, 33) {
                1 => &[STATUS_HASTE],
                2 => &[STATUS_LIGHT_SPEED],
                4 => &["rfb.status.blessed"],
                5 => &["rfb.status.hero", "rfb.status.heroism"],
                6 => &[STATUS_BERSERK],
                7 => &[STATUS_PROTECTION_FROM_EVIL],
                8 => &[STATUS_INVULNERABILITY],
                9 => &[STATUS_WRAITHFORM],
                15 => &[STATUS_TELEPATHY],
                16 => &[STATUS_REGENERATION],
                17 => &[STATUS_VENGEANCE],
                18 => &[STATUS_MAGIC_ARMOR],
                19 => &[
                    STATUS_BASIC_RESISTANCE,
                    STATUS_THERMAL_RESISTANCE,
                    crate::effect::STATUS_POISON_RESISTANCE,
                    "rfb.status.resist-acid",
                    "rfb.status.resist-electricity",
                    "rfb.status.resist-fire",
                    "rfb.status.resist-cold",
                    "rfb.status.resist-poison",
                ],
                20 => &[STATUS_ULTIMATE_RESISTANCE],
                21 => &["rfb.status.elemental-brand"],
                22 => &["rfb.status.elemental-immunity"],
                24 => &[STATUS_MANA_BRAND],
                28 => &[STATUS_INVENTORY_PROTECTION],
                // Other original branches belong to statuses/classes not opened here;
                // failed attempts still consume their original 1..33 draw.
                _ => &[],
            };
            let before = self.player.statuses.len();
            self.player
                .statuses
                .retain(|status| !kinds.contains(&status.kind_id.as_str()));
            if self.player.statuses.len() != before {
                return true;
            }
        }
        false
    }

    fn equipped_baby_curse(&mut self, events: &mut Vec<DomainEvent>) {
        let fear = self.roll_damage(4, 42) as u32;
        self.add_curse_status(STATUS_FEAR, fear);
        let stun = ego::randint1(&mut self.rng, 42);
        self.add_curse_status(STATUS_STUN, u32::from(stun));
        let all = ego::one_in(&mut self.rng, 2);
        if !all {
            let duration = 10 + u32::from(ego::randint1(&mut self.rng, 10));
            self.add_curse_status(STATUS_HALLUCINATION, duration);
        }
        let attributes = [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ];
        for attribute in attributes {
            if all || ego::one_in(&mut self.rng, 3) {
                self.resolve_monster_attribute_drain(attribute);
            }
        }
        if ego::one_in(&mut self.rng, 5) && self.progress.experience > 0 {
            self.add_virtue(rfb_protocol::VirtueKindDto::Knowledge, -5);
            let amount = self.progress.experience
                / if self.player_hold_life_sources() > 0 {
                    24
                } else {
                    8
                };
            self.apply_player_experience_drain(amount, "equipment.curse", events);
        }
        if !ego::one_in(&mut self.rng, 4) {
            self.curse_disenchant_equipment();
        }
        let attribute = attributes[self.rng.bounded(6) as usize];
        let amount = 12 + ego::randint1(&mut self.rng, 6) as u8;
        let hp = self.effective_player_max_hp();
        let resources = self.player_resource_maxima();
        apply_permanent_attribute_drain(&mut self.progress, attribute, amount, &mut self.rng);
        self.refresh_after_attribute_change(hp, &resources);
    }

    fn curse_disenchant_equipment(&mut self) {
        // RFB high-resistance save uses power 33 and 30/40% for one/two sources.
        let resist = match self
            .effective_player_resistances()
            .level(DamageType::Disenchant)
        {
            ResistanceLevel::Vulnerable => -30,
            ResistanceLevel::Normal => 0,
            ResistanceLevel::Resistant => 30,
            ResistanceLevel::Strong => 40,
            ResistanceLevel::Immune => 100,
        };
        if (self.rng.bounded(33) as i32) < resist {
            return;
        }
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                matches!(item.location, ItemLocation::Equipped { .. })
                    && self
                        .content
                        .item(&item.kind_id)
                        .and_then(|kind| kind.rfb_base_kind)
                        .is_some_and(|kind| matches!(kind.tval, 16..=23 | 30..=38))
            })
            .map(|(index, item)| {
                (
                    match &item.location {
                        ItemLocation::Equipped { slot_id } => slot_id.clone(),
                        _ => unreachable!(),
                    },
                    index,
                )
            })
            .collect::<Vec<_>>();
        candidates.sort();
        if candidates.is_empty() {
            return;
        }
        let choice = if candidates.len() == 1 {
            0
        } else {
            self.rng.bounded(candidates.len() as u64) as usize
        };
        let index = candidates[choice].1;
        let Some(object) = item_value::instance::value_object(&self.content, &self.items[index])
        else {
            return;
        };
        if object.to_h <= 0 && object.to_d <= 0 && object.to_a <= 0 && object.pval <= 1 {
            return;
        }
        if object.artifact && self.rng.bounded(100) < 71 {
            return;
        }
        if object.flags.contains("RES_DISEN") {
            return;
        }
        let mut changed = false;
        let enchantments = &mut self.items[index].enchantments;
        for (old, value) in [
            (object.to_h, &mut enchantments.to_hit),
            (object.to_d, &mut enchantments.to_damage),
            (object.to_a, &mut enchantments.to_armor),
        ] {
            if old > 0 {
                *value -= 1;
                changed = true;
            }
            if old - 1 > 5 && self.rng.bounded(100) < 20 {
                *value -= 1;
            }
        }
        if object.pval > 1 && ego::one_in(&mut self.rng, 13) {
            ego::curses::decrease_shared_pval(&self.content, &mut self.items[index]);
            changed = true;
        }
        if changed {
            self.add_virtue(rfb_protocol::VirtueKindDto::Harmony, 1);
            self.add_virtue(rfb_protocol::VirtueKindDto::Enchantment, -2);
        }
    }

    pub(in crate::game) fn curse_danger_level(&self, level: u16, monster_summoner: bool) -> u16 {
        if !self.player_has_equipped_curse_effect(ItemCurseEffectDto::Danger) {
            return level;
        }
        let mut bonus = if monster_summoner {
            (level.saturating_add(15) / 10).max(4)
        } else {
            (level.saturating_add(40) / 20).max(4)
        };
        let in_dungeon = self.content.world(&self.world_id).is_some_and(|world| {
            world
                .procedural_floors
                .iter()
                .any(|floor| floor.id == self.current_floor_id && floor.dungeon_id.is_some())
        });
        if !in_dungeon {
            bonus -= bonus / 3;
        }
        level.saturating_add(bonus.min(127_u16.saturating_sub(level)))
    }
}
