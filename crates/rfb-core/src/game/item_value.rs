// SPDX-License-Identifier: MPL-2.0
//! RFB master `object2.c:obj_value_real` / `object3.c`, COST_REAL only.
//! Scoring consumes complete object properties; it never identifies or rolls an item.

use std::collections::BTreeSet;
pub(super) mod instance;

pub(super) fn obj_value_real(
    content: &rfb_content::ContentCatalog,
    item: &crate::state::ItemInstance,
) -> Option<i32> {
    object_value(instance::value_object(content, item)?)
}

#[cfg(test)]
mod tests;

/// Source-level inputs after materialization. Base dice/multiplier are the kind's,
/// while all other numbers describe the actual object being scored.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub(super) struct ValueObject {
    pub tval: u16,
    pub sval: u16,
    pub pval: i32,
    pub flags: BTreeSet<String>,
    pub ac: i32,
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    pub base_to_h: i32,
    pub dd: i32,
    pub ds: i32,
    pub base_dd: i32,
    pub base_ds: i32,
    pub mult: i32,
    pub base_mult: i32,
    pub weight: i32,
    pub capacity: i32,
    pub ego: u32,
    pub fixed_artifact: u32,
    pub artifact: bool,
    pub permanent_curse: bool,
    pub activation_value: i32,
    pub activation_timeout: i32,
}

impl ValueObject {
    fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    fn remove_opposites(&mut self) {
        for flag in [
            "STR",
            "INT",
            "WIS",
            "DEX",
            "CON",
            "CHR",
            "SPEED",
            "STEALTH",
            "MAGIC_MASTERY",
            "LIFE",
        ] {
            let opposite = format!("DEC_{flag}");
            if self.has(flag) && self.has(&opposite) {
                self.flags.remove(flag);
                self.flags.remove(&opposite);
            }
        }
        for element in [
            "ACID", "ELEC", "FIRE", "COLD", "POIS", "LITE", "DARK", "BLIND", "CONF", "SOUND",
            "SHARDS", "NETHER", "NEXUS", "CHAOS", "DISEN", "FEAR",
        ] {
            let resistance = format!("RES_{element}");
            let vulnerability = format!("VULN_{element}");
            if self.has(&resistance) && self.has(&vulnerability) {
                self.flags.remove(&resistance);
                self.flags.remove(&vulnerability);
            }
        }
    }

    /// Each source group restarts its count. Some callers truncate after each
    /// addition (s32b accumulator), others only once (double accumulator).
    fn score_group(&self, entries: &[(&str, i32)], truncate_each: bool) -> (f64, i32) {
        let mut score = 0.0;
        let mut count = 0;
        for &(flag, amount) in entries {
            if self.has(flag) {
                let c = f64::from(count);
                score += f64::from(amount as u32) * (1.0 + c / 10.0 + c * c / 50.0);
                if truncate_each {
                    score = f64::from(c_i32(score));
                }
                count += 1;
            }
        }
        (score, count)
    }

    fn abilities(&self) -> i32 {
        let groups: &[&[(&str, i32)]] = &[
            &[
                ("THROWING", 100),
                ("WARNING", 100),
                ("LITE", 100),
                ("DARKNESS", 100),
                ("SLOW_DIGEST", 100),
                ("SEE_INVIS", 500),
                ("FREE_ACT", 750),
            ],
            &[
                ("ESP_ORC", 500),
                ("ESP_TROLL", 500),
                ("ESP_GIANT", 500),
                ("ESP_GOOD", 500),
                ("ESP_ANIMAL", 600),
                ("ESP_UNDEAD", 600),
                ("ESP_DEMON", 600),
                ("ESP_DRAGON", 700),
                ("ESP_HUMAN", 700),
                ("ESP_NONLIVING", 1500),
                ("ESP_LIVING", 1500),
            ],
            &[
                ("SUST_STR", 1000),
                ("SUST_INT", 1000),
                ("SUST_WIS", 1000),
                ("SUST_DEX", 1000),
                ("SUST_CHR", 1000),
                ("SUST_CON", 1000),
            ],
            &[
                ("LEVITATION", 1800),
                ("HOLD_LIFE", 1000),
                ("REGEN", 1000),
                ("EASY_SPELL", 2000),
                ("REGEN_MANA", 5000),
                ("LORE2", 5000),
            ],
            &[
                ("ESP_UNIQUE", 5000),
                ("REFLECT", 5000),
                ("NIGHT_VISION", 6000),
                ("ESP_EVIL", 3000),
                ("DEC_MANA", 9000),
                ("TELEPATHY", 10000),
            ],
        ];
        c_i32(
            groups
                .iter()
                .map(|group| self.score_group(group, false).0)
                .sum(),
        )
    }

