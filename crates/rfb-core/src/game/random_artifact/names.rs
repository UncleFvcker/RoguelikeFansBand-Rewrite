// Adapted from RFB master; upstream copyright and terms are preserved in NOTICE.
use super::Generator;
use crate::rng::RfbRng;
use std::collections::BTreeSet;

/// quark 0 is null; quark 1 is the empty emergency string.
pub(super) const QUARK_CAPACITY: usize = 2047;

pub(super) fn intern(quarks: &mut BTreeSet<String>, name: String) -> String {
    if quarks.contains(&name) {
        return name;
    }
    if quarks.len() == QUARK_CAPACITY {
        return String::new();
    }
    quarks.insert(name.clone());
    name
}

/// files.c:get_rnd_line: adjacent N: aliases belong to the same block.
/// Reservoir sampling consumes one draw per line (apart from bounded(1)).
pub(super) fn random_line(rng: &mut RfbRng, table: &str, entry: i32) -> String {
    let mut lines = table.trim_start_matches('\u{feff}').lines();
    if !lines.by_ref().any(|line| {
        line.strip_prefix("N:").is_some_and(|rest| {
            rest.starts_with('*')
                || rest
                    .split(':')
                    .next()
                    .and_then(|n| n.trim().parse::<i32>().ok())
                    == Some(entry)
        })
    }) {
        return String::new();
    }
    let mut result = String::new();
    let mut count = 0;
    for line in lines {
        if line.starts_with("N:") || line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            break;
        }
        count += 1;
        if count == 1 || rng.bounded(count) == 0 {
            result = line.to_owned();
        }
    }
    result
}

impl Generator<'_, '_> {
    fn name_table(&mut self, power: usize) -> (&'static str, i32, usize) {
        let bias = self.bias as i32;
        match self.object.tval {
            39 => (
                if self.has("DARKNESS") {
                    "lite_drk.txt"
                } else {
                    [
                        "lite_cursed.txt",
                        "lite_low.txt",
                        "lite_med.txt",
                        "lite_high.txt",
                    ][power]
                },
                bias,
                40,
            ),
            45 => (
                [
                    "ring_cursed.txt",
                    "ring_low.txt",
                    "ring_med.txt",
                    "ring_high.txt",
                ][power],
                bias,
                60,
            ),
            40 => (
                [
                    "amu_cursed.txt",
                    "amu_low.txt",
                    "amu_med.txt",
                    "amu_high.txt",
                ][power],
                bias,
                80,
            ),
            19 => (
                "ranged.txt",
                if self.one(2) {
                    i32::from(self.object.sval)
                } else {
                    0
                },
                60,
            ),
            30..=38 => {
                let file = if self.zero(100) < 40 {
                    match self.object.tval {
                        36..=38 => "aa_med.txt",
                        30 => "ab_med.txt",
                        35 => "ac_med.txt",
                        31 => "ag_med.txt",
                        32 | 33 => "ah_med.txt",
                        _ => "as_med.txt",
                    }
                } else {
                    ["a_cursed.txt", "a_med.txt", "a_med.txt", "a_high.txt"][power]
                };
                (file, bias, 25)
            }
            _ => {
                let odds = if self.object.tval == 20 {
                    2
                } else if bias == 0 {
                    6
                } else {
                    17
                };
                if self.one(odds) {
                    return (
                        "w_types.txt",
                        i32::from(self.object.tval) * 100 + i32::from(self.object.sval),
                        25,
                    );
                }
                let file = if self.object.tval != 20
                    && self.zero(100) < if power == 2 { 33 } else { 55 }
                {
                    match self.object.tval {
                        23 => "w_sword.txt",
                        21 => "w_hafted.txt",
                        _ => "w_pole.txt",
                    }
                } else {
                    ["w_cursed.txt", "w_med.txt", "w_med.txt", "w_high.txt"][power]
                };
                (file, bias, 25)
            }
        }
    }

    pub(super) fn random_name(&mut self, quarks: &BTreeSet<String>, power: usize) -> String {
        let mut name = String::new();
        for _ in 0..102 {
            let (file, entry, attempts) = self.name_table(power);
            let table = &self.data.name_tables[file];
            for _ in 0..attempts {
                name = random_line(self.rng, table, entry);
                if quarks.len() == QUARK_CAPACITY || !quarks.contains(&name) {
                    break;
                }
            }
            if !name.is_empty() {
                break;
            }
        }
        name
    }
}
