// SPDX-License-Identifier: MPL-2.0

use super::*;
#[cfg(test)]
#[path = "item_curses/tests.rs"]
mod consumer_tests;
mod periodic;
mod ty_curse;

const EQUIPMENT_CURSE_INTERVAL_TICKS: u32 = 10;
#[cfg(test)]
const RANDOM_TELEPORT_ONE_IN: u64 = 200;

impl Game {
    pub(super) fn item_has_rfb_flag(&self, item: &ItemInstance, flag: &str) -> bool {
        item.intrinsic_properties.rfb_flags.contains(flag)
            || item
                .rolled_affixes
                .iter()
                .any(|roll| roll.properties.rfb_flags.contains(flag))
            || self
                .content
                .item(&item.kind_id)
                .and_then(|kind| kind.rfb_value.as_ref())
                .is_some_and(|value| value.flags.contains(flag))
            || item.affix_ids.iter().any(|id| {
                self.content
                    .affix(id)
                    .and_then(|affix| affix.rfb_ego.as_ref())
                    .is_some_and(|ego| ego.flags.contains(flag))
            })
    }

    pub(super) fn item_has_intrinsic_curse_effect(
        &self,
        item: &ItemInstance,
        effect: ItemCurseEffectDto,
    ) -> bool {
        let flag = match effect {
            ItemCurseEffectDto::Aggravate => "AGGRAVATE",
            ItemCurseEffectDto::Teleport => "TELEPORT",
            ItemCurseEffectDto::TyCurse => "TY_CURSE",
            ItemCurseEffectDto::DrainExperience => "DRAIN_EXP",
            _ => return false,
        };
        self.item_has_rfb_flag(item, flag)
    }

    pub(super) fn item_has_heavy_curse(&self, item: &ItemInstance) -> bool {
        match item.curse {
            Some(ItemCurseSeverityDto::Heavy) => true,
            Some(ItemCurseSeverityDto::Permanent) => {
                item.intrinsic_properties.rfb_heavy_curse
                    || item
                        .rolled_affixes
                        .iter()
                        .any(|roll| roll.properties.rfb_heavy_curse)
                    || self.item_has_rfb_flag(item, "HEAVY_CURSE")
            }
            _ => false,
        }
    }

    pub(super) fn player_has_equipped_curse_effect(&self, effect: ItemCurseEffectDto) -> bool {
        self.items
            .iter()
            .any(|item| self.item_has_active_equipped_curse_effect(item, effect))
    }

    pub(super) fn equipped_curse_penalty(
        &self,
        item: &ItemInstance,
        effect: ItemCurseEffectDto,
        normal: i32,
        heavy: i32,
    ) -> i32 {
        if self.item_has_active_equipped_curse_effect(item, effect) {
            if self.item_has_heavy_curse(item) {
                heavy
            } else {
                normal
            }
        } else {
            0
        }
    }

    pub(super) fn item_has_active_equipped_curse_effect(
        &self,
        item: &ItemInstance,
        effect: ItemCurseEffectDto,
    ) -> bool {
        matches!(item.location, ItemLocation::Equipped { .. })
            && (self.item_has_intrinsic_curse_effect(item, effect)
                || (item.curse.is_some()
                    && (item.intrinsic_curse_effects.contains(&effect)
                        || item
                            .rolled_affixes
                            .iter()
                            .any(|rolled| rolled.curse_effects.contains(&effect)))))
    }

    pub(super) fn player_has_equipped_aggravation(&self) -> bool {
        self.items.iter().any(|item| {
            self.item_has_active_equipped_curse_effect(item, ItemCurseEffectDto::Aggravate)
        })
    }

    pub(super) fn player_aggravates_monsters(&self) -> bool {
        self.player_has_equipped_aggravation() && self.player_fairy_stealth_race_id().is_none()
    }

