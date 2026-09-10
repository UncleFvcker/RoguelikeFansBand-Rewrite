// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378: gf.c mental projections, shared by their actual callers.

use super::player_combat::monster_stun_amount;
use super::*;
use rfb_protocol::AbilityControlOutcomeDto;

const CHARM_ATTRIBUTE_ADJUSTMENT: [i32; 38] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 5, 6, 6, 7, 7,
    8, 8, 8, 9, 9, 9,
];

pub(super) struct PsychicDamage {
    pub damage: i32,
    statuses: Vec<(&'static str, i32)>,
    teleport: bool,
}

impl Game {
    pub(super) fn make_player_pet(&mut self, index: usize) {
        let target_id = self.entities[index].id.clone();
        if let Some(pack) = self.entities[index].pack.clone() {
            if pack.role == rfb_protocol::MonsterPackRoleDto::Leader || pack.leader_id == target_id
            {
                for entity in &mut self.entities {
                    if entity
                        .pack
                        .as_ref()
                        .is_some_and(|value| value.id == pack.id)
                    {
                        entity.pack = None;
                    }
                }
            } else {
                self.entities[index].pack = None;
            }
        }
        self.entities[index].controller_id = Some(self.player.id.clone());
        self.entities[index].friendly = false;
    }

    pub(super) fn resolve_psychic_charm(
        &mut self,
        index: usize,
        effect_index: u8,
        power: u16,
    ) -> AbilityEffectResolutionDto {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("charm target must exist")
            .clone();
        let has = |tag: &str| definition.tags.iter().any(|value| value == tag);
        let questor = has("questor")
            || definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.task_id.is_some());
        let mut power = i32::from(power)
            + CHARM_ATTRIBUTE_ADJUSTMENT[usize::from(
                self.effective_player_attributes()
                    .index(AttributeKind::Charisma)
                    .min(37),
            )]
            - 1
            + i32::from(self.virtue_current(VirtueKindDto::Harmony)) / 10
            - i32::from(self.virtue_current(VirtueKindDto::Individualism)) / 20;
        if has("unique") || has("unique2") || questor {
            power = power * 18 / 25;
        }
        let power = power.max(1) as u16;
        let difficulty = if has("guardian") || questor {
            50
        } else if has("unique") {
            25
        } else {
            0
        };
        let level = definition.level.max(difficulty);
        let was_friend = self.actor_is_friendly(&self.entities[index]);
        let (roll, outcome) = if self.entity_is_player_aligned(index) {
            (None, AbilityControlOutcomeDto::AlreadyControlled)
        } else {
            let roll = (self.rng.bounded(u64::from(power)) + 1) as u16;
            let outcome =
                if has("resist-all") || has("no-pet") || self.entities[index].no_pet || questor {
                    AbilityControlOutcomeDto::Ineligible
                } else if level > u32::from(roll) || self.player_has_equipped_aggravation() {
                    let no_pet = self.rng.bounded(5) == 0;
                    if no_pet && !was_friend {
                        self.entities[index].no_pet = true;
                    }
                    AbilityControlOutcomeDto::Resisted
                } else {
                    let pet = difficulty == 0
                        || u32::from(power) > level * 2
                        || self.rng.bounded(10) == 0;
                    if pet {
                        self.make_player_pet(index);
                    } else {
                        self.entities[index].friendly = true;
                    }
                    if pet || !was_friend {
                        self.add_virtue(VirtueKindDto::Individualism, -1);
                        if has("animal") {
                            self.add_virtue(VirtueKindDto::Nature, 1);
                        }
                    }
                    if pet {
                        AbilityControlOutcomeDto::Controlled
                    } else {
                        AbilityControlOutcomeDto::Friendly
                    }
                };
            (Some(roll), outcome)
        };
        AbilityEffectResolutionDto::Control {
            effect_index,
            category: "any-monster".to_owned(),
            power,
            target_entity_id: self.entities[index].id.clone(),
            target_kind_id: definition.id,
            target_level: level,
            roll,
            outcome,
        }
    }