    fn auras(&self) -> i32 {
        c_i32(
            self.score_group(
                &[
                    ("AURA_FIRE", 1452),
                    ("AURA_COLD", 1452),
                    ("AURA_ELEC", 1694),
                    ("AURA_SHARDS", 2420),
                    ("AURA_REVENGE", 6050),
                ],
                true,
            )
            .0,
        )
    }

    fn stats(&self) -> i32 {
        let pval = self.pval.clamp(-10, 10);
        let mult = pval * (pval + 1) / 2 + pval;
        let mut cost = 0.0;
        for (negative, entries) in [
            (
                false,
                vec![
                    ("STR", 400),
                    ("INT", 333),
                    ("WIS", 333),
                    ("DEX", 367),
                    ("CON", 433),
                    ("CHR", 267),
                ],
            ),
            (
                true,
                vec![
                    ("DEC_STR", 400),
                    ("DEC_INT", 333),
                    ("DEC_WIS", 333),
                    ("DEC_DEX", 367),
                    ("DEC_CON", 433),
                    ("DEC_CHR", 267),
                ],
            ),
            (
                false,
                vec![
                    ("MAGIC_MASTERY", 1000),
                    ("MAGIC_RESISTANCE", 1000),
                    ("STEALTH", 300),
                    ("SPELL_CAP", 666),
                    ("SPELL_POWER", 1666),
                    ("LIFE", 1000),
                ],
            ),
            (
                true,
                vec![
                    ("DEC_MAGIC_MASTERY", 1000),
                    ("DEC_STEALTH", 300),
                    ("DEC_SPELL_CAP", 666),
                    ("DEC_SPELL_POWER", 1666),
                    ("DEC_LIFE", 1000),
                ],
            ),
        ] {
            let mut count = 0;
            for (flag, amount) in entries {
                if self.has(flag) {
                    let c = f64::from(count);
                    let score = f64::from((amount * mult) as u32) * (1.0 + c / 10.0 + c * c / 50.0);
                    cost = f64::from(c_i32(if negative { cost - score } else { cost + score }));
                    count += 1;
                }
            }
        }
        if self.has("DEC_SPEED") {
            let c = [
                "DEC_MAGIC_MASTERY",
                "DEC_STEALTH",
                "DEC_SPELL_CAP",
                "DEC_SPELL_POWER",
                "DEC_LIFE",
            ]
            .iter()
            .filter(|flag| self.has(flag))
            .count() as f64;
            cost = f64::from(c_i32(
                cost - f64::from(1000 * pval * pval) * (1.0 + c / 10.0 + c * c / 50.0),
            ));
        }
        c_i32(cost)
    }

