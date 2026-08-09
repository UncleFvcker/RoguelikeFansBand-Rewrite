// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::mogaminator::{
    CompiledMogaminator, CompiledMogaminatorRule, MogaminatorAction, MogaminatorDiagnostic,
    MogaminatorDisposition, MogaminatorExpression, MogaminatorFunction, MogaminatorLineKind,
    MogaminatorPredicate, MogaminatorVariable, compile_mogaminator, mogaminator_search_matches,
};
use rfb_localization::{Locale, Localizer, MogaminatorNames};
use rfb_protocol::{
    LocaleDto, MogaminatorActionDto, MogaminatorDiagnosticDto, MogaminatorDispositionDto,
    MogaminatorDto, MogaminatorItemMatchDto, MogaminatorLineDto, MogaminatorLineKindDto,
    MogaminatorSaveDto,
};

const DEFAULT_ZH_CN_SOURCE: &str = "# 墨家名器规则\n";
const DEFAULT_EN_US_SOURCE: &str = "# Mogaminator rules\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MogaminatorState {
    pub(super) enabled: bool,
    pub(super) zh_cn_source: String,
    pub(super) en_us_source: String,
}

impl Default for MogaminatorState {
    fn default() -> Self {
        Self {
            enabled: false,
            zh_cn_source: DEFAULT_ZH_CN_SOURCE.to_owned(),
            en_us_source: DEFAULT_EN_US_SOURCE.to_owned(),
        }
    }
}

impl MogaminatorState {
    pub(super) fn from_save(saved: MogaminatorSaveDto) -> Result<Self, Vec<MogaminatorDiagnostic>> {
        compile_mogaminator(&saved.zh_cn_source)?;
        compile_mogaminator(&saved.en_us_source)?;
        Ok(Self {
            enabled: saved.enabled,
            zh_cn_source: saved.zh_cn_source,
            en_us_source: saved.en_us_source,
        })
    }

    pub(super) fn to_save(&self) -> MogaminatorSaveDto {
        MogaminatorSaveDto {
            enabled: self.enabled,
            zh_cn_source: self.zh_cn_source.clone(),
            en_us_source: self.en_us_source.clone(),
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
}

impl Game {
    pub(super) fn configure_mogaminator(
        &mut self,
        enabled: bool,
        locale: LocaleDto,
        source: String,
    ) -> Vec<MogaminatorDiagnostic> {
        match compile_mogaminator(&source) {
            Ok(_) => {
                self.mogaminator.set_source(locale, source);
                self.mogaminator.enabled = enabled;
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
            locale: self.interface_locale,
            source: source.to_owned(),
            default_source: default_source(self.interface_locale).to_owned(),
            diagnostics: diagnostics.into_iter().map(diagnostic_dto).collect(),
            lines,
            matches: matches.unwrap_or_default(),
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
            .item_name(&self.content, &item.kind_id, &affix_ids)
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
                    MogaminatorFunction::Not => !truthy(&values[0]),
                    MogaminatorFunction::Equal => compare_values(&values[0], &values[1]).is_eq(),
                    MogaminatorFunction::LessOrEqual => {
                        !compare_values(&values[0], &values[1]).is_gt()
                    }
                    MogaminatorFunction::GreaterOrEqual => {
                        !compare_values(&values[0], &values[1]).is_lt()
                    }
                };
                if result { "1" } else { "0" }.to_owned()
            }
        }
    }

    fn mogaminator_variable_value(&self, variable: MogaminatorVariable, locale: Locale) -> String {
        match variable {
            MogaminatorVariable::Level => self.progress.level.to_string(),
            MogaminatorVariable::Money => self.gold.to_string(),
            MogaminatorVariable::Race | MogaminatorVariable::Class => {
                let Some(build) = &self.build else {
                    return String::new();
                };
                let Ok((_, race, class, _)) = build_definitions(&self.content, build) else {
                    return String::new();
                };
                let key = match variable {
                    MogaminatorVariable::Race => &race.name_key,
                    MogaminatorVariable::Class => &class.name_key,
                    _ => unreachable!(),
                };
                Localizer::new(locale)
                    .and_then(|localizer| localizer.format_exact(locale, key, None))
                    .unwrap_or_default()
            }
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
            MogaminatorPredicate::Collecting => self.items.iter().any(|other| {
                other.id != item.id
                    && other.location == ItemLocation::Inventory
                    && inventory::item_instances_stack_compatible(other, item)
            }),
            MogaminatorPredicate::MoreBonusThan(value) => {
                identification != ItemIdentificationDto::Unexamined
                    && u32::from(
                        item.enchantments
                            .to_hit
                            .max(item.enchantments.to_damage)
                            .max(item.enchantments.to_armor),
                    ) > value
            }
            MogaminatorPredicate::MoreWeightThan(value) => {
                aware && u32::from(definition.weight_tenths_pound) > value.saturating_mul(10)
            }
            MogaminatorPredicate::MoreChargesThan(value) => {
                aware && item.charges.is_some_and(|charges| charges.maximum > value)
            }
            MogaminatorPredicate::Weapons => slot == Some("weapon") || tagged("weapon"),
            MogaminatorPredicate::Shooters => slot == Some("launcher") || tagged("shooter"),
            MogaminatorPredicate::Ammo => tagged("ammunition"),
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
            _ => unreachable!("the compiler rejects unsupported Mogaminator predicates"),
        }
    }
}

fn localization_locale(locale: LocaleDto) -> Locale {
    match locale {
        LocaleDto::EnUs => Locale::EnUs,
        LocaleDto::ZhCn => Locale::ZhCn,
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

    #[test]
    fn character_keeps_independent_bilingual_sources_and_applies_atomically() {
        let mut game = Game::new(7);
        let initial_hash = game.state_hash();
        assert!(
            game.configure_mogaminator(true, LocaleDto::ZhCn, "物品".to_owned())
                .is_empty()
        );
        assert!(
            game.configure_mogaminator(true, LocaleDto::EnUs, "weapons".to_owned())
                .is_empty()
        );

        game.interface_locale = LocaleDto::ZhCn;
        assert_eq!(game.mogaminator_dto(Vec::new()).source, "物品");
        game.interface_locale = LocaleDto::EnUs;
        assert_eq!(game.mogaminator_dto(Vec::new()).source, "weapons");

        let diagnostics =
            game.configure_mogaminator(false, LocaleDto::EnUs, "rare weapons".to_owned());
        assert_eq!(diagnostics.len(), 1);
        let dto = game.mogaminator_dto(Vec::new());
        assert!(dto.enabled);
        assert_eq!(dto.source, "weapons");
        assert_ne!(game.state_hash(), initial_hash);

        let restored =
            Game::from_save(game.to_save()).expect("Mogaminator state should round-trip");
        assert_eq!(restored.interface_locale, LocaleDto::EnUs);
        assert_eq!(restored.mogaminator, game.mogaminator);
    }

    #[test]
    fn enabled_rules_return_the_first_match_without_executing_it() {
        let mut game = Game::new(11);
        game.interface_locale = LocaleDto::ZhCn;
        assert!(
            game.configure_mogaminator(true, LocaleDto::ZhCn, "~物品\n!物品".to_owned())
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
}
