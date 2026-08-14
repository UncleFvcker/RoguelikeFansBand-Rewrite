// SPDX-License-Identifier: MPL-2.0

use rfb_content::{
    ActorDamageType, ActorResistanceLevel, ChaosPatronDefinition, ChaosPatronRewardKind,
    ContentCatalog, ItemQuality, TechniqueAttribute,
};

use super::*;

pub(super) const CHAOS_GIFT_MUTATION_ID: &str = "rfb.mutation.chaos-gift";
pub(super) const PURPLE_GIFT_MUTATION_ID: &str = "rfb.mutation.purple-gift";
const POLYMORPH_ABILITY_ID: &str = "rfb.ability.mutation.polymorph";
const CHAOS_AFFIX_ID: &str = "demo.affix.chaos";
const PATRON_LOOT_TABLE_ID: &str = "demo.loot-table.base-items";
const ATTRIBUTES: [AttributeKind; 6] = [
    AttributeKind::Strength,
    AttributeKind::Intelligence,
    AttributeKind::Wisdom,
    AttributeKind::Dexterity,
    AttributeKind::Constitution,
    AttributeKind::Charisma,
];
const EXTRA_CHAOS_WEAPON_RESISTANCES: [ActorDamageType; 14] = [
    ActorDamageType::Acid,
    ActorDamageType::Electricity,
    ActorDamageType::Fire,
    ActorDamageType::Cold,
    ActorDamageType::Poison,
    ActorDamageType::Light,
    ActorDamageType::Dark,
    ActorDamageType::Confusion,
    ActorDamageType::Nether,
    ActorDamageType::Nexus,
    ActorDamageType::Sound,
    ActorDamageType::Shards,
    ActorDamageType::Disenchant,
    ActorDamageType::Time,
];

pub(super) fn chaos_patrons(content: &ContentCatalog) -> &[ChaosPatronDefinition] {
    content
        .mutation(CHAOS_GIFT_MUTATION_ID)
        .map_or(&[], |mutation| mutation.chaos_patrons.as_slice())
}

pub(super) fn initial_chaos_patron_id(
    content: &ContentCatalog,
    rng: &mut RfbRng,
) -> Option<String> {
    let patrons = chaos_patrons(content);
    (!patrons.is_empty()).then(|| {
        let index = usize::try_from(rng.bounded(patrons.len() as u64))
            .expect("chaos patron index must fit usize");
        patrons[index].id.clone()
    })
}

impl Game {
    pub(super) fn chaos_patron(&self) -> Option<&ChaosPatronDefinition> {
        let patron_id = self.chaos_patron_id.as_deref()?;
        chaos_patrons(&self.content)
            .iter()
            .find(|patron| patron.id == patron_id)
    }

    pub(super) fn chaos_patron_state_is_valid(&self) -> bool {
        let patrons = chaos_patrons(&self.content);
        match self.chaos_patron_id.as_deref() {
            Some(patron_id) => patrons.iter().any(|patron| patron.id == patron_id),
            None => patrons.is_empty(),
        }
    }

    pub(super) fn process_chaos_patron_level_rewards(
        &mut self,
        events: &mut Vec<DomainEvent>,
        event_cursor: &mut usize,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        while *event_cursor < events.len() {
            let level = match &events[*event_cursor] {
                DomainEvent::PlayerLevelGained {
                    level,
                    reached_new_maximum: true,
                    ..
                } => Some(*level),
                _ => None,
            };
            *event_cursor += 1;
            if let Some(level) = level
                && self
                    .progress
                    .active_mutation_ids
                    .contains(CHAOS_GIFT_MUTATION_ID)
                && !self.player_is_dead()
            {
                self.resolve_chaos_patron_level_reward(level, events, changed, removed_entities)?;
            }
        }
        Ok(())
    }

