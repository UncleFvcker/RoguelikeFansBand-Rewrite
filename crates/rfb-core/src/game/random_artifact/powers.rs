// Adapted from RFB master; upstream copyright and terms are preserved in NOTICE.
use super::{Bias, Generator};

impl Generator<'_, '_> {
    pub(super) fn plus(&mut self) {
        use Bias::*;
        self.has_pval = true;
        let preferred: &[&str] = match self.bias {
            Warrior => &["STR", "CON", "DEX"],
            Mage => &["INT"],
            Priestly => &["WIS"],
            Ranger => &["DEX", "CON", "STR"],
            Rogue => &["STEALTH", "SEARCH"],
            Strength => &["STR"],
            Intelligence => &["INT"],
            Wisdom => &["WIS"],
            Dexterity => &["DEX"],
            Constitution => &["CON"],
            Charisma => &["CHR"],
            _ => &[],
        };
        for flag in preferred {
            if self.free_biased_flag(flag) {
                return;
            }
        }
        if self.bias == Mage && self.object.tval == 31 && self.free_biased_flag("MAGIC_MASTERY") {
            return;
        }
        if matches!(self.bias, Mage | Priestly)
            && (self.object.tval, self.object.sval) == (36, 2)
            && !self.has("DEC_MANA")
            && self.one(3)
        {
            self.add("DEC_MANA");
            if self.one(2) {
                return;
            }
        }
        match self.roll(if self.weapon() { 23 } else { 19 }) {
            n @ 1..=12 => {
                let i = (n - 1) as usize / 2;
                self.add(["STR", "INT", "WIS", "DEX", "CON", "CHR"][i]);
                if self.bias == None && !self.one(13) {
                    self.bias = [
                        Strength,
                        Intelligence,
                        Wisdom,
                        Dexterity,
                        Constitution,
                        Charisma,
                    ][i];
                } else if self.bias == None && i < 5 && self.one(if i == 4 { 9 } else { 7 }) {
                    self.bias = [Warrior, Mage, Priestly, Rogue, Ranger][i];
                }
            }
            13 | 14 => {
                self.add("STEALTH");
                if self.bias == None && self.one(3) {
                    self.bias = Rogue;
                }
            }
            15 | 16 => {
                self.add("SEARCH");
                if self.bias == None && self.one(9) {
                    self.bias = Ranger;
                }
            }
            17 | 18 => self.add("INFRA"),
            19 => {
                self.add("SPEED");
                if self.bias == None && self.one(11) {
                    self.bias = Rogue;
                }
            }
            20 | 21 => {
                let flag = if self.one(3) { "SPEED" } else { "TUNNEL" };
                self.add(flag);
            }
            _ => {
                if self.object.tval == 19 || self.slaying != 0 {
                    self.plus();
                } else if self.one(13) {
                    self.add("BLOWS");
                    if self.bias == None && self.one(11) {
                        self.bias = Warrior;
                    }
                } else {
                    self.slay();
                }
            }
        }
    }

    fn check_immunity(&mut self) -> bool {
        if self.roll(100) < self.level - 40 {
            return true;
        }
        if self.immunity_replaced {
            return false;
        }
        let count = self.roll(3);
        self.immunity_replaced = true;
        if self.melee() || self.armor() {
            for _ in 0..count {
                if self.melee() {
                    self.slay();
                } else {
                    self.resistance();
                }
            }
            let choice = self.roll(10);
            if choice <= 2 && !self.has("SPEED") {
                self.add("SPEED");
                self.has_pval = true;
            } else if choice <= 6 {
                self.add("SHOW_MODS");
                self.object.to_h += 5 + self.roll(10);
                self.object.to_d += 5 + self.roll(10);
            } else if choice <= 8 && self.melee() && !self.has("BLOWS") {
                self.add("BLOWS");
                self.has_pval = true;
            } else if choice <= 8 && self.armor() {
                self.object.to_a += 5 + self.roll(10);
                self.add_all(&["FREE_ACT", "SEE_INVIS", "HOLD_LIFE", "LEVITATION"]);
            } else {
                self.add_all(&["STR", "DEX", "CON"]);
                if self.melee() {
                    self.add_all(&["FREE_ACT", "SEE_INVIS"]);
                }
                self.has_pval = true;
            }
        } else if matches!(self.object.tval, 40 | 45) {
            match self.roll(3) {
                1 if !self.has("SPEED") => {
                    self.add("SPEED");
                    self.has_pval = true;
                }
                1 | 2 => {
                    self.add("SHOW_MODS");
                    self.object.to_h += 5 + self.roll(10);
                    self.object.to_d += 5 + self.roll(10);
                }
                _ => {
                    for flag in ["STR", "INT", "WIS", "DEX", "CON", "CHR"] {
                        if self.one(2) {
                            self.add(flag);
                            self.has_pval = true;
                        }
                    }
                }
            }
        }
        false
    }