    pub(super) fn resolve_psychic_domination(
        &mut self,
        index: usize,
        source: &str,
        power: u16,
    ) -> AbilityEffectResolutionDto {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("domination target must exist")
            .clone();
        let has = |tag: &str| definition.tags.iter().any(|value| value == tag);
        let questor = has("questor")
            || definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.task_id.is_some());
        let outcome = if self.entity_is_player_side(index) {
            AbilityControlOutcomeDto::AlreadyControlled
        } else if has("resist-all") {
            AbilityControlOutcomeDto::Ineligible
        } else {
            let target_roll = self.rng.bounded(u64::from(definition.level).max(1)) + 1;
            let power_roll = self.rng.bounded(u64::from(power).max(1)) + 1;
            if target_roll > power_roll {
                if (has("demon") || has("undead"))
                    && definition.level > u32::from(self.progress.level / 2)
                    && self.rng.bounded(2) == 0
                    && self.rng.bounded(100 + u64::from(definition.level / 2))
                        >= self.player_derived_stats().saving_throw_skill.value.max(0) as u64
                {
                    match self.rng.bounded(4) {
                        0 => self.apply_player_mental_status(
                            STATUS_STUN,
                            i32::from(power / 2),
                            source,
                        ),
                        1 => self.apply_player_mental_status(
                            STATUS_CONFUSION,
                            i32::from(power / 2),
                            source,
                        ),
                        _ if !self.actor_has_status_immunity(index, STATUS_FEAR) => {
                            self.apply_player_mental_status(STATUS_FEAR, i32::from(power), source)
                        }
                        _ => {}
                    }
                }
                AbilityControlOutcomeDto::Resisted
            } else if !has("unique")
                && !questor
                && power > 29
                && self.rng.bounded(100) + 1 < u64::from(power)
            {
                self.make_player_pet(index);
                AbilityControlOutcomeDto::Controlled
            } else {
                let (kind, amount) = match self.rng.bounded(4) {
                    0 => (STATUS_STUN, i32::from(power / 2)),
                    1 if !has("unique") => (STATUS_CONFUSION, i32::from(power / 2)),
                    1 => (STATUS_CONFUSION, 0),
                    _ => (STATUS_FEAR, i32::from(power)),
                };
                if amount > 0
                    && (!matches!(kind, STATUS_STUN | STATUS_CONFUSION)
                        || !self.actor_has_status_immunity(index, kind))
                {
                    self.apply_monster_mental_status(index, kind, amount as u32, source);
                    AbilityControlOutcomeDto::Affected
                } else {
                    AbilityControlOutcomeDto::Resisted
                }
            }
        };
        AbilityEffectResolutionDto::Control {
            effect_index: 0,
            category: "domination".to_owned(),
            power,
            target_entity_id: self.entities[index].id.clone(),
            target_kind_id: definition.id,
            target_level: definition.level,
            roll: None,
            outcome,
        }
    }

    pub(super) fn prepare_psychic_damage(
        &mut self,
        index: usize,
        source: &str,
        kind: DamageType,
        amount: i32,
        events: &mut Vec<DomainEvent>,
    ) -> PsychicDamage {
        let mut result = PsychicDamage {
            damage: amount,
            statuses: Vec::new(),
            teleport: false,
        };
        if !matches!(
            kind,
            DamageType::Psi
                | DamageType::PsiDrain
                | DamageType::PsiStorm
                | DamageType::Telekinesis
                | DamageType::PsySpear
        ) {
            return result;
        }
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("psychic target must exist")
            .clone();
        let has = |tag: &str| definition.tags.iter().any(|value| value == tag);
        let level = definition.level;
        if has("resist-all")
            || (kind != DamageType::Telekinesis
                && kind != DamageType::PsySpear
                && has("empty-mind"))
            || (kind == DamageType::Psi
                && !visibility::has_line_of_sight(
                    self,
                    self.entities[index].position,
                    self.player.position,
                ))
        {
            result.damage = 0;
            return result;
        }
        if kind == DamageType::PsySpear {
            return result;
        }
        if matches!(kind, DamageType::Psi | DamageType::PsiDrain) {
            let resisted = has("stupid")
                || has("weird-mind")
                || has("animal")
                || u64::from(level) > self.rng.bounded((amount.max(1) as u64) * 3) + 1;
            if resisted {
                result.damage /= 3;
                if (has("demon") || has("undead"))
                    && level > u32::from(self.progress.level / 2)
                    && self.rng.bounded(2) == 0
                {
                    let saved = self.rng.bounded(100 + u64::from(level / 2))
                        < self.player_derived_stats().saving_throw_skill.value.max(0) as u64;
                    if !saved {
                        if kind == DamageType::PsiDrain && self.player_mana_can_be_drained() {
                            let drain = self.roll_psychic_dice(5, result.damage) / 2;
                            if let Some(mana) = self.resources.get_mut("demo.resource.mana") {
                                mana.current = mana.current.saturating_sub(drain as u32);
                            }
                        }
                        self.apply_psychic_backlash_damage(
                            source,
                            &definition.id,
                            result.damage,
                            kind,
                            events,
                        );
                        if kind == DamageType::Psi && self.rng.bounded(4) == 0 {
                            match self.rng.bounded(4) {
                                0 => {
                                    let amount = 3 + self.roll_psychic_dice(1, result.damage);
                                    self.apply_player_mental_status(
                                        STATUS_CONFUSION,
                                        amount,
                                        source,
                                    );
                                }
                                1 => {
                                    let amount = self.roll_psychic_dice(1, result.damage);
                                    self.apply_player_mental_status(STATUS_STUN, amount, source);
                                }
                                2 if !self.actor_has_status_immunity(index, STATUS_FEAR) => {
                                    self.apply_player_mental_status(STATUS_FEAR, 50, source)
                                }
                                3 => {
                                    let saved = self.psychic_free_action_save(level);
                                    if !saved {
                                        let amount = self.roll_psychic_dice(1, result.damage);
                                        self.apply_player_mental_status(
                                            STATUS_PARALYSIS,
                                            amount,
                                            source,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    result.damage = 0;
                }
            } else if kind == DamageType::PsiDrain && result.damage > 0 {
                let amount = (self.roll_psychic_dice(5, result.damage) / 4) as u32;
                if let Some(mana) = self.resources.get_mut("demo.resource.mana") {
                    let before = mana.current;
                    mana.current = mana.current.saturating_add(amount).min(mana.maximum);
                    events.push(DomainEvent::ResourceRecovered {
                        resolution: ResourceRecoveryResolutionDto {
                            resource_id: "demo.resource.mana".to_owned(),
                            before,
                            after: mana.current,
                            recovered: mana.current - before,
                        },
                    });
                }
            }
            if kind == DamageType::Psi && result.damage > 0 && self.rng.bounded(4) == 0 {
                let kind = [STATUS_CONFUSION, STATUS_STUN, STATUS_FEAR, STATUS_SLEEP]
                    [self.rng.bounded(4) as usize];
                let duration = 3 + self.roll_psychic_dice(1, result.damage);
                result.statuses.push((kind, duration));
            }
            return result;
        }
        if kind == DamageType::PsiStorm && (has("stupid") || has("weird-mind")) {
            result.damage /= 3;
            return result;
        }
        if self.rng.bounded(4) == 0
            && self.riding_actor_id.as_deref() != Some(&self.entities[index].id)
        {
            result.teleport = !has("guardian");
        }
        if kind == DamageType::Telekinesis || self.rng.bounded(2) == 0 {
            let stun = monster_stun_amount(if kind == DamageType::PsiStorm {
                amount * 2 / 3
            } else {
                amount
            });
            let multiplier = if has("unique") { 2 } else { 1 };
            if u64::from(level) * multiplier <= 5 + self.rng.bounded(amount.max(1) as u64) + 1 {
                let resists_stun = kind == DamageType::Telekinesis
                    && [DamageType::Sound, DamageType::Force]
                        .into_iter()
                        .any(|kind| {
                            matches!(
                                self.entities[index].resistances.level(kind),
                                ResistanceLevel::Resistant
                                    | ResistanceLevel::Strong
                                    | ResistanceLevel::Immune
                            )
                        });
                if !resists_stun {
                    result.statuses.push((STATUS_STUN, stun));
                }
            }
        }
        if kind == DamageType::PsiStorm && self.rng.bounded(4) == 0 {
            match self.rng.bounded(3) {
                0 => {
                    let duration = if self.actor_has_status_immunity(index, STATUS_CONFUSION) {
                        2
                    } else {
                        3 + self.roll_psychic_dice(1, (amount + 9) / 10)
                    };
                    result.statuses.push((STATUS_CONFUSION, duration));
                }
                branch => {
                    let kind = if branch == 1 {
                        STATUS_FEAR
                    } else {
                        STATUS_SLEEP
                    };
                    if !self.actor_has_status_immunity(index, kind) || self.rng.bounded(2) != 0 {
                        let duration = 3 + self.roll_psychic_dice(1, amount);
                        result.statuses.push((kind, duration));
                    }
                }
            }
        }
        result
    }

    pub(super) fn apply_psychic_damage_riders(
        &mut self,
        index: usize,
        source: &str,
        result: PsychicDamage,
        changed: &mut BTreeSet<Position>,
    ) {
        for (kind, amount) in result.statuses {
            if matches!(kind, STATUS_STUN | STATUS_CONFUSION)
                && self.actor_has_status_immunity(index, kind)
            {
                continue;
            }
            self.apply_monster_mental_status(index, kind, amount as u32, source);
        }
        if result.teleport {
            self.passive_teleport_actor(index, 7, changed);
        }
    }

    pub(super) fn apply_monster_mental_status(
        &mut self,
        index: usize,
        kind: &str,
        mut amount: u32,
        source: &str,
    ) {
        let current = self.entities[index]
            .statuses
            .iter()
            .find(|status| status.kind_id == kind)
            .map_or(0, |status| status.remaining_ticks);
        if current > 0 {
            if kind == STATUS_CONFUSION {
                amount /= 2;
            } else if kind == STATUS_STUN {
                amount = (amount / (1 + current / 20)).max(1);
            }
        }
        let mut status = monster_combat::melee_status(kind, amount, source);
        if kind == STATUS_SLEEP {
            status.stacking = StatusStacking::Replace;
        }
        apply_status(&mut self.entities[index].statuses, status);
    }

    pub(super) fn apply_player_mental_status(&mut self, kind: &str, amount: i32, source: &str) {
        if amount <= 0 {
            return;
        }
        if self.player_is_berserker() && self.player_status_immunities().contains(kind) {
            return;
        }
        let mut status = monster_combat::melee_status(kind, amount as u32, source);
        if kind == STATUS_STUN {
            status.status.intensity = self
                .player
                .statuses
                .iter()
                .filter(|value| value.kind_id == kind)
                .map(|value| value.intensity)
                .max()
                .unwrap_or(0)
                .saturating_add(amount as u16);
        }
        apply_status(&mut self.player.statuses, status);
    }

    pub(super) fn roll_psychic_dice(&mut self, dice: u16, sides: i32) -> i32 {
        (0..dice)
            .map(|_| (self.rng.bounded(sides.max(1) as u64) + 1) as i32)
            .sum()
    }

    fn psychic_free_action_save(&mut self, level: u32) -> bool {
        // berserker.c grants three FREE_ACT sources, which always save.
        if self.player_is_berserker() {
            return true;
        }
        let mut count = self
            .player
            .statuses
            .iter()
            .filter(|status| status.granted_status_immunities.contains(STATUS_PARALYSIS))
            .count();
        count += usize::from(self.character_definitions().is_some_and(|(_, race, _, _)| {
            race.status_immunities
                .iter()
                .any(|kind| kind == STATUS_PARALYSIS)
        }));
        count += self
            .content
            .mutations()
            .filter(|mutation| {
                self.progress.active_mutation_ids.contains(&mutation.id)
                    && mutation
                        .status_immunities
                        .iter()
                        .any(|kind| kind == STATUS_PARALYSIS)
            })
            .count();
        // Each equipped object contributes one FREE_ACT, even when several of its properties grant it.
        count += self.items.iter().filter(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))).filter(|item| {
            self.content.item(&item.kind_id).is_some_and(|definition| definition.status_immunities.iter().any(|kind| kind == STATUS_PARALYSIS))
                || item.affix_ids.iter().any(|id| self.content.affix(id).is_some_and(|affix| affix.status_immunities.iter().any(|kind| kind == STATUS_PARALYSIS)))
                || item.intrinsic_properties.status_immunities.iter().any(|kind| kind == STATUS_PARALYSIS)
                || item.rolled_affixes.iter().any(|affix| affix.properties.status_immunities.iter().any(|kind| kind == STATUS_PARALYSIS))
        }).count();
        if count >= 3 || (count == 2 && level < 42) {
            return true;
        }
        let saving = self.player_derived_stats().saving_throw_skill.value.max(0) as u64;
        (0..count).any(|_| self.rng.bounded(100 + u64::from(level / 2)) < saving)
    }

    fn player_mana_can_be_drained(&self) -> bool {
        !self
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.strong-mind")
    }

    pub(super) fn actor_spell_damage_percent(
        &mut self,
        index: usize,
        kind: DamageType,
        damage: i32,
    ) -> u8 {
        self.actor_incoming_damage_percent(index, damage, kind == DamageType::PsySpear)
    }

    pub(super) fn actor_incoming_damage_percent(
        &mut self,
        index: usize,
        damage: i32,
        pierces: bool,
    ) -> u8 {
        let invulnerable = self.entities[index]
            .statuses
            .iter()
            .any(|status| status.kind_id == crate::effect::STATUS_INVULNERABILITY);
        let pierces = pierces || (damage > 0 && invulnerable && self.rng.bounded(13) == 0);
        self.entities[index]
            .statuses
            .iter()
            .filter(|status| !pierces || status.kind_id != crate::effect::STATUS_INVULNERABILITY)
            .map(|status| status.incoming_damage_percent)
            .min()
            .unwrap_or(100)
    }

    pub(super) fn player_spell_damage_percent(&mut self, kind: DamageType, damage: i32) -> u8 {
        let invulnerable = self.player_has_status_kind(crate::effect::STATUS_INVULNERABILITY);
        let pierces = kind == DamageType::PsySpear
            || damage >= 9000
            || (damage > 0 && invulnerable && self.rng.bounded(13) == 0);
        self.player
            .statuses
            .iter()
            .filter(|status| {
                !(pierces && status.kind_id == crate::effect::STATUS_INVULNERABILITY
                    || kind == DamageType::PsySpear && status.kind_id == STATUS_WRAITHFORM)
            })
            .map(|status| status.incoming_damage_percent)
            .min()
            .unwrap_or(100)
    }
}