    fn resistances(&self) -> i32 {
        let (mut cost, mut low) = self.score_group(
            &[
                ("RES_ACID", 3000),
                ("RES_ELEC", 3000),
                ("RES_FIRE", 3500),
                ("RES_COLD", 3000),
                ("RES_POIS", 2500),
            ],
            false,
        );
        let light = i32::from(self.has("RES_LITE"));
        let mut big = if low + light >= 5 {
            2 - light
        } else if low + light >= 2 {
            1 - light
        } else {
            -light
        };
        let (score, mut high) = self.score_group(
            &[
                ("RES_LITE", 2500),
                ("RES_DARK", 4000),
                ("RES_CONF", 4500),
                ("RES_NETHER", 5500),
                ("RES_NEXUS", 3500),
                ("RES_CHAOS", 6000),
                ("RES_SOUND", 4000),
                ("RES_SHARDS", 7000),
                ("RES_DISEN", 5500),
                ("RES_TIME", 9001),
            ],
            false,
        );
        cost += score;
        cost += self
            .score_group(&[("RES_BLIND", 1000), ("RES_FEAR", 2500)], false)
            .0;
        high += i32::from(self.has("RES_FEAR"));
        big += high;
        let (score, count) = self.score_group(
            &[
                ("IM_ACID", 12000),
                ("IM_ELEC", 14000),
                ("IM_FIRE", 15000),
                ("IM_COLD", 14000),
            ],
            false,
        );
        cost += score;
        big += count;
        low += count * 2;
        let (score, count) = self.score_group(
            &[
                ("VULN_ACID", 5000),
                ("VULN_ELEC", 5000),
                ("VULN_FIRE", 5000),
                ("VULN_COLD", 5000),
                ("VULN_POIS", 5000),
                ("VULN_LITE", 5000),
                ("VULN_DARK", 5000),
                ("VULN_BLIND", 2000),
                ("VULN_CONF", 5000),
                ("VULN_NETHER", 5000),
                ("VULN_NEXUS", 5000),
                ("VULN_CHAOS", 5000),
                ("VULN_SOUND", 5000),
                ("VULN_SHARDS", 5000),
                ("VULN_DISEN", 5000),
            ],
            false,
        );
        cost -= score;
        big -= (count + 1) / 2;
        high -= count;
        let honorary = i32::from(self.has("SPEED")) + i32::from(self.has("TELEPATHY"));
        big += honorary + i32::from(self.has("REFLECT"));
        for count in [big, high.min(low) + honorary] {
            if count >= 3 {
                cost += f64::from((300 + 300 * (count - 2)) * (count - 2));
            }
        }
        c_i32(cost)
    }

    fn brands(&self) -> i32 {
        let mut cost = 0;
        let mut count = 0;
        for (flag, score) in [
            ("KILL_EVIL", 22000),
            ("KILL_LIVING", 15000),
            ("KILL_DEMON", 14000),
            ("KILL_UNDEAD", 14000),
            ("KILL_HUMAN", 14000),
            ("KILL_DRAGON", 12500),
            ("BRAND_VAMP", 12500),
            ("BRAND_DARK", 12500),
            ("KILL_GOOD", 10000),
            ("SLAY_EVIL", 10000),
            ("BRAND_POIS", 8500),
            ("SLAY_LIVING", 8500),
            ("SLAY_DEMON", 7500),
            ("SLAY_UNDEAD", 7500),
            ("SLAY_HUMAN", 7500),
            ("BRAND_ACID", 7000),
            ("BRAND_ELEC", 7000),
            ("KILL_ANIMAL", 6500),
            ("SLAY_DRAGON", 6500),
            ("BRAND_FIRE", 6000),
            ("BRAND_COLD", 6000),
            ("KILL_GIANT", 5500),
            ("SLAY_GOOD", 5000),
            ("KILL_TROLL", 4000),
            ("KILL_ORC", 4000),
            ("SLAY_ANIMAL", 3250),
            ("SLAY_GIANT", 2750),
            ("SLAY_TROLL", 2000),
            ("SLAY_ORC", 2000),
        ] {
            if self.has(flag) {
                count += 1;
                cost += score * 3 / (count * 2 + 1);
            }
        }
        cost
    }

    fn speed(&self) -> i32 {
        if self.has("SPEED") {
            1000 * self.pval + 500 * self.pval * self.pval.abs()
        } else {
            0
        }
    }

    fn minor_bonuses(&self, tunnel: i32) -> i32 {
        (200 * i32::from(self.has("SEARCH"))
            + 400 * i32::from(self.has("INFRA"))
            + tunnel * i32::from(self.has("TUNNEL")))
            * self.pval
    }

