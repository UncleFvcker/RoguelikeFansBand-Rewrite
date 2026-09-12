// SPDX-License-Identifier: MPL-2.0

use rfb_content::ContentCatalog;
use rfb_protocol::{VirtueDto, VirtueKindDto};

use crate::{rng::RfbRng, stats::CharacterBuildIdentity};

use super::Game;

pub(super) const VIRTUE_SLOT_COUNT: usize = 8;

const RANDOM_VIRTUES: [VirtueKindDto; 29] = [
    VirtueKindDto::Sacrifice,
    VirtueKindDto::Sacrifice,
    VirtueKindDto::Sacrifice,
    VirtueKindDto::Compassion,
    VirtueKindDto::Compassion,
    VirtueKindDto::Compassion,
    VirtueKindDto::Valour,
    VirtueKindDto::Valour,
    VirtueKindDto::Valour,
    VirtueKindDto::Valour,
    VirtueKindDto::Valour,
    VirtueKindDto::Valour,
    VirtueKindDto::Honour,
    VirtueKindDto::Honour,
    VirtueKindDto::Honour,
    VirtueKindDto::Honour,
    VirtueKindDto::Honour,
    VirtueKindDto::Justice,
    VirtueKindDto::Justice,
    VirtueKindDto::Justice,
    VirtueKindDto::Justice,
    VirtueKindDto::Temperance,
    VirtueKindDto::Temperance,
    VirtueKindDto::Harmony,
    VirtueKindDto::Harmony,
    VirtueKindDto::Patience,
    VirtueKindDto::Patience,
    VirtueKindDto::Patience,
    VirtueKindDto::Diligence,
];