    fn resolve_chaos_patron_level_reward(
        &mut self,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let patron = self
            .chaos_patron()
            .expect("validated character must have a chaos patron")
            .clone();
        events.push(DomainEvent::ChaosPatronReward {
            patron_id: patron.id.clone(),
            patron_name: patron.name.clone(),
        });
        if self.rng.bounded(6) == 0 {
            self.gain_random_mutation(events);
            return Ok(());
        }

        let nasty_chance = if level == 13 {
            2
        } else if level.is_multiple_of(13) {
            3
        } else if level.is_multiple_of(14) {
            12
        } else {
            6
        };
        let mut reward_index = if self.rng.bounded(nasty_chance) == 0 {
            usize::try_from(self.rng.bounded(20)).expect("reward index must fit usize")
        } else {
            usize::try_from(self.rng.bounded(15) + 5).expect("reward index must fit usize")
        };
        if reward_index < 5 && !level.is_multiple_of(13) {
            while reward_index < 5 {
                let denominator = reward_index + usize::from(level >= 13) + 1;
                if self.rng.bounded(denominator as u64) != 0 {
                    break;
                }
                reward_index += 1;
            }
        }
        let reward = patron.rewards[reward_index];
        self.apply_chaos_patron_reward(&patron, reward, level, events, changed, removed_entities)
    }

