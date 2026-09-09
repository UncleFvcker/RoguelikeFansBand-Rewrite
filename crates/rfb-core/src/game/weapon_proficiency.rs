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
    build: Option<&CharacterBuildIdentity>,
    progress: &CharacterProgress,
    item_kind_id: &str,
) -> Option<ResolvedWeaponProficiency> {
    let build = build?;
    let profile = content
        .class(&build.class_id)?
        .weapon_proficiency
        .as_ref()?;
    let item = content.item(item_kind_id)?;
    let base_item_id = item
        .weapon_proficiency_base_item_id
        .as_deref()
        .unwrap_or(item.id.as_str());
    let base_item = content.item(base_item_id)?;
    if base_item.melee_profile.is_none() && base_item.projectile_profile.is_none() {
        return None;
    }
    let mut bounds = profile.overrides.get(base_item_id).copied().unwrap_or(
        rfb_content::WeaponProficiencyBoundsDefinition {
            initial: profile.default_initial,
            maximum: profile.default_maximum,
        },
    );
    // RFB master a0d92b6378: skills.c::skills_weapon_max uses the native race (prace).
    if build.race_id == "rfb-legacy.race.tonberry" && base_item_id == "demo.item.sabre" {
        bounds.maximum = WEAPON_EXP_MASTER;
    }
    if let Some(maximum) = content
        .mutations()
        .filter(|mutation| progress.active_mutation_ids.contains(&mutation.id))
        .filter_map(|mutation| mutation.weapon_proficiency_maximum)
        .max()
    {
        bounds.maximum = bounds.maximum.max(maximum);
    }
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
            .unwrap_or(bounds.initial)
            .min(bounds.maximum),
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
    build: Option<&CharacterBuildIdentity>,
    progress: &CharacterProgress,
) -> bool {
    if progress.dual_wielding_proficiency
        > build
            .and_then(|build| content.class(&build.class_id))
            .map_or(0, |class| class.dual_wielding_maximum)
    {
        return false;
    }
    if build
        .and_then(|build| content.class(&build.class_id))
        .and_then(|class| class.weapon_proficiency.as_ref())
        .is_none()
    {
        return progress.weapon_proficiencies.is_empty();
    }
    progress
        .weapon_proficiencies
        .iter()
        .all(|(item_id, current)| {
            resolve_weapon_proficiency(content, build, progress, item_id).is_some_and(|resolved| {
                resolved.base_item_id == *item_id
                    && *current > resolved.initial
                    && *current <= WEAPON_EXP_MASTER
            })
        })
}

impl Game {
    pub(super) fn train_dual_wielding(&mut self, monster_level: u32) {
        let maximum = self
            .character_definitions()
            .map_or(0, |(_, _, class, _)| class.dual_wielding_maximum);
        let current = self.progress.dual_wielding_proficiency;
        if current >= maximum || (i32::from(current) - 1000) / 200 >= monster_level as i32 {
            return;
        }
        let gain = match current {
            0..4000 => 80,
            4000..6000 => 4,
            6000..7000 => 1,
            _ => u16::from(self.rng.bounded(3) == 0),
        };
        self.progress.dual_wielding_proficiency = (current + gain).min(maximum);
    }

    pub(super) fn dual_wielding_accuracy_per_mille(&self, item_id: &str) -> i32 {
        let weapons = self.equipped_melee_weapons();
        let Some(hand) = weapons.iter().position(|item| item.id == item_id) else {
            return 1000;
        };
        let pair_start = hand / 2 * 2;
        if weapons.len() <= pair_start + 1 {
            return 1000;
        }
        let item = weapons[hand];
        let genji = self
            .player_equipment_passives()
            .contains(&EquipmentPassive::DualWielding);
        let mut weight = i32::from(self.item_instance_weight(item));
        let mut percent = 650 * i32::from(self.progress.dual_wielding_proficiency) / 8000;
        if genji {
            percent += 150;
            if weight >= 130 {
                weight -= (weight - 130) / 2;
            }
        }
        let divisor =
            self.character_definitions()
                .map_or(8, |(_, _, class, _)| match class.id.as_str() {
                    "demo.class.warrior" => 18,
                    "demo.class.paladin" => 12,
                    _ => 8,
                });
        if self
            .content
            .item(&weapons[pair_start + 1].kind_id)
            .and_then(|item| item.rfb_base_kind)
            .is_some_and(|base| base.tval == 23 && matches!(base.sval, 5 | 13))
        {
            percent += 50;
        }
        percent += 10 * (130 - weight) / divisor;
        if self
            .content
            .item(&item.kind_id)
            .and_then(|item| item.rfb_base_kind)
            .is_some_and(|base| base.tval == 22)
            && self.item_instance_weight(item) > 100
        {
            percent -= 50;
        }
        let percent = percent.clamp(100, 1000);
        if hand >= 2 {
            (percent * 90 / 100).max(100)
        } else {
            percent
        }
    }

    fn weapon_proficiency(&self, item_kind_id: &str) -> Option<ResolvedWeaponProficiency> {
        resolve_weapon_proficiency(
            &self.content,
            self.build.as_ref(),
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
        let equipped: std::collections::BTreeSet<_> = self
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            .filter_map(|item| self.weapon_proficiency(&item.kind_id))
            .map(|resolved| resolved.base_item_id)
            .collect();
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
                // RFB master: defines.h TV_* and cmd4.c _prof_weapon_heading.
                use rfb_protocol::WeaponProficiencyGroupDto as Group;
                let group = match item.rfb_base_kind.map(|kind| kind.tval) {
                    Some(23) => Group::Sword,
                    Some(22) => Group::Polearm,
                    Some(21) => Group::Hafted,
                    Some(20) => Group::Digging,
                    Some(19) => Group::Bow,
                    _ => Group::Other,
                };
                Some(rfb_protocol::WeaponProficiencyDto {
                    item_kind_id: item.id.clone(),
                    name_key: item.name_key.clone(),
                    category,
                    group,
                    equipped: equipped.contains(&resolved.base_item_id),
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