pub(super) fn initial_virtues(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
    rng: &mut RfbRng,
) -> [VirtueDto; VIRTUE_SLOT_COUNT] {
    let mut kinds = Vec::with_capacity(VIRTUE_SLOT_COUNT);
    if let Some(identity) = identity {
        let build = content
            .build(&identity.build_id)
            .expect("resolved character build must remain available");
        let class = content
            .class(&identity.class_id)
            .expect("resolved character class must remain available");

        match class.id.as_str() {
            "demo.class.berserker" => {
                kinds.extend([VirtueKindDto::Valour, VirtueKindDto::Individualism]);
            }
            "demo.class.mindcrafter" => kinds.extend([
                VirtueKindDto::Harmony,
                VirtueKindDto::Enlightenment,
                VirtueKindDto::Patience,
            ]),
            "demo.class.mage" => {
                kinds.extend([VirtueKindDto::Knowledge, VirtueKindDto::Enchantment])
            }
            "demo.class.high-mage" => kinds.extend([
                VirtueKindDto::Enlightenment,
                VirtueKindDto::Enchantment,
                VirtueKindDto::Knowledge,
            ]),
            "demo.class.archer" | "demo.class.ranger" => {
                kinds.extend([VirtueKindDto::Nature, VirtueKindDto::Temperance]);
            }
            "demo.class.priest" => {
                kinds.extend([VirtueKindDto::Faith, VirtueKindDto::Temperance]);
            }
            "demo.class.warrior" => {
                kinds.extend([VirtueKindDto::Valour, VirtueKindDto::Honour]);
            }
            _ => {}
        }

        match identity.race_id.as_str() {
            "demo.race.rfb-human" | "rfb-legacy.race.dunadan" | "rfb-legacy.race.tonberry" => {
                kinds.push(VirtueKindDto::Individualism);
            }
            "demo.race.vampire-lord" => kinds.push(VirtueKindDto::Unlife),
            "rfb-legacy.race.barbarian" | "rfb-legacy.race.half-orc" => {
                kinds.push(VirtueKindDto::Valour);
            }
            "rfb-legacy.race.high-elf" | "rfb-legacy.race.kutar" => {
                kinds.push(VirtueKindDto::Vitality)
            }
            "rfb-legacy.race.hobbit" | "rfb-legacy.race.ogre" => {
                kinds.push(VirtueKindDto::Temperance)
            }
            "rfb-legacy.race.kobold" | "rfb-legacy.race.snotling" => {
                kinds.push(VirtueKindDto::Honour)
            }
            "rfb-legacy.race.amberite" => kinds.push(VirtueKindDto::Honour),
            "rfb-legacy.race.beastman" => kinds.push(VirtueKindDto::Chance),
            "rfb-legacy.race.dwarf" => kinds.push(VirtueKindDto::Diligence),
            "rfb-legacy.race.klackon" => kinds.push(VirtueKindDto::Diligence),
            "rfb-legacy.race.dark-elf" => kinds.push(VirtueKindDto::Enchantment),
            "rfb-legacy.race.shadow-fairy" => kinds.push(VirtueKindDto::Enchantment),
            "rfb-legacy.race.mindflayer" => kinds.push(VirtueKindDto::Enlightenment),
            "rfb-legacy.race.imp" => kinds.push(VirtueKindDto::Faith),
            "rfb-legacy.race.golem" => kinds.push(VirtueKindDto::Justice),
            "rfb-legacy.race.archon" => kinds.push(VirtueKindDto::Justice),
            "rfb-legacy.race.sprite" => kinds.push(VirtueKindDto::Nature),
            "rfb-legacy.race.einheri"
            | "rfb-legacy.race.skeleton"
            | "rfb-legacy.race.zombie"
            | "rfb-legacy.race.spectre" => kinds.push(VirtueKindDto::Unlife),
            "rfb-legacy.race.wood-elf" | "rfb-legacy.race.ent" => kinds.push(VirtueKindDto::Nature),
            "rfb-legacy.race.draconian-red"
            | "rfb-legacy.race.draconian-white"
            | "rfb-legacy.race.draconian-blue"
            | "rfb-legacy.race.draconian-black"
            | "rfb-legacy.race.draconian-green"
            | "rfb-legacy.race.draconian-bronze"
            | "rfb-legacy.race.draconian-crystal"
            | "rfb-legacy.race.draconian-gold"
            | "rfb-legacy.race.draconian-shadow" => {
                kinds.push(VirtueKindDto::Enchantment);
            }
            "rfb-legacy.race.nibelung" => kinds.push(VirtueKindDto::Patience),
            "rfb-legacy.race.gnome" | "rfb-legacy.race.cyclops" | "rfb-legacy.race.tomte" => {
                kinds.push(VirtueKindDto::Knowledge);
            }
            "rfb-legacy.race.half-giant" => kinds.push(VirtueKindDto::Justice),
            "rfb-legacy.race.half-troll" => kinds.push(VirtueKindDto::Valour),
            "rfb-legacy.race.half-titan" => kinds.push(VirtueKindDto::Harmony),
            "rfb-legacy.race.boit" | "rfb-legacy.race.yeek" => kinds.push(VirtueKindDto::Sacrifice),
            _ => {}
        }

        for realm in [
            build.first_realm_id.as_deref(),
            build.second_realm_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(kind) = realm_virtue(realm, &kinds) {
                kinds.push(kind);
            }
        }
    }

    for index in 0..kinds.len() {
        if kinds[..index].contains(&kinds[index]) {
            kinds[index] = random_virtue(&kinds, rng);
        }
    }
    while kinds.len() < VIRTUE_SLOT_COUNT {
        let kind = random_virtue(&kinds, rng);
        kinds.push(kind);
    }

    std::array::from_fn(|index| VirtueDto {
        kind: kinds[index],
        value: 0,
    })
}

