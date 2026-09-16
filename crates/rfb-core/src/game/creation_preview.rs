// SPDX-License-Identifier: MPL-2.0

use rfb_content::{CastingAttribute, ContentCatalog, SkillKind};
use rfb_protocol::{
    AttributeKindDto, AttributeSourceKindDto, CharacterCreationPreviewDto, CreationAttributeDto,
    CreationSkillDto, CreationSourceDto, PlayerBuildDto, SkillProgressDto,
};

use super::{
    Game, load_built_in_content,
    progression::{
        CharacterDefinitions, build_attribute_steps, build_definitions,
        character_experience_percent, character_skill_progress, combine_percentages,
        effective_attributes, initial_character_attributes, resolve_character_build,
        validate_birth_combination,
    },
};
use crate::{
    CoreError,
    stats::{AttributeKind, CharacterProgress},
};

impl Game {
    /// Reads definitions only: no Game, world, RNG or character record is created.
    pub fn preview_character_creation(
        build_id: &str,
        race_id: &str,
    ) -> Result<CharacterCreationPreviewDto, CoreError> {
        let content = load_built_in_content()?;
        let identity = resolve_character_build(&content, Some(build_id), Some(race_id))?
            .expect("an explicit validated build resolves an identity");
        validate_birth_combination(&identity)?;
        let definitions = build_definitions(&content, &identity)?;
        let (build, race, class, personality) = definitions;
        let natural = initial_character_attributes(build);
        let kinds = [
            (AttributeKind::Strength, AttributeKindDto::Strength),
            (AttributeKind::Intelligence, AttributeKindDto::Intelligence),
            (AttributeKind::Wisdom, AttributeKindDto::Wisdom),
            (AttributeKind::Dexterity, AttributeKindDto::Dexterity),
            (AttributeKind::Constitution, AttributeKindDto::Constitution),
            (AttributeKind::Charisma, AttributeKindDto::Charisma),
        ];
        let mut attributes = kinds
            .iter()
            .map(|(kind, dto)| CreationAttributeDto {
                attribute: *dto,
                natural: natural.value(*kind),
                modifier: 0,
                effective: 0,
            })
            .collect::<Vec<_>>();
        let effective = effective_attributes(
            natural,
            build_attribute_steps(Some(definitions), 1, 0),
            None,
            CharacterProgress::attribute_cap(false),
            |step, _, _| {
                let m = step.modifiers;
                for (row, modifier) in attributes.iter_mut().zip([
                    m.strength,
                    m.intelligence,
                    m.wisdom,
                    m.dexterity,
                    m.constitution,
                    m.charisma,
                ]) {
                    row.modifier += modifier;
                }
            },
        );
        for (row, (kind, _)) in attributes.iter_mut().zip(kinds) {
            row.effective = effective.value(kind);
        }
        let progress = character_skill_progress(&content, Some(&identity), 1)?;
        let mut skills = Vec::new();
        for kind in [
            SkillKind::Disarming,
            SkillKind::Device,
            SkillKind::SavingThrow,
            SkillKind::Stealth,
            SkillKind::Search,
            SkillKind::Perception,
            SkillKind::Melee,
            SkillKind::Ranged,
        ] {
            let definition = content
                .skill_by_kind(kind)
                .ok_or_else(|| CoreError::UnknownCharacterBuild(build_id.to_owned()))?;
            let skill = progress
                .get(&definition.id)
                .ok_or_else(|| CoreError::UnknownCharacterBuild(build_id.to_owned()))?;
            skills.push(CreationSkillDto {
                skill: SkillProgressDto {
                    id: definition.id.clone(),
                    name_key: definition.name_key.clone(),
                    current: skill.current,
                    maximum: skill.maximum,
                    base: skill.base,
                    growth_per_ten_levels: skill.growth_per_ten_levels,
                },
                rating_key: skill_rating(kind, skill.base, skill.growth_per_ten_levels).into(),
            });
        }
        Ok(CharacterCreationPreviewDto {
            build: PlayerBuildDto {
                build_id: build.id.clone(),
                build_name_key: build.name_key.clone(),
                race_id: race.id.clone(),
                race_name_key: race.name_key.clone(),
                class_id: class.id.clone(),
                class_name_key: class.name_key.clone(),
                personality_id: personality.id.clone(),
                personality_name_key: personality.name_key.clone(),
                life_percent: combine_percentages([
                    race.life_percent,
                    class.life_percent,
                    personality.life_percent,
                ]),
                experience_percent: character_experience_percent(Some(definitions)),
            },
            attributes,
            skills,
            base_hp: race
                .base_hp
                .saturating_add(class.base_hp)
                .saturating_add(personality.base_hp),
            casting_attribute: class.casting_profile.as_ref().map(|profile| {
                match profile.casting_attribute {
                    CastingAttribute::Strength => AttributeKindDto::Strength,
                    CastingAttribute::Intelligence => AttributeKindDto::Intelligence,
                    CastingAttribute::Wisdom => AttributeKindDto::Wisdom,
                    CastingAttribute::Dexterity => AttributeKindDto::Dexterity,
                    CastingAttribute::Constitution => AttributeKindDto::Constitution,
                    CastingAttribute::Charisma => AttributeKindDto::Charisma,
                }
            }),
            experience_note_key: (race.id == "rfb-legacy.race.android")
                .then(|| "creation-preview-android-experience".into()),
            sources: creation_sources(&content, definitions),
            features: super::creation_traits::creation_features(&content, definitions),
        })
    }
}

