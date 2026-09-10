// Adapted from RFB master; upstream copyright and terms are preserved in NOTICE.
use super::{Bias, Generator};

impl Generator<'_, '_> {
    pub(super) fn finish(&mut self, level: i32, scroll: bool) {
        if self.object.tval == 45 && self.one(15) {
            let mut again = true;
            if self.one(2) {
                if self.object.to_h > 0 {
                    self.object.to_h = -self.object.to_h;
                } else if self.one(4) {
                    self.object.to_h = -self.roll(10);
                    self.object.to_d = if self.one(2) {
                        -self.roll(10)
                    } else {
                        self.roll(7)
                    };
                    again = false;
                }
            }
            if again && self.one(2) {
                if self.object.to_d > 0 {
                    self.object.to_d = -self.object.to_d;
                } else if self.one(6) {
                    self.object.to_d = -self.roll(10);
                    self.object.to_h = if self.one(2) {
                        -self.roll(10)
                    } else {
                        self.roll(7)
                    };
                }
            }
        }
        if self.object.tval == 45 && self.one(30) {
            if self.object.to_a > 0 {
                self.object.to_a = -self.object.to_a;
            } else if self.one(2) {
                self.object.to_a = -self.roll(12);
            }
        }
        if self.has_pval {
            if self.has("BLOWS") {
                if matches!(self.object.tval, 45 | 31) {
                    self.object.pval = self.roll(2);
                    if self.one(6) {
                        self.object.pval += 1;
                    }
                } else {
                    self.object.pval = self.roll(3);
                    if self.one(15) {
                        self.object.pval += 1;
                    }
                    if (self.object.tval, self.object.sval) == (23, 33) {
                        self.object.pval += self.roll(2);
                    }
                }
            } else {
                if scroll {
                    self.object.pval = 0;
                }
                loop {
                    self.object.pval += 1;
                    if !(self.object.pval < self.roll(5) || self.one(self.object.pval + 5)) {
                        break;
                    }
                }
            }
        }
        if self.object.pval > 4 && !self.one(12) {
            self.object.pval = 4;
        }
        if self.object.tval == 20 && !self.has("BLOWS") {
            self.object.pval = (self.object.pval + self.roll(2)).min(6);
        }
        if self.has("LIFE") && self.object.pval > 2 {
            self.object.pval = self.roll(2);
            if self.one(12) {
                self.object.pval += 1;
            }
        }
        if self.has("WEAPONMASTERY") && self.object.pval > 2 {
            self.object.pval = if self.one(6) { 3 } else { 2 };
        }
        if self.has("MAGIC_MASTERY") && self.object.pval > 2 {
            self.object.pval = if self.one(30) { 3 } else { 2 };
        }
        if self.armor() {
            self.object.to_a += self.roll(5) + self.bonus(5, level) + self.bonus(10, level);
            let max = if self.body() { 25 } else { 20 };
            if self.object.to_a > max - 5 {
                self.object.to_a = super::trim(self.rng, self.object.to_a, max - 5, max, level);
            }
        } else if self.weapon() {
            let h = self.roll(5) + self.bonus(5, level) + self.bonus(10, level);
            let d = self.roll(5) + self.bonus(5, level) + self.bonus(10, level);
            self.object.to_h += h;
            self.object.to_d += d;
            if self.object.to_h > 22 {
                self.object.to_h = super::trim(self.rng, self.object.to_h, 20, 25, level);
            }
            // The source passes to_h to the second trim as well.
            if self.object.to_d > 20 {
                self.object.to_d = super::trim(self.rng, self.object.to_h, 20, 25, level);
            }
            if self.has("WIS") && self.object.pval > 0 {
                self.add("BLESSED");
            }
        }
        if self.has("ORDER") {
            self.remove("BRAND_WILD");
            self.object.dd *= self.object.ds;
            self.object.ds = 1;
        } else if self.has("BRAND_WILD") {
            self.object.ds *= self.object.dd;
            self.object.dd = 1;
        }
        self.add_all(&["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"]);
        if self.has("BRAND_FIRE") {
            self.add("LITE");
        }
        if !self.has_activation
            && !(16..=18).contains(&self.object.tval)
            && self.one(if self.armor() { 10 } else { 5 })
        {
            self.random_activation();
        }
        if self.armor() || matches!(self.object.tval, 45 | 40) {
            let (upper, lower) = if self.armor() { (20, 10) } else { (25, 20) };
            while self.object.to_d + self.object.to_h > upper {
                if self.one(self.object.to_d) && self.one(self.object.to_h) {
                    break;
                }
                self.object.to_d -= self.zero(3);
                self.object.to_h -= self.zero(3);
            }
            while self.object.to_d + self.object.to_h > lower && !self.immunity_replaced {
                if self.one(self.object.to_d) || self.one(self.object.to_h) {
                    break;
                }
                self.object.to_d -= self.zero(3);
                self.object.to_h -= self.zero(3);
            }
        }
        if matches!(self.bias, Bias::Mage | Bias::Intelligence) && self.object.tval == 31 {
            self.add("FREE_ACT");
        }
        if self.harp() {
            self.object.to_h = 0;
            self.object.to_d = 0;
            self.remove("SHOW_MODS");
        }
        if self.object.tval == 19 {
            let energy = crate::game::item_value::bow_energy(self.object.sval);
            if self.has("XTRA_MIGHT") && self.object.mult != 0 {
                self.object.mult += (25 + self.bonus(75, self.level)) * energy / 10000;
                self.remove("XTRA_MIGHT");
            }
            self.object.to_d = self.object.to_d * energy / 7150;
        }
        if (self.object.tval, self.object.sval) == (23, 32) {
            self.object.to_h = 0;
            self.object.to_d = 0;
            for flag in [
                "BLOWS",
                "BRAND_MANA",
                "SLAY_ANIMAL",
                "SLAY_EVIL",
                "SLAY_UNDEAD",
                "SLAY_DEMON",
                "SLAY_ORC",
                "SLAY_TROLL",
                "SLAY_GIANT",
                "SLAY_DRAGON",
                "KILL_DRAGON",
                "SLAY_HUMAN",
                "VORPAL",
                "BRAND_POIS",
                "BRAND_ACID",
                "BRAND_ELEC",
                "BRAND_FIRE",
                "BRAND_COLD",
            ] {
                self.remove(flag);
            }
        }
    }