fn realm_virtue(realm: &str, present: &[VirtueKindDto]) -> Option<VirtueKindDto> {
    let contains = |kind| present.contains(&kind);
    match realm {
        "life" => Some(if contains(VirtueKindDto::Vitality) {
            VirtueKindDto::Temperance
        } else {
            VirtueKindDto::Vitality
        }),
        "sorcery" => Some(if contains(VirtueKindDto::Knowledge) {
            VirtueKindDto::Enchantment
        } else {
            VirtueKindDto::Knowledge
        }),
        "nature" => Some(if contains(VirtueKindDto::Nature) {
            VirtueKindDto::Harmony
        } else {
            VirtueKindDto::Nature
        }),
        "chaos" => Some(if contains(VirtueKindDto::Chance) {
            VirtueKindDto::Individualism
        } else {
            VirtueKindDto::Chance
        }),
        "death" | "necromancy" => Some(VirtueKindDto::Unlife),
        "trump" => Some(VirtueKindDto::Knowledge),
        "arcane" => None,
        "craft" => Some(if contains(VirtueKindDto::Enchantment) {
            VirtueKindDto::Individualism
        } else {
            VirtueKindDto::Enchantment
        }),
        "daemon" => Some(if contains(VirtueKindDto::Justice) {
            VirtueKindDto::Faith
        } else {
            VirtueKindDto::Justice
        }),
        "crusade" => Some(if contains(VirtueKindDto::Justice) {
            VirtueKindDto::Honour
        } else {
            VirtueKindDto::Justice
        }),
        "hex" => Some(if contains(VirtueKindDto::Compassion) {
            VirtueKindDto::Justice
        } else {
            VirtueKindDto::Compassion
        }),
        _ => None,
    }
}

fn random_virtue(present: &[VirtueKindDto], rng: &mut RfbRng) -> VirtueKindDto {
    loop {
        let kind = RANDOM_VIRTUES
            [usize::try_from(rng.bounded(RANDOM_VIRTUES.len() as u64)).expect("roll must fit")];
        if !present.contains(&kind) {
            return kind;
        }
    }
}

pub(super) fn validate_virtues(virtues: &[VirtueDto]) -> bool {
    virtues.len() == VIRTUE_SLOT_COUNT
        && virtues
            .iter()
            .all(|virtue| (-125..=125).contains(&virtue.value))
        && virtues.iter().enumerate().all(|(index, virtue)| {
            !virtues[..index]
                .iter()
                .any(|other| other.kind == virtue.kind)
        })
}

impl Game {
    pub(super) fn apply_book_spell_cast_virtues(
        &mut self,
        ability_id: &str,
        cost: u32,
        fail: u8,
        first: bool,
    ) {
        use VirtueKindDto::*;
        let realm = self
            .book_spell_realm(ability_id)
            .expect("active book spell");
        let changes: &[(VirtueKindDto, i16)] = match realm {
            "life" => &[
                (Temperance, 1),
                (Compassion, 1),
                (Vitality, 1),
                (Diligence, 1),
            ],
            "death" => &[(Unlife, 1), (Justice, -1), (Faith, -1), (Vitality, -1)],
            "daemon" => &[(Justice, -1), (Faith, -1), (Honour, -1), (Temperance, -1)],
            "crusade" => &[(Faith, 1), (Justice, 1), (Sacrifice, 1), (Honour, 1)],
            "nature" => &[(Nature, 1), (Harmony, 1)],
            _ => &[],
        };
        if first {
            if changes.is_empty() {
                self.add_virtue(Knowledge, 1);
            } else {
                for &(kind, amount) in changes {
                    self.add_virtue(kind, amount);
                }
            }
        }
        for &(kind, amount) in changes {
            if self.rng.bounded(100 + u64::from(self.progress.level)) + 1 < u64::from(cost) {
                self.add_virtue(kind, amount);
            }
        }
        if self.rng.bounded(100) + 1 < u64::from(fail) {
            self.add_virtue(Chance, 1);
        }
    }

    pub(super) fn apply_book_spell_failure_virtues(&mut self, realm: &str, fail: u8) {
        use VirtueKindDto::*;
        let (kind, amount) = match realm {
            "life" => (Vitality, -1),
            "death" => (Unlife, -1),
            "nature" => (Nature, -1),
            "daemon" => (Justice, 1),
            "crusade" => (Justice, -1),
            _ => (Knowledge, -1),
        };
        if self.rng.bounded(100) + 1 < u64::from(fail) {
            self.add_virtue(kind, amount);
        }
        if self.rng.bounded(100) + 1 >= u64::from(fail) {
            self.add_virtue(Chance, -1);
        }
    }