    fn apply_chaos_patron_reward(
        &mut self,
        patron: &ChaosPatronDefinition,
        reward: ChaosPatronRewardKind,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match reward {
            ChaosPatronRewardKind::PolymorphSelf => {
                let ability = self
                    .content
                    .ability(POLYMORPH_ABILITY_ID)
                    .expect("polymorph ability must remain available")
                    .clone();
                self.resolve_player_polymorph_self_effect(&ability, events);
            }
            ChaosPatronRewardKind::GainExperience => {
                let amount = self.progress.experience / 2 + 10;
                self.apply_unscaled_player_experience(amount.min(100_000), events);
            }
            ChaosPatronRewardKind::LoseExperience => {
                self.apply_player_experience_drain(
                    self.progress.experience / 6,
                    &patron.id,
                    events,
                );
            }
            ChaosPatronRewardKind::GoodObject => {
                self.drop_patron_loot(&patron.id, 1, ItemQuality::Fine, changed)?;
            }
            ChaosPatronRewardKind::GreatObject => {
                self.drop_patron_loot(&patron.id, 1, ItemQuality::Exceptional, changed)?;
            }
            ChaosPatronRewardKind::ChaosWeapon => {
                self.drop_chaos_weapon(level, changed)?;
            }
            ChaosPatronRewardKind::GoodObjects => {
                let count = u16::try_from(self.rng.bounded(2) + 2).expect("count must fit u16");
                self.drop_patron_loot(&patron.id, count, ItemQuality::Fine, changed)?;
            }
            ChaosPatronRewardKind::GreatObjects => {
                let count = u16::try_from(self.rng.bounded(2) + 2).expect("count must fit u16");
                self.drop_patron_loot(&patron.id, count, ItemQuality::Exceptional, changed)?;
            }
            ChaosPatronRewardKind::TyCurse => self.apply_nonlethal_ty_curse(level, &patron.id),
            ChaosPatronRewardKind::SummonMonsters => {
                let count = u8::try_from(self.rng.bounded(5) + 2).expect("count must fit u8");
                self.summon_patron_creatures(
                    &patron.id,
                    "any-monster",
                    level,
                    count,
                    true,
                    true,
                    events,
                    changed,
                );
            }
            ChaosPatronRewardKind::HighSummon => {
                self.high_patron_summon(&patron.id, level, events, changed);
            }
            ChaosPatronRewardKind::Havoc => {
                self.apply_chaos_havoc(&patron.id, level, events, changed, removed_entities)?;
            }
            ChaosPatronRewardKind::GainAttribute => {
                let attribute = self.patron_attribute(patron);
                self.increase_patron_attribute(attribute);
            }
            ChaosPatronRewardKind::LoseAttribute => {
                let attribute = self.patron_attribute(patron);
                self.drain_patron_attribute(attribute, 10, true);
            }
            ChaosPatronRewardKind::RuinAttributes => self.ruin_patron_attributes(true),
            ChaosPatronRewardKind::AugmentAttributes => {
                for attribute in ATTRIBUTES {
                    self.increase_patron_attribute(attribute);
                }
            }
            ChaosPatronRewardKind::PolymorphWounds => {
                self.resolve_polymorph_wounds(&patron.id);
            }
            ChaosPatronRewardKind::FullHealing => self.apply_patron_full_healing(events),
            ChaosPatronRewardKind::HurtBadly => {
                let damage = i32::from(level)
                    .saturating_mul(4)
                    .min(self.effective_player_max_hp().saturating_mul(2) / 5);
                self.damage_patron_area(
                    &patron.id,
                    ActorDamageType::Disintegrate,
                    damage,
                    4,
                    events,
                    changed,
                    removed_entities,
                )?;
                let damage = resolve_damage(
                    DamagePacket::new(damage, DamageType::Disintegrate),
                    ResistanceLevel::Normal,
                );
                self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            }
            ChaosPatronRewardKind::CurseWeapon => {
                self.curse_equipped_item(CurseEquippedItemRequest::new(
                    EquippedItemCurseTarget::Weapon,
                ));
            }
            ChaosPatronRewardKind::CurseArmor => {
                self.curse_equipped_item(CurseEquippedItemRequest::new(
                    EquippedItemCurseTarget::Armor,
                ));
            }
            ChaosPatronRewardKind::Anger => {
                self.apply_patron_anger(&patron.id, level, events, changed, removed_entities);
            }
            ChaosPatronRewardKind::Wrath => {
                self.apply_patron_wrath(&patron.id, level, events, changed);
            }
            ChaosPatronRewardKind::Destruction => {
                self.apply_patron_destruction(events, changed, removed_entities);
            }
            ChaosPatronRewardKind::Genocide => {
                self.apply_patron_genocide(false, changed, removed_entities);
            }
            ChaosPatronRewardKind::MassGenocide => {
                self.apply_patron_genocide(true, changed, removed_entities);
            }
            ChaosPatronRewardKind::DispelMonsters => {
                self.damage_patron_area(
                    &patron.id,
                    ActorDamageType::Physical,
                    i32::from(level).saturating_mul(4),
                    u8::MAX,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            ChaosPatronRewardKind::Ignore => {}
            ChaosPatronRewardKind::UndeadServant => self.summon_patron_creatures(
                &patron.id, "undead", level, 1, false, false, events, changed,
            ),
            ChaosPatronRewardKind::DemonServant => self.summon_patron_creatures(
                &patron.id, "demon", level, 1, false, false, events, changed,
            ),
            ChaosPatronRewardKind::MonsterServant => self.summon_patron_creatures(
                &patron.id,
                "any-monster",
                level,
                1,
                false,
                false,
                events,
                changed,
            ),
        }
        Ok(())
    }

    fn patron_attribute(&mut self, patron: &ChaosPatronDefinition) -> AttributeKind {
        if self.rng.bounded(3) == 0
            && let Some(attribute) = patron.favored_attribute
        {
            return attribute_kind(attribute);
        }
        ATTRIBUTES[usize::try_from(self.rng.bounded(6)).expect("attribute index must fit usize")]
    }

    fn increase_patron_attribute(&mut self, attribute: AttributeKind) {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let threshold = self
            .player_luck_bias()
            .attribute_increase_threshold(self.progress.maximum_attributes.value(attribute));
        let victorious = self.victory_level_cap_unlocked();
        let outcome = apply_permanent_attribute_increase(
            &mut self.progress,
            attribute,
            victorious,
            threshold,
            &mut self.rng,
        );
        if outcome.changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
    }

    pub(super) fn drain_patron_attribute(
        &mut self,
        attribute: AttributeKind,
        amount: u8,
        permanent: bool,
    ) {
        if self.player_sustains_attribute(attribute) {
            return;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let changed = if permanent {
            self.progress
                .permanently_drain_attribute(attribute, amount, &mut self.rng)
        } else {
            self.progress
                .drain_attribute_by(attribute, amount, &mut self.rng)
        };
        if changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
    }

    fn ruin_patron_attributes(&mut self, permanent: bool) {
        for attribute in ATTRIBUTES {
            let amount = u8::try_from(self.rng.bounded(15) + 11).expect("drain must fit u8");
            self.drain_patron_attribute(attribute, amount, permanent);
        }
    }

    fn apply_patron_full_healing(&mut self, events: &mut Vec<DomainEvent>) {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        apply_experience_restoration(&mut self.progress);
        apply_life_force_restoration(
            &mut self.progress,
            LifeForceRestorationRequest::at_least(1_000),
        );
        for attribute in ATTRIBUTES {
            apply_attribute_restoration(&mut self.progress, attribute);
        }
        self.player.statuses.retain(|status| {
            !matches!(
                status.kind_id.as_str(),
                STATUS_POISON
                    | STATUS_BLINDNESS
                    | STATUS_CONFUSION
                    | STATUS_HALLUCINATION
                    | STATUS_STUN
                    | STATUS_BLEEDING
            )
        });
        self.apply_player_experience(0, events);
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        self.player.hp = self.effective_player_max_hp();
    }

    pub(super) fn apply_nonlethal_ty_curse(&mut self, level: u16, source_id: &str) {
        let maximum_damage = self.player.hp.saturating_sub(1).max(0);
        let damage = i32::from(level)
            .saturating_add(
                i32::try_from(self.rng.bounded(u64::from(level.max(1))) + 1)
                    .expect("curse damage must fit i32"),
            )
            .min(maximum_damage);
        let damage = resolve_damage(
            DamagePacket::new(damage, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        for status_kind_id in [STATUS_CONFUSION, STATUS_STUN] {
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: status_kind_id.to_owned(),
                        intensity: 1,
                        remaining_ticks: u32::from(level.max(5)),
                        source_id: Some(source_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
        }
    }

    fn apply_patron_anger(
        &mut self,
        patron_id: &str,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        match self.rng.bounded(4) {
            0 => self.apply_nonlethal_ty_curse(level, patron_id),
            1 => self.high_patron_summon(patron_id, level, events, changed),
            2 => {
                let target = if self.rng.bounded(2) == 0 {
                    EquippedItemCurseTarget::Weapon
                } else {
                    EquippedItemCurseTarget::Armor
                };
                self.curse_equipped_item(CurseEquippedItemRequest::new(target));
            }
            _ => self.ruin_patron_attributes(true),
        }
        let _ = removed_entities;
    }

    fn apply_patron_wrath(
        &mut self,
        patron_id: &str,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let damage = self
            .effective_player_max_hp()
            .saturating_mul(2)
            .saturating_div(3)
            .min(i32::from(level).saturating_mul(4));
        let damage = resolve_damage(
            DamagePacket::new(damage, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        if self.player_is_dead() {
            return;
        }
        self.ruin_patron_attributes(false);
        self.high_patron_summon(patron_id, level, events, changed);
        self.apply_nonlethal_ty_curse(level, patron_id);
        if self.rng.bounded(2) == 0 {
            self.curse_equipped_item(CurseEquippedItemRequest::new(
                EquippedItemCurseTarget::Weapon,
            ));
        }
        if self.rng.bounded(2) == 0 {
            self.curse_equipped_item(CurseEquippedItemRequest::new(
                EquippedItemCurseTarget::Armor,
            ));
        }
    }

    fn apply_chaos_havoc(
        &mut self,
        patron_id: &str,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match self.rng.bounded(3) {
            0 => self.damage_patron_area(
                patron_id,
                ActorDamageType::Chaos,
                i32::from(level).saturating_mul(5),
                8,
                events,
                changed,
                removed_entities,
            )?,
            1 => self.high_patron_summon(patron_id, level, events, changed),
            _ => self.apply_patron_destruction(events, changed, removed_entities),
        }
        Ok(())
    }

    fn apply_patron_destruction(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let plan = self.plan_area_destruction(
            13,
            17,
            "demo.terrain.floor",
            "demo.terrain.wall",
            "demo.terrain.quartz-vein",
            "demo.terrain.magma-vein",
        );
        self.apply_area_destruction_plan(plan, events, changed, removed_entities);
    }

    fn apply_patron_genocide(
        &mut self,
        mass: bool,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let candidate_ids = if mass {
            self.entities
                .iter()
                .filter(|entity| {
                    entity.hp > 0 && rfb_distance(self.player.position, entity.position) <= 20
                })
                .map(|entity| entity.id.clone())
                .collect()
        } else {
            let kinds = self
                .entities
                .iter()
                .filter(|entity| entity.hp > 0)
                .map(|entity| entity.kind_id.clone())
                .collect::<BTreeSet<_>>();
            if kinds.is_empty() {
                Vec::new()
            } else {
                let selected = kinds
                    .iter()
                    .nth(
                        usize::try_from(self.rng.bounded(kinds.len() as u64))
                            .expect("genocide kind index must fit usize"),
                    )
                    .expect("nonempty kind set must contain selected index");
                self.entities
                    .iter()
                    .filter(|entity| entity.hp > 0 && entity.kind_id == *selected)
                    .map(|entity| entity.id.clone())
                    .collect()
            }
        };
        self.resolve_genocide_candidates(
            candidate_ids,
            if mass {
                AbilityGenocideScopeDefinition::Nearby
            } else {
                AbilityGenocideScopeDefinition::Glyph
            },
            1_000,
            false,
            changed,
            removed_entities,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn damage_patron_area(
        &mut self,
        patron_id: &str,
        damage_type: ActorDamageType,
        damage: i32,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && (radius == u8::MAX
                        || rfb_distance(self.player.position, entity.position) <= u32::from(radius))
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let position = self.entities[index].position;
            self.resolve_ability_damage_to_entity(
                index,
                patron_id,
                DamageType::from(damage_type),
                damage,
                ProjectileTrace {
                    origin: self.player.position,
                    impact: position,
                    landing: position,
                    traversed: vec![position],
                },
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn summon_patron_creatures(
        &mut self,
        patron_id: &str,
        category: &str,
        maximum_level: u16,
        count: u8,
        hostile: bool,
        allow_unique: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let candidates = self.summon_category_candidate_kind_ids(
            category,
            None,
            maximum_level.max(1),
            allow_unique,
        );
        let positions = self
            .open_positions_around_for_actor_kinds(self.player.position, 3, &candidates)
            .into_iter()
            .take(usize::from(count))
            .collect::<Vec<_>>();
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: patron_id,
                owner_id: &owner_id,
                category,
                count_dice: 0,
                count_sides: 0,
                count_bonus: count,
                maximum_count: None,
                hostile,
                group_chance_percent: 0,
                group_count_dice: 0,
                group_count_sides: 0,
                group_count_bonus: count,
                duration_turns: 0,
            },
            candidates,
            positions,
            changed,
        );
        events.push(DomainEvent::AbilitySummoned {
            ability_id: patron_id.to_owned(),
            resolution,
        });
    }

    fn high_patron_summon(
        &mut self,
        patron_id: &str,
        level: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let count = u8::try_from(self.rng.bounded(3) + 2).expect("count must fit u8");
        self.summon_patron_creatures(
            patron_id,
            "any-monster",
            level.saturating_add(25),
            count,
            true,
            true,
            events,
            changed,
        );
    }

    fn drop_patron_loot(
        &mut self,
        patron_id: &str,
        count: u16,
        quality: ItemQuality,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let generated = self.generate_loot_instances_internal(
            &LootContext {
                table_id: PATRON_LOOT_TABLE_ID.to_owned(),
                floor_id: self.current_floor_id.clone(),
                depth: self.progress.level.min(9),
                source: LootSource::ItemUse {
                    item_id: patron_id.to_owned(),
                },
            },
            ItemLocation::Ground(self.player.position),
            false,
            Some(count),
            quality.into(),
        )?;
        self.items.extend(generated);
        changed.insert(self.player.position);
        Ok(())
    }

    fn drop_chaos_weapon(
        &mut self,
        level: u16,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let roll = u16::try_from(self.rng.bounded(u64::from(level.max(1))) + 1)
            .expect("weapon level roll must fit u16");
        let kind_id = chaos_weapon_kind_id(roll);
        let dungeon_depth = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .map_or(1, |floor| floor.depth.max(1));
        let enchantment = |rng: &mut RfbRng| {
            3 + i16::try_from(rng.bounded(u64::from(dungeon_depth)) + 1)
                .expect("enchantment roll must fit i16")
                % 10
        };
        let extra_resistance = EXTRA_CHAOS_WEAPON_RESISTANCES[usize::try_from(
            self.rng
                .bounded(EXTRA_CHAOS_WEAPON_RESISTANCES.len() as u64),
        )
        .expect("resistance index must fit usize")];
        let mut properties = AffixPropertyBundleDefinition::default();
        properties
            .resistances
            .insert(extra_resistance, ActorResistanceLevel::Resistant);
        let id = self.allocate_item_instance_id()?;
        let (activation, charges) =
            initial_item_runtime_state(&self.content, &mut self.rng, kind_id, 1);
        self.items.push(ItemInstance {
            id,
            kind_id: kind_id.to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Exceptional,
            affix_ids: vec![CHAOS_AFFIX_ID.to_owned()],
            rolled_affixes: vec![RolledAffixState {
                affix_id: CHAOS_AFFIX_ID.to_owned(),
                properties,
            }],
            enchantments: ItemEnchantmentsDto {
                to_hit: enchantment(&mut self.rng),
                to_damage: enchantment(&mut self.rng),
                to_armor: 0,
            },
            curse: initial_item_curse(&self.content, kind_id),
            permanent_destruction_immunities: Default::default(),
            activation,
            charges,
            fuel: initial_item_fuel(&self.content, kind_id),
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Ground(self.player.position),
        });
        changed.insert(self.player.position);
        Ok(())
    }
}

const fn attribute_kind(attribute: TechniqueAttribute) -> AttributeKind {
    match attribute {
        TechniqueAttribute::Strength => AttributeKind::Strength,
        TechniqueAttribute::Intelligence => AttributeKind::Intelligence,
        TechniqueAttribute::Wisdom => AttributeKind::Wisdom,
        TechniqueAttribute::Dexterity => AttributeKind::Dexterity,
        TechniqueAttribute::Constitution => AttributeKind::Constitution,
        TechniqueAttribute::Charisma => AttributeKind::Charisma,
    }
}

pub(super) const fn chaos_weapon_kind_id(level_roll: u16) -> &'static str {
    match level_roll {
        0 | 1 => "demo.item.dagger",
        2 | 3 => "demo.item.main-gauche",
        4 => "demo.item.tanto",
        5 | 6 => "demo.item.rapier",
        7 | 8 => "demo.item.small-sword",
        9 | 10 => "demo.item.basillard",
        11..=13 => "demo.item.short-sword",
        14 | 15 => "demo.item.sabre",
        16 | 17 => "demo.item.cutlass",
        18 => "demo.item.wakizashi",
        19 => "demo.item.khopesh",
        20 => "demo.item.tulwar",
        21 => "demo.item.broad-sword",
        22 | 23 => "demo.item.long-sword",
        24 | 25 => "demo.item.scimitar",
        26 => "demo.item.ninjato",
        27 => "demo.item.katana",
        28 | 29 => "demo.item.bastard-sword",
        30 => "demo.item.falchion",
        31 => "demo.item.claymore",
        32 => "demo.item.espadon",
        33 => "demo.item.two-handed-sword",
        34 => "demo.item.flamberge",
        35 => "demo.item.no-dachi",
        36 => "demo.item.executioners-sword",
        37 => "demo.item.zweihander",
        38 => "demo.item.falcon-sword",
        _ => "demo.item.blade-of-chaos",
    }
}
