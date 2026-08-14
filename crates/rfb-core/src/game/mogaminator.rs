// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::mogaminator::{
    CompiledMogaminator, CompiledMogaminatorRule, MogaminatorAction, MogaminatorDiagnostic,
    MogaminatorDisposition, MogaminatorExpression, MogaminatorFunction, MogaminatorLineKind,
    MogaminatorPredicate, MogaminatorVariable, compile_mogaminator, mogaminator_search_matches,
};
use rfb_content::AmmunitionTypeDefinition;
use rfb_localization::{Locale, Localizer, MogaminatorNames};
use rfb_protocol::{
    AutoGetModeDto, AutoGetTargetDto, LocaleDto, MogaminatorActionDto, MogaminatorDiagnosticDto,
    MogaminatorDispositionDto, MogaminatorDto, MogaminatorItemMatchDto, MogaminatorLineDto,
    MogaminatorLineKindDto, MogaminatorPendingQueryDto, MogaminatorPendingQuerySaveDto,
    MogaminatorSaveDto,
};

const DEFAULT_ZH_CN_SOURCE: &str = include_str!("mogaminator-default-zh-CN.prf");
const DEFAULT_EN_US_SOURCE: &str = include_str!("mogaminator-default-en-US.prf");

pub(super) enum MogaminatorItemResolution {
    PickUp {
        outcome: PickUpOutcome,
    },
    Destroyed {
        kind_id: String,
        quantity: u32,
        rule_line: u32,
        ground: bool,
        captured_actor: Option<CapturedActor>,
    },
    DestroyUnavailable {
        item_id: String,
        reason: String,
        rule_line: u32,
    },
    Inscribed {
        kind_id: String,
        inscription: String,
        rule_line: u32,
    },
    Identified {
        source_kind_id: String,
        target_item_id: String,
        target_kind_id: String,
        full: bool,
        changed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MogaminatorPendingQuery {
    pub(super) item_id: String,
    pub(super) rule_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MogaminatorState {
    pub(super) enabled: bool,
    pub(super) leave_destroyed_items: bool,
    pub(super) auto_get_mode: AutoGetModeDto,
    pub(super) zh_cn_source: String,
    pub(super) en_us_source: String,
    pub(super) pending_query: Option<MogaminatorPendingQuery>,
    pub(super) dismissed_query_item_ids: BTreeSet<String>,
    pub(super) wanted_actor_kind_ids: BTreeSet<String>,
}

impl Default for MogaminatorState {
    fn default() -> Self {
        Self {
            enabled: false,
            leave_destroyed_items: false,
            auto_get_mode: AutoGetModeDto::Off,
            zh_cn_source: DEFAULT_ZH_CN_SOURCE.to_owned(),
            en_us_source: DEFAULT_EN_US_SOURCE.to_owned(),
            pending_query: None,
            dismissed_query_item_ids: BTreeSet::new(),
            wanted_actor_kind_ids: BTreeSet::new(),
        }
    }
}

impl MogaminatorState {
    pub(super) fn from_save(saved: MogaminatorSaveDto) -> Result<Self, Vec<MogaminatorDiagnostic>> {
        compile_mogaminator(&saved.zh_cn_source)?;
        compile_mogaminator(&saved.en_us_source)?;
        Ok(Self {
            enabled: saved.enabled,
            leave_destroyed_items: saved.leave_destroyed_items,
            auto_get_mode: saved.auto_get_mode,
            zh_cn_source: saved.zh_cn_source,
            en_us_source: saved.en_us_source,
            pending_query: saved.pending_query.map(|pending| MogaminatorPendingQuery {
                item_id: pending.item_id,
                rule_line: pending.rule_line,
            }),
            dismissed_query_item_ids: saved.dismissed_query_item_ids.into_iter().collect(),
            wanted_actor_kind_ids: saved.wanted_actor_kind_ids.into_iter().collect(),
        })
    }

    pub(super) fn to_save(&self) -> MogaminatorSaveDto {
        MogaminatorSaveDto {
            enabled: self.enabled,
            leave_destroyed_items: self.leave_destroyed_items,
            auto_get_mode: self.auto_get_mode,
            zh_cn_source: self.zh_cn_source.clone(),
            en_us_source: self.en_us_source.clone(),
            pending_query: self.pending_query.as_ref().map(|pending| {
                MogaminatorPendingQuerySaveDto {
                    item_id: pending.item_id.clone(),
                    rule_line: pending.rule_line,
                }
            }),
            dismissed_query_item_ids: self.dismissed_query_item_ids.iter().cloned().collect(),
            wanted_actor_kind_ids: self.wanted_actor_kind_ids.iter().cloned().collect(),
        }
    }

    fn source(&self, locale: LocaleDto) -> &str {
        match locale {
            LocaleDto::EnUs => &self.en_us_source,
            LocaleDto::ZhCn => &self.zh_cn_source,
        }
    }

    fn set_source(&mut self, locale: LocaleDto, source: String) {
        match locale {
            LocaleDto::EnUs => self.en_us_source = source,
            LocaleDto::ZhCn => self.zh_cn_source = source,
        }
    }

    pub(super) fn for_character(content: &ContentCatalog, seed: u64) -> Self {
        let mut state = Self::default();
        let mut candidates = content
            .actor_definitions()
            .filter(|actor| {
                actor.tags.iter().any(|tag| tag == "unique")
                    && (actor.corpse_item_kind_id.is_some() || actor.remains.is_some())
            })
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>();
        candidates.sort();
        let mut rng = RfbRng::seeded(seed ^ 0x4D4F_4741_5741_4E54);
        let wanted_count = candidates.len().min(20);
        for index in 0..wanted_count {
            let remaining = candidates.len() - index;
            let offset = usize::try_from(rng.bounded(remaining as u64))
                .expect("bounded candidate index must fit usize");
            candidates.swap(index, index + offset);
        }
        state.wanted_actor_kind_ids = candidates.into_iter().take(wanted_count).collect();
        state
    }
}

impl Game {
    pub(super) fn configure_mogaminator(
        &mut self,
        enabled: bool,
        leave_destroyed_items: bool,
        auto_get_mode: AutoGetModeDto,
        locale: LocaleDto,
        source: String,
    ) -> Vec<MogaminatorDiagnostic> {
        match compile_mogaminator(&source) {
            Ok(_) => {
                self.mogaminator.set_source(locale, source);
                self.mogaminator.enabled = enabled;
                self.mogaminator.leave_destroyed_items = leave_destroyed_items;
                self.mogaminator.auto_get_mode = auto_get_mode;
                self.mogaminator.pending_query = None;
                self.mogaminator.dismissed_query_item_ids.clear();
                Vec::new()
            }
            Err(diagnostics) => diagnostics,
        }
    }

    pub(super) fn mogaminator_dto(
        &self,
        diagnostics: Vec<MogaminatorDiagnostic>,
    ) -> MogaminatorDto {
        let source = self.mogaminator.source(self.interface_locale);
        let compiled = compile_mogaminator(source)
            .expect("saved Mogaminator sources are compiled before becoming authoritative");
        let lines = compiled
            .program
            .lines
            .iter()
            .map(|line| {
                let (kind, action, predicate_count, search) = match &line.kind {
                    MogaminatorLineKind::Blank => (MogaminatorLineKindDto::Blank, None, 0, None),
                    MogaminatorLineKind::Comment => {
                        (MogaminatorLineKindDto::Comment, None, 0, None)
                    }
                    MogaminatorLineKind::Condition(_) => {
                        (MogaminatorLineKindDto::Condition, None, 0, None)
                    }
                    MogaminatorLineKind::Rule(rule) => (
                        MogaminatorLineKindDto::Rule,
                        Some(action_dto(rule.action, rule.inscription.clone())),
                        saturating_u32(rule.predicates.len()),
                        (!rule.search.text.is_empty()).then(|| rule.search.text.clone()),
                    ),
                };
                MogaminatorLineDto {
                    line_number: saturating_u32(line.line_number),
                    kind,
                    action,
                    predicate_count,
                    search,
                }
            })
            .collect();
        let matches = self
            .mogaminator
            .enabled
            .then(|| self.mogaminator_matches(&compiled));
        MogaminatorDto {
            enabled: self.mogaminator.enabled,
            leave_destroyed_items: self.mogaminator.leave_destroyed_items,
            auto_get_mode: self.mogaminator.auto_get_mode,
            auto_get_target: self.mogaminator_auto_get_target(),
            locale: self.interface_locale,
            source: source.to_owned(),
            default_source: default_source(self.interface_locale).to_owned(),
            diagnostics: diagnostics.into_iter().map(diagnostic_dto).collect(),
            lines,
            matches: matches.unwrap_or_default(),
            pending_query: self.mogaminator.pending_query.as_ref().and_then(|pending| {
                self.items
                    .iter()
                    .find(|item| item.id == pending.item_id)
                    .map(|item| MogaminatorPendingQueryDto {
                        item_id: item.id.clone(),
                        item_kind_id: item.kind_id.clone(),
                        quantity: item.quantity,
                        rule_line: pending.rule_line,
                    })
            }),
        }
    }

    fn mogaminator_matches(&self, compiled: &CompiledMogaminator) -> Vec<MogaminatorItemMatchDto> {
        let locale = localization_locale(self.interface_locale);
        let Ok(names) = MogaminatorNames::new(locale) else {
            return Vec::new();
        };
        let mut matches = self
            .items
            .iter()
            .filter(|item| self.item_is_known_to_mogaminator_projection(item))
            .filter_map(|item| {
                self.mogaminator_match_for_item(compiled, &names, locale, item)
                    .map(|(line_number, action)| MogaminatorItemMatchDto {
                        item_id: item.id.clone(),
                        line_number: saturating_u32(line_number),
                        action: action_dto(action.0, action.1),
                    })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        matches
    }

    pub(super) fn mogaminator_auto_get_target(&self) -> Option<AutoGetTargetDto> {
        self.mogaminator_auto_get_candidates()
            .into_iter()
            .min_by(|left, right| (left.0, left.1.as_str()).cmp(&(right.0, right.1.as_str())))
            .map(|candidate| AutoGetTargetDto {
                object_id: candidate.1,
                position: candidate.2,
            })
    }

    pub(super) fn mogaminator_auto_get_position(&self, object_id: &str) -> Option<Position> {
        self.mogaminator_auto_get_candidates()
            .into_iter()
            .find(|candidate| candidate.1 == object_id)
            .map(|candidate| candidate.2)
    }

    fn mogaminator_auto_get_candidates(&self) -> Vec<(u32, String, Position)> {
        let mode = self.mogaminator.auto_get_mode;
        if mode == AutoGetModeDto::Off {
            return Vec::new();
        }

        let origin = self.player.position;
        let mut candidates = Vec::new();
        let mut add_candidate = |id: &str, position: Position| {
            let distance = rfb_distance(origin, position);
            let projectable = has_line_of_effect(self, origin, position);
            if (mode == AutoGetModeDto::Wanted && !projectable)
                || (mode == AutoGetModeDto::Ammo && !projectable && distance > 18)
                || (position != origin && self.next_local_travel_direction(position).is_none())
            {
                return;
            }
            candidates.push((distance, id.to_owned(), position));
        };

        match mode {
            AutoGetModeDto::Off => {}
            AutoGetModeDto::Ammo => {
                for item in &self.items {
                    let ItemLocation::Ground(position) = &item.location else {
                        continue;
                    };
                    if self.item_is_discovered(&item.id)
                        && item
                            .inscription
                            .as_deref()
                            .is_some_and(|inscription| inscription.contains("=g"))
                    {
                        add_candidate(&item.id, *position);
                    }
                }
            }
            AutoGetModeDto::Wanted => {
                for pile in &self.gold_piles {
                    if pile.discovered {
                        add_candidate(&pile.id, pile.position);
                    }
                }
                let locale = localization_locale(self.interface_locale);
                let names = MogaminatorNames::new(locale)
                    .expect("bundled Mogaminator matching names must remain available");
                let compiled = compile_mogaminator(self.mogaminator.source(self.interface_locale))
                    .expect("authoritative Mogaminator sources are validated before use");
                for item in &self.items {
                    let ItemLocation::Ground(position) = &item.location else {
                        continue;
                    };
                    if !self.item_is_discovered(&item.id) {
                        continue;
                    }
                    let wanted = self
                        .mogaminator_match_for_item(&compiled, &names, locale, item)
                        .is_some_and(|(_, (action, _))| match action.disposition {
                            MogaminatorDisposition::PickUp => true,
                            MogaminatorDisposition::Destroy => {
                                !self.mogaminator.leave_destroyed_items
                                    && self.can_destroy_item(item).is_ok()
                            }
                            MogaminatorDisposition::Leave | MogaminatorDisposition::Query => false,
                        });
                    if wanted {
                        add_candidate(&item.id, *position);
                    }
                }
            }
        }

        candidates
    }

    pub(super) fn apply_mogaminator_at_player(
        &mut self,
    ) -> Result<Vec<MogaminatorItemResolution>, CoreError> {
        if !self.mogaminator.enabled {
            return Ok(Vec::new());
        }
        let mut item_ids = self
            .items
            .iter()
            .filter(|item| item.location == ItemLocation::Ground(self.player.position))
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        item_ids.sort();
        self.apply_mogaminator_to_items(item_ids, true)
    }

    pub(super) fn apply_mogaminator_to_carried_items(
        &mut self,
        mut item_ids: Vec<String>,
    ) -> Result<Vec<MogaminatorItemResolution>, CoreError> {
        item_ids.sort();
        self.apply_mogaminator_to_items(item_ids, false)
    }

    pub(super) fn apply_mogaminator_to_items(
        &mut self,
        item_ids: Vec<String>,
        allow_pickup: bool,
    ) -> Result<Vec<MogaminatorItemResolution>, CoreError> {
        if !self.mogaminator.enabled || item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let locale = localization_locale(self.interface_locale);
        let names = MogaminatorNames::new(locale)
            .expect("bundled Mogaminator matching names must remain available");
        let compiled = compile_mogaminator(self.mogaminator.source(self.interface_locale))
            .expect("authoritative Mogaminator sources are validated before use");
        let mut outcomes = Vec::new();
        for item_id in item_ids {
            let Some((mut line_number, mut action, mut inscription)) = self
                .items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| self.mogaminator_match_for_item(&compiled, &names, locale, item))
                .map(|(line, (action, inscription))| (saturating_u32(line), action, inscription))
            else {
                continue;
            };
            if action.auto_identify
                && let Some(resolution) = self.mogaminator_auto_identify(&item_id)
            {
                outcomes.push(resolution);
                let Some((new_line, (new_action, new_inscription))) = self
                    .items
                    .iter()
                    .find(|item| item.id == item_id)
                    .and_then(|item| {
                        self.mogaminator_match_for_item(&compiled, &names, locale, item)
                    })
                else {
                    continue;
                };
                line_number = saturating_u32(new_line);
                action = new_action;
                inscription = new_inscription;
            }
            let Some((quantity, kind_id, uninscribed, ground, captured_actor)) = self
                .items
                .iter()
                .find(|item| item.id == item_id)
                .map(|item| {
                    (
                        item.quantity,
                        item.kind_id.clone(),
                        item.inscription.is_none(),
                        matches!(item.location, ItemLocation::Ground(_)),
                        item.captured_actor.clone(),
                    )
                })
            else {
                continue;
            };
            if let Some(inscription) = inscription.filter(|_| uninscribed) {
                self.inscribe_item(&item_id, Some(inscription.clone()))
                    .expect("matched Mogaminator item must remain inscribable");
                outcomes.push(MogaminatorItemResolution::Inscribed {
                    kind_id: kind_id.clone(),
                    inscription,
                    rule_line: line_number,
                });
            }
            match action.disposition {
                MogaminatorDisposition::PickUp if allow_pickup => {
                    outcomes.push(MogaminatorItemResolution::PickUp {
                        outcome: self.pick_up_item_at_player(Some(&item_id))?,
                    });
                }
                MogaminatorDisposition::Destroy if !self.mogaminator.leave_destroyed_items => {
                    match self.destroy_item(&item_id, quantity) {
                        Ok(outcome) => outcomes.push(MogaminatorItemResolution::Destroyed {
                            kind_id: outcome.kind_id,
                            quantity: outcome.quantity,
                            rule_line: line_number,
                            ground,
                            captured_actor,
                        }),
                        Err(reason) => {
                            outcomes.push(MogaminatorItemResolution::DestroyUnavailable {
                                item_id,
                                reason: reason.reason().to_owned(),
                                rule_line: line_number,
                            });
                        }
                    }
                }
                MogaminatorDisposition::Query
                    if allow_pickup
                        && self.mogaminator.pending_query.is_none()
                        && !self.mogaminator.dismissed_query_item_ids.contains(&item_id) =>
                {
                    self.mogaminator.pending_query = Some(MogaminatorPendingQuery {
                        item_id,
                        rule_line: line_number,
                    });
                }
                MogaminatorDisposition::PickUp
                | MogaminatorDisposition::Destroy
                | MogaminatorDisposition::Leave
                | MogaminatorDisposition::Query => {}
            }
        }
        Ok(outcomes)
    }

    pub(super) fn clear_stale_mogaminator_query(&mut self) {
        let valid = self
            .mogaminator
            .pending_query
            .as_ref()
            .is_some_and(|pending| {
                self.items.iter().any(|item| {
                    item.id == pending.item_id
                        && item.location == ItemLocation::Ground(self.player.position)
                })
            });
        if !valid {
            self.mogaminator.pending_query = None;
        }
        self.mogaminator
            .dismissed_query_item_ids
            .retain(|item_id| self.items.iter().any(|item| item.id == *item_id));
    }

    pub(super) fn resolve_mogaminator_query(
        &mut self,
        item_id: &str,
        pick_up: bool,
    ) -> Result<Option<PickUpOutcome>, CoreError> {
        self.clear_stale_mogaminator_query();
        if self
            .mogaminator
            .pending_query
            .as_ref()
            .is_none_or(|pending| pending.item_id != item_id)
        {
            return Ok(None);
        }
        self.mogaminator.pending_query = None;
        if pick_up {
            self.mogaminator.dismissed_query_item_ids.remove(item_id);
            return self.pick_up_item_at_player(Some(item_id)).map(Some);
        }
        self.mogaminator
            .dismissed_query_item_ids
            .insert(item_id.to_owned());
        Ok(None)
    }

    fn mogaminator_auto_identify(
        &mut self,
        target_item_id: &str,
    ) -> Option<MogaminatorItemResolution> {
        let target = self.items.iter().find(|item| item.id == target_item_id)?;
        if self.item_identification(target) != ItemIdentificationDto::Unexamined {
            return None;
        }

        let mut sources = self
            .items
            .iter()
            .filter(|item| {
                item.id != target_item_id
                    && item.location == ItemLocation::Inventory
                    && self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
            })
            .filter_map(|item| {
                let definition = self.content.item(&item.kind_id)?;
                if definition.tags.iter().any(|tag| tag == "scroll")
                    && self.player_has_status_kind(STATUS_BLINDNESS)
                {
                    return None;
                }
                if let (Some(activation), Some(charges), Some(generation)) = (
                    item.activation.as_ref(),
                    item.charges,
                    definition.device_generation.as_ref(),
                ) && let Some(profile) = generation.activations.iter().find(|profile| {
                    profile.id == activation.profile_id
                        && matches!(profile.effect, ItemUseEffectDefinition::IdentifyItem { .. })
                        && charges.current >= activation.cost
                }) {
                    let ItemUseEffectDefinition::IdentifyItem { full } = profile.effect else {
                        unreachable!();
                    };
                    return Some((
                        0_u8,
                        item.id.clone(),
                        item.kind_id.clone(),
                        full,
                        true,
                        activation.cost,
                    ));
                }
                let action = definition.use_action.as_ref()?;
                let ItemUseEffectDefinition::IdentifyItem { full } = action.effect else {
                    return None;
                };
                if let Some(charge) = action.charges {
                    let charges = item.charges?;
                    (charges.current >= charge.cost).then(|| {
                        (
                            0_u8,
                            item.id.clone(),
                            item.kind_id.clone(),
                            full,
                            true,
                            charge.cost,
                        )
                    })
                } else {
                    Some((1_u8, item.id.clone(), item.kind_id.clone(), full, false, 0))
                }
            })
            .collect::<Vec<_>>();
        sources.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
        let (_, source_item_id, source_kind_id, full, charged, cost) =
            sources.into_iter().next()?;
        let source_index = self
            .items
            .iter()
            .position(|item| item.id == source_item_id)?;
        if charged {
            let charges = self.items[source_index].charges.as_mut()?;
            charges.current = charges.current.saturating_sub(cost);
        } else if self.items[source_index].quantity > 1 {
            self.items[source_index].quantity -= 1;
        } else {
            let removed = self.items.remove(source_index);
            self.item_property_knowledge.remove(&removed.id);
        }
        self.mark_item_aware(&source_kind_id);
        let outcome =
            self.identify_item_instance(target_item_id, ItemIdentificationRequest::new(full));
        Some(MogaminatorItemResolution::Identified {
            source_kind_id,
            target_item_id: outcome.item_id,
            target_kind_id: outcome.item_kind_id,
            full: outcome.full,
            changed: outcome.changed,
        })
    }

    pub(super) fn record_mogaminator_resolutions(
        &mut self,
        outcomes: Vec<MogaminatorItemResolution>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let mut handled_item = false;
        for outcome in outcomes {
            match outcome {
                MogaminatorItemResolution::PickUp { outcome } => {
                    handled_item |= self.record_pick_up_outcome(outcome, events, changed);
                }
                MogaminatorItemResolution::Destroyed {
                    kind_id,
                    quantity,
                    rule_line,
                    ground,
                    captured_actor,
                } => {
                    if let Some(captured_actor) = captured_actor {
                        self.release_captured_actor_near(
                            captured_actor,
                            self.player.position,
                            false,
                            events,
                            changed,
                        );
                    }
                    if ground {
                        changed.insert(self.player.position);
                    }
                    events.push(DomainEvent::ItemDestroyed {
                        target_kind_id: kind_id,
                        quantity,
                        rule_line: Some(rule_line),
                    });
                    handled_item = true;
                }
                MogaminatorItemResolution::DestroyUnavailable {
                    item_id,
                    reason,
                    rule_line,
                } => events.push(DomainEvent::ItemDestroyUnavailable {
                    item_id,
                    reason,
                    rule_line: Some(rule_line),
                }),
                MogaminatorItemResolution::Inscribed {
                    kind_id,
                    inscription,
                    rule_line,
                } => events.push(DomainEvent::ItemInscribed {
                    target_kind_id: kind_id,
                    inscription: Some(inscription),
                    rule_line: Some(rule_line),
                }),
                MogaminatorItemResolution::Identified {
                    source_kind_id,
                    target_item_id,
                    target_kind_id,
                    full,
                    changed,
                } => events.push(DomainEvent::ItemIdentified {
                    display_name_key: self.item_display_name_key(&source_kind_id),
                    source_kind_id,
                    resolution: ItemIdentifyResolutionDto {
                        item_id: target_item_id,
                        item_kind_id: target_kind_id,
                        full,
                        changed,
                    },
                }),
            }
        }
        handled_item
    }

    fn item_is_known_to_mogaminator_projection(&self, item: &ItemInstance) -> bool {
        match item.location {
            ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
            ItemLocation::Ground(_) => self
                .item_property_knowledge
                .get(&item.id)
                .is_some_and(|knowledge| knowledge.discovered),
            ItemLocation::CarriedBy { .. }
            | ItemLocation::Shop { .. }
            | ItemLocation::Home { .. } => false,
        }
    }

    fn mogaminator_match_for_item(
        &self,
        compiled: &CompiledMogaminator,
        names: &MogaminatorNames,
        locale: Locale,
        item: &ItemInstance,
    ) -> Option<(usize, (MogaminatorAction, Option<String>))> {
        let mut affix_ids = item.affix_ids.clone();
        affix_ids.extend(
            item.rolled_affixes
                .iter()
                .map(|affix| affix.affix_id.clone()),
        );
        let name = names
            .item_name(
                &self.content,
                &item.kind_id,
                &affix_ids,
                item.activation
                    .as_ref()
                    .map(|activation| activation.profile_id.as_str()),
            )
            .ok()?;
        compiled.rules.iter().find_map(|compiled_rule| {
            self.mogaminator_rule_matches(compiled_rule, locale, item, &name)
                .then(|| {
                    (
                        compiled_rule.line_number,
                        (
                            compiled_rule.rule.action,
                            compiled_rule.rule.inscription.clone(),
                        ),
                    )
                })
        })
    }

    fn mogaminator_rule_matches(
        &self,
        compiled: &CompiledMogaminatorRule,
        locale: Locale,
        item: &ItemInstance,
        name: &str,
    ) -> bool {
        compiled
            .condition
            .as_ref()
            .is_none_or(|condition| self.evaluate_mogaminator_condition(condition, locale))
            && compiled
                .rule
                .predicates
                .iter()
                .all(|predicate| self.mogaminator_predicate_matches(*predicate, item))
            && mogaminator_search_matches(&compiled.rule.search, name)
    }

    fn evaluate_mogaminator_condition(
        &self,
        expression: &MogaminatorExpression,
        locale: Locale,
    ) -> bool {
        truthy(&self.mogaminator_expression_value(expression, locale))
    }

    fn mogaminator_expression_value(
        &self,
        expression: &MogaminatorExpression,
        locale: Locale,
    ) -> String {
        match expression {
            MogaminatorExpression::Literal(value) => value.clone(),
            MogaminatorExpression::Variable(variable) => {
                self.mogaminator_variable_value(*variable, locale)
            }
            MogaminatorExpression::Call {
                function,
                arguments,
            } => {
                let values = arguments
                    .iter()
                    .map(|argument| self.mogaminator_expression_value(argument, locale))
                    .collect::<Vec<_>>();
                let result = match function {
                    MogaminatorFunction::Or => values.iter().any(|value| truthy(value)),
                    MogaminatorFunction::And => values.iter().all(|value| truthy(value)),
                    MogaminatorFunction::Not => values.iter().all(|value| !truthy(value)),
                    MogaminatorFunction::Equal => values[1..]
                        .iter()
                        .any(|value| compare_values(&values[0], value).is_eq()),
                    MogaminatorFunction::LessOrEqual => values
                        .windows(2)
                        .all(|pair| !compare_values(&pair[0], &pair[1]).is_gt()),
                    MogaminatorFunction::GreaterOrEqual => values
                        .windows(2)
                        .all(|pair| !compare_values(&pair[0], &pair[1]).is_lt()),
                };
                if result { "1" } else { "0" }.to_owned()
            }
        }
    }

    fn mogaminator_variable_value(&self, variable: MogaminatorVariable, locale: Locale) -> String {
        match variable {
            MogaminatorVariable::Level => self.progress.level.to_string(),
            MogaminatorVariable::Money => self.gold.to_string(),
            MogaminatorVariable::Race
            | MogaminatorVariable::Class
            | MogaminatorVariable::Subclass
            | MogaminatorVariable::Speciality
            | MogaminatorVariable::FirstRealm
            | MogaminatorVariable::SecondRealm => {
                let Some(build) = &self.build else {
                    return String::new();
                };
                let Ok((definition, race, class, _)) = build_definitions(&self.content, build)
                else {
                    return String::new();
                };
                let key = match variable {
                    MogaminatorVariable::Race => Some(race.name_key.as_str()),
                    MogaminatorVariable::Class => Some(class.name_key.as_str()),
                    MogaminatorVariable::Subclass => definition.subclass_name_key.as_deref(),
                    MogaminatorVariable::Speciality => definition.speciality_name_key.as_deref(),
                    MogaminatorVariable::FirstRealm => {
                        return definition
                            .first_realm_id
                            .as_deref()
                            .map(|id| localized_realm_name(id, locale))
                            .unwrap_or_default();
                    }
                    MogaminatorVariable::SecondRealm => {
                        return definition
                            .second_realm_id
                            .as_deref()
                            .map(|id| localized_realm_name(id, locale))
                            .unwrap_or_default();
                    }
                    _ => unreachable!(),
                };
                let Some(key) = key else {
                    return String::new();
                };
                Localizer::new(locale)
                    .and_then(|localizer| localizer.format_exact(locale, key, None))
                    .unwrap_or_default()
            }
            MogaminatorVariable::Selling => match locale {
                Locale::EnUs => "On",
                Locale::ZhCn => "开启",
            }
            .to_owned(),
            _ => unreachable!("the compiler rejects unsupported Mogaminator variables"),
        }
    }

    fn mogaminator_predicate_matches(
        &self,
        predicate: MogaminatorPredicate,
        item: &ItemInstance,
    ) -> bool {
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("runtime items must retain their definitions");
        let identification = self.item_identification(item);
        let aware = self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware;
        let known_affixes = self
            .item_property_knowledge
            .get(&item.id)
            .map(|knowledge| &knowledge.known_affix_ids);
        let slot = definition.equipment_slot.as_deref();
        let tagged = |tag: &str| definition.tags.iter().any(|candidate| candidate == tag);
        let build_definition = self
            .build
            .as_ref()
            .and_then(|build| self.content.build(&build.build_id));
        let class = self
            .build
            .as_ref()
            .and_then(|build| self.content.class(&build.class_id));
        let book = definition
            .ability_book_id
            .as_deref()
            .and_then(|book_id| self.content.ability_book(book_id));
        let corpse_actor = item
            .origin_actor_kind_id
            .as_deref()
            .and_then(|actor_id| self.content.actor(actor_id));
        match predicate {
            MogaminatorPredicate::All | MogaminatorPredicate::Items => true,
            MogaminatorPredicate::Unaware => !aware,
            MogaminatorPredicate::Unsensed | MogaminatorPredicate::Unidentified => {
                identification == ItemIdentificationDto::Unexamined
            }
            MogaminatorPredicate::Identified => identification != ItemIdentificationDto::Unexamined,
            MogaminatorPredicate::FullyIdentified => {
                identification == ItemIdentificationDto::Identified
            }
            MogaminatorPredicate::Average => {
                self.visible_item_quality(item) == Some(ItemQualityDto::Ordinary)
            }
            MogaminatorPredicate::Good => matches!(
                self.visible_item_quality(item),
                Some(ItemQualityDto::Fine | ItemQualityDto::Exceptional)
            ),
            MogaminatorPredicate::Cursed => self.visible_item_curse(item).is_some(),
            MogaminatorPredicate::Ego => known_affixes.is_some_and(|affixes| !affixes.is_empty()),
            MogaminatorPredicate::Artifact => {
                identification != ItemIdentificationDto::Unexamined && tagged("artifact")
            }
            MogaminatorPredicate::Nameless => {
                identification != ItemIdentificationDto::Unexamined
                    && !tagged("artifact")
                    && known_affixes.is_none_or(BTreeSet::is_empty)
            }
            MogaminatorPredicate::Rare => definition.mogaminator_rare,
            MogaminatorPredicate::Common => !definition.mogaminator_rare,
            MogaminatorPredicate::Worthless => aware && definition.base_value == 0,
            MogaminatorPredicate::DiceBoosted => {
                identification != ItemIdentificationDto::Unexamined
                    && definition.melee_profile.as_ref().is_some_and(|base| {
                        self.item_melee_profile(item).is_some_and(|actual| {
                            actual.damage.dice != base.damage_dice
                                || actual.damage.sides != base.damage_sides
                        })
                    })
            }
            MogaminatorPredicate::Icky => slot.is_some_and(|slot| {
                class.is_some_and(|class| {
                    class
                        .icky_equipment_slots
                        .iter()
                        .any(|candidate| candidate == slot)
                })
            }),
            MogaminatorPredicate::Unreadable => book.is_some_and(|book| {
                class.is_none_or(|class| {
                    class.casting_profile.is_none()
                        || !self.active_casting_book_ids().contains(&book.id.as_str())
                })
            }),
            MogaminatorPredicate::FirstRealm => book.is_some_and(|book| {
                book.realm_id.as_ref()
                    == build_definition.and_then(|build| build.first_realm_id.as_ref())
            }),
            MogaminatorPredicate::SecondRealm => book.is_some_and(|book| {
                book.realm_id.as_ref()
                    == build_definition.and_then(|build| build.second_realm_id.as_ref())
            }),
            MogaminatorPredicate::FirstBook => book.is_some_and(|book| book.rank == Some(1)),
            MogaminatorPredicate::SecondBook => book.is_some_and(|book| book.rank == Some(2)),
            MogaminatorPredicate::ThirdBook => book.is_some_and(|book| book.rank == Some(3)),
            MogaminatorPredicate::FourthBook => book.is_some_and(|book| book.rank == Some(4)),
            MogaminatorPredicate::Collecting => self.items.iter().any(|other| {
                other.id != item.id
                    && other.location == ItemLocation::Inventory
                    && inventory::item_instances_stack_compatible(other, item)
            }),
            MogaminatorPredicate::Special => class.is_some_and(|class| {
                definition.tags.iter().any(|tag| {
                    class
                        .special_item_tags
                        .iter()
                        .any(|candidate| candidate == tag)
                })
            }),
            MogaminatorPredicate::Unusable => slot.is_some_and(|slot| {
                !self
                    .body_slots
                    .iter()
                    .any(|body_slot| body_slot.slot_type == slot)
            }),
            MogaminatorPredicate::Wanted => item
                .origin_actor_kind_id
                .as_ref()
                .is_some_and(|id| self.mogaminator.wanted_actor_kind_ids.contains(id)),
            MogaminatorPredicate::Unique => {
                corpse_actor.is_some_and(|actor| actor.tags.iter().any(|tag| tag == "unique"))
            }
            MogaminatorPredicate::Human => {
                corpse_actor.is_some_and(|actor| actor.tags.iter().any(|tag| tag == "human"))
            }
            MogaminatorPredicate::MoreDiceThan(value) => {
                identification != ItemIdentificationDto::Unexamined
                    && self.item_melee_profile(item).is_some_and(|profile| {
                        u32::from(profile.damage.dice)
                            .saturating_mul(u32::from(profile.damage.sides))
                            > value
                    })
            }
            MogaminatorPredicate::MoreBonusThan(value) => {
                identification != ItemIdentificationDto::Unexamined
                    && u32::try_from(
                        item.enchantments
                            .to_hit
                            .max(item.enchantments.to_damage)
                            .max(item.enchantments.to_armor),
                    )
                    .unwrap_or_default()
                        > value
            }
            MogaminatorPredicate::MoreWeightThan(value) => {
                aware && u32::from(definition.weight_tenths_pound) > value.saturating_mul(10)
            }
            MogaminatorPredicate::MoreChargesThan(value) => {
                aware && item.charges.is_some_and(|charges| charges.current > value)
            }
            MogaminatorPredicate::MoreLevelThan(value) => {
                let mut level = corpse_actor
                    .map(|actor| actor.level)
                    .unwrap_or_else(|| u32::from(definition.generation_level));
                if identification != ItemIdentificationDto::Unexamined {
                    if let Some(activation) = &item.activation {
                        level = level.max(u32::from(activation.power));
                    }
                    if let Some(known_affixes) = known_affixes {
                        level = level.max(
                            known_affixes
                                .iter()
                                .filter_map(|affix_id| self.content.affix(affix_id))
                                .map(|affix| u32::from(affix.generation_level))
                                .max()
                                .unwrap_or(0),
                        );
                    }
                }
                level > value
            }
            MogaminatorPredicate::MoreValueThan(value) => aware && definition.base_value > value,
            MogaminatorPredicate::Weapons => slot == Some("weapon") || tagged("weapon"),
            MogaminatorPredicate::FavoriteWeapons => class.is_some_and(|class| {
                class.favorite_weapon_tags.iter().any(|favorite| {
                    (favorite == "weapon" && (slot == Some("weapon") || tagged("weapon")))
                        || (favorite == "shooter"
                            && (slot == Some("launcher") || tagged("shooter")))
                        || tagged(favorite)
                })
            }),
            MogaminatorPredicate::HaftedWeapons => tagged("hafted"),
            MogaminatorPredicate::Diggers => tagged("digger"),
            MogaminatorPredicate::Shooters => slot == Some("launcher") || tagged("shooter"),
            MogaminatorPredicate::Ammo => tagged("ammunition"),
            MogaminatorPredicate::Arrows => definition
                .ammunition_profile
                .as_ref()
                .is_some_and(|profile| profile.ammunition_type == AmmunitionTypeDefinition::Arrow),
            MogaminatorPredicate::Armors => tagged("armor"),
            MogaminatorPredicate::Shields => slot == Some("shield"),
            MogaminatorPredicate::Suits => slot == Some("body"),
            MogaminatorPredicate::Cloaks => slot == Some("cloak"),
            MogaminatorPredicate::Helms => slot == Some("head"),
            MogaminatorPredicate::Gloves => slot == Some("gloves"),
            MogaminatorPredicate::Boots => slot == Some("boots"),
            MogaminatorPredicate::Lights => slot == Some("light") || tagged("light-source"),
            MogaminatorPredicate::Rings => slot == Some("ring"),
            MogaminatorPredicate::Amulets => slot == Some("amulet"),
            MogaminatorPredicate::Spellbooks => tagged("spellbook"),
            MogaminatorPredicate::Wands => tagged("wand"),
            MogaminatorPredicate::Staves => tagged("staff"),
            MogaminatorPredicate::Rods => tagged("rod"),
            MogaminatorPredicate::Potions => tagged("potion"),
            MogaminatorPredicate::Scrolls => tagged("scroll"),
            MogaminatorPredicate::Junk => tagged("junk"),
            MogaminatorPredicate::Corpses => tagged("corpse"),
            MogaminatorPredicate::Skeletons => tagged("skeleton"),
        }
    }
}

fn localization_locale(locale: LocaleDto) -> Locale {
    match locale {
        LocaleDto::EnUs => Locale::EnUs,
        LocaleDto::ZhCn => Locale::ZhCn,
    }
}

fn localized_realm_name(realm_id: &str, locale: Locale) -> String {
    match (realm_id, locale) {
        ("death", Locale::ZhCn) => "死亡".to_owned(),
        ("death", Locale::EnUs) => "Death".to_owned(),
        ("healing", Locale::ZhCn) => "治愈".to_owned(),
        ("healing", Locale::EnUs) => "Healing".to_owned(),
        ("echo", Locale::ZhCn) => "回声".to_owned(),
        ("echo", Locale::EnUs) => "Echo".to_owned(),
        _ => realm_id.to_owned(),
    }
}

fn default_source(locale: LocaleDto) -> &'static str {
    match locale {
        LocaleDto::EnUs => DEFAULT_EN_US_SOURCE,
        LocaleDto::ZhCn => DEFAULT_ZH_CN_SOURCE,
    }
}

fn action_dto(action: MogaminatorAction, inscription: Option<String>) -> MogaminatorActionDto {
    MogaminatorActionDto {
        disposition: match action.disposition {
            MogaminatorDisposition::PickUp => MogaminatorDispositionDto::PickUp,
            MogaminatorDisposition::Destroy => MogaminatorDispositionDto::Destroy,
            MogaminatorDisposition::Leave => MogaminatorDispositionDto::Leave,
            MogaminatorDisposition::Query => MogaminatorDispositionDto::Query,
        },
        display: action.display,
        auto_identify: action.auto_identify,
        inscription,
    }
}

fn diagnostic_dto(diagnostic: MogaminatorDiagnostic) -> MogaminatorDiagnosticDto {
    MogaminatorDiagnosticDto {
        line: saturating_u32(diagnostic.line),
        column: saturating_u32(diagnostic.column),
        code: diagnostic.code.as_str().to_owned(),
        arguments: diagnostic.arguments,
    }
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn truthy(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value != "0"
}

fn compare_values(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.trim().parse::<i64>(), right.trim().parse::<i64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
            .cmp(
                &right
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_lowercase(),
            ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfb_protocol::GameCommand;

    fn auto_get_test_game(mode: AutoGetModeDto, source: &str) -> Game {
        let mut game = Game::new(41);
        game.entities.clear();
        game.items.clear();
        game.gold_piles.clear();
        game.terrain.fill("demo.terrain.surface-path".to_owned());
        game.explored.fill(true);
        game.interface_locale = LocaleDto::EnUs;
        assert!(
            game.configure_mogaminator(true, false, mode, LocaleDto::EnUs, source.to_owned(),)
                .is_empty()
        );
        game
    }

    fn add_auto_get_item(
        game: &mut Game,
        id: &str,
        kind_id: &str,
        position: Position,
        inscription: Option<&str>,
        discovered: bool,
    ) {
        game.items.push(ItemInstance {
            id: id.to_owned(),
            kind_id: kind_id.to_owned(),
            quantity: 1,
            inscription: inscription.map(str::to_owned),
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            permanent_destruction_immunities: Default::default(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Ground(position),
        });
        game.item_property_knowledge.insert(
            id.to_owned(),
            inventory::ItemPropertyKnowledgeState {
                discovered,
                ..Default::default()
            },
        );
    }

    fn auto_get_target(game: &Game) -> Option<(String, Position)> {
        game.mogaminator_dto(Vec::new())
            .auto_get_target
            .map(|target| (target.object_id, target.position))
    }

    fn dispatch_auto_get(game: &mut Game, object_id: &str) -> GameUpdate {
        game.dispatch(GameCommandEnvelope {
            command_seq: game.last_command_seq + 1,
            expected_revision: game.revision,
            command: GameCommand::AutoGet {
                object_id: object_id.to_owned(),
            },
        })
        .expect("auto-get command should execute")
    }

    #[test]
    fn character_keeps_independent_bilingual_sources_and_applies_atomically() {
        let mut game = Game::new(7);
        assert_eq!(game.mogaminator.auto_get_mode, AutoGetModeDto::Off);
        let initial_hash = game.state_hash();
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Ammo,
                LocaleDto::ZhCn,
                "物品".to_owned(),
            )
            .is_empty()
        );
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Ammo,
                LocaleDto::EnUs,
                "weapons".to_owned(),
            )
            .is_empty()
        );

        game.interface_locale = LocaleDto::ZhCn;
        assert_eq!(game.mogaminator_dto(Vec::new()).source, "物品");
        game.interface_locale = LocaleDto::EnUs;
        assert_eq!(game.mogaminator_dto(Vec::new()).source, "weapons");

        let diagnostics = game.configure_mogaminator(
            false,
            false,
            AutoGetModeDto::Wanted,
            LocaleDto::EnUs,
            "?:[EQU $SYS windows]\nweapons".to_owned(),
        );
        assert_eq!(diagnostics.len(), 1);
        let dto = game.mogaminator_dto(Vec::new());
        assert!(dto.enabled);
        assert_eq!(dto.auto_get_mode, AutoGetModeDto::Ammo);
        assert_eq!(dto.source, "weapons");
        assert_ne!(game.state_hash(), initial_hash);

        let restored =
            Game::from_save(game.to_save()).expect("Mogaminator state should round-trip");
        assert_eq!(restored.interface_locale, LocaleDto::EnUs);
        assert_eq!(restored.mogaminator, game.mogaminator);
    }

    #[test]
    fn auto_get_uses_original_projectability_distance_and_stable_id_order() {
        let mut game = auto_get_test_game(AutoGetModeDto::Ammo, "items");
        let start = game.player.position;
        let alpha = Position {
            x: start.x,
            y: start.y + 2,
        };
        let zeta = Position {
            x: start.x + 2,
            y: start.y,
        };
        add_auto_get_item(
            &mut game,
            "g1.ammo.zeta",
            "demo.item.arrow",
            zeta,
            Some("=g"),
            true,
        );
        add_auto_get_item(
            &mut game,
            "g1.ammo.alpha",
            "demo.item.arrow",
            alpha,
            Some("=g"),
            true,
        );
        add_auto_get_item(
            &mut game,
            "g1.ammo.hidden",
            "demo.item.arrow",
            Position {
                x: start.x + 1,
                y: start.y,
            },
            Some("=g"),
            false,
        );
        assert_eq!(
            auto_get_target(&game),
            Some(("g1.ammo.alpha".to_owned(), alpha))
        );

        let wall = Position {
            x: start.x + 1,
            y: start.y,
        };
        let wall_index = game.index(wall).expect("wall position should be valid");
        game.terrain[wall_index] = "demo.terrain.wall".to_owned();
        game.items.retain(|item| item.id != "g1.ammo.alpha");
        game.item_property_knowledge.remove("g1.ammo.alpha");
        assert_eq!(
            auto_get_target(&game),
            Some(("g1.ammo.zeta".to_owned(), zeta)),
            "ammo mode keeps the original within-18 non-projectable exception"
        );

        game.mogaminator.auto_get_mode = AutoGetModeDto::Wanted;
        assert_eq!(auto_get_target(&game), None);
    }

    #[test]
    fn wanted_auto_get_does_not_reveal_hidden_gold() {
        let mut game = auto_get_test_game(AutoGetModeDto::Wanted, "~items");
        let position = Position {
            x: game.player.position.x + 2,
            y: game.player.position.y,
        };
        game.gold_piles.push(GoldPile {
            id: "g1.gold".to_owned(),
            position,
            amount: 17,
            appearance: GoldAppearanceDto::Silver,
            discovered: false,
        });

        assert_eq!(auto_get_target(&game), None);
        assert!(game.gold_pile_dtos().is_empty());
        assert_eq!(game.cell_dto(position).item_id, None);

        game.gold_piles[0].discovered = true;
        assert_eq!(
            auto_get_target(&game),
            Some(("g1.gold".to_owned(), position))
        );
        assert_eq!(game.gold_pile_dtos().len(), 1);
        assert_eq!(game.cell_dto(position).item_id.as_deref(), Some("g1.gold"));
    }

    #[test]
    fn auto_get_moves_one_step_then_picks_up_ammo_without_time() {
        let mut game = auto_get_test_game(AutoGetModeDto::Ammo, "~items");
        let start = game.player.position;
        let target = Position {
            x: start.x + 2,
            y: start.y,
        };
        add_auto_get_item(
            &mut game,
            "g2.ammo",
            "demo.item.arrow",
            target,
            Some("=g"),
            true,
        );

        let initial_tick = game.world_tick;
        dispatch_auto_get(&mut game, "g2.ammo");
        assert_eq!(
            game.player.position,
            Position {
                x: start.x + 1,
                y: start.y,
            }
        );
        assert!(game.world_tick > initial_tick);

        dispatch_auto_get(&mut game, "g2.ammo");
        assert_eq!(game.player.position, target);
        let arrival_tick = game.world_tick;
        dispatch_auto_get(&mut game, "g2.ammo");
        assert_eq!(game.world_tick, arrival_tick);
        assert!(
            game.items
                .iter()
                .any(|item| { item.id == "g2.ammo" && item.location == ItemLocation::Inventory })
        );
    }

    #[test]
    fn auto_get_rejects_stale_id_and_collects_only_the_locked_gold() {
        let mut game = auto_get_test_game(AutoGetModeDto::Wanted, "~items");
        let position = game.player.position;
        game.gold_piles.extend([
            GoldPile {
                id: "g2.gold.alpha".to_owned(),
                position,
                amount: 11,
                appearance: GoldAppearanceDto::Gold,
                discovered: true,
            },
            GoldPile {
                id: "g2.gold.zeta".to_owned(),
                position,
                amount: 29,
                appearance: GoldAppearanceDto::Silver,
                discovered: true,
            },
        ]);
        assert_eq!(
            auto_get_target(&game),
            Some(("g2.gold.alpha".to_owned(), position))
        );

        let initial_gold = game.gold;
        let initial_tick = game.world_tick;
        dispatch_auto_get(&mut game, "g2.gold.missing");
        assert_eq!(game.gold, initial_gold);
        assert_eq!(game.gold_piles.len(), 2);
        assert_eq!(game.world_tick, initial_tick);

        dispatch_auto_get(&mut game, "g2.gold.zeta");
        assert_eq!(game.gold, initial_gold + 29);
        assert_eq!(game.gold_piles.len(), 1);
        assert_eq!(game.gold_piles[0].id, "g2.gold.alpha");
        assert_eq!(game.world_tick, initial_tick);
    }

    #[test]
    fn bilingual_defaults_compile_and_choose_the_same_rules() {
        compile_mogaminator(DEFAULT_EN_US_SOURCE).expect("English defaults should compile");
        compile_mogaminator(DEFAULT_ZH_CN_SOURCE).expect("Chinese defaults should compile");

        let mut game = Game::new(11);
        game.mogaminator.enabled = true;
        game.interface_locale = LocaleDto::EnUs;
        let english = game.mogaminator_dto(Vec::new()).matches;
        game.interface_locale = LocaleDto::ZhCn;
        let chinese = game.mogaminator_dto(Vec::new()).matches;

        assert!(!english.is_empty());
        assert_eq!(chinese, english);
    }

    #[test]
    fn bilingual_defaults_match_every_current_item_kind_equally() {
        let mut game = Game::new(11);
        let kind_ids = game
            .content
            .item_definitions()
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        game.items = kind_ids
            .iter()
            .enumerate()
            .map(|(index, kind_id)| ItemInstance {
                id: format!("m6.item.{index}"),
                kind_id: kind_id.clone(),
                quantity: 1,
                inscription: None,
                origin_actor_kind_id: None,
                origin_kind: None,
                damage_dice_override: None,
                discount_percent: 0,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                rolled_affixes: Vec::new(),
                permanent_destruction_immunities: Default::default(),
                enchantments: ItemEnchantmentsDto::default(),
                curse: None,
                activation: None,
                charges: None,
                fuel: None,
                device_recovery_progress: 0,
                captured_actor: None,
                location: ItemLocation::Ground(game.player.position),
            })
            .collect();
        let english = compile_mogaminator(DEFAULT_EN_US_SOURCE).expect("English defaults");
        let chinese = compile_mogaminator(DEFAULT_ZH_CN_SOURCE).expect("Chinese defaults");
        let english_names = MogaminatorNames::new(Locale::EnUs).expect("English names");
        let chinese_names = MogaminatorNames::new(Locale::ZhCn).expect("Chinese names");
        assert_eq!(chinese.rules.len(), english.rules.len());
        for (chinese_rule, english_rule) in chinese.rules.iter().zip(&english.rules) {
            assert_eq!(chinese_rule.line_number, english_rule.line_number);
            assert_eq!(chinese_rule.rule.action, english_rule.rule.action);
            assert_eq!(chinese_rule.rule.inscription, english_rule.rule.inscription);
            let english_active = english_rule.condition.as_ref().is_none_or(|condition| {
                game.evaluate_mogaminator_condition(condition, Locale::EnUs)
            });
            let chinese_active = chinese_rule.condition.as_ref().is_none_or(|condition| {
                game.evaluate_mogaminator_condition(condition, Locale::ZhCn)
            });
            assert_eq!(
                chinese_active, english_active,
                "line {}",
                english_rule.line_number
            );
        }
        for aware in [false, true] {
            game.item_knowledge.clear();
            if aware {
                for kind_id in &kind_ids {
                    game.mark_item_aware(kind_id);
                }
            }
            for (appraised, identified) in [(false, false), (true, false), (true, true)] {
                game.item_property_knowledge = game
                    .items
                    .iter()
                    .map(|item| {
                        (
                            item.id.clone(),
                            inventory::ItemPropertyKnowledgeState {
                                discovered: true,
                                appraised,
                                identified,
                                known_affix_ids: BTreeSet::new(),
                            },
                        )
                    })
                    .collect();
                for (chinese_rule, english_rule) in chinese.rules.iter().zip(&english.rules) {
                    for item in &game.items {
                        let english_matches =
                            english_rule.rule.predicates.iter().all(|predicate| {
                                game.mogaminator_predicate_matches(*predicate, item)
                            });
                        let chinese_matches =
                            chinese_rule.rule.predicates.iter().all(|predicate| {
                                game.mogaminator_predicate_matches(*predicate, item)
                            });
                        let english_name = english_names
                            .item_name(&game.content, &item.kind_id, &[], None)
                            .expect("English item name");
                        let chinese_name = chinese_names
                            .item_name(&game.content, &item.kind_id, &[], None)
                            .expect("Chinese item name");
                        assert_eq!(
                            chinese_matches
                                && mogaminator_search_matches(
                                    &chinese_rule.rule.search,
                                    &chinese_name,
                                ),
                            english_matches
                                && mogaminator_search_matches(
                                    &english_rule.rule.search,
                                    &english_name,
                                ),
                            "{} at line {} ({aware}/{appraised}/{identified})",
                            item.kind_id,
                            english_rule.line_number
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn original_conditions_compare_against_every_candidate() {
        let compiled = compile_mogaminator("?:[EQU first second first]\nitems")
            .expect("original variadic equality should compile");
        let condition = compiled.rules[0]
            .condition
            .as_ref()
            .expect("rule should retain its condition");
        assert!(Game::new(7).evaluate_mogaminator_condition(condition, Locale::EnUs));
    }

    #[test]
    fn enabled_rules_return_the_first_match() {
        let mut game = Game::new(11);
        game.interface_locale = LocaleDto::ZhCn;
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Off,
                LocaleDto::ZhCn,
                "~物品\n!物品".to_owned(),
            )
            .is_empty()
        );

        let locations_before = game
            .items
            .iter()
            .map(|item| (item.id.clone(), item.location.clone()))
            .collect::<Vec<_>>();
        let dto = game.mogaminator_dto(Vec::new());
        assert!(!dto.matches.is_empty());
        assert!(dto.matches.iter().all(|matched| {
            matched.line_number == 1
                && matched.action.disposition == MogaminatorDispositionDto::Leave
        }));
        assert_eq!(
            locations_before,
            game.items
                .iter()
                .map(|item| (item.id.clone(), item.location.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn query_rules_round_trip_and_rejection_is_not_repeated() {
        let mut game = Game::new(11);
        game.interface_locale = LocaleDto::ZhCn;
        game.player.position = Position { x: 4, y: 3 };
        game.items.clear();
        add_auto_get_item(
            &mut game,
            "test.item.query",
            "demo.item.ration-of-food",
            Position { x: 4, y: 3 },
            None,
            true,
        );
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Off,
                LocaleDto::ZhCn,
                ";物品".to_owned(),
            )
            .is_empty()
        );

        game.apply_mogaminator_at_player()
            .expect("query rule should create a pending confirmation");
        let pending = game
            .mogaminator
            .pending_query
            .clone()
            .expect("one ground item should be pending");
        let mut restored = Game::from_save(game.to_save()).expect("pending query should reload");
        assert_eq!(restored.mogaminator.pending_query, Some(pending.clone()));

        assert!(
            restored
                .resolve_mogaminator_query(&pending.item_id, false)
                .expect("query rejection should resolve")
                .is_none()
        );
        restored
            .apply_mogaminator_at_player()
            .expect("rejected item should be skipped on rescan");
        assert!(restored.mogaminator.pending_query.is_none());
        assert!(
            restored
                .mogaminator
                .dismissed_query_item_ids
                .contains(&pending.item_id)
        );
    }

    #[test]
    fn deferred_predicates_use_content_build_book_and_corpse_data() {
        let game = crate::game::tests::support::test_caster_game(19);
        let book = game
            .items
            .iter()
            .find(|item| item.kind_id == "demo.item.black-mass")
            .expect("test caster should start with a death book");
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::FirstRealm, book));
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::SecondBook, book));
        assert!(!game.mogaminator_predicate_matches(MogaminatorPredicate::Unreadable, book));

        let wanted_id = game
            .mogaminator
            .wanted_actor_kind_ids
            .iter()
            .next()
            .expect("the built-in content has wanted uniques")
            .clone();
        let mut corpse = game.items[0].clone();
        corpse.kind_id = "demo.item.corpse-remains".to_owned();
        corpse.origin_actor_kind_id = Some(wanted_id);
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::Wanted, &corpse));
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::Unique, &corpse));

        let mut rare = game.items[0].clone();
        rare.kind_id = "demo.item.adamantine-bolt".to_owned();
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::Rare, &rare));
        assert!(game.mogaminator_predicate_matches(MogaminatorPredicate::MoreLevelThan(39), &rare));
    }
}