    pub(super) fn resistance(&mut self) {
        use Bias::*;
        let element = match self.bias {
            Acid => Some("ACID"),
            Electricity => Some("ELEC"),
            Fire => Some("FIRE"),
            Cold => Some("COLD"),
            _ => Option::None,
        };
        if let Some(element) = element {
            if self.free_biased_flag(&format!("RES_{element}")) {
                return;
            }
            if element != "ACID"
                && (35..=37).contains(&self.object.tval)
                && self.free_biased_flag(&format!("AURA_{element}"))
            {
                return;
            }
            if self.one(20) && !self.has(&format!("IM_{element}")) && self.check_immunity() {
                self.add(&format!("IM_{element}"));
                if !self.one(7) {
                    for other in ["ACID", "ELEC", "COLD", "FIRE"] {
                        if other != element {
                            self.remove(&format!("IM_{other}"));
                        }
                    }
                }
                if self.one(2) {
                    return;
                }
            }
        } else {
            match self.bias {
                Poison => {
                    if self.free_biased_flag("RES_POIS") {
                        return;
                    }
                }
                Warrior => {
                    if !self.one(3) && self.free_biased_flag("RES_FEAR") {
                        return;
                    }
                    if self.one(9) && self.free_biased_flag("NO_MAGIC") {
                        return;
                    }
                }
                Necromantic => {
                    for flag in ["RES_NETHER", "RES_POIS", "RES_DARK"] {
                        if self.free_biased_flag(flag) {
                            return;
                        }
                    }
                }
                Chaos => {
                    for flag in ["RES_CHAOS", "RES_CONF", "RES_DISEN"] {
                        if self.free_biased_flag(flag) {
                            return;
                        }
                    }
                }
                _ => (),
            }
        }
        let n = self.roll(42);
        match n {
            1..=4 => {
                if !self.one(12) {
                    self.resistance();
                } else if self.check_immunity() {
                    self.add(["IM_ACID", "IM_ELEC", "IM_COLD", "IM_FIRE"][(n - 1) as usize]);
                    if self.bias == None {
                        self.bias = [Acid, Electricity, Cold, Fire][(n - 1) as usize];
                    }
                }
            }
            5..=16 => {
                let i = match n {
                    5 | 6 | 13 => 0,
                    7 | 8 | 14 => 1,
                    9 | 10 | 15 => 2,
                    _ => 3,
                };
                self.add(["RES_ACID", "RES_ELEC", "RES_FIRE", "RES_COLD"][i]);
                if self.bias == None {
                    self.bias = [Acid, Electricity, Fire, Cold][i];
                }
            }
            17 | 18 => {
                self.add("RES_POIS");
                if self.bias == None && !self.one(4) {
                    self.bias = Poison;
                } else if self.bias == None && self.one(2) {
                    self.bias = Necromantic;
                } else if self.bias == None && self.one(2) {
                    self.bias = Rogue;
                }
            }
            19 | 20 => {
                self.add("RES_FEAR");
                if self.bias == None && self.one(3) {
                    self.bias = Warrior;
                }
            }
            21 => self.add("RES_LITE"),
            22 => self.add("RES_DARK"),
            23 | 24 => self.add("RES_BLIND"),
            25 | 26 => {
                self.add("RES_CONF");
                if self.bias == None && self.one(6) {
                    self.bias = Chaos;
                }
            }
            27 | 28 => self.add("RES_SOUND"),
            29 | 30 => self.add("RES_SHARDS"),
            31 | 32 => {
                self.add("RES_NETHER");
                if self.bias == None && self.one(3) {
                    self.bias = Necromantic;
                }
            }
            33 | 34 => self.add("RES_NEXUS"),
            35 | 36 => {
                self.add("RES_CHAOS");
                if self.bias == None && self.one(2) {
                    self.bias = Chaos;
                }
            }
            37 | 38 => self.add("RES_DISEN"),
            41 => {
                if matches!(self.object.tval, 34 | 32 | 35 | 37) {
                    self.add("REFLECT");
                } else {
                    self.resistance();
                }
            }
            _ => {
                let (flag, bias) = match n {
                    39 => ("AURA_ELEC", Electricity),
                    40 => ("AURA_FIRE", Fire),
                    _ => ("AURA_COLD", Cold),
                };
                if (35..=37).contains(&self.object.tval) {
                    self.add(flag);
                } else {
                    self.resistance();
                }
                if self.bias == None {
                    self.bias = bias;
                }
            }
        }
    }

