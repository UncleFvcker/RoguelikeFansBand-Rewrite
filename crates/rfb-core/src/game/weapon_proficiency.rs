// SPDX-License-Identifier: MPL-2.0

use super::*;

const WEAPON_EXP_BEGINNER: u16 = 4_000;
const WEAPON_EXP_MASTER: u16 = 8_000;
const WEAPON_GAIN: [(i32, i32); 9] = [
    (0, 1_280),
    (1_000, 640),
    (2_000, 320),
    (3_000, 160),
    (4_000, 80),
    (5_000, 40),
    (6_000, 20),
    (7_000, 10),
    (8_000, 1),
];
const MONSTER_SKILL_CEILING: [(i32, i32); 5] = [
    (1, 2_000),
    (20, 5_000),
    (30, 6_000),
    (60, 7_500),
    (80, 8_000),
];
const PLAYER_MINIMUM_MONSTER_LEVEL: [(i32, i32); 6] =
    [(20, 1), (30, 10), (35, 15), (40, 25), (45, 30), (50, 35)];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedWeaponProficiency {
    base_item_id: String,
    initial: u16,
    maximum: u16,
    current: u16,
    crossbow: bool,
}

fn interpolate(value: i32, table: &[(i32, i32)]) -> i32 {
    let Some(&(first_x, first_y)) = table.first() else {
        return 0;
    };
    if value < first_x {
        return first_y;
    }
    for pair in table.windows(2) {
        let (left_x, left_y) = pair[0];
        let (right_x, right_y) = pair[1];
        if value < right_x {
            return left_y + (value - left_x) * (right_y - left_y) / (right_x - left_x);
        }
    }
    table.last().map_or(0, |(_, result)| *result)
}

fn resolve_weapon_proficiency(
    content: &ContentCatalog,
    class_id: Option<&str>,
    progress: &CharacterProgress,
    item_kind_id: &str,
) -> Option<ResolvedWeaponProficiency> {
    let profile = content.class(class_id?)?.weapon_proficiency.as_ref()?;
    let item = content.item(item_kind_id)?;
    let base_item_id = item
        .weapon_proficiency_base_item_id
        .as_deref()
        .unwrap_or(item.id.as_str());
    let base_item = content.item(base_item_id)?;
    if base_item.melee_profile.is_none() && base_item.projectile_profile.is_none() {
        return None;
    }
    let bounds = profile.overrides.get(base_item_id).copied().unwrap_or(
        rfb_content::WeaponProficiencyBoundsDefinition {
            initial: profile.default_initial,
            maximum: profile.default_maximum,
        },
    );
    let crossbow = base_item
        .projectile_profile
        .as_ref()
        .is_some_and(|launcher| {
            launcher.ammunition_type == rfb_content::AmmunitionTypeDefinition::Bolt
        });
    Some(ResolvedWeaponProficiency {
        base_item_id: base_item_id.to_owned(),
        initial: bounds.initial,
        maximum: bounds.maximum,
        current: progress
            .weapon_proficiencies
            .get(base_item_id)
            .copied()
            .unwrap_or(bounds.initial),
        crossbow,
    })
}

fn proficiency_bonus(current: u16, crossbow: bool) -> i32 {
    if crossbow {
        i32::from(current) / 400
    } else {
        (i32::from(current) - i32::from(WEAPON_EXP_BEGINNER)) / 200
    }
}

pub(super) fn proficiency_rank(current: u16) -> rfb_protocol::ProficiencyRankDto {
    use rfb_protocol::ProficiencyRankDto;

    match current {
        ..4_000 => ProficiencyRankDto::Unskilled,
        4_000..6_000 => ProficiencyRankDto::Beginner,
        6_000..7_000 => ProficiencyRankDto::Skilled,
        7_000..8_000 => ProficiencyRankDto::Expert,
        _ => ProficiencyRankDto::Master,
    }
}

pub(super) fn weapon_proficiency_progress_is_valid(
    content: &ContentCatalog,
    class_id: Option<&str>,
    progress: &CharacterProgress,
) -> bool {
    if class_id
        .and_then(|id| content.class(id))
        .and_then(|class| class.weapon_proficiency.as_ref())
        .is_none()
    {
        return progress.weapon_proficiencies.is_empty();
    }
    progress
        .weapon_proficiencies
        .iter()
        .all(|(item_id, current)| {
            resolve_weapon_proficiency(content, class_id, progress, item_id).is_some_and(
                |resolved| {
                    resolved.base_item_id == *item_id
                        && *current > resolved.initial
                        && *current <= resolved.maximum
                        && *current <= WEAPON_EXP_MASTER
                },
            )
        })
}

impl Game {
    fn weapon_proficiency(&self, item_kind_id: &str) -> Option<ResolvedWeaponProficiency> {
        resolve_weapon_proficiency(
            &self.content,
            self.build.as_ref().map(|build| build.class_id.as_str()),
            &self.progress,
            item_kind_id,
        )
    }

    pub(super) fn weapon_proficiency_hit_modifier(
        &self,
        item_kind_id: &str,
    ) -> Option<(String, i32)> {
        let resolved = self.weapon_proficiency(item_kind_id)?;
        Some((
            resolved.base_item_id,
            proficiency_bonus(resolved.current, resolved.crossbow).saturating_mul(3),
        ))
    }

