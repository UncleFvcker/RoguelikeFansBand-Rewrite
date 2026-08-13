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
            "demo.class.high-mage" => kinds.extend([
                VirtueKindDto::Enlightenment,
                VirtueKindDto::Enchantment,
                VirtueKindDto::Knowledge,
            ]),
            "demo.class.archer" => {
                kinds.extend([VirtueKindDto::Nature, VirtueKindDto::Temperance]);
            }
            "demo.class.warrior" => {
                kinds.extend([VirtueKindDto::Valour, VirtueKindDto::Honour]);
            }
            _ => {}
        }

        match identity.race_id.as_str() {
            "demo.race.rfb-human" => kinds.push(VirtueKindDto::Individualism),
            "demo.race.vampire-lord" => kinds.push(VirtueKindDto::Unlife),
            "rfb-legacy.race.half-orc" => kinds.push(VirtueKindDto::Valour),
            "rfb-legacy.race.high-elf" => kinds.push(VirtueKindDto::Vitality),
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
