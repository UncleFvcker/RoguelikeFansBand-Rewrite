// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) const RATION_ITEM_KIND_ID: &str = "demo.item.ration-of-food";
const GOLEM_RACE_ID: &str = "rfb-legacy.race.golem";
const ZOMBIE_RACE_ID: &str = "rfb-legacy.race.zombie";

const WORLD_PROCESS_INTERVAL_TICKS: u32 = 10;
const NORMAL_DIGESTION_INTERVAL_TICKS: u32 = 50;
const NUTRITION_BLOATED: u16 = 15_000;
pub(super) const NUTRITION_FULL: u16 = 10_000;
const NUTRITION_HUNGRY: u16 = 2_000;
pub(super) const NUTRITION_WEAK: u16 = 1_000;
const NUTRITION_FAINT: u16 = 500;
pub(super) const NUTRITION_STARVING: u16 = 100;
const REGENERATION_WEAK_FACTOR: u64 = 98;
const REGENERATION_FAINT_FACTOR: u64 = 33;

pub(super) fn starting_ration_quantity(
    build: Option<&CharacterBuildIdentity>,
    rng: &mut RfbRng,
) -> Option<u32> {
    build
        .is_some_and(|identity| {
            !matches!(identity.race_id.as_str(), GOLEM_RACE_ID | ZOMBIE_RACE_ID)
        })
        .then(|| u32::try_from(rng.bounded(5) + 5).expect("birth ration quantity must fit u32"))
}

impl Game {
    pub(super) const fn nutrition_state(&self) -> rfb_protocol::NutritionStateDto {
        nutrition_state(self.nutrition)
    }

    pub(super) const fn nutrition_regeneration_factor(&self) -> u64 {
        if self.nutrition >= NUTRITION_WEAK {
            NATURAL_HP_REGENERATION_FACTOR
        } else if self.nutrition < NUTRITION_STARVING {
            0
        } else if self.nutrition < NUTRITION_FAINT {
            REGENERATION_FAINT_FACTOR
        } else {
            REGENERATION_WEAK_FACTOR
        }
    }

    pub(super) fn increase_nutrition(&mut self, amount: u16) -> u16 {
        let before = self.nutrition;
        self.nutrition = self
            .nutrition
            .saturating_add(amount)
            .min(rfb_protocol::PLAYER_NUTRITION_MAXIMUM);
        if self.nutrition > before {
            self.fasting = false;
        }
        self.nutrition - before
    }

    pub(super) fn process_fasting(&mut self, events: &mut Vec<DomainEvent>) {
        if !self.fasting
            || !self.world_tick.is_multiple_of(WORLD_PROCESS_INTERVAL_TICKS)
            || self.rng.bounded(7) != 0
            || self.rng.bounded(u64::from(NUTRITION_HUNGRY)) < u64::from(self.nutrition)
        {
            return;
        }

        match self.rng.bounded(8) {
            branch @ 0..=5 => {
                let attribute = [
                    AttributeKind::Strength,
                    AttributeKind::Intelligence,
                    AttributeKind::Wisdom,
                    AttributeKind::Dexterity,
                    AttributeKind::Constitution,
                    AttributeKind::Charisma,
                ][usize::try_from(branch).expect("fasting restoration branch must fit usize")];
                self.restore_player_attribute(attribute);
            }
            6 => {
                apply_experience_restoration(&mut self.progress);
                self.apply_player_experience(0, events);
            }
            7 => {
                self.restore_player_life_force(LifeForceRestorationRequest::add(150));
            }
            _ => unreachable!("bounded fasting restoration branch must fit 1d8"),
        }
    }

    pub(super) fn process_hunger(&mut self, events: &mut Vec<DomainEvent>) {
        if !self.world_tick.is_multiple_of(WORLD_PROCESS_INTERVAL_TICKS) {
            return;
        }

        let before_state = self.nutrition_state();
        if self.nutrition >= NUTRITION_BLOATED {
            self.nutrition = self.nutrition.saturating_sub(100);
        } else if self
            .world_tick
            .is_multiple_of(NORMAL_DIGESTION_INTERVAL_TICKS)
        {
            let speed = derived_speed(&self.player_derived_stats().speed);
            let mut digestion = u16::try_from(energy_gain(speed))
                .expect("scheduler energy gain must fit nutrition")
                .clamp(1, 100);
            if self.player_slow_digestion() {
                digestion = (digestion / 2).max(1);
            }
            self.nutrition = self.nutrition.saturating_sub(digestion);
        }
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }

        if self.nutrition < NUTRITION_FAINT
            && !self.player_has_status_kind(STATUS_PARALYSIS)
            && self.rng.bounded(100) < 10
        {
            let duration = u32::try_from(self.rng.bounded(4) + 1)
                .expect("hunger paralysis duration must fit u32");
            apply_status_application(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_PARALYSIS.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: None,
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Replace,
                },
            );
            events.push(DomainEvent::PlayerFaintedFromHunger { duration });
        }

        if self.nutrition < NUTRITION_STARVING {
            let amount = i32::from((NUTRITION_STARVING - self.nutrition) / 10);
            if amount == 0 {
                return;
            }
            let damage = resolve_damage(
                DamagePacket::new(amount, DamageType::Physical),
                ResistanceLevel::Normal,
            );
            let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            let damage = application.damage;
            events.push(DomainEvent::PlayerDamagedByStarvation { damage });
            if application.fatal {
                events.push(DomainEvent::PlayerDiedFromStarvation { damage });
            }
        }
    }
}

pub(super) const fn nutrition_state(nutrition: u16) -> rfb_protocol::NutritionStateDto {
    use rfb_protocol::NutritionStateDto;

    if nutrition >= NUTRITION_BLOATED {
        NutritionStateDto::Bloated
    } else if nutrition >= NUTRITION_FULL {
        NutritionStateDto::Full
    } else if nutrition >= NUTRITION_HUNGRY {
        NutritionStateDto::Normal
    } else if nutrition >= NUTRITION_WEAK {
        NutritionStateDto::Hungry
    } else if nutrition >= NUTRITION_FAINT {
        NutritionStateDto::Weak
    } else if nutrition >= NUTRITION_STARVING {
        NutritionStateDto::Faint
    } else {
        NutritionStateDto::Starving
    }
}