    pub(super) fn wake_monster_for_equipped_aggravation(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if !self.player_aggravates_monsters() {
            return;
        }
        let before = self.entities[index].statuses.len();
        self.entities[index]
            .statuses
            .retain(|status| status.kind_id != STATUS_SLEEP);
        if self.entities[index].statuses.len() == before {
            return;
        }
        self.entities[index].alerted = true;
        changed.insert(self.entities[index].position);
        events.push(DomainEvent::EntityStatusExpired {
            target_kind_id: self.entities[index].kind_id.clone(),
            status_kind_id: STATUS_SLEEP.to_owned(),
        });
    }

    pub(super) fn process_equipped_curse_effects(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if !self
            .world_tick
            .is_multiple_of(EQUIPMENT_CURSE_INTERVAL_TICKS)
        {
            return Ok(());
        }
        // mauler.c: carried or equipped Vice drains gold; exact zero survives.
        let maul_index = self
            .items
            .iter()
            .position(|item| {
                item.location == ItemLocation::Inventory && self.item_is_fixed_artifact(item, 279)
            })
            .or_else(|| {
                self.items.iter().position(|item| {
                    matches!(item.location, ItemLocation::Equipped { .. })
                        && self.item_is_fixed_artifact(item, 279)
                })
            });
        if let Some(index) = maul_index {
            let amount = self.rng.bounded(u64::from(self.progress.level.max(1))) as u32 + 1;
            let insufficient = self.gold < amount;
            self.gold = self.gold.saturating_sub(amount);
            let message_key = if insufficient {
                self.blast_item(index);
                Some("item-vice-blasted")
            } else if self.gold < 1000 {
                Some("item-vice-low-gold")
            } else if self.rng.bounded(111) == 0 {
                Some("item-vice-gold-draining")
            } else {
                None
            };
            if let Some(message_key) = message_key {
                events.push(DomainEvent::ItemSpecialMessage {
                    message_key: message_key.to_owned(),
                });
            }
        }
        // dungeon.c: the Jewel's periodic life loss is independent of curses.
        if self.items.iter().any(|item| {
            item.kind_id == "demo.item.jewel-of-judgement"
                && matches!(item.location, ItemLocation::Equipped { .. })
        }) && !self.player_has_anti_magic()
            && self.rng.bounded(999) == 0
        {
            self.resolve_item_life_loss(
                "demo.item.jewel-of-judgement",
                u32::from(self.progress.level.min(50)),
                events,
            );
            changed.insert(self.player.position);
            if self.player_is_dead() {
                return Ok(());
            }
        }
        let intrinsic_teleport = self.items.iter().any(|item| {
            matches!(item.location, ItemLocation::Equipped { .. })
                && item.curse.is_none()
                && self.item_has_intrinsic_curse_effect(item, ItemCurseEffectDto::Teleport)
        });
        if intrinsic_teleport && ego::one_in(&mut self.rng, 200) {
            let mut chosen = None;
            let mut count = 0;
            for (index, item) in self.items.iter().enumerate() {
                if matches!(item.location, ItemLocation::Equipped { .. })
                    && self.item_has_intrinsic_curse_effect(item, ItemCurseEffectDto::Teleport)
                    && item
                        .inscription
                        .as_deref()
                        .is_none_or(|text| !text.contains('.'))
                {
                    count += 1;
                    if ego::one_in(&mut self.rng, count) {
                        chosen = Some(index);
                    }
                }
            }
            if chosen.is_some() {
                self.curse_teleport(50, events, changed);
            }
        }
        if self.player_has_equipped_artifact(85) && self.rng.bounded(100) == 0 {
            self.chainsword_noise(events);
        }
        if let Some(source) = self
            .items
            .iter()
            .find(|item| {
                self.item_has_active_equipped_curse_effect(item, ItemCurseEffectDto::TyCurse)
            })
            .map(|item| item.kind_id.clone())
            && self.rng.bounded(200) == 0
        {
            self.resolve_equipped_ty_curse(&source, events, changed, removed_entities)?;
            changed.insert(self.player.position);
            if self.player_is_dead() {
                return Ok(());
            }
        }
        self.process_other_equipped_curses(events, changed);
        Ok(())
    }