    pub(super) fn misc(&mut self) {
        use Bias::*;
        let flag = match self.bias {
            Ranger | Constitution => "SUST_CON",
            Strength => "SUST_STR",
            Wisdom => "SUST_WIS",
            Intelligence => "SUST_INT",
            Dexterity => "SUST_DEX",
            Charisma => "SUST_CHR",
            Chaos => "TELEPORT",
            _ => "",
        };
        if !flag.is_empty() && self.free_biased_flag(flag) {
            return;
        }
        if self.bias == Fire {
            self.add("LITE");
        }
        match self.roll(33) {
            n @ 1..=6 => {
                self.add(
                    [
                        "SUST_STR", "SUST_INT", "SUST_WIS", "SUST_DEX", "SUST_CON", "SUST_CHR",
                    ][(n - 1) as usize],
                );
                if self.bias == None {
                    self.bias = [
                        Strength,
                        Intelligence,
                        Wisdom,
                        Dexterity,
                        Constitution,
                        Charisma,
                    ][(n - 1) as usize];
                }
            }
            7 | 8 | 14 => self.add("FREE_ACT"),
            9 => {
                self.add("HOLD_LIFE");
                if self.bias == None && self.one(5) {
                    self.bias = Priestly;
                } else if self.bias == None && self.one(6) {
                    self.bias = Necromantic;
                }
            }
            10 | 11 => self.add("LITE"),
            12 | 13 => self.add("LEVITATION"),
            15..=17 => self.add("SEE_INVIS"),
            19 | 20 => self.add("SLOW_DIGEST"),
            21 | 22 => self.add("REGEN"),
            23 => self.add("TELEPORT"),
            24..=26 => {
                if self.armor() {
                    self.misc();
                } else {
                    self.object.to_a = 4 + self.roll(11);
                }
            }
            27..=29 => {
                self.add("SHOW_MODS");
                let mut h = 4 + self.roll(11);
                let mut d = 4 + self.roll(11);
                if !matches!(self.object.tval, 20..=23 | 31 | 45) {
                    h /= 2;
                    d /= 2;
                }
                self.object.to_h += h;
                self.object.to_d += d;
            }
            30 => self.add("NO_MAGIC"),
            31 => {
                let n = self.zero(100);
                self.add(
                    if n < if self.class_id == "demo.class.berserker" {
                        10
                    } else {
                        90
                    } {
                        "WARNING"
                    } else {
                        "NO_TELE"
                    },
                );
            }
            32 => self.add("WARNING"),
            18 => match self.roll(3) {
                1 => {
                    self.add("ESP_EVIL");
                    if self.bias == None && self.one(3) {
                        self.bias = Law;
                    }
                }
                2 => {
                    self.add("ESP_NONLIVING");
                    if self.bias == None && self.one(3) {
                        self.bias = Mage;
                    }
                }
                _ => {
                    self.add("TELEPATHY");
                    if self.bias == None && self.one(9) {
                        self.bias = Mage;
                    }
                }
            },
            _ => {
                let count = self.roll(3) as usize;
                let a = self.roll(8);
                let mut b = self.roll(7);
                if b >= a {
                    b += 1;
                }
                let mut c = self.roll(6);
                if c >= a {
                    c += 1;
                }
                if c >= b {
                    c += 1;
                }
                for n in [a, b, c][..count].iter().rev() {
                    self.add(
                        [
                            "ESP_ANIMAL",
                            "ESP_UNDEAD",
                            "ESP_DEMON",
                            "ESP_ORC",
                            "ESP_TROLL",
                            "ESP_GIANT",
                            "ESP_HUMAN",
                            "ESP_GOOD",
                        ][(*n - 1) as usize],
                    );
                    if self.bias != None {
                        continue;
                    }
                    match n {
                        1 => {
                            if self.one(4) {
                                self.bias = Ranger;
                            }
                        }
                        2 => {
                            if self.one(3) {
                                self.bias = Priestly;
                            } else if self.one(6) {
                                self.bias = Necromantic;
                            }
                        }
                        7 => {
                            if self.one(6) {
                                self.bias = Rogue;
                            }
                        }
                        8 if self.one(3) => self.bias = Law,
                        _ => (),
                    }
                }
            }
        }
    }