    pub(super) fn book_spell_alignment_modifier(&self, ability_id: &str) -> i32 {
        use VirtueKindDto::*;
        let mut alignment: i32 = self
            .entities
            .iter()
            .filter(|actor| self.actor_is_player_aligned(actor))
            .filter_map(|actor| self.actor_runtime_definition(actor))
            .map(|kind| {
                kind.level as i32
                    * (i32::from(kind.tags.iter().any(|tag| tag == "good"))
                        - i32::from(kind.tags.iter().any(|tag| tag == "evil")))
            })
            .sum();
        for virtue in &self.virtues {
            alignment += i32::from(virtue.value)
                * match virtue.kind {
                    Justice => 2,
                    Chance | Nature | Harmony => 0,
                    Unlife => -1,
                    _ => 1,
                };
        }
        for virtue in self
            .virtues
            .iter()
            .filter(|virtue| matches!(virtue.kind, Nature | Harmony))
        {
            alignment = if alignment > 0 {
                (alignment - i32::from(virtue.value) / 2).max(0)
            } else if alignment < 0 {
                (alignment + i32::from(virtue.value) / 2).min(0)
            } else {
                0
            };
        }
        // RFB virtue.c::virtue_mod_spell_fail: WIS casters have a 10% ceiling.
        let maximum = if self.casting_profile().is_some_and(|profile| {
            profile.casting_attribute == rfb_content::CastingAttribute::Wisdom
        }) {
            10
        } else {
            5
        };
        match self.book_spell_realm(ability_id) {
            Some("nature") if alignment.abs() > 50 => {
                (1 + (alignment.abs() - 51) * (maximum - 1) / 150).min(maximum)
            }
            Some("life" | "crusade") if alignment < -20 => {
                (1 + (-alignment - 21) * (maximum - 1) / 130).min(maximum)
            }
            Some("life" | "crusade") if alignment > 150 => -1,
            Some("death" | "daemon") if alignment > 20 => {
                (1 + (alignment - 21) * (maximum - 1) / 130).min(maximum)
            }
            Some("death" | "daemon") if alignment < -150 => -1,
            _ => 0,
        }
    }

    pub(super) fn apply_invulnerability_opening_virtues(&mut self) {
        self.add_virtue(VirtueKindDto::Unlife, -2);
        self.add_virtue(VirtueKindDto::Honour, -2);
        self.add_virtue(VirtueKindDto::Sacrifice, -3);
        self.add_virtue(VirtueKindDto::Valour, -5);
    }

    pub(super) fn virtue_current(&self, kind: VirtueKindDto) -> i16 {
        self.virtues
            .iter()
            .find(|virtue| virtue.kind == kind)
            .map_or(0, |virtue| virtue.value)
    }

    pub(super) fn add_virtue(&mut self, kind: VirtueKindDto, amount: i16) {
        let Some(index) = self.virtues.iter().position(|virtue| virtue.kind == kind) else {
            return;
        };
        let current = self.virtues[index].value;
        let attempted = i32::from(current) + i32::from(amount);
        if amount > 0 {
            for threshold in [50_i16, 80, 100] {
                if attempted > i32::from(threshold) && self.rng.bounded(2) == 0 {
                    self.virtues[index].value = current.max(threshold);
                    return;
                }
            }
        } else {
            for threshold in [-50_i16, -80, -100] {
                if attempted < i32::from(threshold) && self.rng.bounded(2) == 0 {
                    self.virtues[index].value = current.min(threshold);
                    return;
                }
            }
        }
        self.virtues[index].value =
            i16::try_from(attempted.clamp(-125, 125)).expect("bounded virtue value must fit i16");
    }

    pub(super) fn adjust_roll_by_chance_virtue(&mut self, mut roll: i32) -> i32 {
        let chance = self.virtue_current(VirtueKindDto::Chance);
        if chance > 0 {
            while self.rng.bounded(400) + 1 < u64::from(chance.unsigned_abs()) {
                roll = roll.saturating_add(1);
            }
        } else if chance < 0 {
            while self.rng.bounded(400) + 1 < u64::from(chance.unsigned_abs()) {
                roll = roll.saturating_sub(1);
            }
        }
        roll
    }
}