    fn finalize(&self, mut price: i32) -> i32 {
        let activation = if self.activation_timeout != 0 {
            self.activation_value
                * interpolate(
                    self.activation_timeout,
                    &[
                        (1, 150),
                        (3, 130),
                        (10, 120),
                        (30, 110),
                        (75, 100),
                        (160, 90),
                        (300, 80),
                        (450, 75),
                    ],
                )
                / 100
        } else {
            self.activation_value
        };
        price = price.wrapping_add(activation);
        if self.ego == 81 {
            price /= 3;
        }
        price = price.wrapping_add(match self.fixed_artifact {
            282..=290 | 297 | 328 => 5000,
            381 => 10000,
            275 => 25000,
            226 => 50000,
            _ => 0,
        });
        if self.has("IGNORE_INVULN") {
            price = price.wrapping_add(8000);
        }
        if self.has("AGGRAVATE") {
            price = if self.tval == 36 && self.sval == 50 {
                price.wrapping_mul(5) / 6
            } else {
                price.wrapping_mul(8) / 10
            };
        }
        if self.has("NO_TELE") && self.tval != 40 {
            price = price.wrapping_mul(7) / 10;
        }
        if self.has("DRAIN_EXP") {
            price = price.wrapping_mul(9) / 10;
        }
        if self.has("TY_CURSE") && !self.permanent_curse {
            price = price.wrapping_mul(5) / 10;
        }
        if self.permanent_curse {
            price = price.wrapping_mul(8) / 10;
        }
        if !matches!(self.tval, 39 | 40 | 45) && !self.artifact {
            price = price.wrapping_add(1).wrapping_mul(3) / 4;
        }
        if price <= 0 {
            i32::from(self.ego != 0 || self.artifact)
        } else {
            price
        }
    }

    fn jewelry(&self) -> i32 {
        let base = match self.tval {
            39 => 1000,
            40 => 800,
            45 => 400,
            _ => return 0,
        };
        let mut price = base
            + self.resistances()
            + self.abilities()
            + self.brands()
            + self.minor_bonuses(0)
            + self.speed()
            + self.auras();
        price = price.wrapping_add(self.stats());
        for (flag, amount) in [("NO_MAGIC", 7000), ("NO_TELE", 5000), ("NO_SUMMON", 20000)] {
            if self.has(flag) {
                price += amount;
            }
        }
        if self.has("WEAPONMASTERY") {
            price += (7500 + 2500 * self.pval.abs()) * self.pval;
        }
        for (flag, amount) in [
            ("BLOWS", 15000),
            ("DEC_BLOWS", -15000),
            ("XTRA_SHOTS", 7500),
            ("XTRA_MIGHT", 3000),
        ] {
            if self.has(flag) {
                price += amount * self.pval;
            }
        }
        if self.to_a != 0 {
            let x = f64::from(self.to_a * self.to_a.abs());
            price = c_i32(f64::from(price) * (1000.0 + x) / 1000.0 + 20.0 * x);
        }
        price += 100 * self.to_h + 10 * self.to_h * self.to_h.abs();
        price += if matches!(self.ego, 207 | 208 | 224) || self.fixed_artifact == 378 {
            25 * self.to_d * self.to_d.abs()
        } else {
            100 * self.to_d + 25 * self.to_d * self.to_d.abs() + 3 * self.to_d.pow(3)
        };
        self.finalize(price)
    }

    fn light_or_quiver(&self) -> i32 {
        let mut price = if self.tval == 46 {
            30 + (self.capacity - 60).max(0).pow(2) / 5
                + match self.ego {
                    266 => 1500,
                    268 => 5000,
                    _ => 0,
                }
        } else {
            (match self.sval {
                0 => 1,
                1 => 30,
                _ => 250,
            }) + if self.ego == 237 { 100 } else { 0 }
        };
        price = sum_cost(&[
            price,
            self.resistances(),
            self.abilities(),
            self.speed(),
            self.stats().wrapping_mul(2),
            self.minor_bonuses(0),
            self.auras(),
        ]);
        if self.has("NO_MAGIC") {
            price += 7000;
        }
        if self.has("NO_TELE") {
            price += 5000;
        }
        if self.tval == 39 && self.has("NO_SUMMON") {
            price += 20000;
        }
        self.finalize(price)
    }

