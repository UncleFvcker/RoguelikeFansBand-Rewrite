// SPDX-License-Identifier: MPL-2.0

use super::*;

const EQUIPMENT_CURSE_INTERVAL_TICKS: u32 = 10;
const RANDOM_TELEPORT_ONE_IN: u64 = 200;

fn rfb_ego_intrinsic_curse_effect(source_index: Option<u32>, effect: ItemCurseEffectDto) -> bool {
    matches!(
        (source_index, effect),
        (Some(11 | 21 | 27), ItemCurseEffectDto::Aggravate)
            | (Some(15), ItemCurseEffectDto::Teleport)
    )
}

impl Game {
    fn item_has_active_equipped_curse_effect(
        &self,
        item: &ItemInstance,
        effect: ItemCurseEffectDto,
    ) -> bool {
        matches!(item.location, ItemLocation::Equipped { .. })
            && item.rolled_affixes.iter().any(|rolled| {
                rolled.curse_effects.contains(&effect)
                    && (item.curse.is_some()
                        || rfb_ego_intrinsic_curse_effect(
                            self.content
                                .affix(&rolled.affix_id)
                                .and_then(|affix| affix.rfb_ego.as_ref())
                                .map(|ego| ego.source_index),
                            effect,
                        ))
            })
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
    ) {
        if !self
            .world_tick
            .is_multiple_of(EQUIPMENT_CURSE_INTERVAL_TICKS)
        {
            return;
        }
        let cursed_teleport = self.items.iter().any(|item| {
            item.curse.is_some()
                && self.item_has_active_equipped_curse_effect(item, ItemCurseEffectDto::Teleport)
        });
        let intrinsic_teleport = !cursed_teleport
            && self.items.iter().any(|item| {
                item.inscription
                    .as_deref()
                    .is_none_or(|inscription| !inscription.contains('.'))
                    && self
                        .item_has_active_equipped_curse_effect(item, ItemCurseEffectDto::Teleport)
            });
        if (!cursed_teleport && !intrinsic_teleport)
            || self.rng.bounded(RANDOM_TELEPORT_ONE_IN) != 0
        {
            return;
        }

        let candidates = self.random_teleport_candidates(if cursed_teleport { 40 } else { 50 });
        if candidates.is_empty() {
            return;
        }
        let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded equipment teleport candidate index must fit usize");
        events.extend(self.relocate_player(candidates[index], changed));
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
        assert!(rfb_ego_intrinsic_curse_effect(
            Some(11),
            ItemCurseEffectDto::Aggravate
        ));
        assert!(rfb_ego_intrinsic_curse_effect(
            Some(21),
            ItemCurseEffectDto::Aggravate
        ));
        assert!(rfb_ego_intrinsic_curse_effect(
            Some(27),
            ItemCurseEffectDto::Aggravate
        ));
        assert!(rfb_ego_intrinsic_curse_effect(
            Some(15),
            ItemCurseEffectDto::Teleport
        ));
        assert!(!rfb_ego_intrinsic_curse_effect(
            Some(12),
            ItemCurseEffectDto::Aggravate
        ));
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
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new());
        assert_eq!(game.player.position, Position { x: 3, y: 3 });
        assert_eq!(game.rng_draw_counter(), draws_before);

        game.world_tick = 10;
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new());
        assert_ne!(game.player.position, Position { x: 3, y: 3 });

        game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
        game.player.position = Position { x: 3, y: 3 };
        game.rng = RfbRng::seeded(seed);
        game.world_tick = 20;
        let draws_before = game.rng_draw_counter();
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new());
        assert_eq!(game.player.position, Position { x: 3, y: 3 });
        assert_eq!(game.rng_draw_counter(), draws_before);
    }
}