    pub(super) fn player_weapon_proficiencies(&self) -> Vec<rfb_protocol::WeaponProficiencyDto> {
        self.content
            .item_definitions()
            .filter(|item| item.weapon_proficiency_base_item_id.is_none())
            .filter_map(|item| {
                let category = if item.projectile_profile.is_some() {
                    rfb_protocol::WeaponProficiencyCategoryDto::Launcher
                } else if item.melee_profile.is_some() {
                    rfb_protocol::WeaponProficiencyCategoryDto::Melee
                } else {
                    return None;
                };
                let resolved = self.weapon_proficiency(&item.id)?;
                Some(rfb_protocol::WeaponProficiencyDto {
                    item_kind_id: item.id.clone(),
                    name_key: item.name_key.clone(),
                    category,
                    rank: proficiency_rank(resolved.current),
                    current: resolved.current,
                    maximum: resolved.maximum,
                    hit_bonus: proficiency_bonus(resolved.current, resolved.crossbow),
                })
            })
            .collect()
    }

    pub(super) fn train_weapon_proficiency(
        &mut self,
        item_instance_id: &str,
        monster_level: u32,
    ) -> Option<String> {
        let item_kind_id = self
            .items
            .iter()
            .find(|item| item.id == item_instance_id)?
            .kind_id
            .clone();
        let resolved = self.weapon_proficiency(&item_kind_id)?;
        if resolved.current >= resolved.maximum
            || i32::try_from(monster_level).unwrap_or(i32::MAX)
                < interpolate(
                    i32::from(self.progress.level),
                    &PLAYER_MINIMUM_MONSTER_LEVEL,
                )
            || i32::from(resolved.current)
                >= interpolate(
                    i32::try_from(monster_level).unwrap_or(i32::MAX),
                    &MONSTER_SKILL_CEILING,
                )
        {
            return None;
        }

        let step = interpolate(i32::from(resolved.current), &WEAPON_GAIN);
        let mut increase = step / 10;
        let remainder = step % 10;
        if remainder != 0
            && self.rng.bounded(10) < u64::try_from(remainder).expect("positive remainder")
        {
            increase += 1;
        }
        if increase <= 0 {
            return None;
        }
        let next = resolved
            .current
            .saturating_add(u16::try_from(increase).expect("weapon gain must fit u16"))
            .min(resolved.maximum);
        if next == resolved.current {
            return None;
        }
        let old_bonus = proficiency_bonus(resolved.current, resolved.crossbow);
        let new_bonus = proficiency_bonus(next, resolved.crossbow);
        self.progress
            .weapon_proficiencies
            .insert(resolved.base_item_id.clone(), next);
        (old_bonus != new_bonus).then_some(resolved.base_item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_interpolation_clamps_and_truncates_like_rfb() {
        assert_eq!(interpolate(-1, &WEAPON_GAIN), 1_280);
        assert_eq!(interpolate(4_000, &WEAPON_GAIN), 80);
        assert_eq!(interpolate(4_500, &WEAPON_GAIN), 60);
        assert_eq!(interpolate(9_000, &WEAPON_GAIN), 1);
    }

    #[test]
    fn bows_and_crossbows_keep_their_distinct_original_bonus_formulas() {
        assert_eq!(proficiency_bonus(2_000, false), -10);
        assert_eq!(proficiency_bonus(4_000, false), 0);
        assert_eq!(proficiency_bonus(8_000, false), 20);
        assert_eq!(proficiency_bonus(2_000, true), 5);
        assert_eq!(proficiency_bonus(8_000, true), 20);
    }

    #[test]
    fn original_rank_boundaries_are_projected_exactly() {
        use rfb_protocol::ProficiencyRankDto;

        assert_eq!(proficiency_rank(3_999), ProficiencyRankDto::Unskilled);
        assert_eq!(proficiency_rank(4_000), ProficiencyRankDto::Beginner);
        assert_eq!(proficiency_rank(6_000), ProficiencyRankDto::Skilled);
        assert_eq!(proficiency_rank(7_000), ProficiencyRankDto::Expert);
        assert_eq!(proficiency_rank(8_000), ProficiencyRankDto::Master);
    }

    #[test]
    fn active_class_supplies_distinct_birth_values_and_training_caps() {
        for (build_id, expected) in [
            ("demo.build.warrior", (4_000, 7_000)),
            ("demo.build.high-mage-death", (2_000, 4_000)),
            ("demo.build.archer", (4_000, 8_000)),
            ("demo.build.paladin-death", (2_000, 6_000)),
            ("demo.build.cavalry", (4_000, 8_000)),
        ] {
            let game = Game::new_with_build(1, build_id).expect("official build should create");
            let resolved = game
                .weapon_proficiency("demo.item.short-bow")
                .expect("short bow should have class proficiency");
            assert_eq!((resolved.initial, resolved.maximum), expected, "{build_id}");
            assert_eq!(resolved.current, resolved.initial, "{build_id}");
        }
    }
}