fn creation_sources(
    content: &ContentCatalog,
    (_, race, class, personality): CharacterDefinitions<'_>,
) -> Vec<CreationSourceDto> {
    [
        (
            AttributeSourceKindDto::Race,
            &race.name_key,
            &race.modifiers,
            race.life_percent,
            race.experience_percent,
            race.base_hp,
            &race.skill_set_id,
        ),
        (
            AttributeSourceKindDto::Class,
            &class.name_key,
            &class.modifiers,
            class.life_percent,
            class.experience_percent,
            class.base_hp,
            &class.skill_set_id,
        ),
        (
            AttributeSourceKindDto::Personality,
            &personality.name_key,
            &personality.modifiers,
            personality.life_percent,
            personality.experience_percent,
            personality.base_hp,
            &personality.skill_set_id,
        ),
    ]
    .into_iter()
    .map(
        |(kind, name, modifiers, life_percent, experience_percent, base_hp, skill_set)| {
            let entries = &content
                .skill_set(skill_set)
                .expect("validated source skill set")
                .entries;
            let skills = entries
                .iter()
                .map(|entry| {
                    let definition = content
                        .skill(&entry.skill_id)
                        .expect("validated skill reference");
                    let skill = crate::stats::SkillProgress::at_level(
                        entry.base,
                        entry.growth_per_ten_levels,
                        definition.maximum,
                        1,
                    );
                    SkillProgressDto {
                        id: definition.id.clone(),
                        name_key: definition.name_key.clone(),
                        current: skill.current,
                        maximum: skill.maximum,
                        base: skill.base,
                        growth_per_ten_levels: skill.growth_per_ten_levels,
                    }
                })
                .collect();
            CreationSourceDto {
                kind,
                name_key: name.clone(),
                modifiers: super::stat_modifiers_dto(modifiers),
                life_percent,
                experience_percent,
                base_hp,
                skills,
            }
        },
    )
    .collect()
}

// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
// src/spoilers.c::*_skill_desc and src/skills.c::skills_describe.
// Rewrite growthPerTenLevels has the same scale as ordinary class extra_skills:
// current = base + growth * level / 10. Use combined Core growth, not current value.
fn skill_rating(kind: SkillKind, base: i32, growth: i32) -> &'static str {
    let total = i64::from(base) + 4 * i64::from(growth);
    let (amount, divisor) = match kind {
        SkillKind::Disarming => (total - 30, 7),
        SkillKind::Device => (total - 45, 4),
        SkillKind::SavingThrow => (total - 56, 4),
        SkillKind::Stealth => (total * 3 + 1, 2),
        SkillKind::Search | SkillKind::Perception => (total, 6),
        SkillKind::Melee => (total - 50, 10),
        SkillKind::Ranged => (total - 48, 8),
        _ => unreachable!("birth overview only requests the eight RFB comparison skills"),
    };
    if amount < 0 {
        return "creation-rating-very-bad";
    }
    match amount / divisor {
        0..=1 => "creation-rating-bad",
        2..=3 => "creation-rating-poor",
        4..=5 => "creation-rating-fair",
        6..=7 => "creation-rating-good",
        8 => "creation-rating-very-good",
        9..=10 => "creation-rating-excellent",
        11..=13 => "creation-rating-superb",
        14..=17 => "creation-rating-heroic",
        _ => "creation-rating-amber",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ItemLocation;

    #[test]
    fn creation_details_show_sources_and_real_unlock_conditions() {
        let berserker =
            Game::preview_character_creation("demo.build.berserker", "demo.race.rfb-human")
                .unwrap();
        assert_eq!(berserker.sources.len(), 3);
        for row in &berserker.attributes {
            let modifiers = berserker
                .sources
                .iter()
                .map(|source| match row.attribute {
                    AttributeKindDto::Strength => source.modifiers.strength,
                    AttributeKindDto::Intelligence => source.modifiers.intelligence,
                    AttributeKindDto::Wisdom => source.modifiers.wisdom,
                    AttributeKindDto::Dexterity => source.modifiers.dexterity,
                    AttributeKindDto::Constitution => source.modifiers.constitution,
                    AttributeKindDto::Charisma => source.modifiers.charisma,
                })
                .sum::<i32>();
            assert_eq!(row.modifier, modifiers);
        }
        let content = load_built_in_content().unwrap();
        let recall_name = &content
            .ability("demo.ability.berserker-recall")
            .unwrap()
            .name_key;
        let recall = berserker
            .features
            .iter()
            .find(|row| &row.name_key == recall_name)
            .unwrap();
        assert_eq!(
            (recall.minimum_level, recall.acquisition_key.as_str()),
            (10, "creation-acquire-power")
        );
        assert!(
            berserker
                .features
                .iter()
                .any(|row| row.name_key == "status-stun-name" && row.minimum_level == 35)
        );
        let android =
            Game::preview_character_creation("demo.build.warrior", "rfb-legacy.race.android")
                .unwrap();
        assert!(
            android
                .features
                .iter()
                .any(|row| row.minimum_level == 1 && row.maximum_level == Some(9))
        );
        assert!(
            android
                .features
                .iter()
                .any(|row| row.minimum_level == 35 && row.maximum_level == Some(44))
        );
        let priest = Game::preview_character_creation(
            "demo.build.priest-life-sorcery",
            "demo.race.rfb-human",
        )
        .unwrap();
        assert!(
            priest
                .features
                .iter()
                .any(|row| row.acquisition_key == "creation-acquire-prayer")
        );
        let evil_power = &content
            .ability("demo.ability.priest-evocation")
            .unwrap()
            .name_key;
        assert!(
            !priest
                .features
                .iter()
                .any(|row| &row.name_key == evil_power)
        );
    }

    #[test]
    fn creation_preview_matches_new_character_without_equipment() {
        for (build, race, casting) in [
            ("demo.build.warrior", "demo.race.rfb-human", None),
            (
                "demo.build.high-mage-death",
                "rfb-legacy.race.high-elf",
                Some(AttributeKindDto::Intelligence),
            ),
            ("demo.build.berserker", "demo.race.rfb-human", None),
            ("demo.build.warrior", "rfb-legacy.race.android", None),
        ] {
            let preview = Game::preview_character_creation(build, race).unwrap();
            let mut game = Game::new_with_build_race_and_name(
                19,
                build,
                race,
                "Preview",
                Game::default_behavior_preferences(),
            )
            .unwrap();
            game.items
                .retain(|item| !matches!(&item.location, ItemLocation::Equipped { .. }));
            let snapshot = game.snapshot();
            assert_eq!(preview.build, snapshot.player.build.unwrap());
            assert_eq!(preview.casting_attribute, casting);
            let actual = game.effective_player_attributes();
            for (row, kind) in preview.attributes.iter().zip([
                AttributeKind::Strength,
                AttributeKind::Intelligence,
                AttributeKind::Wisdom,
                AttributeKind::Dexterity,
                AttributeKind::Constitution,
                AttributeKind::Charisma,
            ]) {
                assert_eq!(row.natural, game.progress.attributes.value(kind));
                assert_eq!(row.effective, actual.value(kind), "{build}/{race}/{kind:?}");
            }
            assert_eq!(preview.skills.len(), 8);
            for row in &preview.skills {
                let actual = snapshot
                    .player
                    .progress
                    .skills
                    .iter()
                    .find(|skill| skill.id == row.skill.id)
                    .unwrap();
                assert_eq!(&row.skill, actual);
            }
            assert_eq!(
                preview.experience_note_key.is_some(),
                race == "rfb-legacy.race.android"
            );
            let before = game.snapshot();
            assert_eq!(
                preview,
                Game::preview_character_creation(build, race).unwrap()
            );
            assert_eq!(
                before,
                game.snapshot(),
                "preview must not alter an existing session"
            );
        }
    }

    #[test]
    fn creation_preview_rejects_unknown_and_birth_incompatible_choices() {
        assert!(matches!(
            Game::preview_character_creation("missing", "demo.race.rfb-human"),
            Err(CoreError::UnknownCharacterBuild(_))
        ));
        assert!(matches!(
            Game::preview_character_creation("demo.build.warrior", "missing"),
            Err(CoreError::UnknownCharacterRace(_))
        ));
        for (build, race) in [
            ("demo.build.duelist", "rfb-legacy.race.tonberry"),
            ("demo.build.cavalry", "rfb-legacy.race.centaur"),
        ] {
            assert!(matches!(
                Game::preview_character_creation(build, race),
                Err(CoreError::CharacterRaceUnavailable(_))
            ));
            assert!(matches!(
                Game::new_with_build_race_and_name(
                    19,
                    build,
                    race,
                    "Preview",
                    Game::default_behavior_preferences()
                ),
                Err(CoreError::CharacterRaceUnavailable(_))
            ));
        }
    }

    #[test]
    fn creation_skill_ratings_use_rfb_growth_and_negative_thresholds() {
        assert_eq!(
            skill_rating(SkillKind::Disarming, 25, 12),
            "creation-rating-good"
        );
        assert_eq!(
            skill_rating(SkillKind::Device, 18, 7),
            "creation-rating-bad"
        );
        assert_eq!(
            skill_rating(SkillKind::Device, 44, 0),
            "creation-rating-very-bad"
        );
        assert_eq!(
            skill_rating(SkillKind::Melee, 70, 30),
            "creation-rating-heroic"
        );
        assert_eq!(
            skill_rating(SkillKind::Stealth, 1, 0),
            "creation-rating-poor"
        );
    }
}