    pub(super) fn sanitize(&mut self) {
        if self.has("LITE") && self.has("DARKNESS") {
            let flag = if self.one(2) { "DARKNESS" } else { "LITE" };
            self.remove(flag);
        }
        for flag in ["STR", "INT", "WIS", "DEX", "CON", "CHR"] {
            let opposite = format!("DEC_{flag}");
            if self.has(flag) && self.has(&opposite) {
                self.remove(flag);
                self.remove(&opposite);
            }
        }
        if self.has("VORPAL2") {
            self.add("VORPAL");
        }
    }

    pub(super) fn slot_weight(&self) -> i32 {
        super::slot_weight(&self.object, self.class_id)
    }

    fn normalized(&mut self, amount: i32, percent: i32) -> i32 {
        let n = amount * 10 * percent / 100;
        n / 10 + i32::from(n % 10 != 0 && self.zero(10) < n % 10)
    }

    pub(super) fn roll_powers(&mut self, level: i32) -> i32 {
        let percent = self.slot_weight() * 100 / 80;
        let spread = self.normalized(6, percent);
        let max = self.normalized(12, percent);
        let mut powers = self.roll(spread) + 1;
        while self.one(powers) || self.one(7 * 90 / level) || self.one(10 * 70 / level) {
            powers += 1;
        }
        if self.one(12) {
            powers *= 2;
        }
        powers.min(max)
    }

    fn hit_damage(&mut self, level: i32) {
        self.add("SHOW_MODS");
        self.object.to_h += self.roll(5) + self.bonus(5, level);
        self.object.to_d += self.roll(5) + self.bonus(5, level);
    }

    fn spell_power(&mut self) {
        self.add_all(&["SPELL_POWER", "DEC_STR", "DEC_DEX", "DEC_CON"]);
        self.has_pval = true;
    }

    pub(super) fn set_activation(&mut self, index: usize) {
        let profile = &self.data.device_generation.activations[index];
        self.activation = Some(profile);
        self.has_activation = true;
        self.object.flags.insert("ACTIVATE".to_owned());
        self.object.activation_value = profile.rfb_value.expect("validated RFB activation value");
        self.object.activation_timeout = i32::from(
            profile
                .recovery
                .or(self.data.device_generation.recovery)
                .expect("source activations have recovery")
                .interval_ticks,
        ) / 10;
    }

    pub(super) fn random_activation(&mut self) {
        let bias = if self.bias == Bias::None {
            0
        } else {
            1 << (self.bias as u8 - 1)
        };
        let eligible = |i: usize| {
            let profile = &self.data.device_generation.activations[i];
            profile.device_check_difficulty >= self.level / 3
                && (bias == 0 || self.data.activation_biases[i] & bias != 0)
        };
        let total: u64 = self
            .data
            .device_generation
            .activations
            .iter()
            .enumerate()
            .filter(|(i, _)| eligible(*i))
            .map(|(_, p)| u64::from(p.weight))
            .sum();
        if total == 0 {
            return;
        }
        let mut roll = self.rng.bounded(total);
        for (i, profile) in self.data.device_generation.activations.iter().enumerate() {
            if !eligible(i) {
                continue;
            }
            if roll < u64::from(profile.weight) {
                self.set_activation(i);
                return;
            }
            roll -= u64::from(profile.weight);
        }
    }