    pub(super) fn slay(&mut self) {
        use Bias::*;
        if self.object.tval == 19 {
            if matches!(self.object.sval, 50 | 51) {
                if self.one(2) {
                    self.plus();
                    self.has_pval = true;
                } else {
                    self.high_resistance();
                }
            } else if self.harp() {
                if self.one(2) {
                    self.resistance();
                } else {
                    self.plus();
                    self.has_pval = true;
                }
            } else {
                let n = self.roll(6);
                if n == 1 && !self.one(6) {
                    self.slay_aux();
                } else if n <= 3 {
                    self.add("XTRA_MIGHT");
                    if !self.one(2) {
                        self.remove("XTRA_SHOTS");
                    }
                    if self.bias == None && self.one(9) {
                        self.bias = Ranger;
                    }
                } else {
                    self.add("XTRA_SHOTS");
                    if !self.one(2) {
                        self.remove("XTRA_MIGHT");
                    }
                    self.has_pval = true;
                    if self.bias == None && self.one(9) {
                        self.bias = Ranger;
                    }
                }
            }
            return;
        }
        let preferred: &[&str] = match self.bias {
            Chaos => &["BRAND_CHAOS"],
            Necromantic => &["BRAND_VAMP"],
            Ranger => &["SLAY_ANIMAL"],
            Poison => &["BRAND_POIS"],
            Fire => &["BRAND_FIRE"],
            Cold => &["BRAND_COLD"],
            Electricity => &["BRAND_ELEC"],
            Acid => &["BRAND_ACID"],
            Law => &["SLAY_EVIL", "SLAY_UNDEAD", "SLAY_DEMON"],
            _ => &[],
        };
        for flag in preferred {
            if self.free_biased_flag(flag) {
                return;
            }
        }
        if self.bias == Priestly && matches!(self.object.tval, 22 | 23) {
            self.add("BLESSED");
        }
        if self.bias == Necromantic && !self.has("BRAND_POIS") && self.one(2) {
            self.add("BRAND_POIS");
            if self.one(2) {
                return;
            }
        }
        if self.bias == Rogue {
            if matches!((self.object.tval, self.object.sval), (23, 4) | (22, 2)) {
                self.add("THROWING");
            }
            if self.free_biased_flag("BRAND_POIS") {
                return;
            }
        }
        self.slay_aux();
    }