    fn armor(&self) -> i32 {
        let mut price = if self.ac < 10 {
            self.ac.pow(3)
        } else {
            1000 + 200 * (self.ac - 10) + 20 * (self.ac - 10).pow(2)
        };
        if self.tval == 33 {
            price += match self.sval {
                11 => 1000,
                12 => 2000,
                _ => 0,
            };
        }
        if self.to_a != 0 {
            let mut boost = interpolate(
                self.to_a.abs(),
                &[
                    (1, 200),
                    (5, 1500),
                    (10, 4000),
                    (15, 7500),
                    (20, 11000),
                    (25, 15000),
                    (30, 20000),
                    (100, 90000),
                ],
            );
            if !self.has("IGNORE_ACID") {
                boost /= 2;
            }
            price += boost * self.to_a.signum();
        }
        price = sum_cost(&[
            price,
            self.resistances(),
            self.abilities(),
            self.brands(),
            self.speed(),
            self.stats(),
            self.minor_bonuses(800),
            self.auras(),
        ]);
        if self.has("BLOWS") {
            price += 15000 * self.pval;
        }
        if self.has("DEC_BLOWS") {
            price -= 15000 * self.pval;
        }
        if self.has("DUAL_WIELDING") {
            price += 20000;
        }
        let to_h = self.to_h - self.base_to_h.min(0);
        price += 250 * to_h + 25 * to_h * to_h.abs();
        price += if self.to_d > 20 {
            35000 + (self.to_d - 20) * 1000
        } else if self.ego == 126 {
            25 * self.to_d * self.to_d.abs()
        } else {
            750 * self.to_d + 50 * self.to_d * self.to_d.abs()
        };
        let result = self.finalize(price);
        if self.tval == 36 && self.sval == 50 {
            result.max(1)
        } else {
            result
        }
    }

    fn weapon(&self) -> i32 {
        if self.tval == 23 {
            match self.sval {
                32 => return 2500,
                34 => return 1,
                _ => {}
            }
        }
        let mut slay = 1.0;
        let mut count = 0;
        for (flag, kill, normal) in [
            ("EVIL", 2.5 * 0.8, 1.0 * 0.8),
            ("UNDEAD", 4.0 * 0.1, 2.0 * 0.1),
            ("DEMON", 4.0 * 0.15, 2.0 * 0.15),
            ("LIVING", 2.5 * 0.7, 1.0 * 0.7),
            ("GOOD", 2.5 * 0.1, 1.0 * 0.1),
            ("BRAND_ACID", 1.5 * 0.15, 0.0),
            ("BRAND_ELEC", 1.5 * 0.15, 0.0),
            ("BRAND_FIRE", 1.5 * 0.1, 0.0),
            ("BRAND_COLD", 1.5 * 0.1, 0.0),
            ("BRAND_DARK", 3.0 * 0.1, 0.0),
            ("DRAGON", 4.0 * 0.1, 2.0 * 0.1),
            ("HUMAN", 3.0 * 0.1, 1.5 * 0.1),
            ("GIANT", 4.0 * 0.075, 2.0 * 0.075),
            ("BRAND_POIS", 1.5 * 0.075, 0.0),
            ("ORC", 4.0 * 0.01, 2.0 * 0.01),
            ("TROLL", 4.0 * 0.1, 2.0 * 0.1),
            ("ANIMAL", 3.0 * 0.2, 1.5 * 0.2),
        ] {
            let amount = if flag.starts_with("BRAND_") {
                self.has(flag).then_some(kill)
            } else if self.has(&format!("KILL_{flag}")) {
                Some(kill)
            } else {
                self.has(&format!("SLAY_{flag}")).then_some(normal)
            };
            if let Some(mut amount) = amount {
                for _ in 0..count {
                    amount *= 0.7;
                }
                count += 1;
                slay += amount;
            }
        }
        if self.has("BRAND_CHAOS") {
            slay += 0.2;
        }
        if self.has("BRAND_VAMP") {
            slay += 0.1;
        }
        if self.fixed_artifact == 294 {
            slay += 1.0 * 0.8;
        }
        if self.has("BRAND_MANA") {
            slay = (slay * 1.5 + 1.0) * 0.25 + slay * 0.75;
        }
        if self.has("VORPAL2") {
            slay *= 1.67;
        } else if self.has("VORPAL") {
            slay *= 1.22;
        }
        if self.has("STUN") {
            slay *= 1.18;
        }
        let base = f64::from(self.base_dd) * (f64::from(self.base_ds) + 1.0) / 2.0;
        let mut damage = (f64::from(self.dd) * (f64::from(self.ds) + 1.0) / 2.0 * slay
            + f64::from(self.to_d))
        .max(1.0);
        if self.has("BLOWS") {
            damage += (damage + 40.0) * f64::from(self.pval) / 10.0;
        }
        let extra = damage - base;
        let mut price = sum_cost(&[
            c_i32(extra * 100.0),
            c_i32(extra * extra * 5.0),
            c_i32(damage * damage * damage * 0.2),
        ]);
        for (flag, amount) in [("BRAND_VAMP", 3000), ("IMPACT", 250), ("BRAND_WILD", 10000)] {
            if self.has(flag) {
                price += amount;
            }
        }
        price += if self.to_h <= 10 {
            100 * self.to_h
        } else {
            10 * self.to_h * self.to_h
        };
        price = sum_cost(&[
            price,
            self.resistances() * 7 / 10,
            self.abilities(),
            self.speed(),
            self.stats(),
            self.auras(),
            self.minor_bonuses(if self.tval == 20 && self.pval <= 2 {
                150
            } else {
                1000
            }),
            500 * self.to_a,
        ]);
        if (20..=23).contains(&self.tval)
            && self.fixed_artifact != 136
            && (self.weight > 99 || self.tval == 22 || (self.tval == 21 && self.sval == 51))
        {
            price += 75;
        }
        price += (self.weight - 20).max(0);
        self.finalize(price)
    }