    pub(super) fn construct(&mut self, mut powers: i32, level: i32) {
        let mut resistance = false;
        let mut armor = false;
        let mut damage = false;
        let mut hit = false;
        let mut esp = 0;
        let tval = self.object.tval;
        match tval {
            45 => {
                if self.one(2) {
                    self.add("FREE_ACT");
                }
                if self.one(2) {
                    self.add("SEE_INVIS");
                }
                if self.roll(150) < powers {
                    self.add("LIFE");
                    self.has_pval = true;
                    powers -= 1;
                }
            }
            40 => {
                if self.one(3) {
                    self.add("FREE_ACT");
                }
                if self.one(3) {
                    self.add("HOLD_LIFE");
                }
            }
            39 => {
                if self.one(3) {
                    self.add("HOLD_LIFE");
                }
                if self.one(10) {
                    self.add("DARKNESS");
                }
            }
            30 => {
                if self.one(4) {
                    self.add("FREE_ACT");
                }
                if self.one(7) {
                    self.add("LEVITATION");
                }
            }
            31 => {
                if self.one(4) {
                    self.add("FREE_ACT");
                }
            }
            32 if self.one(4) => self.add("SEE_INVIS"),
            _ => (),
        }
        while powers > 0 {
            powers -= 1;
            let n = if (16..=18).contains(&self.object.tval) {
                0
            } else {
                self.roll(if self.object.tval == 19 { 9 } else { 7 })
            };
            match self.object.tval {
                16..=18 => {
                    if self.roll(225) < level && self.slaying == 0 {
                        if self.one(3) {
                            powers -= self.object.dd - 1;
                            self.object.dd *= 2;
                            self.slaying += self.object.dd;
                        } else {
                            powers += 2;
                            loop {
                                self.object.dd += 1;
                                powers -= 1;
                                self.slaying += 1;
                                if !self.one(self.object.dd) {
                                    break;
                                }
                            }
                            loop {
                                self.object.ds += 1;
                                powers -= 1;
                                self.slaying += 1;
                                if !self.one(self.object.ds) {
                                    break;
                                }
                            }
                        }
                    } else {
                        self.slay();
                    }
                }
                39 => match n {
                    1 | 2 => self.plus(),
                    3 => {
                        if self.one(45) {
                            self.add("TELEPATHY");
                        } else if self.one(15) {
                            let i = self
                                .data
                                .device_generation
                                .activations
                                .iter()
                                .position(|p| p.id.ends_with(".clairvoyance"))
                                .expect("source activation pool contains clairvoyance");
                            self.set_activation(i);
                        } else if self.one(77) {
                            self.spell_power();
                        } else {
                            self.plus();
                        }
                    }
                    4..=6 => {
                        if self.one(3) {
                            self.high_resistance();
                        } else {
                            self.resistance();
                        }
                    }
                    _ => self.misc(),
                },
                20..=23 => match n {
                    1 | 2 => self.plus(),
                    3 => {
                        if self.slaying == 0
                            && (self.object.tval, self.object.sval) != (22, 50)
                            && ((self.object.tval, self.object.sval) == (23, 33)
                                || !self.has("BLOWS"))
                        {
                            let max = 81.min(self.object.base_dd * self.object.base_ds * 4);
                            let mut dd = self.object.dd + 1;
                            let mut ds = self.object.ds;
                            self.slaying += 1;
                            while self.zero(1000) < 4000 / (dd * ds) {
                                if self.one(2) {
                                    dd += 1;
                                    if dd * ds > max {
                                        dd -= 1;
                                        break;
                                    }
                                } else {
                                    ds += 1;
                                    if dd * ds > max {
                                        ds -= 1;
                                        break;
                                    }
                                }
                                powers -= 1;
                                self.slaying += 1;
                            }
                            self.object.dd = dd;
                            self.object.ds = ds;
                        } else if !damage && !hit {
                            if self.one(7) {
                                self.object.to_h += self.roll(8);
                                self.object.to_d += self.roll(8);
                                hit = true;
                                damage = true;
                            } else if self.one(2) {
                                self.object.to_h += self.roll(8);
                                hit = true;
                            } else {
                                self.object.to_d += self.roll(8);
                                damage = true;
                            }
                        } else {
                            self.resistance();
                        }
                    }
                    4 => self.misc(),
                    _ => self.slay(),
                },
                19 => match n {
                    1 | 2 => self.plus(),
                    3..=5 => self.slay(),
                    6 | 7 => self.misc(),
                    _ => self.resistance(),
                },
                45 | 40 => match n {
                    1 | 2 => self.plus(),
                    3 if self.object.tval == 45 => {
                        if self.one(10) {
                            self.add("SPEED");
                            self.has_pval = true;
                        } else if (self.bias == Bias::Mage && self.one(10)) || self.one(50) {
                            self.add("DEC_MANA");
                        } else if self.one(20) {
                            self.add("WEAPONMASTERY");
                            self.has_pval = true;
                        } else if self.one(10) && self.roll(150) < level - 50 {
                            self.add("BLOWS");
                            self.has_pval = true;
                        } else if self.one(50) {
                            self.add("XTRA_MIGHT");
                            if !self.one(7) {
                                self.remove("XTRA_SHOTS");
                            }
                            self.has_pval = true;
                        } else if self.one(50) {
                            self.add("XTRA_SHOTS");
                            if !self.one(7) {
                                self.remove("XTRA_MIGHT");
                            }
                            self.has_pval = true;
                        } else if self.one(15) {
                            self.add_all(&[
                                "SUST_STR",
                                "SUST_INT",
                                "SUST_WIS",
                                "SUST_DEX",
                                "SUST_CON",
                                "SUST_CHR",
                                "HOLD_LIFE",
                            ]);
                            powers -= 1;
                        } else if self.one(77) {
                            self.spell_power();
                        } else if self.one(3) {
                            self.hit_damage(level);
                        } else if self.one(3) {
                            self.object.to_a += self.roll(5) + self.bonus(5, level);
                        } else if self.one(20) {
                            let i = self.zero(5) as usize;
                            self.add(
                                [
                                    "BRAND_ACID",
                                    "BRAND_ELEC",
                                    "BRAND_FIRE",
                                    "BRAND_COLD",
                                    "BRAND_VAMP",
                                ][i],
                            );
                        } else {
                            self.high_resistance();
                        }
                    }
                    3 => {
                        if self.one(45) {
                            self.add("TELEPATHY");
                        } else if (self.bias == Bias::Mage && self.one(10)) || self.one(50) {
                            self.add("DEC_MANA");
                        } else if self.one(7) {
                            self.add("REFLECT");
                        } else if self.one(30) {
                            self.add("MAGIC_MASTERY");
                            self.has_pval = true;
                        } else if self.one(77) {
                            self.spell_power();
                        } else if self.one(3) {
                            self.hit_damage(level);
                        } else {
                            self.high_resistance();
                        }
                    }
                    4 if self.one(3) => self.hit_damage(level),
                    4 if self.one(2) => self.plus(),
                    4 if self.one(5) => self.high_resistance(),
                    4 | 5 if self.object.tval == 45 && self.one(2) => self.plus(),
                    4..=6 => self.resistance(),
                    _ => self.misc(),
                },
                _ => match n {
                    1 | 2 => self.plus(),
                    3 => {
                        if !resistance && (self.body() || self.object.tval == 34) && self.one(4) {
                            self.add_all(&["RES_ACID", "RES_COLD", "RES_FIRE", "RES_ELEC"]);
                            if self.one(3) {
                                self.add("RES_POIS");
                            }
                            resistance = true;
                            powers -= 3;
                        } else if self.object.tval == 30 && self.one(3) {
                            self.add("SPEED");
                            self.has_pval = true;
                        } else if self.object.tval == 30 && self.one(2) {
                            self.add("LEVITATION");
                        } else if (self.object.tval == 33 && self.one(2 + esp * 10))
                            || (self.object.tval == 32 && self.one(7 + esp * 10))
                        {
                            let mut properties =
                                rfb_content::AffixPropertyBundleDefinition::default();
                            let extra = crate::game::ego::add_esp_strong(self.rng, &mut properties);
                            crate::game::ego::add_esp_weak(self.rng, &mut properties, extra);
                            self.object.properties(&properties);
                            esp += 1;
                        } else if !armor {
                            self.object.to_a += self.roll(8);
                            if self.body() && self.one(7) {
                                self.object.ac += 5;
                            }
                            armor = true;
                        } else if self.object.tval == 31 && self.one(2) {
                            self.add("SHOW_MODS");
                            self.object.to_h = 4 + self.roll(11);
                            self.object.to_d = 4 + self.roll(11);
                        } else if self.object.tval == 31 && self.one(2) {
                            self.add("DEX");
                            self.has_pval = true;
                        } else if self.object.tval == 31 && self.one(77) {
                            self.add("BLOWS");
                            self.has_pval = true;
                        } else {
                            self.high_resistance();
                        }
                    }
                    4 if !(self.body() || self.object.tval == 34) || self.one(3) => self.misc(),
                    _ => self.resistance(),
                },
            }
        }
    }
}