    fn slay_aux(&mut self) {
        use Bias::*;
        match self.roll(if self.object.tval == 19 {
            25
        } else {
            29 + self.level / 15
        }) {
            1 | 2 => self.add("SLAY_ANIMAL"),
            3 | 4 => self.add("SLAY_HUMAN"),
            5 => {
                let flag = if self.one(5) { "SLAY_EVIL" } else { "SLAY_ORC" };
                self.add(flag);
            }
            6 => self.add("SLAY_ORC"),
            7 | 8 => self.add("SLAY_TROLL"),
            9 | 10 => self.add("SLAY_GIANT"),
            11 | 12 => {
                let flag = if self.roll(150) <= self.level {
                    "KILL_DRAGON"
                } else {
                    "SLAY_DRAGON"
                };
                self.add(flag);
            }
            n @ (13..=16 | 30 | 31) => {
                self.add(match n {
                    13 | 14 => "SLAY_DEMON",
                    15 | 16 => "SLAY_UNDEAD",
                    30 => "KILL_DEMON",
                    _ => "KILL_UNDEAD",
                });
                if self.bias == None && self.one(if n < 30 { 9 } else { 3 }) {
                    self.bias = Priestly;
                }
            }
            17..=19 => {
                if self.object.tval == 23 {
                    self.add("VORPAL");
                    if self.bias == None && self.one(9) {
                        self.bias = Warrior;
                    }
                } else {
                    self.slay_aux();
                }
            }
            20 => {
                if matches!(self.object.tval, 20 | 21) {
                    self.add("IMPACT");
                } else {
                    self.slay_aux();
                }
            }
            n @ 21..=24 => {
                self.add(
                    ["BRAND_FIRE", "BRAND_COLD", "BRAND_ELEC", "BRAND_ACID"][(n - 21) as usize],
                );
                if self.bias == None {
                    self.bias = [Fire, Cold, Electricity, Acid][(n - 21) as usize];
                }
            }
            25 => {
                self.add("BRAND_POIS");
                if self.bias == None && !self.one(3) {
                    self.bias = Poison;
                } else if self.bias == None && self.one(6) {
                    self.bias = Necromantic;
                } else if self.bias == None {
                    self.bias = Rogue;
                }
            }
            26 | 29 => {
                self.add("BRAND_VAMP");
                if self.bias == None {
                    self.bias = Necromantic;
                }
            }
            27 => {
                if (self.has("KILL_EVIL") && !self.one(100))
                    || (self.has("SLAY_EVIL") && !self.one(20))
                {
                    self.slay_aux();
                } else {
                    self.add("BRAND_MANA");
                    if self.bias == None {
                        self.bias = if self.one(2) { Mage } else { Priestly };
                    }
                }
            }
            28 => {
                self.add("BRAND_CHAOS");
                if self.bias == None {
                    self.bias = Chaos;
                }
            }
            _ => {
                if !self.has("ORDER") && !self.has("BRAND_WILD") && self.one(100) {
                    self.add("ORDER");
                } else if !self.has("BRAND_WILD") && !self.has("ORDER") && self.one(100) {
                    self.add("BRAND_WILD");
                } else {
                    if self.roll(1250) <= self.level - 50 {
                        self.add("KILL_EVIL");
                    } else if self.object.tval == 23 && self.roll(625) <= self.level - 50 {
                        self.add("VORPAL2");
                        return;
                    } else if self.object.tval == 21 && self.roll(625) <= self.level - 50 {
                        self.add("STUN");
                        return;
                    } else {
                        self.add("SLAY_EVIL");
                    }
                    if self.bias == None && self.one(2) {
                        self.bias = Law;
                    } else if self.bias == None && self.one(9) {
                        self.bias = Priestly;
                    }
                }
            }
        }
    }

    pub(super) fn high_resistance(&mut self) {
        let mut properties = rfb_content::AffixPropertyBundleDefinition::default();
        crate::game::ego::add_one_high_resistance(self.rng, &mut properties);
        self.object.properties(&properties);
    }
}