    fn bow(&self) -> i32 {
        let average = |mult| match self.sval {
            2 => mult * 22 / 10,
            12 | 13 | 63 => mult * 27 / 10,
            23 | 24 => mult * 30 / 10,
            _ => 0,
        };
        let base = average(self.base_mult);
        let mut price = if base == 0 {
            50
        } else {
            let extra = (average(self.mult) - base).max(0);
            let mut price =
                base / 20 + (base - 440).pow(2) / 600 + 150 * extra / 10 + extra * extra * 25 / 100;
            for flag in [
                "BRAND_POIS",
                "BRAND_ACID",
                "BRAND_ELEC",
                "BRAND_FIRE",
                "BRAND_COLD",
            ] {
                if self.has(flag) {
                    price = price * 5 / 4;
                }
            }
            let energy = match self.sval {
                2 => 7150,
                12 => 8888,
                23 => 12000,
                24 => 13333,
                _ => 10000,
            };
            price += 150 * self.to_d;
            if self.to_d > 10 {
                price += (self.to_d - 10).pow(2) * 15 * 10000 / energy;
            }
            if self.has("XTRA_SHOTS") {
                price += price * self.pval * 15 / 100;
            }
            price = price * 10000 / energy;
            price + 100 * self.to_h + 10 * self.to_h * self.to_h.abs()
        };
        price = sum_cost(&[
            price,
            self.resistances(),
            self.abilities(),
            self.speed(),
            self.stats(),
            self.minor_bonuses(0),
            self.auras(),
            500 * self.to_a,
            self.to_a * self.to_a.abs() * 30,
            self.weight,
        ]);
        self.finalize(price)
    }
}

fn sum_cost(terms: &[i32]) -> i32 {
    terms.iter().copied().fold(0, i32::wrapping_add)
}

fn c_i32(value: f64) -> i32 {
    // RFB's unsigned flag scores overflow signed conversion for pval -1/-2.
    // C leaves this undefined; match the x86-64 GCC reference's CVTTSD2SI
    // indefinite integer, rather than Rust's saturating conversion.
    if value.trunc() < f64::from(i32::MIN) || value.trunc() > f64::from(i32::MAX) {
        i32::MIN
    } else {
        value as i32
    }
}

fn interpolate(x: i32, points: &[(i32, i32)]) -> i32 {
    if x < points[0].0 {
        return points[0].1;
    }
    for pair in points.windows(2) {
        let [(x0, y0), (x1, y1)] = pair else {
            unreachable!()
        };
        if x < *x1 {
            return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
        }
    }
    points.last().unwrap().1
}

pub(super) fn object_value(mut object: ValueObject) -> Option<i32> {
    object.remove_opposites();
    Some(match object.tval {
        20..=23 => object.weapon(),
        16..=18 => {
            let value = object.weapon() / 25
                + match object.ego {
                    184 => 200,
                    185 => 50,
                    183 => 150,
                    _ => 0,
                };
            value.max(1)
        }
        19 => object.bow(),
        30..=38 => object.armor(),
        40 | 45 => object.jewelry(),
        39 if object.artifact => object.jewelry(),
        39 | 46 => object.light_or_quiver(),
        _ => return None,
    })
}