    pub(super) fn chainsword_noise(&mut self, events: &mut Vec<DomainEvent>) {
        // master:lib/file/chainswd.txt has six authoritative (English) lines.
        let line = self.rng.bounded(6) + 1;
        events.push(DomainEvent::ItemSpecialMessage {
            message_key: format!("item-chainsword-noise-{line}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{effect::StatusInstance, game::inventory::RemoveEquippedCursesRequest};

    fn equipped_weapon_index(game: &Game) -> usize {
        game.items
            .iter()
            .position(|item| {
                matches!(
                    &item.location,
                    ItemLocation::Equipped { slot_id }
                        if game.body_slot_type(slot_id) == Some("weapon")
                )
            })
            .expect("warrior should start with an equipped weapon")
    }

    fn add_curse_effect(
        game: &mut Game,
        effect: ItemCurseEffectDto,
        curse: Option<ItemCurseSeverityDto>,
    ) {
        let index = equipped_weapon_index(game);
        game.items[index].curse = curse;
        game.items[index].rolled_affixes.push(RolledAffixState {
            affix_id: "rfb-legacy.affix.slaying".to_owned(),
            curse_effects: BTreeSet::from([effect]),
            ..RolledAffixState::default()
        });
    }

    fn sleeping_status() -> StatusInstance {
        StatusInstance {
            kind_id: STATUS_SLEEP.to_owned(),
            intensity: 1,
            remaining_ticks: 100,
            source_id: Some("test.cursed-weapon".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }

    #[test]
    fn equipped_aggravation_wakes_sleepers_until_its_curse_is_removed() {
        let mut game = Game::new_with_build(1, "demo.build.warrior").expect("warrior build");
        game.entities.clear();
        game.push_generated_actor(
            "test.curse-sleeper".to_owned(),
            "demo.actor.small-kobold",
            Position { x: 4, y: 3 },
        );
        game.entities[0].statuses.push(sleeping_status());
        add_curse_effect(
            &mut game,
            ItemCurseEffectDto::Aggravate,
            Some(ItemCurseSeverityDto::Heavy),
        );

        game.wake_monster_for_equipped_aggravation(0, &mut Vec::new(), &mut BTreeSet::new());
        assert!(
            game.entities[0]
                .statuses
                .iter()
                .all(|status| status.kind_id != STATUS_SLEEP)
        );

        game.entities[0].statuses.push(sleeping_status());
        game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
        game.wake_monster_for_equipped_aggravation(0, &mut Vec::new(), &mut BTreeSet::new());
        assert!(
            game.entities[0]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLEEP)
        );
    }

    #[test]
    fn shadow_fairy_form_converts_equipped_aggravation_into_a_stealth_penalty() {
        let mut game = Game::new_with_build_race_and_name(
            421,
            "demo.build.warrior",
            "demo.race.rfb-human",
            Game::DEFAULT_PLAYER_NAME,
            Game::default_behavior_preferences(),
        )
        .expect("Human warrior should create");
        let mut form = crate::game::monster_combat::melee_status(
            STATUS_PLAYER_POLYMORPH,
            100,
            "test.shadow-fairy-form",
        )
        .status;
        form.granted_race_id = Some("rfb-legacy.race.shadow-fairy".to_owned());
        game.player.statuses.push(form);
        let stealth_before = game.player_derived_stats().stealth_skill.value;

        add_curse_effect(
            &mut game,
            ItemCurseEffectDto::Aggravate,
            Some(ItemCurseSeverityDto::Heavy),
        );
        assert!(game.player_has_equipped_aggravation());
        assert!(!game.player_aggravates_monsters());
        assert_eq!(
            game.player_derived_stats().stealth_skill.value,
            stealth_before
                .saturating_sub(3)
                .min(stealth_before.saturating_add(2) / 2)
                .max(0),
        );

        game.entities.clear();
        game.push_generated_actor(
            "test.shadow-fairy-sleeper".to_owned(),
            "demo.actor.small-kobold",
            Position { x: 4, y: 3 },
        );
        game.entities[0].statuses.push(sleeping_status());
        game.wake_monster_for_equipped_aggravation(0, &mut Vec::new(), &mut BTreeSet::new());
        assert!(
            game.entities[0]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLEEP)
        );

        game.player
            .statuses
            .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
        assert!(game.player_aggravates_monsters());
        game.wake_monster_for_equipped_aggravation(0, &mut Vec::new(), &mut BTreeSet::new());
        assert!(
            game.entities[0]
                .statuses
                .iter()
                .all(|status| status.kind_id != STATUS_SLEEP)
        );
    }

    #[test]
    fn intrinsic_ego_drawbacks_survive_without_a_curse_severity() {
        let mut game = Game::new_with_build(2, "demo.build.warrior").unwrap();
        let index = equipped_weapon_index(&game);
        for (flag, effect) in [
            ("AGGRAVATE", ItemCurseEffectDto::Aggravate),
            ("TELEPORT", ItemCurseEffectDto::Teleport),
            ("TY_CURSE", ItemCurseEffectDto::TyCurse),
            ("DRAIN_EXP", ItemCurseEffectDto::DrainExperience),
        ] {
            game.items[index].intrinsic_properties.rfb_flags = BTreeSet::from([flag.to_owned()]);
            game.items[index].curse = None;
            assert!(game.item_has_active_equipped_curse_effect(&game.items[index], effect));
            game.items[index].intrinsic_properties.rfb_flags.clear();
            assert!(!game.item_has_active_equipped_curse_effect(&game.items[index], effect));
        }
    }

    #[test]
    fn darkness_is_an_intrinsic_equipment_radius_penalty() {
        let mut game = Game::new_with_build(2, "demo.build.warrior").expect("warrior build");
        let index = equipped_weapon_index(&game);
        game.items[index].rolled_affixes.push(RolledAffixState {
            affix_id: "test.affix.light".to_owned(),
            properties: AffixPropertyBundleDefinition {
                equipment_bonuses: EquipmentBonuses {
                    light_radius: 1,
                    ..EquipmentBonuses::default()
                },
                ..AffixPropertyBundleDefinition::default()
            },
            ..RolledAffixState::default()
        });
        assert_eq!(game.player_light_radius(), Some(1));
        game.items[index].curse = Some(ItemCurseSeverityDto::Heavy);
        game.items[index].rolled_affixes.push(RolledAffixState {
            affix_id: "test.affix.death-darkness".to_owned(),
            properties: AffixPropertyBundleDefinition {
                equipment_bonuses: EquipmentBonuses {
                    light_radius: -1,
                    ..EquipmentBonuses::default()
                },
                ..AffixPropertyBundleDefinition::default()
            },
            ..RolledAffixState::default()
        });
        assert_eq!(game.player_light_radius(), None);

        game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
        assert_eq!(game.player_light_radius(), None);
    }

    #[test]
    fn random_teleport_checks_once_per_rfb_world_interval_and_stops_after_uncursing() {
        let seed = (0..10_000_u64)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(RANDOM_TELEPORT_ONE_IN) == 0
            })
            .expect("a deterministic teleport seed should exist");
        let mut game = Game::new_with_build(3, "demo.build.warrior").expect("warrior build");
        game.entities.clear();
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 3, y: 3 };
        add_curse_effect(
            &mut game,
            ItemCurseEffectDto::Teleport,
            Some(ItemCurseSeverityDto::Heavy),
        );
        game.rng = RfbRng::seeded(seed);
        game.world_tick = 9;
        let draws_before = game.rng_draw_counter();
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert_eq!(game.player.position, Position { x: 3, y: 3 });
        assert_eq!(game.rng_draw_counter(), draws_before);

        game.world_tick = 10;
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert_ne!(game.player.position, Position { x: 3, y: 3 });

        game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
        game.player.position = Position { x: 3, y: 3 };
        game.rng = RfbRng::seeded(seed);
        game.world_tick = 20;
        let draws_before = game.rng_draw_counter();
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert_eq!(game.player.position, Position { x: 3, y: 3 });
        assert_eq!(game.rng_draw_counter(), draws_before);
    }
}
