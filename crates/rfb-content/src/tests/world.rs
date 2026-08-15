use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn loot_table_validation_uses_a_current_static_table() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid = artifact.content.clone();
    invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .entries
        .iter_mut()
        .for_each(|entry| entry.weight = 0);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .roll_chance_percent = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid = artifact.content.clone();
    let entry = &mut invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .entries[0];
    entry.min_depth = 2;
    entry.max_depth = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));
}

#[test]
fn loot_table_validation_allows_distinct_allocations_for_one_item() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut valid = artifact.content.clone();
    let table = valid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    let mut second_allocation = table.entries[0].clone();
    second_allocation.min_depth = second_allocation.min_depth.saturating_add(1);
    second_allocation.weight = 0;
    table.entries.push(second_allocation);
    validate_and_normalize(&mut valid).expect("distinct source allocations should be valid");

    let mut invalid = artifact.content.clone();
    let table = invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    table.entries.push(table.entries[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut valid = artifact.content.clone();
    let table = valid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    let mut allocation = table.entries[0].clone();
    allocation.max_depth = u16::MAX;
    table.entries = (0..512)
        .map(|min_depth| LootEntryDefinition {
            min_depth,
            ..allocation.clone()
        })
        .collect();
    validate_and_normalize(&mut valid).expect("a full RFB allocation table should fit");

    let mut invalid = artifact.content.clone();
    let table = invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    allocation.min_depth = 512;
    table.entries = valid
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("normalized static loot table should exist")
        .entries
        .clone();
    table.entries.push(allocation);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));
}

#[test]
fn loot_table_validation_accepts_exactly_one_quality_source() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let policy = LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent: 75,
        great_cap_percent: 20,
    };

    let mut valid = artifact.content.clone();
    let table = valid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    table.quality_weights.clear();
    table.quality_policy = Some(policy);
    validate_and_normalize(&mut valid).expect("RFB depth policy should replace static weights");

    let mut invalid = artifact.content.clone();
    invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .quality_policy = Some(policy);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .quality_weights
        .clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid = artifact.content.clone();
    let table = invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist");
    table.quality_weights.clear();
    table.quality_policy = Some(LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent: 101,
        great_cap_percent: 20,
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));
}

#[test]
fn procedural_floor_validation_uses_current_warrens_floors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid = artifact.content.clone();
    invalid
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .expect("Warrens depth one should exist")
        .loot_table_id = Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
    ));
}

#[test]
fn task_validation_uses_current_warrens_tasks() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid = artifact.content.clone();
    invalid
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist")
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.pest-control")
        .expect("pest task should exist")
        .target_placements[0]
        .objective_index = u32::MAX;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidTask(_))
    ));

    let mut invalid = artifact.content.clone();
    let task = invalid
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist")
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.pest-control")
        .expect("pest task should exist");
    task.prerequisite_task_id = Some(task.id.clone());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidTask(_))
    ));
}

#[test]
fn warrens_encounter_roster_matches_the_supported_legacy_ecology() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let table = artifact
        .content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.warrens")
        .expect("fixture should contain the Warrens encounter table");

    assert!(table.entries.is_empty());
    let policy = table
        .global_allocation
        .as_ref()
        .expect("Warrens should use the original global allocator");
    assert_eq!(
        policy
            .preferred_glyphs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["k", "K", "y", "Y", "r", "R", "f", "F", "c", "C", "b", "B"]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    assert!(policy.preferred_tags.is_empty());
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let mut allocation = artifact
        .content
        .actors
        .iter()
        .filter(|actor| !actor.tags.iter().any(|tag| tag == "orc-cave"))
        .filter(|actor| !actor.tags.iter().any(|tag| tag == "outpost-quest"))
        .filter_map(|actor| {
            actor.allocation.as_ref().map(|entry| {
                (
                    actor.id.as_str(),
                    entry.legacy_index,
                    entry.rarity,
                    entry.max_depth,
                )
            })
        })
        .collect::<Vec<_>>();
    allocation.sort_unstable();
    assert_eq!(
        allocation,
        vec![
            ("demo.actor.2-headed-hydra", 301, 2, 70),
            ("demo.actor.abyss-worm-mass", 214, 4, 40),
            ("demo.actor.agent-of-black-market", 14, 1, 0),
            ("demo.actor.aimless-looking-merchant", 16, 1, 0),
            ("demo.actor.air-hound", 338, 2, 90),
            ("demo.actor.air-spirit", 227, 2, 50),
            ("demo.actor.aquatic-golem", 910, 1, 70),
            ("demo.actor.arthur-pendragon", 1111, 1, 999),
            ("demo.actor.asura", 1374, 5, 999),
            ("demo.actor.aude", 1148, 1, 999),
            ("demo.actor.baby-black-dragon", 166, 2, 40),
            ("demo.actor.baby-blue-dragon", 163, 2, 40),
            ("demo.actor.baby-green-dragon", 165, 2, 40),
            ("demo.actor.baby-multi-hued-dragon", 204, 2, 40),
            ("demo.actor.baby-red-dragon", 167, 2, 40),
            ("demo.actor.baby-white-dragon", 164, 2, 40),
            ("demo.actor.balcmeg-the-relentless", 1182, 2, 999),
            ("demo.actor.ball-lightning", 300, 1, 60),
            ("demo.actor.bandit", 150, 2, 40),
            ("demo.actor.barracuda", 96, 2, 40),
            ("demo.actor.battle-scarred-veteran", 18, 1, 0),
            ("demo.actor.behemoth", 716, 3, 999),
            ("demo.actor.berserker", 293, 3, 80),
            ("demo.actor.black-harpy", 157, 1, 60),
            ("demo.actor.black-mamba", 210, 3, 40),
            ("demo.actor.black-naga", 71, 1, 30),
            ("demo.actor.black-ogre", 262, 2, 60),
            ("demo.actor.black-orc", 244, 2, 50),
            ("demo.actor.blink-dog", 312, 2, 70),
            ("demo.actor.blinking-dot", 22, 1, 10),
            ("demo.actor.blinking-light", 1279, 3, 44),
            ("demo.actor.bloodfang-the-wolf", 170, 1, 999),
            ("demo.actor.bloodshot-eye", 129, 3, 40),
            ("demo.actor.bloodshot-icky-thing", 155, 3, 40),
            ("demo.actor.blubbering-icky-thing", 41, 1, 20),
            ("demo.actor.blubbering-idiot", 9, 1, 0),
            ("demo.actor.blue-horror", 189, 3, 40),
            ("demo.actor.blue-icky-thing", 252, 4, 50),
            ("demo.actor.blue-ringed-octopus", 1308, 1, 50),
            ("demo.actor.blue-yeek", 52, 1, 20),
            ("demo.actor.boadile", 869, 40, 50),
            ("demo.actor.boldor-king-of-the-yeeks", 237, 3, 999),
            ("demo.actor.bomb-mosquito", 1017, 3, 20),
            ("demo.actor.box-jellyfish", 1309, 1, 50),
            ("demo.actor.brodda-the-easterling", 169, 2, 999),
            ("demo.actor.broken-death-sword", 953, 5, 40),
            ("demo.actor.brown-mold", 113, 1, 40),
            ("demo.actor.brown-yeek", 141, 1, 40),
            ("demo.actor.brumby", 1334, 2, 35),
            ("demo.actor.bullroarer-the-hobbit", 914, 3, 999),
            ("demo.actor.bunyip", 1322, 2, 60),
            ("demo.actor.burning-bush", 1307, 3, 999),
            ("demo.actor.bush-ranger", 1326, 2, 50),
            ("demo.actor.buzzy-beetle", 951, 4, 60),
            ("demo.actor.camelot-knight", 1117, 1, 999),
            ("demo.actor.carnivorous-flying-monkey", 145, 2, 40),
            ("demo.actor.carrion", 361, 1, 70),
            ("demo.actor.cassowary", 1327, 2, 50),
            ("demo.actor.caustic-icky-thing", 132, 2, 40),
            ("demo.actor.cave-lizard", 82, 1, 30),
            ("demo.actor.cave-orc", 126, 1, 40),
            ("demo.actor.cave-spider", 60, 1, 30),
            ("demo.actor.chaffinch", 4, 3, 0),
            ("demo.actor.chameleon", 1040, 1, 999),
            ("demo.actor.chaos-beastman", 318, 2, 999),
            ("demo.actor.chaos-shapechanger", 203, 2, 40),
            ("demo.actor.cheerful-leprechaun", 258, 2, 50),
            ("demo.actor.chimera", 341, 1, 70),
            ("demo.actor.chiokovo", 997, 3, 30),
            ("demo.actor.clear-hound", 282, 3, 50),
            ("demo.actor.clear-icky-thing", 26, 1, 10),
            ("demo.actor.clear-mushroom-patch", 184, 2, 40),
            ("demo.actor.clear-worm-mass", 79, 2, 30),
            ("demo.actor.cloaker", 243, 5, 50),
            ("demo.actor.cold-hound", 308, 2, 70),
            ("demo.actor.copperhead-snake", 106, 1, 40),
            ("demo.actor.creeping-copper-coins", 85, 2, 40),
            ("demo.actor.creeping-gold-coins", 195, 3, 40),
            ("demo.actor.creeping-mithril-coins", 239, 4, 50),
            ("demo.actor.creeping-silver-coins", 117, 2, 40),
            ("demo.actor.crow", 61, 2, 20),
            ("demo.actor.crow-of-durthang", 1224, 2, 40),
            ("demo.actor.crypt-creep", 124, 2, 40),
            ("demo.actor.culverin", 867, 2, 50),
            ("demo.actor.daemonette-of-slaanesh", 319, 2, 999),
            (
                "demo.actor.dailai-dongzhu-captain-of-southerings",
                1075,
                2,
                999
            ),
            ("demo.actor.dark-elf", 122, 2, 40),
            ("demo.actor.dark-elven-lord", 348, 2, 70),
            ("demo.actor.dark-elven-mage", 178, 1, 40),
            ("demo.actor.dark-elven-priest", 226, 1, 50),
            ("demo.actor.dark-elven-warrior", 182, 1, 40),
            ("demo.actor.dark-naga", 265, 2, 50),
            ("demo.actor.death-sword", 107, 5, 40),
            ("demo.actor.demonite", 1029, 2, 50),
            ("demo.actor.devilfish", 918, 4, 50),
            ("demo.actor.dimetrodon", 1223, 3, 90),
            ("demo.actor.dingo", 1320, 1, 30),
            (
                "demo.actor.disembodied-hand-that-strangled-people",
                112,
                2,
                40
            ),
            ("demo.actor.disenchanter-eye", 104, 2, 40),
            ("demo.actor.disenchanter-mold", 192, 2, 40),
            ("demo.actor.door-mimic", 311, 6, 80),
            ("demo.actor.drider", 234, 2, 50),
            ("demo.actor.drop-bear", 1315, 255, 40),
            ("demo.actor.druid", 241, 2, 50),
            ("demo.actor.duck", 1241, 1, 25),
            ("demo.actor.duck-quacked-platypus", 1325, 1, 36),
            ("demo.actor.dweller-on-the-threshold", 263, 5, 50),
            ("demo.actor.eagle", 172, 2, 40),
            ("demo.actor.earth-hound", 337, 2, 90),
            ("demo.actor.earth-spirit", 305, 2, 80),
            ("demo.actor.einheri-berserker", 1344, 10, 999),
            ("demo.actor.electric-eel", 346, 2, 70),
            ("demo.actor.energy-hound", 309, 2, 70),
            ("demo.actor.ewok", 92, 2, 40),
            ("demo.actor.fang-farmer-maggots-dog", 55, 2, 999),
            ("demo.actor.farmer-maggot", 8, 3, 0),
            ("demo.actor.fastitocalon", 704, 3, 999),
            ("demo.actor.filthy-street-urchin", 1, 2, 0),
            ("demo.actor.fire-hound", 307, 2, 70),
            ("demo.actor.fire-spirit", 306, 2, 80),
            ("demo.actor.flaming-crow", 1312, 2, 40),
            ("demo.actor.flesh-golem", 256, 1, 50),
            ("demo.actor.floating-eye", 32, 1, 10),
            ("demo.actor.floating-orb", 912, 2, 50),
            ("demo.actor.flying-skull", 273, 3, 50),
            ("demo.actor.forest-troll", 297, 1, 80),
            ("demo.actor.freesia", 57, 1, 999),
            ("demo.actor.freezing-sphere", 298, 1, 50),
            ("demo.actor.frost-giant", 278, 3, 70),
            ("demo.actor.frost-spirit", 1249, 2, 80),
            ("demo.actor.frosty-jelly", 84, 1, 40),
            ("demo.actor.fruit-bat", 37, 1, 10),
            ("demo.actor.frumious-bandersnatch", 232, 2, 50),
            ("demo.actor.garkain", 1328, 2, 60),
            ("demo.actor.gazer", 218, 1, 50),
            ("demo.actor.gelatinous-cube", 286, 4, 50),
            ("demo.actor.gertrude", 1150, 1, 999),
            ("demo.actor.ghast", 327, 1, 70),
            ("demo.actor.ghost-skier", 1339, 2, 70),
            ("demo.actor.giant-black-ant", 49, 1, 20),
            ("demo.actor.giant-black-dragon-fly", 322, 2, 999),
            ("demo.actor.giant-bronze-dragon-fly", 320, 1, 999),
            ("demo.actor.giant-brown-bat", 114, 1, 40),
            ("demo.actor.giant-clear-centipede", 276, 2, 30),
            ("demo.actor.giant-cockroach", 1007, 2, 40),
            ("demo.actor.giant-flea", 259, 1, 50),
            ("demo.actor.giant-fruit-fly", 197, 6, 40),
            ("demo.actor.giant-funnel-web-spider", 1313, 2, 60),
            ("demo.actor.giant-gold-dragon-fly", 325, 2, 999),
            ("demo.actor.giant-green-dragon-fly", 287, 2, 50),
            ("demo.actor.giant-green-frog", 56, 1, 20),
            ("demo.actor.giant-grey-rat", 156, 1, 40),
            ("demo.actor.giant-leech", 95, 1, 40),
            ("demo.actor.giant-moth", 1273, 2, 12),
            ("demo.actor.giant-mutant-ant", 1324, 2, 70),
            ("demo.actor.giant-octopus", 266, 2, 50),
            ("demo.actor.giant-pink-ant", 168, 2, 80),
            ("demo.actor.giant-pink-frog", 121, 1, 40),
            ("demo.actor.giant-pink-scorpion", 304, 1, 80),
            ("demo.actor.giant-piranha", 187, 2, 90),
            ("demo.actor.giant-salamander", 143, 1, 40),
            ("demo.actor.giant-slug", 120, 1, 40),
            ("demo.actor.giant-spider", 175, 2, 40),
            ("demo.actor.giant-squid", 482, 3, 70),
            ("demo.actor.giant-tarantula", 275, 3, 60),
            ("demo.actor.giant-white-ant", 75, 1, 30),
            ("demo.actor.giant-white-centipede", 24, 1, 10),
            ("demo.actor.giant-white-dragon-fly", 250, 3, 50),
            ("demo.actor.giant-white-louse", 69, 1, 30),
            ("demo.actor.giant-white-mouse", 27, 1, 10),
            ("demo.actor.giant-white-rat", 86, 1, 40),
            ("demo.actor.giant-white-tick", 176, 2, 40),
            ("demo.actor.giant-wombat", 1332, 3, 50),
            ("demo.actor.giant-yellow-toad", 1329, 6, 40),
            ("demo.actor.gibbering-mouther", 253, 4, 50),
            ("demo.actor.giganto-the-gargantuan", 650, 6, 999),
            ("demo.actor.glyptodont", 1222, 3, 90),
            ("demo.actor.gnome-mage", 281, 2, 60),
            ("demo.actor.goblin", 87, 1, 40),
            ("demo.actor.godzilla", 832, 2, 999),
            ("demo.actor.golfimbul-the-hill-orc-chief", 215, 3, 999),
            ("demo.actor.goomba", 924, 1, 20),
            ("demo.actor.gorbag-the-orc-captain", 315, 3, 999),
            ("demo.actor.grape-jelly", 212, 3, 40),
            ("demo.actor.great-eagle", 335, 2, 70),
            (
                "demo.actor.greater-cyber-wyrm-angel-daemon-lich",
                1337,
                50,
                999
            ),
            ("demo.actor.greater-hell-beast", 39, 6, 999),
            ("demo.actor.greater-kraken", 775, 2, 999),
            ("demo.actor.green-glutton-ghost", 100, 1, 40),
            ("demo.actor.green-jelly", 66, 1, 30),
            ("demo.actor.green-mold", 146, 2, 40),
            ("demo.actor.green-naga", 94, 1, 40),
            ("demo.actor.green-worm-mass", 31, 1, 10),
            ("demo.actor.gremlin", 153, 4, 40),
            ("demo.actor.grey-icky-thing", 103, 1, 40),
            ("demo.actor.grey-mold", 20, 1, 10),
            ("demo.actor.grey-seer", 1296, 3, 50),
            ("demo.actor.grid-bug", 34, 3, 20),
            ("demo.actor.griffon", 279, 1, 50),
            ("demo.actor.grip-farmer-maggots-dog", 53, 2, 999),
            ("demo.actor.grishnakh-the-hill-orc", 186, 3, 999),
            ("demo.actor.grizzly-bear", 191, 1, 45),
            ("demo.actor.guardian-naga", 269, 2, 50),
            ("demo.actor.gwaihir-the-windlord", 410, 1, 999),
            ("demo.actor.hairy-mold", 190, 2, 40),
            ("demo.actor.half-orc", 264, 3, 50),
            ("demo.actor.hammerhead", 292, 3, 50),
            ("demo.actor.helga", 1149, 1, 999),
            ("demo.actor.hellcat", 222, 1, 50),
            ("demo.actor.herringfolt-the-great-wild-boar", 1278, 1, 999),
            ("demo.actor.hibagon", 983, 10, 30),
            ("demo.actor.hill-giant", 255, 3, 60),
            ("demo.actor.hill-orc", 149, 1, 40),
            ("demo.actor.hippocampus", 207, 1, 40),
            ("demo.actor.hippogriff", 209, 1, 40),
            ("demo.actor.hobbes-the-tiger", 200, 2, 999),
            ("demo.actor.hobo", 10, 1, 0),
            ("demo.actor.homonculus", 280, 3, 50),
            ("demo.actor.hopper-ant", 1323, 3, 36),
            ("demo.actor.horse", 956, 1, 20),
            ("demo.actor.hugin-the-scheming-raven", 1346, 4, 999),
            ("demo.actor.hummerhorn", 289, 5, 50),
            ("demo.actor.hunting-hawk-of-julian", 151, 2, 40),
            ("demo.actor.huorn", 329, 1, 70),
            ("demo.actor.illusionist", 240, 2, 50),
            ("demo.actor.imp", 296, 2, 80),
            ("demo.actor.insect-swarm", 38, 1, 10),
            ("demo.actor.irish-wolfhound-of-flora", 254, 2, 50),
            ("demo.actor.ixitxachitl", 220, 1, 50),
            ("demo.actor.ixitxachitl-priest", 328, 1, 80),
            ("demo.actor.jackal", 35, 1, 5),
            ("demo.actor.jambavan-king-of-the-beasts", 1385, 6, 999),
            ("demo.actor.jaws", 467, 2, 999),
            ("demo.actor.jibaku-ghost", 1012, 2, 40),
            ("demo.actor.jormungand-the-midgard-serpent", 854, 1, 999),
            ("demo.actor.jumping-fireball", 299, 1, 50),
            ("demo.actor.kamikaze-yeek", 179, 1, 40),
            ("demo.actor.kangaroo", 1317, 2, 50),
            ("demo.actor.killer-bee", 174, 2, 40),
            ("demo.actor.killer-brown-beetle", 236, 2, 50),
            ("demo.actor.killer-whale", 363, 1, 80),
            ("demo.actor.king-cobra", 171, 2, 40),
            (
                "demo.actor.king-duosi-the-chief-of-southerings",
                1076,
                2,
                999
            ),
            (
                "demo.actor.king-mulu-the-chief-of-southerings",
                1077,
                2,
                999
            ),
            ("demo.actor.knight-archer", 219, 1, 50),
            ("demo.actor.kobold", 30, 1, 30),
            ("demo.actor.kutar", 1020, 4, 30),
            (
                "demo.actor.lady-zhurong-the-avatar-of-flame-spirit",
                1074,
                2,
                999
            ),
            ("demo.actor.lagduf-the-snaga", 140, 2, 999),
            ("demo.actor.landmine", 333, 5, 999),
            ("demo.actor.large-brown-snake", 28, 1, 10),
            ("demo.actor.large-grey-snake", 90, 1, 40),
            ("demo.actor.large-kobold", 102, 1, 40),
            ("demo.actor.large-white-snake", 21, 1, 10),
            ("demo.actor.large-yellow-snake", 59, 1, 20),
            ("demo.actor.lemure", 148, 3, 40),
            ("demo.actor.lesser-kraken", 740, 2, 999),
            ("demo.actor.leviathan", 782, 3, 999),
            ("demo.actor.light-hound", 271, 2, 60),
            ("demo.actor.lion", 1321, 2, 50),
            ("demo.actor.livingstone", 336, 4, 70),
            ("demo.actor.lizard-king", 332, 3, 999),
            ("demo.actor.lizardman", 290, 3, 50),
            ("demo.actor.lost-soul", 133, 2, 40),
            ("demo.actor.lousy-the-king-of-louses", 1063, 3, 999),
            ("demo.actor.lug-the-grotesque", 1183, 3, 999),
            ("demo.actor.lurker", 247, 3, 50),
            ("demo.actor.lynx", 1347, 2, 40),
            ("demo.actor.mad-bear", 1028, 1, 40),
            ("demo.actor.makara", 1377, 2, 999),
            ("demo.actor.manes", 128, 2, 40),
            ("demo.actor.mangy-looking-leper", 13, 1, 0),
            ("demo.actor.master-yeek", 224, 2, 40),
            ("demo.actor.mathmag-the-prince-of-whales", 1251, 2, 999),
            ("demo.actor.mauhur-the-orc-captain", 1072, 3, 999),
            ("demo.actor.meneldor-the-swift", 384, 1, 999),
            ("demo.actor.meng-huo-the-king-of-southerings", 1030, 2, 999),
            ("demo.actor.meng-you-the-brother-of-meng-huo", 1073, 2, 999),
            ("demo.actor.metallic-blue-centipede", 67, 1, 30),
            ("demo.actor.metallic-green-centipede", 42, 1, 20),
            ("demo.actor.metallic-red-centipede", 77, 1, 30),
            ("demo.actor.mi-go", 274, 2, 50),
            ("demo.actor.mine-dog", 221, 4, 50),
            ("demo.actor.mirkwood-spider", 277, 2, 50),
            ("demo.actor.moaning-spirit", 231, 2, 50),
            ("demo.actor.moire-queen-of-rebma", 615, 3, 999),
            ("demo.actor.mongbat", 235, 3, 50),
            ("demo.actor.monkey-of-nikko", 925, 3, 40),
            ("demo.actor.moon-beast", 223, 1, 50),
            ("demo.actor.mordred", 1119, 2, 999),
            ("demo.actor.morgana-le-fay", 1118, 2, 999),
            ("demo.actor.mori-troll", 1060, 255, 999),
            ("demo.actor.mutant-manta-ray", 1333, 5, 90),
            ("demo.actor.nami-the-mate", 1021, 4, 999),
            ("demo.actor.nandi-the-bull-of-shiva", 1381, 4, 999),
            ("demo.actor.nar-the-dwarf", 996, 2, 999),
            ("demo.actor.nekomata", 986, 3, 40),
            ("demo.actor.nether-worm-mass", 213, 4, 40),
            ("demo.actor.newt", 23, 1, 10),
            ("demo.actor.nibelung", 111, 1, 40),
            ("demo.actor.nick-the-butcher", 19, 4, 0),
            ("demo.actor.night-lizard", 134, 2, 40),
            ("demo.actor.nixie", 248, 1, 50),
            ("demo.actor.nizukil-prince-of-rats", 1299, 255, 999),
            (
                "demo.actor.noborta-kesyta-the-yeek-president",
                1059,
                255,
                999
            ),
            ("demo.actor.novice-archaeologist", 45, 3, 30),
            ("demo.actor.novice-archer", 116, 2, 40),
            ("demo.actor.novice-mage", 93, 2, 40),
            ("demo.actor.novice-mindcrafter", 1054, 1, 50),
            ("demo.actor.novice-paladin", 147, 2, 40),
            ("demo.actor.novice-priest", 109, 2, 40),
            ("demo.actor.novice-ranger", 142, 1, 40),
            ("demo.actor.novice-rogue", 44, 1, 30),
            ("demo.actor.novice-warrior", 110, 2, 40),
            ("demo.actor.noxious-fume", 884, 4, 50),
            ("demo.actor.nue", 984, 1, 60),
            ("demo.actor.nurgling", 139, 2, 40),
            ("demo.actor.ochre-jelly", 245, 3, 50),
            ("demo.actor.ogre", 238, 2, 50),
            ("demo.actor.ogrillon", 1057, 2, 60),
            ("demo.actor.orc-berserker", 1188, 3, 80),
            ("demo.actor.orc-captain", 285, 3, 50),
            ("demo.actor.orc-digger", 1108, 1, 50),
            ("demo.actor.orc-shaman", 162, 1, 40),
            ("demo.actor.orcish-artillery", 954, 3, 40),
            ("demo.actor.orfax-son-of-boldor", 180, 3, 999),
            ("demo.actor.owlbear", 188, 1, 40),
            ("demo.actor.ozmanian-devil", 1330, 2, 36),
            ("demo.actor.paladin", 1038, 1, 80),
            ("demo.actor.panther", 198, 2, 40),
            ("demo.actor.phantom-warrior", 152, 1, 40),
            ("demo.actor.phase-spider", 331, 2, 999),
            ("demo.actor.pink-horror", 242, 3, 50),
            ("demo.actor.pink-jelly", 131, 1, 40),
            ("demo.actor.pink-naga", 130, 2, 40),
            ("demo.actor.piranha", 70, 1, 60),
            ("demo.actor.pitiful-looking-beggar", 12, 1, 0),
            ("demo.actor.plague-rat", 1298, 2, 40),
            ("demo.actor.plaguebearer-of-nurgle", 268, 2, 50),
            ("demo.actor.polar-bear", 1340, 3, 60),
            ("demo.actor.polar-cat", 1392, 5, 999),
            ("demo.actor.poltergeist", 65, 1, 30),
            ("demo.actor.portuguese-man-o-war", 160, 2, 40),
            ("demo.actor.potion-mimic", 310, 3, 80),
            ("demo.actor.priest", 225, 1, 50),
            ("demo.actor.pseudo-dragon", 193, 2, 50),
            ("demo.actor.pumpkin-man", 1146, 1, 999),
            ("demo.actor.purple-mushroom-patch", 108, 2, 40),
            ("demo.actor.quartz-vein", 911, 2, 60),
            ("demo.actor.quasit", 294, 2, 50),
            ("demo.actor.quiver-slot", 185, 2, 40),
            ("demo.actor.quylthulg", 342, 2, 999),
            ("demo.actor.radiant-kavu", 1071, 1, 50),
            ("demo.actor.radiation-eye", 80, 1, 30),
            ("demo.actor.rakshasa", 1386, 4, 999),
            ("demo.actor.ranger", 1039, 1, 80),
            ("demo.actor.rat-thing", 115, 1, 40),
            ("demo.actor.rattlesnake", 119, 1, 40),
            ("demo.actor.raven", 68, 2, 30),
            ("demo.actor.raving-lunatic", 11, 1, 0),
            ("demo.actor.red-mold", 324, 1, 40),
            ("demo.actor.red-worm-mass", 105, 1, 40),
            ("demo.actor.robin-hood-the-outlaw", 138, 2, 999),
            ("demo.actor.rock-lizard", 33, 1, 10),
            ("demo.actor.rock-mole", 161, 2, 40),
            ("demo.actor.rotting-corpse", 125, 1, 40),
            ("demo.actor.rust-monster", 284, 2, 50),
            ("demo.actor.sabre-tooth-tiger", 339, 2, 70),
            ("demo.actor.sadie-the-rainbow-serpent", 1331, 4, 999),
            ("demo.actor.salamander", 50, 1, 20),
            ("demo.actor.sand-dweller", 183, 1, 40),
            ("demo.actor.sasquatch", 343, 3, 70),
            ("demo.actor.scrawny-cat", 2, 3, 0),
            ("demo.actor.scrawny-horse", 955, 1, 0),
            ("demo.actor.scruffy-little-dog", 7, 3, 0),
            ("demo.actor.scruffy-looking-hobbit", 74, 1, 30),
            ("demo.actor.sea-giant", 1276, 3, 999),
            ("demo.actor.seahorse", 443, 2, 80),
            ("demo.actor.servant-of-glaaki", 181, 1, 40),
            ("demo.actor.shadow-creature-of-fiona", 201, 2, 40),
            ("demo.actor.shadow-hound", 272, 2, 60),
            ("demo.actor.shagrat-the-orc-captain", 314, 2, 999),
            ("demo.actor.shallow-puddle", 885, 6, 30),
            ("demo.actor.shambling-mound", 316, 2, 999),
            ("demo.actor.sheep", 1226, 4, 20),
            ("demo.actor.shrieker-mushroom-patch", 40, 1, 50),
            ("demo.actor.shrieking-eel", 1252, 2, 75),
            ("demo.actor.silver-jelly", 73, 2, 30),
            ("demo.actor.sir-galahad", 1114, 1, 999),
            ("demo.actor.sir-gareth", 1115, 1, 999),
            ("demo.actor.sir-gawain", 1113, 1, 999),
            ("demo.actor.sir-kay", 1116, 1, 999),
            ("demo.actor.sir-lancelot", 1112, 1, 999),
            ("demo.actor.skaven", 158, 1, 40),
            ("demo.actor.skaven-shaman", 217, 1, 50),
            ("demo.actor.skeleton-human", 228, 1, 50),
            ("demo.actor.skeleton-kobold", 91, 1, 40),
            ("demo.actor.skeleton-orc", 136, 1, 40),
            ("demo.actor.slime-mold", 962, 4, 30),
            ("demo.actor.slimy-jelly", 101, 1, 40),
            ("demo.actor.slimy-ooze", 64, 2, 30),
            ("demo.actor.slimy-worm-mass", 58, 1, 20),
            ("demo.actor.slush-pile", 99, 1, 40),
            ("demo.actor.small-kobold", 29, 1, 30),
            ("demo.actor.smeagol", 63, 2, 999),
            ("demo.actor.snaga", 118, 1, 40),
            ("demo.actor.snaga-sapper", 251, 2, 50),
            ("demo.actor.snow-golem", 1280, 1, 35),
            ("demo.actor.snow-leopard", 1338, 2, 50),
            ("demo.actor.software-bug", 246, 2, 90),
            ("demo.actor.soldier-ant", 36, 1, 10),
            ("demo.actor.space-monster", 144, 2, 40),
            ("demo.actor.sparrow", 3, 3, 0),
            ("demo.actor.sphinx", 295, 2, 50),
            ("demo.actor.spider-bomb", 1016, 4, 50),
            ("demo.actor.spotted-jelly", 233, 3, 50),
            ("demo.actor.spotted-mushroom-patch", 72, 1, 30),
            ("demo.actor.stone-giant", 321, 3, 70),
            ("demo.actor.stone-golem", 323, 2, 70),
            ("demo.actor.stunwall", 326, 5, 50),
            ("demo.actor.sugriva-lord-of-kishkindha", 1368, 4, 999),
            ("demo.actor.swamp-rabbit", 1387, 7, 42),
            ("demo.actor.swamp-thing", 302, 2, 70),
            ("demo.actor.swordfish", 88, 2, 40),
            ("demo.actor.swordsman", 216, 1, 50),
            ("demo.actor.taipan", 1311, 3, 50),
            ("demo.actor.tax-collector", 199, 3, 40),
            ("demo.actor.tengu", 194, 1, 40),
            ("demo.actor.the-borshin", 177, 2, 999),
            ("demo.actor.the-ghost-q", 1003, 3, 999),
            ("demo.actor.the-icky-queen", 909, 5, 999),
            ("demo.actor.the-questing-beast", 1122, 7, 999),
            (
                "demo.actor.the-wicked-witch-of-the-south-east",
                1306,
                3,
                999
            ),
            ("demo.actor.thorondor", 468, 1, 999),
            ("demo.actor.tiger", 230, 2, 50),
            ("demo.actor.tiger-snake", 1310, 1, 50),
            ("demo.actor.time-initiate", 1091, 3, 40),
            ("demo.actor.tin-golem", 1318, 2, 40),
            ("demo.actor.trapdoor-spider", 1314, 2, 35),
            ("demo.actor.trench-wurm", 1070, 1, 50),
            ("demo.actor.ufthak-of-cirith-ungol", 260, 3, 999),
            ("demo.actor.ugluk-the-uruk", 350, 4, 999),
            ("demo.actor.ulfast-son-of-ulfang", 291, 3, 999),
            ("demo.actor.umber-hulk", 283, 1, 50),
            ("demo.actor.undead-devilfish", 913, 4, 50),
            ("demo.actor.undead-mass", 202, 2, 40),
            ("demo.actor.unruly-horse", 957, 2, 30),
            ("demo.actor.unstable-worm-mass", 876, 4, 50),
            ("demo.actor.uruk", 313, 1, 60),
            ("demo.actor.vali-king-of-the-vanaras", 1369, 4, 999),
            ("demo.actor.vanara", 1367, 4, 999),
            ("demo.actor.vanara-sage", 1375, 7, 999),
            ("demo.actor.vlasta", 249, 3, 50),
            ("demo.actor.vorpal-bunny", 205, 3, 40),
            ("demo.actor.wallaby", 1316, 2, 30),
            ("demo.actor.war-bear", 173, 1, 40),
            ("demo.actor.warg", 257, 2, 50),
            ("demo.actor.warrens-keeper", 135, 3, 999),
            ("demo.actor.water-hound", 340, 3, 90),
            ("demo.actor.water-spirit", 303, 1, 70),
            ("demo.actor.weir", 344, 2, 70),
            ("demo.actor.wererat", 270, 2, 50),
            ("demo.actor.werewolf", 347, 2, 60),
            ("demo.actor.whale", 345, 4, 70),
            ("demo.actor.white-harpy", 51, 1, 20),
            ("demo.actor.white-icky-thing", 25, 1, 10),
            ("demo.actor.white-shark", 317, 1, 999),
            ("demo.actor.white-wolf", 211, 1, 40),
            ("demo.actor.white-worm-mass", 89, 1, 40),
            ("demo.actor.wild-cat", 62, 2, 20),
            ("demo.actor.wild-rabbit", 5, 3, 0),
            ("demo.actor.wolf", 196, 1, 40),
            ("demo.actor.wolf-farmer-maggots-dog", 54, 2, 999),
            ("demo.actor.wood-spider", 127, 3, 40),
            ("demo.actor.woodsman", 6, 1, 0),
            ("demo.actor.wormtongue-agent-of-saruman", 137, 2, 999),
            ("demo.actor.wounded-bear", 159, 1, 999),
            ("demo.actor.wutugu-the-chief-of-southerings", 1078, 2, 999),
            ("demo.actor.wyvern", 334, 2, 999),
            ("demo.actor.yellow-jelly", 48, 1, 20),
            ("demo.actor.yellow-light", 81, 1, 30),
            ("demo.actor.yellow-mold", 76, 1, 30),
            ("demo.actor.yellow-mushroom-patch", 47, 1, 20),
            ("demo.actor.yellow-worm-mass", 78, 2, 30),
            ("demo.actor.yeti", 154, 3, 40),
            ("demo.actor.yowie", 1335, 3, 50),
            ("demo.actor.zog", 98, 2, 40),
            ("demo.actor.zombified-human", 229, 1, 50),
            ("demo.actor.zombified-kobold", 123, 1, 40),
            ("demo.actor.zombified-orc", 208, 1, 40),
        ]
    );

    let orc_cave = artifact
        .content
        .actors
        .iter()
        .filter(|actor| actor.tags.iter().any(|tag| tag == "orc-cave"))
        .collect::<Vec<_>>();
    assert_eq!(orc_cave.len(), 838);

    for id in [
        "demo.actor.bunyip",
        "demo.actor.cassowary",
        "demo.actor.drop-bear",
        "demo.actor.ghost-skier",
        "demo.actor.giant-funnel-web-spider",
        "demo.actor.giant-mutant-ant",
        "demo.actor.herringfolt-the-great-wild-boar",
        "demo.actor.kangaroo",
        "demo.actor.polar-bear",
        "demo.actor.taipan",
    ] {
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should remain globally imported"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
        let allocation = actor
            .allocation
            .as_ref()
            .expect("retained actor should preserve its source allocation");
        assert!(
            allocation.wild_only
                || allocation
                    .legacy_dungeon_indices
                    .iter()
                    .any(|index| *index != 3)
        );
    }
    let (orc_cave, low_level_orc_cave): (Vec<_>, Vec<_>) =
        orc_cave.into_iter().partition(|actor| actor.level >= 21);
    assert_eq!(
        low_level_orc_cave
            .iter()
            .map(|actor| actor.id.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "demo.actor.clay-golem",
            "demo.actor.magic-mushroom-patch",
            "demo.actor.plague-monk",
            "demo.actor.rat-ogre",
            "demo.actor.skaven-assassin",
            "demo.actor.swamp-rat",
            "demo.actor.the-variant-maintainer",
        ]
        .into_iter()
        .collect()
    );
    let mut level_counts = [0_usize; 107];
    let mut source_indices = BTreeSet::new();
    for actor in orc_cave {
        assert!((21..=127).contains(&actor.level));
        let allocation = actor
            .allocation
            .as_ref()
            .expect("Orc Cave candidates should remain globally allocatable");
        assert!(source_indices.insert(allocation.legacy_index));
        level_counts[(actor.level - 21) as usize] += 1;
    }
    assert_eq!(
        level_counts,
        [
            16, 14, 13, 18, 25, 17, 19, 19, 19, 21, 7, 12, 30, 15, 27, 28, 19, 19, 11, 42, 12, 6,
            12, 14, 14, 7, 10, 6, 6, 17, 11, 8, 3, 8, 17, 9, 2, 6, 4, 18, 5, 4, 5, 2, 12, 3, 9, 4,
            6, 9, 10, 7, 5, 4, 6, 7, 7, 9, 7, 13, 2, 4, 5, 4, 13, 16, 4, 7, 6, 18, 4, 8, 4, 5, 1,
            3, 3, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 2,
        ]
    );

    let mouse = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.giant-white-mouse")
        .and_then(|actor| actor.allocation.as_ref())
        .expect("White mouse allocation");
    assert!(mouse.multiplies);
    assert_eq!(mouse.random_movement_percent, 50);
    let warg = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.warg")
        .and_then(|actor| actor.allocation.as_ref())
        .expect("Warg allocation");
    assert_eq!(
        warg.friends.map(|friends| (friends.dice, friends.sides)),
        Some((3, 3))
    );
    assert_eq!(warg.random_movement_percent, 25);

    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be in the shallow Warrens batch"))
    };
    let worms = actor("demo.actor.green-worm-mass")
        .allocation
        .as_ref()
        .expect("Green Worm Mass allocation");
    assert!(worms.multiplies);
    assert_eq!(worms.random_movement_percent, 75);
    assert_eq!(
        actor("demo.actor.jackal")
            .allocation
            .as_ref()
            .and_then(|allocation| allocation.friends)
            .map(|friends| (friends.dice, friends.sides)),
        Some((3, 3))
    );
    let grid_light = actor("demo.actor.grid-bug")
        .light
        .as_ref()
        .expect("Grid Bug intrinsic light");
    assert_eq!((grid_light.radius, grid_light.intrinsic), (1, true));
    assert!(
        actor("demo.actor.bomb-mosquito")
            .melee_routine
            .as_ref()
            .and_then(|routine| routine.blows.first())
            .is_some_and(|blow| blow.self_destructs)
    );
    assert!(actor("demo.actor.grey-mold").movement.never_moves);
    let blinking_dot = actor("demo.actor.blinking-dot");
    assert!(blinking_dot.movement.never_moves);
    assert!(
        blinking_dot
            .monster_casting
            .as_ref()
            .is_some_and(|casting| {
                casting.frequency_percent == 50
                    && casting.abilities.len() == 1
                    && casting.abilities[0].ability_id == "demo.ability.blink"
            })
    );
    assert!(artifact.content.abilities.iter().any(|ability| {
        ability.id == "demo.ability.blink"
            && matches!(
                ability.effect,
                AbilityEffectDefinition::BlinkSelf { radius: 10 }
            )
    }));
    assert!(
        actor("demo.actor.smeagol")
            .terrain_interaction
            .picks_up_items
    );
    assert!(
        actor("demo.actor.gremlin")
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().any(|blow| {
                blow.effects
                    .iter()
                    .any(|effect| matches!(effect, MeleeBlowEffectDefinition::EatFood { .. }))
            }))
    );
    assert!(
        actor("demo.actor.bullroarer-the-hobbit")
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().any(|blow| {
                blow.effects
                    .iter()
                    .any(|effect| matches!(effect, MeleeBlowEffectDefinition::EatItem { .. }))
            }))
    );
    for id in ["demo.actor.blue-yeek", "demo.actor.black-naga"] {
        let drop = actor(id)
            .death_drop
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should keep DROP_60"));
        assert_eq!(
            drop.item_table_id.as_deref(),
            Some("demo.loot-table.base-items")
        );
        assert_eq!(
            drop.chance_rolls
                .iter()
                .map(|roll| roll.percent)
                .collect::<Vec<_>>(),
            vec![60]
        );
    }
}

#[test]
fn global_monster_allocation_accepts_known_actor_tags() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut content = artifact.content.clone();
    let allocation = content
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.warrens")
        .and_then(|table| table.global_allocation.as_mut())
        .expect("Warrens global allocation policy");
    allocation.preferred_glyphs.clear();
    allocation.preferred_tags = vec!["animal".to_owned()];
    validate_and_normalize(&mut content).expect("known actor tag should be accepted");

    let allocation = content
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.warrens")
        .and_then(|table| table.global_allocation.as_mut())
        .expect("Warrens global allocation policy");
    allocation.preferred_tags = vec!["missing-monster-tag".to_owned()];
    assert!(matches!(
        validate_and_normalize(&mut content),
        Err(ContentError::InvalidEncounterTable(_))
    ));
}

#[test]
fn special_mechanics_batch_keeps_each_imported_contract_narrow() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    assert!(
        actor("demo.actor.grape-jelly")
            .melee_routine
            .as_ref()
            .is_some_and(
                |routine| routine.blows.iter().any(|blow| blow.effects.iter().any(
                    |effect| matches!(effect, MeleeBlowEffectDefinition::DrainExperience { .. })
                ))
            )
    );
    assert_eq!(
        actor("demo.actor.plague-rat").contact_auras,
        vec![ActorContactAuraDefinition {
            damage_type: ActorDamageType::Poison,
            damage_dice: 1,
            damage_sides: 2,
            chance_percent: None,
            ravages_time: false,
        }]
    );
    assert!(
        actor("demo.actor.chaos-shapechanger")
            .tags
            .iter()
            .any(|tag| tag == "shapechanger")
    );
    let archer_drop = actor("demo.actor.knight-archer")
        .death_drop
        .as_ref()
        .expect("Knight archer should retain its themed drop");
    assert_eq!(
        archer_drop.theme_table_id.as_deref(),
        Some("demo.loot-table.archer")
    );
    assert_eq!(archer_drop.theme_chance_percent, 50);
    assert_eq!(
        actor("demo.actor.king-duosi-the-chief-of-southerings")
            .allocation
            .as_ref()
            .expect("King Duosi allocation")
            .legacy_dungeon_indices,
        [31]
    );
    assert_eq!(
        actor("demo.actor.wallaby")
            .allocation
            .as_ref()
            .expect("Wallaby allocation")
            .legacy_dungeon_indices,
        [35]
    );
}

#[test]
fn level_twelve_noncasters_reuse_existing_actor_contracts() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ids = [
        "demo.actor.yeti",
        "demo.actor.grizzly-bear",
        "demo.actor.black-mamba",
        "demo.actor.white-wolf",
        "demo.actor.nether-worm-mass",
        "demo.actor.abyss-worm-mass",
        "demo.actor.golfimbul-the-hill-orc-chief",
        "demo.actor.swordsman",
        "demo.actor.ixitxachitl",
        "demo.actor.mine-dog",
        "demo.actor.hellcat",
        "demo.actor.air-spirit",
        "demo.actor.skeleton-human",
        "demo.actor.zombified-human",
        "demo.actor.tiger",
        "demo.actor.frumious-bandersnatch",
        "demo.actor.spotted-jelly",
        "demo.actor.mauhur-the-orc-captain",
        "demo.actor.meng-you-the-brother-of-meng-huo",
        "demo.actor.swamp-rabbit",
    ];
    for id in ids {
        assert_eq!(actor(id).level, 12, "{id} should remain level 12");
        assert!(
            actor(id).monster_casting.is_none(),
            "{id} must not import possessor-only S: hints as monster casting"
        );
    }

    assert!(
        actor("demo.actor.nether-worm-mass")
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.multiplies)
    );
    assert!(
        actor("demo.actor.mine-dog")
            .melee_routine
            .as_ref()
            .and_then(|routine| routine.blows.first())
            .is_some_and(|blow| blow.self_destructs)
    );
    assert_eq!(
        actor("demo.actor.meng-you-the-brother-of-meng-huo")
            .allocation
            .as_ref()
            .expect("Meng You allocation")
            .legacy_dungeon_indices,
        [31]
    );
}

#[test]
fn level_twelve_casters_share_parameterized_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    assert_eq!(
        ability_ids("demo.actor.gazer"),
        ["rfb-legacy.ability.confuse", "rfb-legacy.ability.paralyze"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.moon-beast"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.heal-36",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.master-yeek"),
        [
            "rfb-legacy.ability.ball-poison-12d2",
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.escape",
            "rfb-legacy.ability.slow",
            "rfb-legacy.ability.summon-legacy-import-l12-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.priest"),
        [
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.heal-36",
            "rfb-legacy.ability.scare",
            "rfb-legacy.ability.summon-legacy-import-l12-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.dark-elven-priest"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.bolt-physical-2d6-4",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.heal-36",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.moaning-spirit"),
        ["rfb-legacy.ability.escape", "rfb-legacy.ability.scare"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.devilfish"),
        [
            "rfb-legacy.ability.breath-chaos-17-600-r2",
            "rfb-legacy.ability.breath-dark-17-400-r2",
            "rfb-legacy.ability.breath-disenchant-17-500-r2",
            "rfb-legacy.ability.breath-light-17-400-r2",
            "rfb-legacy.ability.breath-sound-17-450-r2",
            "rfb-legacy.ability.breath-time-33-150-r2",
        ]
        .into_iter()
        .collect()
    );
    assert!(
        actor("demo.actor.devilfish")
            .melee_routine
            .as_ref()
            .is_none_or(|routine| routine.blows.is_empty())
    );
    assert_eq!(
        actor("demo.actor.priest")
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.priest")
    );
    assert_eq!(
        actor("demo.actor.dark-elven-priest")
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.evil-priest")
    );
}

#[test]
fn level_thirteen_monsters_reuse_existing_mechanics_and_parameterized_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.drider",
        "demo.actor.mongbat",
        "demo.actor.killer-brown-beetle",
        "demo.actor.boldor-king-of-the-yeeks",
        "demo.actor.ogre",
        "demo.actor.creeping-mithril-coins",
        "demo.actor.druid",
        "demo.actor.cloaker",
        "demo.actor.black-orc",
        "demo.actor.ochre-jelly",
    ] {
        assert_eq!(actor(id).level, 13, "{id} should remain level 13");
    }
    for id in [
        "demo.actor.mongbat",
        "demo.actor.killer-brown-beetle",
        "demo.actor.ogre",
        "demo.actor.creeping-mithril-coins",
        "demo.actor.cloaker",
        "demo.actor.ochre-jelly",
    ] {
        assert!(actor(id).monster_casting.is_none(), "{id} should not cast");
    }

    assert_eq!(
        ability_ids("demo.actor.drider"),
        [
            "rfb-legacy.ability.bolt-physical-2d6-4",
            "rfb-legacy.ability.bolt-physical-3d6",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.curse-3d8",
            "rfb-legacy.ability.darkness",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.boldor-king-of-the-yeeks"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.escape",
            "rfb-legacy.ability.heal-39",
            "rfb-legacy.ability.kin-boldor-king-of-the-yeeks",
            "rfb-legacy.ability.slow",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.druid"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.bolt-electricity-4d8-4",
            "rfb-legacy.ability.bolt-fire-9d8-4",
            "rfb-legacy.ability.haste-self",
            "rfb-legacy.ability.paralyze",
            "rfb-legacy.ability.slow",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.black-orc"),
        ["rfb-legacy.ability.bolt-physical-2d7"]
            .into_iter()
            .collect()
    );
    assert!(actor("demo.actor.cloaker").movement.never_moves);
    assert_eq!(
        actor("demo.actor.black-orc")
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.archer")
    );
    assert_eq!(
        actor("demo.actor.ogre")
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.warrior")
    );
}

#[test]
fn level_fourteen_harvest_reuses_existing_mechanics_and_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.death-sword",
        "demo.actor.software-bug",
        "demo.actor.lurker",
        "demo.actor.nixie",
        "demo.actor.vlasta",
        "demo.actor.giant-white-dragon-fly",
        "demo.actor.snaga-sapper",
        "demo.actor.blue-icky-thing",
        "demo.actor.gibbering-mouther",
        "demo.actor.irish-wolfhound-of-flora",
        "demo.actor.flesh-golem",
        "demo.actor.cheerful-leprechaun",
        "demo.actor.giant-flea",
        "demo.actor.ufthak-of-cirith-ungol",
        "demo.actor.orcish-artillery",
        "demo.actor.hibagon",
        "demo.actor.giant-cockroach",
        "demo.actor.lion",
        "demo.actor.snow-leopard",
    ] {
        assert_eq!(actor(id).level, 14, "{id} should remain level 14");
    }

    assert_eq!(
        ability_ids("demo.actor.giant-white-dragon-fly"),
        ["rfb-legacy.ability.breath-cold-20-900-r2"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.blue-icky-thing"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.scare",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.gibbering-mouther"),
        [
            "rfb-legacy.ability.breath-light-17-400-r2",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.scare",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.cheerful-leprechaun"),
        ["rfb-legacy.ability.blink"].into_iter().collect()
    );
    assert_eq!(
        ability_ids("demo.actor.orcish-artillery"),
        ["rfb-legacy.ability.bolt-physical-3d6"]
            .into_iter()
            .collect()
    );

    let software_bug = actor("demo.actor.software-bug")
        .allocation
        .as_ref()
        .expect("Software bug allocation");
    assert!(software_bug.multiplies);
    assert_eq!(software_bug.random_movement_percent, 75);
    assert!(
        actor("demo.actor.snaga-sapper")
            .melee_routine
            .as_ref()
            .and_then(|routine| routine.blows.get(1))
            .is_some_and(|blow| blow.self_destructs)
    );
    assert_eq!(
        actor("demo.actor.irish-wolfhound-of-flora")
            .allocation
            .as_ref()
            .and_then(|allocation| allocation.friends)
            .map(|friends| (friends.dice, friends.sides)),
        Some((3, 3))
    );
    assert_eq!(
        actor("demo.actor.lion")
            .allocation
            .as_ref()
            .expect("Lion allocation")
            .legacy_dungeon_indices,
        [35]
    );
    assert!(actor("demo.actor.lion").rideable);
    assert!(
        actor("demo.actor.snow-leopard")
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.wild_only)
    );
    assert!(
        actor("demo.actor.ufthak-of-cirith-ungol")
            .tags
            .iter()
            .any(|tag| tag == "unique")
    );
}

#[test]
fn level_fifteen_direct_harvest_reuses_existing_mechanics_and_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.hippogriff",
        "demo.actor.illusionist",
        "demo.actor.black-ogre",
        "demo.actor.half-orc",
        "demo.actor.giant-octopus",
        "demo.actor.guardian-naga",
        "demo.actor.light-hound",
        "demo.actor.shadow-hound",
        "demo.actor.flying-skull",
        "demo.actor.giant-tarantula",
        "demo.actor.giant-clear-centipede",
        "demo.actor.mirkwood-spider",
        "demo.actor.homonculus",
        "demo.actor.clear-hound",
        "demo.actor.carrion",
        "demo.actor.unstable-worm-mass",
        "demo.actor.the-ghost-q",
        "demo.actor.mad-bear",
        "demo.actor.trench-wurm",
        "demo.actor.time-initiate",
        "demo.actor.dimetrodon",
        "demo.actor.duck-quacked-platypus",
        "demo.actor.giant-yellow-toad",
    ] {
        assert_eq!(actor(id).level, 15, "{id} should remain level 15");
    }
    for id in [
        "demo.actor.hippogriff",
        "demo.actor.black-ogre",
        "demo.actor.half-orc",
        "demo.actor.giant-octopus",
        "demo.actor.guardian-naga",
        "demo.actor.flying-skull",
        "demo.actor.giant-tarantula",
        "demo.actor.giant-clear-centipede",
        "demo.actor.mirkwood-spider",
        "demo.actor.homonculus",
        "demo.actor.clear-hound",
        "demo.actor.carrion",
        "demo.actor.unstable-worm-mass",
        "demo.actor.the-ghost-q",
        "demo.actor.mad-bear",
        "demo.actor.trench-wurm",
        "demo.actor.dimetrodon",
        "demo.actor.giant-yellow-toad",
    ] {
        assert!(actor(id).monster_casting.is_none(), "{id} should not cast");
    }

    assert_eq!(
        ability_ids("demo.actor.illusionist"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.escape",
            "rfb-legacy.ability.haste-self",
            "rfb-legacy.ability.paralyze",
            "rfb-legacy.ability.slow",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.light-hound"),
        ["rfb-legacy.ability.breath-light-17-400-r2"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.shadow-hound"),
        ["rfb-legacy.ability.breath-dark-17-400-r2"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.time-initiate"),
        ["rfb-legacy.ability.haste-self", "rfb-legacy.ability.slow",]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.duck-quacked-platypus"),
        ["rfb-legacy.ability.shriek"].into_iter().collect()
    );

    assert!(actor("demo.actor.hippogriff").rideable);
    assert_eq!(
        actor("demo.actor.light-hound")
            .light
            .as_ref()
            .map(|light| (light.radius, light.intrinsic)),
        Some((3, true))
    );
    let unstable = actor("demo.actor.unstable-worm-mass");
    assert!(unstable.allocation.as_ref().is_some_and(
        |allocation| allocation.multiplies && allocation.random_movement_percent == 75
    ));
    assert!(
        unstable
            .melee_routine
            .as_ref()
            .and_then(|routine| routine.blows.first())
            .is_some_and(|blow| blow.self_destructs)
    );
    let ghost = actor("demo.actor.the-ghost-q");
    assert!(ghost.tags.iter().any(|tag| tag == "unique"));
    assert!(ghost.movement.modes.contains(&ActorMovementMode::PassWall));
    assert!(
        ghost
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().any(|blow| blow
                .effects
                .iter()
                .any(|effect| matches!(effect, MeleeBlowEffectDefinition::EatFood { .. }))))
    );
    assert!(actor("demo.actor.trench-wurm").rideable);
    assert!(
        actor("demo.actor.trench-wurm")
            .terrain_interaction
            .destroys_walls
    );
    for id in [
        "demo.actor.duck-quacked-platypus",
        "demo.actor.giant-yellow-toad",
    ] {
        assert_eq!(
            actor(id)
                .allocation
                .as_ref()
                .expect("regional allocation")
                .legacy_dungeon_indices,
            [35]
        );
    }
}

#[test]
fn level_fifteen_parameterized_casters_share_existing_effect_families() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.dweller-on-the-threshold",
        "demo.actor.dark-naga",
        "demo.actor.wererat",
        "demo.actor.mi-go",
        "demo.actor.griffon",
        "demo.actor.floating-orb",
        "demo.actor.undead-devilfish",
        "demo.actor.radiant-kavu",
    ] {
        assert_eq!(actor(id).level, 15, "{id} should remain level 15");
    }

    assert_eq!(
        ability_ids("demo.actor.dweller-on-the-threshold"),
        [
            "rfb-legacy.ability.bolt-acid-7d8-5",
            "rfb-legacy.ability.drain-mana-8",
            "rfb-legacy.ability.scare",
            "rfb-legacy.ability.summon-legacy-import-l15-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.dark-naga"),
        [
            "rfb-legacy.ability.bolt-cold-6d8-5",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.heal-45",
            "rfb-legacy.ability.paralyze",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.wererat"),
        [
            "rfb-legacy.ability.ball-poison-12d2",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.bolt-cold-6d8-5",
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.kin-wererat",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.mi-go"),
        [
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.summon-demon-l15-1d3-1",
            "rfb-legacy.ability.summon-legacy-import-l15-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.griffon"),
        ["rfb-legacy.ability.bolt-physical-4d5"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.floating-orb"),
        ["rfb-legacy.ability.bolt-physical-2d6-5"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.undead-devilfish"),
        [
            "rfb-legacy.ability.breath-disenchant-17-500-r2",
            "rfb-legacy.ability.breath-nether-14-550-r2",
            "rfb-legacy.ability.breath-nexus-33-250-r2",
            "rfb-legacy.ability.breath-poison-17-600-r2",
            "rfb-legacy.ability.breath-time-33-150-r2",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.radiant-kavu"),
        ["rfb-legacy.ability.heal-45"].into_iter().collect()
    );

    assert!(
        actor("demo.actor.dweller-on-the-threshold")
            .movement
            .never_moves
    );
    assert!(actor("demo.actor.floating-orb").movement.never_moves);
    assert!(actor("demo.actor.griffon").rideable);
    assert!(actor("demo.actor.radiant-kavu").rideable);
    assert!(
        actor("demo.actor.undead-devilfish")
            .movement
            .modes
            .contains(&ActorMovementMode::Aquatic)
    );
    assert_eq!(
        actor("demo.actor.radiant-kavu")
            .light
            .as_ref()
            .map(|light| (light.radius, light.intrinsic)),
        Some((1, true))
    );
}

#[test]
fn level_fifteen_p28_p29_bind_narrow_summon_and_target_blink() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should be generated"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    assert_eq!(actor("demo.actor.plaguebearer-of-nurgle").level, 15);
    assert_eq!(actor("demo.actor.gnome-mage").level, 15);
    assert_eq!(
        ability_ids("demo.actor.plaguebearer-of-nurgle"),
        [
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.scare",
            "rfb-legacy.ability.slow",
            "rfb-legacy.ability.summon-ant-l15-1d3-1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.gnome-mage"),
        [
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.blink-other",
            "rfb-legacy.ability.bolt-cold-6d8-5",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.summon-legacy-import-l15-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert!(matches!(
        ability("rfb-legacy.ability.summon-ant-l15-1d3-1").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 15,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 1,
            ..
        } if category == "ant"
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.blink-other").effect,
        AbilityEffectDefinition::BlinkTarget { radius: 10 }
    ));
    assert!(
        actor("demo.actor.plaguebearer-of-nurgle")
            .tags
            .iter()
            .any(|tag| tag == "undead")
    );
    assert_eq!(
        actor("demo.actor.gnome-mage")
            .light
            .as_ref()
            .map(|light| (light.radius, light.intrinsic)),
        Some((1, false))
    );
}

#[test]
fn level_fifteen_p30_buzzy_beetle_reflects_bolts() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.buzzy-beetle")
        .expect("Buzzy beetle should be imported");

    assert_eq!(actor.level, 15);
    assert!(actor.reflects_bolts);
    assert!(actor.monster_casting.is_none());
    assert!(actor.tags.iter().any(|tag| tag == "nonliving"));
    assert_eq!(
        actor
            .melee_routine
            .as_ref()
            .map(|routine| routine.blows.len()),
        Some(4)
    );
}

#[test]
fn level_sixteen_p31_harvest_reuses_existing_mechanics_and_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.pink-horror",
        "demo.actor.rust-monster",
        "demo.actor.orc-captain",
        "demo.actor.gelatinous-cube",
        "demo.actor.giant-green-dragon-fly",
        "demo.actor.hummerhorn",
        "demo.actor.lizardman",
        "demo.actor.ulfast-son-of-ulfang",
        "demo.actor.hammerhead",
        "demo.actor.berserker",
        "demo.actor.ogrillon",
        "demo.actor.orc-berserker",
    ] {
        assert_eq!(actor(id).level, 16, "{id} should remain level 16");
    }
    for id in [
        "demo.actor.rust-monster",
        "demo.actor.gelatinous-cube",
        "demo.actor.hummerhorn",
        "demo.actor.lizardman",
        "demo.actor.ulfast-son-of-ulfang",
        "demo.actor.hammerhead",
        "demo.actor.berserker",
        "demo.actor.ogrillon",
        "demo.actor.orc-berserker",
    ] {
        assert!(actor(id).monster_casting.is_none(), "{id} should not cast");
    }

    assert_eq!(
        ability_ids("demo.actor.pink-horror"),
        ["rfb-legacy.ability.confuse", "rfb-legacy.ability.scare"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.orc-captain"),
        ["rfb-legacy.ability.bolt-physical-3d6"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.giant-green-dragon-fly"),
        ["rfb-legacy.ability.breath-poison-17-600-r2"]
            .into_iter()
            .collect()
    );
    assert!(
        actor("demo.actor.rust-monster")
            .terrain_interaction
            .destroys_items
    );
    assert!(
        actor("demo.actor.gelatinous-cube")
            .terrain_interaction
            .picks_up_items
    );
    assert!(
        actor("demo.actor.hummerhorn")
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.multiplies)
    );
    assert!(
        actor("demo.actor.lizardman")
            .movement
            .modes
            .contains(&ActorMovementMode::Swim)
    );
    assert!(
        actor("demo.actor.hammerhead")
            .movement
            .modes
            .contains(&ActorMovementMode::Aquatic)
    );
    assert!(
        actor("demo.actor.ulfast-son-of-ulfang")
            .tags
            .iter()
            .any(|tag| tag == "unique")
    );
}

#[test]
fn level_sixteen_p32_king_mulu_reuses_category_summoning() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.king-mulu-the-chief-of-southerings")
        .expect("King Mulu should be imported");
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should be generated"))
    };

    assert_eq!(actor.level, 16);
    assert!(actor.tags.iter().any(|tag| tag == "unique"));
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .expect("King Mulu allocation")
            .legacy_dungeon_indices,
        [31]
    );
    assert_eq!(
        actor
            .monster_casting
            .as_ref()
            .expect("King Mulu should retain summoning")
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "rfb-legacy.ability.summon-ant-l16-1d3-1",
            "rfb-legacy.ability.summon-spider-l16-1d3-1",
        ]
        .into_iter()
        .collect()
    );
    for (id, expected_category) in [
        ("rfb-legacy.ability.summon-ant-l16-1d3-1", "ant"),
        ("rfb-legacy.ability.summon-spider-l16-1d3-1", "spider"),
    ] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level: 16,
                count_dice: 1,
                count_sides: 3,
                count_bonus: 1,
                ..
            } if category == expected_category
        ));
    }
}

#[test]
fn level_sixteen_p33_blockers_use_narrow_runtime_contracts() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should be generated"))
    };

    let umber_hulk = actor("demo.actor.umber-hulk");
    assert_eq!(umber_hulk.level, 16);
    assert_eq!(
        umber_hulk.resistances.get(&ActorDamageType::Disintegrate),
        Some(&ActorResistanceLevel::Vulnerable)
    );

    let brumby = actor("demo.actor.brumby");
    assert!(brumby.movement.modes.contains(&ActorMovementMode::Climb));
    assert!(brumby.rideable);
    assert_eq!(
        brumby
            .allocation
            .as_ref()
            .expect("Brumby allocation")
            .legacy_dungeon_indices,
        [35]
    );

    let quasit = actor("demo.actor.quasit");
    let quasit_casting = quasit
        .monster_casting
        .as_ref()
        .expect("Quasit should retain casting");
    assert!(quasit_casting.smart);
    assert!(
        quasit_casting
            .abilities
            .iter()
            .any(|candidate| { candidate.ability_id == "rfb-legacy.ability.teleport-level" })
    );
    assert!(matches!(
        ability("rfb-legacy.ability.teleport-level").effect,
        AbilityEffectDefinition::TeleportLevel
    ));

    let nizukil = actor("demo.actor.nizukil-prince-of-rats");
    assert_eq!(
        nizukil
            .allocation
            .as_ref()
            .expect("Nizukil allocation")
            .task_id
            .as_deref(),
        Some("demo.task.the-sewer")
    );
    assert!(nizukil.tags.iter().any(|tag| tag == "fixed-unique"));
    assert!(nizukil.tags.iter().any(|tag| tag == "no-quest"));
    assert!(matches!(
        ability("rfb-legacy.ability.heal-48").effect,
        AbilityEffectDefinition::Heal { amount: 48 }
    ));
}

#[test]
fn level_seventeen_p34_harvest_reuses_existing_mechanics_and_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.sphinx",
        "demo.actor.forest-troll",
        "demo.actor.2-headed-hydra",
        "demo.actor.swamp-thing",
        "demo.actor.water-spirit",
        "demo.actor.giant-pink-scorpion",
        "demo.actor.earth-spirit",
        "demo.actor.wutugu-the-chief-of-southerings",
    ] {
        assert_eq!(actor(id).level, 17, "{id} should remain level 17");
    }
    for id in [
        "demo.actor.forest-troll",
        "demo.actor.swamp-thing",
        "demo.actor.water-spirit",
        "demo.actor.giant-pink-scorpion",
        "demo.actor.earth-spirit",
        "demo.actor.wutugu-the-chief-of-southerings",
    ] {
        assert!(actor(id).monster_casting.is_none(), "{id} should not cast");
    }

    assert_eq!(
        ability_ids("demo.actor.sphinx"),
        ["rfb-legacy.ability.confuse", "rfb-legacy.ability.scare"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.2-headed-hydra"),
        ["rfb-legacy.ability.scare"].into_iter().collect()
    );

    let sphinx = actor("demo.actor.sphinx");
    assert!(sphinx.rideable);
    assert!(sphinx.movement.modes.contains(&ActorMovementMode::Fly));
    assert_eq!(
        sphinx
            .allocation
            .as_ref()
            .expect("Sphinx allocation")
            .habitats,
        [ActorHabitat::Mountain]
    );

    let forest_troll = actor("demo.actor.forest-troll");
    assert!(forest_troll.regenerates);
    assert_eq!(
        forest_troll.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Vulnerable)
    );

    let hydra = actor("demo.actor.2-headed-hydra");
    assert!(hydra.rideable);
    assert!(hydra.moves_weaker_bodies);
    assert!(hydra.movement.modes.contains(&ActorMovementMode::Swim));

    assert!(
        actor("demo.actor.swamp-thing")
            .melee_routine
            .as_ref()
            .expect("Swamp thing melee")
            .blows
            .iter()
            .flat_map(|blow| &blow.effects)
            .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Terrify { .. }))
    );

    let water_spirit = actor("demo.actor.water-spirit");
    assert_eq!(
        water_spirit
            .allocation
            .as_ref()
            .expect("Water spirit allocation")
            .random_movement_percent,
        25
    );
    assert!(
        water_spirit
            .movement
            .modes
            .contains(&ActorMovementMode::Fly)
    );
    assert!(
        !water_spirit
            .movement
            .modes
            .contains(&ActorMovementMode::PassWall)
    );
    assert!(water_spirit.tags.iter().any(|tag| tag == "nonliving"));

    assert!(
        actor("demo.actor.giant-pink-scorpion")
            .melee_routine
            .as_ref()
            .expect("Giant pink scorpion melee")
            .blows
            .iter()
            .flat_map(|blow| &blow.effects)
            .any(|effect| matches!(
                effect,
                MeleeBlowEffectDefinition::DrainAttributes { attributes, .. }
                    if attributes.contains(&ItemAttributeDefinition::Strength)
            ))
    );

    let earth_spirit = actor("demo.actor.earth-spirit");
    assert!(
        earth_spirit
            .movement
            .modes
            .contains(&ActorMovementMode::PassWall)
    );
    assert_eq!(
        earth_spirit.resistances.get(&ActorDamageType::Disintegrate),
        Some(&ActorResistanceLevel::Vulnerable)
    );

    let wutugu = actor("demo.actor.wutugu-the-chief-of-southerings");
    assert!(wutugu.tags.iter().any(|tag| tag == "unique"));
    assert_eq!(
        wutugu
            .allocation
            .as_ref()
            .expect("Wutugu allocation")
            .legacy_dungeon_indices,
        [31]
    );
    assert_eq!(
        wutugu
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.warrior")
    );
}

#[test]
fn level_seventeen_p35_casters_reuse_parameterized_abilities_and_dwarf_drops() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should be generated"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for id in [
        "demo.actor.hill-giant",
        "demo.actor.imp",
        "demo.actor.nekomata",
        "demo.actor.grey-seer",
        "demo.actor.nar-the-dwarf",
    ] {
        assert_eq!(actor(id).level, 17, "{id} should remain level 17");
    }

    assert_eq!(
        ability_ids("demo.actor.hill-giant"),
        ["rfb-legacy.ability.bolt-physical-1d1-50"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.imp"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.bolt-fire-9d8-5",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.drag",
            "rfb-legacy.ability.escape",
            "rfb-legacy.ability.scare",
            "rfb-legacy.ability.teleport-level",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.nekomata"),
        [
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.scare",
            "rfb-legacy.ability.summon-legacy-import-l17-1d1",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.grey-seer"),
        [
            "rfb-legacy.ability.ball-poison-12d2",
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.blink",
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.kin-grey-seer",
            "rfb-legacy.ability.slow",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        ability_ids("demo.actor.nar-the-dwarf"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.curse-8d8",
            "rfb-legacy.ability.heal-51",
            "rfb-legacy.ability.mind-blast-7d7",
        ]
        .into_iter()
        .collect()
    );

    assert!(matches!(
        ability("rfb-legacy.ability.bolt-physical-1d1-50").effect,
        AbilityEffectDefinition::Damage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 50,
            damage_type: ActorDamageType::Physical,
        }
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.bolt-fire-9d8-5").effect,
        AbilityEffectDefinition::Damage {
            damage_dice: 9,
            damage_sides: 8,
            damage_bonus: 5,
            damage_type: ActorDamageType::Fire,
        }
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.summon-legacy-import-l17-1d1").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 17,
            count_dice: 1,
            count_sides: 1,
            count_bonus: 0,
            ..
        } if category == "legacy-import"
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.kin-grey-seer").effect,
        AbilityEffectDefinition::Summon {
            ref actor_kind_id,
            count: 2,
            radius: 2,
            ..
        } if actor_kind_id == "demo.actor.grey-seer"
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.heal-51").effect,
        AbilityEffectDefinition::Heal { amount: 51 }
    ));

    let hill_giant = actor("demo.actor.hill-giant");
    assert!(
        hill_giant
            .movement
            .modes
            .contains(&ActorMovementMode::Climb)
    );
    assert_eq!(
        hill_giant
            .allocation
            .as_ref()
            .expect("Hill giant allocation")
            .habitats,
        [ActorHabitat::Mountain]
    );

    let grey_seer = actor("demo.actor.grey-seer");
    assert_eq!(
        grey_seer
            .allocation
            .as_ref()
            .expect("Grey seer allocation")
            .task_id
            .as_deref(),
        Some("demo.task.the-sewer")
    );

    let nar = actor("demo.actor.nar-the-dwarf");
    assert!(nar.tags.iter().any(|tag| tag == "unique"));
    assert!(nar.moves_weaker_bodies);
    assert_eq!(
        nar.death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.dwarf")
    );
    assert_eq!(
        artifact
            .content
            .loot_tables
            .iter()
            .find(|table| table.id == "demo.loot-table.dwarf")
            .expect("Dwarf drop table should compile")
            .entries
            .iter()
            .map(|entry| entry.item_kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "demo.item.battle-axe",
            "demo.item.beaked-axe",
            "demo.item.broad-axe",
            "demo.item.iron-helm",
            "demo.item.pair-of-metal-shod-boots",
            "demo.item.small-metal-shield",
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn level_seventeen_p36_spheres_keep_elemental_contact_and_death_damage() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, damage_type) in [
        ("demo.actor.freezing-sphere", ActorDamageType::Cold),
        ("demo.actor.jumping-fireball", ActorDamageType::Fire),
        ("demo.actor.ball-lightning", ActorDamageType::Electricity),
    ] {
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"));
        assert_eq!(actor.level, 17);
        assert!(actor.contact_auras.first().is_some_and(|aura| {
            aura.damage_type == damage_type
                && aura.damage_dice == 1
                && aura.damage_sides == 2
                && aura.chance_percent.is_none()
        }));
        assert!(
            actor
                .melee_routine
                .as_ref()
                .and_then(|routine| routine.blows.first())
                .is_some_and(|blow| {
                    blow.self_destructs
                        && blow.effects.iter().any(|effect| {
                            matches!(
                                effect,
                                MeleeBlowEffectDefinition::Damage {
                                    damage_dice: 8,
                                    damage_sides: 8,
                                    damage_type: effect_damage_type,
                                    armor_mitigated: false,
                                    ..
                                } if *effect_damage_type == damage_type
                            )
                        })
                })
        );
    }
}

#[test]
fn p37a_direct_harvest_keeps_noncasting_monster_mechanics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (level, ids) in [
        (
            18,
            &[
                "fire-spirit",
                "shagrat-the-orc-captain",
                "gorbag-the-orc-captain",
                "white-shark",
                "stunwall",
                "quartz-vein",
                "monkey-of-nikko",
                "tiger-snake",
                "ozmanian-devil",
            ][..],
        ),
        (
            19,
            &[
                "stone-golem",
                "red-mold",
                "aquatic-golem",
                "orc-digger",
                "tin-golem",
            ][..],
        ),
        (
            20,
            &[
                "lizard-king",
                "landmine",
                "wyvern",
                "livingstone",
                "sabre-tooth-tiger",
                "sasquatch",
                "weir",
                "whale",
                "electric-eel",
                "werewolf",
                "ugluk-the-uruk",
                "noxious-fume",
                "nue",
                "spider-bomb",
                "glyptodont",
                "frost-spirit",
                "blue-ringed-octopus",
                "box-jellyfish",
                "yowie",
            ][..],
        ),
    ] {
        for id in ids {
            let definition = actor(&format!("demo.actor.{id}"));
            assert_eq!(definition.level, level, "{id} should keep its level");
            assert!(
                definition.monster_casting.is_none(),
                "{id} should not gain monster casting"
            );
        }
    }

    for (id, damage_type, dice, sides) in [
        ("fire-spirit", ActorDamageType::Fire, 1, 2),
        ("frost-spirit", ActorDamageType::Cold, 1, 2),
        ("box-jellyfish", ActorDamageType::Poison, 2, 15),
    ] {
        assert!(
            actor(&format!("demo.actor.{id}"))
                .contact_auras
                .first()
                .is_some_and(|aura| {
                    aura.damage_type == damage_type
                        && aura.damage_dice == dice
                        && aura.damage_sides == sides
                })
        );
    }

    for id in ["quartz-vein", "livingstone", "noxious-fume"] {
        assert!(
            actor(&format!("demo.actor.{id}"))
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.multiplies),
            "{id} should retain reproduction"
        );
    }
    for id in ["landmine", "spider-bomb"] {
        assert!(
            actor(&format!("demo.actor.{id}"))
                .melee_routine
                .as_ref()
                .and_then(|routine| routine.blows.first())
                .is_some_and(|blow| blow.self_destructs)
        );
    }

    let orc_digger = actor("demo.actor.orc-digger");
    assert!(orc_digger.moves_weaker_bodies);
    assert!(orc_digger.terrain_interaction.destroys_walls);

    let wyvern = actor("demo.actor.wyvern");
    assert!(wyvern.rideable && wyvern.moves_weaker_bodies);
    assert!(wyvern.movement.modes.contains(&ActorMovementMode::Fly));

    assert!(
        actor("demo.actor.lizard-king")
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().any(|blow| blow
                .effects
                .iter()
                .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Terrify { .. }))))
    );
    let blue_ringed = actor("demo.actor.blue-ringed-octopus");
    assert!(
        blue_ringed
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().all(|blow| {
                blow.effects
                    .iter()
                    .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Paralysis { .. }))
                    && blow
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Poison { .. }))
            }))
    );
}

#[test]
fn p37b_direct_harvest_reuses_existing_monster_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>()
    };

    for (ids, ability_id) in [
        (
            &["fire-hound", "chimera"][..],
            "rfb-legacy.ability.breath-fire-20-900-r2",
        ),
        (
            &["cold-hound", "demonite"][..],
            "rfb-legacy.ability.breath-cold-20-900-r2",
        ),
        (
            &["energy-hound"][..],
            "rfb-legacy.ability.breath-electricity-20-900-r2",
        ),
        (
            &["giant-black-dragon-fly", "water-hound"][..],
            "rfb-legacy.ability.breath-acid-20-900-r2",
        ),
        (
            &["giant-gold-dragon-fly"][..],
            "rfb-legacy.ability.breath-sound-17-450-r2",
        ),
        (
            &["air-hound"][..],
            "rfb-legacy.ability.breath-poison-17-600-r2",
        ),
    ] {
        for id in ids {
            assert_eq!(
                ability_ids(&format!("demo.actor.{id}")),
                [ability_id].into_iter().collect(),
                "{id} should reuse its existing breath"
            );
        }
    }

    for id in ["blink-dog", "huorn", "hopper-ant", "phase-spider"] {
        assert_eq!(
            ability_ids(&format!("demo.actor.{id}")),
            ["rfb-legacy.ability.blink", "rfb-legacy.ability.drag"]
                .into_iter()
                .collect(),
            "{id} should only blink and pull its target"
        );
    }
    assert_eq!(
        ability_ids("demo.actor.shambling-mound"),
        ["rfb-legacy.ability.shriek"].into_iter().collect()
    );
    assert_eq!(
        ability_ids("demo.actor.pumpkin-man"),
        [
            "rfb-legacy.ability.blind",
            "rfb-legacy.ability.confuse",
            "rfb-legacy.ability.curse-3d8",
            "rfb-legacy.ability.darkness",
            "rfb-legacy.ability.paralyze",
            "rfb-legacy.ability.scare",
        ]
        .into_iter()
        .collect()
    );

    assert!(
        actor("demo.actor.giant-black-dragon-fly")
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.is_empty())
    );
    assert!(actor("demo.actor.huorn").movement.never_moves);
    let chimera = actor("demo.actor.chimera");
    assert!(chimera.rideable);
    assert!(chimera.movement.modes.contains(&ActorMovementMode::Fly));
}

#[test]
fn p38a_monsters_bind_exact_parameterized_damage_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let assert_abilities = |actor_id: &str, expected: &[&str]| {
        let actual = actor(actor_id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{actor_id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.clone())
            .collect::<BTreeSet<_>>();
        let expected = expected
            .iter()
            .map(|id| format!("rfb-legacy.ability.{id}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{actor_id} ability set");
    };

    for id in ["potion-mimic", "door-mimic"] {
        assert_abilities(
            id,
            &["blind", "bolt-cold-6d8-6", "confuse", "curse-8d8", "scare"],
        );
        assert!(actor(id).movement.never_moves);
    }
    assert_abilities("uruk", &["bolt-physical-3d5"]);
    assert_abilities(
        "chaos-beastman",
        &[
            "bolt-fire-9d8-6",
            "bolt-physical-2d6-6",
            "confuse",
            "escape",
        ],
    );
    assert_abilities("giant-bronze-dragon-fly", &["breath-confusion-17-400-r2"]);
    assert_abilities("stone-giant", &["bolt-physical-1d1-53"]);
    assert_abilities("snow-golem", &["ball-cold-1d1-17"]);
    assert_abilities("bush-ranger", &["bolt-physical-8d6"]);
    assert_abilities(
        "frost-giant",
        &[
            "ball-cold-1d30-10",
            "bolt-cold-6d8-6",
            "bolt-physical-1d1-59",
        ],
    );
    assert_abilities("earth-hound", &["breath-shards-17-500-r2"]);
    assert_abilities(
        "dark-elven-lord",
        &[
            "blind",
            "bolt-cold-6d8-6",
            "bolt-fire-9d8-6",
            "bolt-physical-2d6-6",
            "confuse",
            "darkness",
            "haste-self",
        ],
    );

    for (id, damage_type, dice, sides, bonus) in [
        ("bolt-cold-6d8-6", ActorDamageType::Cold, 6, 8, 6),
        ("bolt-physical-3d5", ActorDamageType::Physical, 3, 5, 0),
        ("bolt-physical-2d6-6", ActorDamageType::Physical, 2, 6, 6),
        ("bolt-fire-9d8-6", ActorDamageType::Fire, 9, 8, 6),
        ("bolt-physical-1d1-53", ActorDamageType::Physical, 1, 1, 53),
        ("bolt-physical-8d6", ActorDamageType::Physical, 8, 6, 0),
        ("bolt-physical-1d1-59", ActorDamageType::Physical, 1, 1, 59),
    ] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type: actual_type,
            } if damage_dice == dice
                && damage_sides == sides
                && damage_bonus == bonus
                && actual_type == damage_type
        ));
    }
    for (id, sides, bonus) in [("ball-cold-1d1-17", 1, 17), ("ball-cold-1d30-10", 30, 10)] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::AreaDamage {
                damage_dice: 1,
                damage_sides,
                damage_bonus,
                damage_type: ActorDamageType::Cold,
                radius: 2,
                ..
            } if damage_sides == sides && damage_bonus == bonus
        ));
    }
    for (id, damage_type, hp_percent, max_damage) in [
        (
            "breath-confusion-17-400-r2",
            ActorDamageType::Confusion,
            17,
            400,
        ),
        ("breath-shards-17-500-r2", ActorDamageType::Shards, 17, 500),
    ] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::BreathDamage {
                hp_percent: actual_percent,
                max_damage: actual_max,
                damage_type: actual_type,
                radius: 2,
            } if actual_percent == hp_percent
                && actual_max == max_damage
                && actual_type == damage_type
        ));
    }
}

#[test]
fn p38b_monsters_bind_exact_healing_and_summoning_parameters() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let assert_abilities = |actor_id: &str, expected: &[&str]| {
        let actual = actor(actor_id)
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{actor_id} should retain monster casting"))
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.clone())
            .collect::<BTreeSet<_>>();
        let expected = expected
            .iter()
            .map(|id| format!("rfb-legacy.ability.{id}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{actor_id} ability set");
    };

    assert_abilities(
        "daemonette-of-slaanesh",
        &[
            "bolt-cold-6d8-6",
            "bolt-fire-9d8-6",
            "confuse",
            "curse-8d8",
            "scare",
            "summon-demon-l18-1d3-1",
        ],
    );
    assert_abilities(
        "meng-huo-the-king-of-southerings",
        &["kin-meng-huo-the-king-of-southerings"],
    );
    assert_abilities(
        "ixitxachitl-priest",
        &[
            "blind",
            "curse-8d8",
            "drag",
            "heal-57",
            "scare",
            "summon-legacy-import-l19-1d1",
        ],
    );
    assert_abilities("quylthulg", &["blink", "summon-legacy-import-l20-1d1"]);
    assert_abilities(
        "paladin",
        &[
            "blind",
            "curse-3d8",
            "curse-8d8",
            "heal-60",
            "scare",
            "slow",
        ],
    );
    assert_abilities(
        "ranger",
        &[
            "blink",
            "bolt-electricity-4d8-6",
            "bolt-physical-2d6-6",
            "bolt-physical-5d6",
            "summon-legacy-import-l20-1d1",
        ],
    );

    assert!(matches!(
        ability("summon-demon-l18-1d3-1").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 18,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 1,
            ..
        } if category == "demon"
    ));
    assert!(matches!(
        ability("kin-meng-huo-the-king-of-southerings").effect,
        AbilityEffectDefinition::Summon {
            ref actor_kind_id,
            count: 2,
            radius: 2,
            ..
        } if actor_kind_id == "demo.actor.meng-huo-the-king-of-southerings"
    ));
    for (id, maximum_level) in [
        ("summon-legacy-import-l19-1d1", 19),
        ("summon-legacy-import-l20-1d1", 20),
    ] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level: actual_level,
                count_dice: 1,
                count_sides: 1,
                count_bonus: 0,
                ..
            } if category == "legacy-import" && actual_level == maximum_level
        ));
    }
    for (id, amount) in [("heal-57", 57), ("heal-60", 60)] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::Heal { amount: actual } if actual == amount
        ));
    }
    assert!(matches!(
        ability("bolt-physical-5d6").effect,
        AbilityEffectDefinition::Damage {
            damage_dice: 5,
            damage_sides: 6,
            damage_bonus: 0,
            damage_type: ActorDamageType::Physical,
        }
    ));
    assert!(matches!(
        ability("bolt-electricity-4d8-6").effect,
        AbilityEffectDefinition::Damage {
            damage_dice: 4,
            damage_sides: 8,
            damage_bonus: 6,
            damage_type: ActorDamageType::Electricity,
        }
    ));
}

#[test]
fn p39_monsters_bind_jump_damage_and_ordered_contact_auras() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    let blinking = actor("blinking-light");
    assert_eq!(blinking.level, 19);
    assert_eq!(
        blinking.monster_casting.as_ref().unwrap().abilities[0].ability_id,
        "rfb-legacy.ability.jump-light-5d5"
    );
    assert!(matches!(
        ability("jump-light-5d5").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 5,
            damage_sides: 5,
            damage_bonus: 0,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: ActorDamageType::Light,
            radius: 5,
            blink_radius: 10,
        }
    ));

    let queen = actor("the-icky-queen");
    assert_eq!(queen.level, 20);
    assert_eq!(
        queen
            .contact_auras
            .iter()
            .map(|aura| (aura.damage_type, aura.damage_dice, aura.damage_sides))
            .collect::<Vec<_>>(),
        vec![
            (ActorDamageType::Poison, 2, 3),
            (ActorDamageType::Acid, 2, 3),
        ]
    );
    let queen_abilities = queen
        .monster_casting
        .as_ref()
        .unwrap()
        .abilities
        .iter()
        .map(|candidate| candidate.ability_id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(queen_abilities.contains("rfb-legacy.ability.drain-mana-11"));
    assert!(queen_abilities.contains("rfb-legacy.ability.kin-the-icky-queen"));
    assert!(matches!(
        ability("drain-mana-11").effect,
        AbilityEffectDefinition::DrainResource { amount: 11 }
    ));
    assert!(matches!(
        ability("kin-the-icky-queen").effect,
        AbilityEffectDefinition::Summon {
            ref actor_kind_id,
            count: 2,
            radius: 2,
            ..
        } if actor_kind_id == "demo.actor.the-icky-queen"
    ));
}

#[test]
fn orc_cave_small_casting_mechanics_reuse_jump_and_category_summons() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, bonus, damage_type) in [
        ("jump-fire-l31", 31, ActorDamageType::Fire),
        ("jump-fire-l32", 32, ActorDamageType::Fire),
        ("jump-poison-l32", 32, ActorDamageType::Poison),
        ("jump-confusion-l32", 32, ActorDamageType::Confusion),
    ] {
        assert!(matches!(
            ability(id).effect,
            AbilityEffectDefinition::JumpDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus,
                damage_multiplier_numerator: 5,
                damage_multiplier_denominator: 4,
                damage_type: actual_type,
                radius: 5,
                blink_radius: 10,
            } if damage_bonus == bonus && actual_type == damage_type
        ));
    }
    assert!(matches!(
        ability("jump-dark-2d4").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 2,
            damage_sides: 4,
            damage_bonus: 0,
            damage_type: ActorDamageType::Dark,
            ..
        }
    ));

    for (actor_id, ability_id, maximum_level) in [
        ("it", "summon-hydra-l24-1d3-1", 24),
        ("gachapin", "summon-hydra-l29-1d3-1", 29),
    ] {
        assert!(
            actor(actor_id)
                .monster_casting
                .as_ref()
                .unwrap()
                .abilities
                .iter()
                .any(|candidate| candidate.ability_id.ends_with(ability_id))
        );
        assert!(matches!(
            ability(ability_id).effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level: actual_level,
                count_dice: 1,
                count_sides: 3,
                count_bonus: 1,
                ..
            } if category == "hydra" && actual_level == maximum_level
        ));
    }
    for id in ["2-headed-hydra", "4-headed-hydra", "5-headed-hydra"] {
        assert!(actor(id).tags.iter().any(|tag| tag == "hydra"));
    }
    assert!(
        actor("gelatinous-cube")
            .tags
            .iter()
            .any(|tag| tag == "gelatinous-cube")
    );
    assert!(matches!(
        ability("summon-gelatinous-cube-l16-1d3").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 16,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 0,
            ..
        } if category == "gelatinous-cube"
    ));
    assert_eq!(
        actor("ninja")
            .death_drop
            .as_ref()
            .and_then(|drop| drop.theme_table_id.as_deref()),
        Some("demo.loot-table.ninja")
    );
}

#[test]
fn orc_cave_vampiric_melee_stays_physical_and_unarmored() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for id in ["vampiric-mist", "black", "vampiric-ixitxachitl"] {
        let vampiric = actor(id)
            .melee_routine
            .as_ref()
            .into_iter()
            .flat_map(|routine| &routine.blows)
            .flat_map(|blow| &blow.effects)
            .filter(|effect| {
                matches!(
                    effect,
                    MeleeBlowEffectDefinition::Damage {
                        damage_type: ActorDamageType::Physical,
                        armor_mitigated: false,
                        vampiric: true,
                        ..
                    }
                )
            })
            .count();
        assert!(vampiric > 0, "{id} should retain its VAMP blow");
    }
}

#[test]
fn orc_cave_animate_dead_preserves_remain_specific_failure_rates() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.animate-dead")
        .expect("animate dead should be imported");
    let AbilityEffectDefinition::Sequence { effects } = &ability.effect else {
        panic!("animate dead should keep corpse and skeleton steps")
    };
    assert!(matches!(
        effects.as_slice(),
        [
            AbilityEffectDefinition::AnimateDead {
                corpse_item_kind_id,
                radius: 5,
                count: 8,
                failure_chance_percent: 20,
                ..
            },
            AbilityEffectDefinition::AnimateDead {
                corpse_item_kind_id: skeleton_item_kind_id,
                radius: 5,
                count: 8,
                failure_chance_percent: 40,
                ..
            }
        ] if corpse_item_kind_id == "demo.item.corpse-remains"
            && skeleton_item_kind_id == "demo.item.skeleton-remains"
    ));

    for id in ["demo.actor.arch-vile", "demo.actor.orc-warlock"] {
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("{id} should be imported"));
        assert!(actor.monster_casting.as_ref().is_some_and(|casting| {
            casting
                .abilities
                .iter()
                .any(|candidate| candidate.ability_id == ability.id)
        }));
    }
}

#[test]
fn orc_cave_contact_auras_and_polymorph_keep_explicit_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, damage_type, dice, sides) in [
        ("kharis-the-powerslave", ActorDamageType::Curse, 1, 3),
        ("hisser", ActorDamageType::Electricity, 2, 3),
        ("flaming-crow", ActorDamageType::Fire, 1, 2),
    ] {
        assert!(actor(id).contact_auras.iter().any(|aura| {
            aura.damage_type == damage_type
                && aura.damage_dice == dice
                && aura.damage_sides == sides
        }));
    }
    assert!(
        !actor("flaming-crow")
            .tags
            .iter()
            .any(|tag| tag == "orc-cave")
    );

    let polymorph = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.polymorph-target")
        .expect("polymorph target should be imported");
    assert!(matches!(
        polymorph.effect,
        AbilityEffectDefinition::PolymorphTarget
    ));
    assert!(
        actor("dokkaebi")
            .monster_casting
            .as_ref()
            .is_some_and(|casting| {
                casting
                    .abilities
                    .iter()
                    .any(|candidate| candidate.ability_id == polymorph.id)
            })
    );
}

#[test]
fn orc_cave_o5_traits_keep_explicit_runtime_tags() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    assert!(
        actor("grendel")
            .tags
            .iter()
            .any(|tag| tag == "aura-revenge")
    );
    assert!(
        actor("jade-monk")
            .tags
            .iter()
            .any(|tag| tag == "aura-revenge")
    );
    assert!(
        actor("fearmaster")
            .tags
            .iter()
            .any(|tag| tag == "aura-fear")
    );
    assert!(actor("tanuki").tags.iter().any(|tag| tag == "tanuki"));
    for id in [
        "suke-san-the-mitsukuni-s-warder",
        "kaku-san-the-mitsukuni-s-warder",
        "silver-angel",
    ] {
        assert!(actor(id).tags.iter().any(|tag| tag == "unique2"));
    }
}

#[test]
fn orc_cave_o6_unlife_stays_separate_from_hp_damage() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for id in ["vampire", "ghoulking"] {
        assert!(
            actor(id)
                .melee_routine
                .as_ref()
                .into_iter()
                .flat_map(|routine| &routine.blows)
                .flat_map(|blow| &blow.effects)
                .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Unlife { .. })),
            "{id} should retain its UNLIFE blow"
        );
    }
}

#[test]
fn orc_cave_o7_binds_othrod_depths_ecology_and_final_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let othrod = content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.othrod-lord-of-the-orcs")
        .expect("Othrod should be imported");
    assert_eq!(othrod.level, 32);
    assert_eq!(othrod.glyph, "o");
    assert!(othrod.tags.iter().any(|tag| tag == "orc"));
    assert!(othrod.tags.iter().any(|tag| tag == "unique"));
    assert!(othrod.tags.iter().any(|tag| tag == "kin-glyph-111"));
    assert_eq!(
        othrod
            .melee_routine
            .as_ref()
            .expect("Othrod should retain four melee blows")
            .blows
            .len(),
        4
    );
    assert!(
        othrod
            .monster_casting
            .as_ref()
            .expect("Othrod should summon orc kin")
            .abilities
            .iter()
            .any(|ability| ability.ability_id == "rfb-legacy.ability.kin-othrod-lord-of-the-orcs")
    );

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.orc-cave")
        .expect("Orc Cave should be active");
    assert_eq!(dungeon.legacy_index, Some(3));
    assert_eq!(dungeon.root_floor_id, "demo.floor.orc-cave-depth-15");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.othrod-lord-of-the-orcs"
    );
    assert!(
        world
            .wilderness
            .as_ref()
            .expect("Middle-earth should retain wilderness")
            .locations
            .iter()
            .any(|location| matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 30, y: 45 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.orc-cave"
            ))
    );

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.orc-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 18);
    assert_eq!(floors.first().map(|floor| floor.depth), Some(15));
    assert_eq!(floors.last().map(|floor| floor.depth), Some(32));
    assert!(
        floors
            .iter()
            .all(|floor| (floor.width, floor.height) == (96, 33))
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.orc-cave-entrance")
    );
    let final_floor = floors.last().expect("Orc Cave should have a final floor");
    assert!(final_floor.final_floor);
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("depth 32 should contain Othrod");
    assert_eq!(guardian.instance_id, "demo.guardian.orc-cave.1");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.orc-cave-final-reward")
    );

    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.orc-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Orc Cave should use global allocation");
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);
    assert_eq!(
        policy
            .preferred_tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["animal", "orc", "troll"].into_iter().collect()
    );
    assert_eq!(
        policy
            .preferred_glyphs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["o", "O", "T", "C"].into_iter().collect()
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.orc-cave-final-reward")
        .expect("Othrod should have a fixed reward table");
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.ring");
    assert_eq!(
        reward.affix_weights[0].affix_id.as_deref(),
        Some("rfb-legacy.affix.combat")
    );
    let combat = content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.combat")
        .expect("the Combat ego should be imported");
    assert_eq!(combat.roll_groups.len(), 1);
    assert_eq!(combat.roll_groups[0].rolls, 3);
    assert_eq!(
        combat.roll_groups[0]
            .candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<u32>(),
        100
    );
}

#[test]
fn p87b_tidal_cave_policy_preserves_the_three_way_legacy_ecology_preference() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let policy = artifact
        .content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.tidal-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Tidal Cave should use global allocation");

    assert!(policy.preferred_glyphs.is_empty());
    assert!(policy.preferred_tags.is_empty());
    assert_eq!(
        policy.preferred_movement_modes,
        [ActorMovementMode::Aquatic, ActorMovementMode::Swim]
    );
    assert_eq!(policy.preferred_habitats, [ActorHabitat::Shore]);
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);
}

#[test]
fn p87c_tidal_cave_binds_depths_water_features_river_and_guardian() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.tidal-cave")
        .expect("Tidal Cave should be active");
    assert_eq!(dungeon.legacy_index, Some(33));
    assert_eq!(dungeon.root_floor_id, "demo.floor.tidal-cave-depth-15");
    assert_eq!(dungeon.guardian_actor_kind_id, "demo.actor.grendel");

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.tidal-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 13);
    assert_eq!(floors.first().map(|floor| floor.depth), Some(15));
    assert_eq!(floors.last().map(|floor| floor.depth), Some(27));
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.tidal-cave-entrance")
    );

    for floor in &floors {
        assert_eq!(
            floor.encounter_table_id.as_deref(),
            Some("demo.encounter-table.tidal-cave")
        );
        assert_eq!(
            floor.terrain_feature_table_id.as_deref(),
            Some("demo.terrain-feature-table.tidal-cave")
        );
        let budget = floor.generation_budget.as_ref().expect("generation budget");
        assert_eq!(budget.feature_placements, Some(96));
        assert_eq!(budget.river_area_tiles, Some(160));
        let layout = floor.layout.as_ref().expect("Tidal Cave layout");
        assert!(layout.lake.is_none());
        assert!(layout.cavern.is_none());
        assert_eq!(
            layout
                .rooms
                .as_ref()
                .expect("room geometry")
                .shapes
                .iter()
                .map(|shape| (shape.shape, shape.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                (ProceduralRoomShape::Rectangle, 1),
                (ProceduralRoomShape::Cavern, 9),
            ])
        );
        let river = layout.river.as_ref().expect("water river");
        assert_eq!(river.deep_terrain_id, "demo.terrain.surface-water-deep");
        assert_eq!(
            river.shallow_terrain_id,
            "demo.terrain.surface-water-shallow"
        );
        assert_eq!(river.chance_one_in, Some(7));
        assert_eq!(
            layout
                .streamers
                .iter()
                .map(|streamer| (streamer.terrain_id.as_str(), streamer.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("demo.terrain.magma-vein", 2),
                ("demo.terrain.quartz-vein", 3),
            ])
        );
    }

    let final_floor = floors.last().expect("depth 27 should exist");
    assert!(final_floor.final_floor);
    let guardian = final_floor.guardian.as_ref().expect("Grendel guardian");
    assert_eq!(guardian.actor_kind_id, "demo.actor.grendel");

    let feature_table = content
        .terrain_feature_tables
        .iter()
        .find(|table| table.id == "demo.terrain-feature-table.tidal-cave")
        .expect("Tidal Cave terrain feature table");
    assert_eq!(feature_table.rolls, 96);
    assert_eq!(feature_table.entries.len(), 1);
    assert_eq!(
        feature_table.entries[0].terrain_id,
        "demo.terrain.surface-water-shallow"
    );
    assert_eq!(
        feature_table.entries[0].placement,
        TerrainFeaturePlacement::Room
    );
}

#[test]
fn p90c_troll_cave_binds_substitution_ecology_terrain_shafts_and_guardian() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.troll-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Troll cave should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["O", "T", "h", "p"]);
    assert_eq!(policy.preferred_tags, ["animal", "troll"]);
    assert_eq!(policy.special_div, 12);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let orc_cave = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.orc-cave")
        .expect("Orc Cave should exist");
    assert_eq!(
        orc_cave.substitution,
        Some(DungeonSubstitutionDefinition {
            alternate_dungeon_id: "demo.dungeon.troll-cave".to_owned(),
            alternate_gate_one_in: None,
        })
    );
    let troll_cave = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.troll-cave")
        .expect("Troll cave should exist");
    assert_eq!(troll_cave.legacy_index, Some(36));
    assert_eq!(troll_cave.root_floor_id, "demo.floor.troll-cave-depth-18");
    assert_eq!(
        troll_cave.guardian_actor_kind_id,
        "demo.actor.spulga-the-troll-priestess"
    );

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.troll-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 19);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (18..=36).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.orc-cave-entrance")
    );
    assert_eq!(
        floors[0].entry_connection_id.as_deref(),
        Some("demo.connection.troll-cave-depth-18-stairs-up")
    );
    assert!(floors.iter().all(|floor| {
        floor.floor_terrain_id == "demo.terrain.dirt"
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.troll-cave")
            && floor
                .layout
                .as_ref()
                .and_then(|layout| layout.rooms.as_ref())
                .is_some_and(|rooms| {
                    rooms.shapes.iter().any(|candidate| {
                        candidate.shape == ProceduralRoomShape::Cavern && candidate.weight == 9
                    })
                })
    }));
    assert!(floors.iter().all(|floor| {
        floor.wall_terrain_id == "demo.terrain.wall"
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.streamers.iter().any(|streamer| {
                    streamer.terrain_id == "demo.terrain.mountain-wall" && streamer.weight == 3
                })
            })
    }));
    assert_eq!(
        floors
            .iter()
            .filter(|floor| {
                floor.layout.as_ref().is_some_and(|layout| {
                    layout.lake.as_ref().is_some_and(|lake| {
                        lake.deep_terrain_id == "demo.terrain.surface-water-deep"
                    })
                })
            })
            .count(),
        2
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| {
                floor.layout.as_ref().is_some_and(|layout| {
                    layout.lake.as_ref().is_some_and(|lake| {
                        lake.deep_terrain_id == "demo.terrain.rubble"
                            && lake.shallow_terrain_id == "demo.terrain.dirt"
                    })
                })
            })
            .count(),
        1
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.river.is_some()))
            .count(),
        18
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| {
                floor
                    .layout
                    .as_ref()
                    .is_some_and(|layout| layout.destroyed.is_some())
            })
            .count(),
        3
    );
    assert_eq!(
        floors
            .iter()
            .flat_map(|floor| &floor.connections)
            .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
            .count(),
        34
    );

    let feature_table = content
        .terrain_feature_tables
        .iter()
        .find(|table| table.id == "demo.terrain-feature-table.troll-cave")
        .expect("Troll cave terrain mix");
    assert_eq!(feature_table.rolls, 240);
    assert_eq!(feature_table.entries.len(), 1);
    assert_eq!(
        feature_table.entries[0].terrain_id,
        "demo.terrain.surface-grass"
    );

    let final_floor = floors.last().expect("depth 36 should exist");
    assert!(final_floor.final_floor);
    let guardian = final_floor.guardian.as_ref().expect("Spulga guardian");
    assert_eq!(guardian.instance_id, "demo.guardian.troll-cave.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.spulga-the-troll-priestess"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.troll-cave-final-reward")
    );
}

#[test]
fn p91b_eyrie_binds_entrance_ecology_shafts_guardians_and_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.eyrie")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Eyrie should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["H", "O", "Y"]);
    assert_eq!(policy.preferred_tags, ["animal", "giant", "troll"]);
    assert_eq!(policy.preferred_movement_modes, [ActorMovementMode::Fly]);
    assert_eq!(policy.preferred_habitats, [ActorHabitat::Mountain]);
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.eyrie")
        .expect("Eyrie should exist");
    assert_eq!(dungeon.legacy_index, Some(14));
    assert_eq!(dungeon.root_floor_id, "demo.floor.eyrie-depth-40");
    assert_eq!(dungeon.guardian_actor_kind_id, "demo.actor.thorondor");
    let entrance_guardian = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Eyrie should have an entrance guardian");
    assert_eq!(
        entrance_guardian.instance_id,
        "demo.guardian.eyrie-entrance.1"
    );
    assert_eq!(entrance_guardian.actor_kind_id, "demo.actor.jubjub-bird");
    assert_eq!(entrance_guardian.position, ContentPosition { x: 47, y: 16 });
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 76, y: 46 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.eyrie"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.eyrie"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 11);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (40..=50).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.eyrie-entrance")
    );
    assert_eq!(
        floors[0].entry_connection_id.as_deref(),
        Some("demo.connection.eyrie-depth-40-stairs-up")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.wall_terrain_id == "demo.terrain.mountain-wall"
            && floor.floor_terrain_id == "demo.terrain.surface-grass"
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.river.is_some()
                    && layout.streamers.is_empty()
                    && layout.rooms.as_ref().is_some_and(|rooms| {
                        rooms.shapes
                            == [ProceduralRoomShapeCandidateDefinition {
                                shape: ProceduralRoomShape::Cavern,
                                weight: 1,
                            }]
                    })
            })
    }));
    assert_eq!(
        floors
            .iter()
            .flat_map(|floor| &floor.connections)
            .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
            .count(),
        18
    );

    let final_floor = floors.last().expect("depth 50 should exist");
    assert!(final_floor.final_floor);
    let guardian = final_floor.guardian.as_ref().expect("Thorondor guardian");
    assert_eq!(guardian.instance_id, "demo.guardian.eyrie.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.thorondor");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.eyrie-final-reward")
    );
    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.eyrie-final-reward")
        .expect("Eyrie reward table");
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.new-life-potion");
}

#[test]
fn p92b_labyrinth_binds_maze_forgetting_guardian_and_recall_rod() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.labyrinth")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Labyrinth should use global allocation");
    assert!(policy.preferred_glyphs.is_empty());
    assert!(policy.preferred_tags.is_empty());
    assert!(policy.preferred_movement_modes.is_empty());
    assert!(policy.preferred_habitats.is_empty());
    assert_eq!(policy.special_div, 64);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.labyrinth")
        .expect("Labyrinth should exist");
    assert_eq!(dungeon.legacy_index, Some(4));
    assert_eq!(dungeon.root_floor_id, "demo.floor.labyrinth-depth-20");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.the-minotaur-of-the-labyrinth"
    );
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 5, y: 48 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.labyrinth"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.labyrinth"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 9);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (20..=28).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.labyrinth-entrance")
    );
    assert!(floors.iter().all(|floor| {
        floor.forget_after_move
            && (floor.width, floor.height) == (66, 22)
            && floor.wall_terrain_id == "demo.terrain.wall"
            && floor.floor_terrain_id == "demo.terrain.floor"
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.mode == ProceduralLayoutMode::MazeOnly
                    && layout
                        .maze
                        .as_ref()
                        .is_some_and(|maze| (maze.width, maze.height) == (61, 17))
                    && layout.rooms.is_none()
                    && layout.streamers.len() == 2
            })
            && floor
                .generation_budget
                .as_ref()
                .is_some_and(|budget| budget.maze_floor_tiles == Some(557))
    }));

    let final_floor = floors.last().expect("depth 28 should exist");
    assert!(final_floor.final_floor);
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("the Minotaur should guard depth 28");
    assert_eq!(guardian.instance_id, "demo.guardian.labyrinth.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.the-minotaur-of-the-labyrinth"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.labyrinth-final-reward")
    );
    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.labyrinth-final-reward")
        .expect("Labyrinth reward table");
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.recall-rod");

    let rod = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.recall-rod")
        .expect("fixed recall rod should exist");
    let activation = &rod
        .device_generation
        .as_ref()
        .expect("reward should be a rod")
        .activations[0];
    assert_eq!(activation.id, "demo.device-activation.recall");
    assert_eq!(activation.device_check_difficulty, 27);
    assert_eq!(
        (
            activation.charges.minimum,
            activation.charges.maximum,
            activation.charges.cost,
        ),
        (40, 40, 15)
    );
    assert!(matches!(
        activation.effect,
        ItemUseEffectDefinition::Recall {
            delay_dice: 1,
            delay_sides: 21,
            delay_bonus: 14,
        }
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.labyrinth")
        .and_then(|table| table.global_allocation.as_mut())
        .expect("Labyrinth policy should exist")
        .special_div = 16;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidEncounterTable(id)) if id == "demo.encounter-table.labyrinth"
    ));
}

#[test]
fn p93b_lonely_mountain_binds_lava_ecology_smaug_and_arkenstone() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.lonely-mountain")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Lonely Mountain should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["D", "d"]);
    assert_eq!(policy.preferred_tags, ["dragon"]);
    assert_eq!(policy.special_div, 10);
    assert_eq!(policy.ambient_chance_one_in, 140);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.lonely-mountain")
        .expect("Lonely Mountain should exist");
    assert_eq!(dungeon.legacy_index, Some(23));
    assert_eq!(dungeon.root_floor_id, "demo.floor.lonely-mountain-depth-30");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.smaug-the-golden"
    );
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 42, y: 58 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.lonely-mountain"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.lonely-mountain"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 11);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (30..=40).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.lonely-mountain-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.wall_terrain_id == "demo.terrain.wall"
            && floor.floor_terrain_id == "demo.terrain.dirt"
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.streamers.len() == 2
                    && layout.river.as_ref().is_none_or(|river| {
                        river.deep_terrain_id == "demo.terrain.surface-lava-deep"
                            && river.shallow_terrain_id == "demo.terrain.surface-lava-shallow"
                    })
                    && layout.rooms.as_ref().is_some_and(|rooms| {
                        rooms.shapes.iter().any(|candidate| {
                            candidate.shape == ProceduralRoomShape::Cavern && candidate.weight == 9
                        }) && rooms.shapes.iter().any(|candidate| {
                            candidate.shape == ProceduralRoomShape::Rectangle
                                && candidate.weight == 1
                        })
                    })
            })
    }));
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.river.is_some()))
            .count(),
        9
    );
    let lake_terrain = floors
        .iter()
        .filter_map(|floor| {
            floor
                .layout
                .as_ref()
                .and_then(|layout| layout.lake.as_ref())
                .map(|lake| lake.deep_terrain_id.as_str())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        lake_terrain,
        [
            "demo.terrain.rubble",
            "demo.terrain.surface-lava-deep",
            "demo.terrain.surface-tree",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.destroyed.is_some()))
            .count(),
        2
    );

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Smaug should guard depth 40");
    assert_eq!(guardian.instance_id, "demo.guardian.lonely-mountain.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.smaug-the-golden");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.lonely-mountain-final-replacement")
    );
    assert_eq!(
        guardian.reward_artifact_item_kind_id.as_deref(),
        Some("demo.item.arkenstone-of-thrain")
    );

    let base = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.arkenstone")
        .expect("Arkenstone base kind should exist");
    assert_eq!((base.generation_level, base.weight_tenths_pound), (50, 5));
    assert_eq!(base.base_value, 25_000);
    assert_eq!(base.equipment_slot.as_deref(), Some("light"));

    let arkenstone = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.arkenstone-of-thrain")
        .expect("the fixed Arkenstone should exist");
    assert_eq!(
        (arkenstone.generation_level, arkenstone.weight_tenths_pound),
        (50, 20)
    );
    assert_eq!(arkenstone.base_value, 100_000);
    assert_eq!(arkenstone.equipment_bonuses.light_radius, 3);
    assert_eq!(
        arkenstone.artifact_generation,
        Some(ArtifactGenerationDefinition {
            source_index: 329,
            base_item_kind_id: "demo.item.arkenstone".to_owned(),
            rarity_one_in: 5,
            instant: true,
            affix_ids: Vec::new(),
        })
    );
    assert_eq!(
        arkenstone.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        arkenstone.resistances.get(&ActorDamageType::Dark),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(
        arkenstone
            .passives
            .contains(&EquipmentPassive::SeeInvisible)
    );
    assert!(arkenstone.passives.contains(&EquipmentPassive::HoldLife));
    let generation = arkenstone
        .device_generation
        .as_ref()
        .expect("the Arkenstone should retain Clairvoyance");
    assert_eq!(
        generation.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 1_500,
            energy_per_mille: 1_000,
        })
    );
    let activation = &generation.activations[0];
    assert_eq!(activation.device_check_difficulty, 50);
    assert_eq!(
        (
            activation.charges.minimum,
            activation.charges.maximum,
            activation.charges.cost,
        ),
        (1, 1, 1)
    );
    let ItemUseEffectDefinition::Sequence { effects } = &activation.effect else {
        panic!("Clairvoyance should retain its ordered full-floor effects");
    };
    assert_eq!(effects.len(), 5);
    assert!(matches!(
        effects[0],
        ItemUseEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Terrain,
            ref category,
            radius: u8::MAX,
            persistent: true,
            through_walls: true,
        } if category == "map"
    ));
    assert!(matches!(
        effects[1],
        ItemUseEffectDefinition::SetFloorGlow {
            glow: true,
            radius: u8::MAX,
            connected_glow: false,
        }
    ));
    assert!(matches!(
        effects[2],
        ItemUseEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Item,
            ref category,
            radius: u8::MAX,
            persistent: false,
            through_walls: true,
        } if category == "item"
    ));
}

#[test]
fn p97d_dragon_lair_binds_dragons_guardians_layers_and_scale_mail() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.dragon-lair")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Dragon's Lair should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["D", "d"]);
    assert_eq!(policy.preferred_tags, ["dragon"]);
    assert_eq!(policy.special_div, 10);
    assert_eq!(policy.ambient_chance_one_in, 140);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.dragon-lair")
        .expect("Dragon's Lair should exist");
    assert_eq!(dungeon.legacy_index, Some(5));
    assert_eq!(dungeon.root_floor_id, "demo.floor.dragon-lair-depth-60");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.tiamat-celestial-dragon-of-evil"
    );
    let entrance_guardian = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Ancient multi-hued dragon should guard the entrance");
    assert_eq!(
        entrance_guardian.actor_kind_id,
        "demo.actor.ancient-multi-hued-dragon"
    );
    assert_eq!(entrance_guardian.position, ContentPosition { x: 41, y: 16 });
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 74, y: 28 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.dragon-lair"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.dragon-lair"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 13);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (60..=72).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.dragon-lair-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.floor_terrain_id == "demo.terrain.dirt"
            && floor.encounter_table_id.as_deref() == Some("demo.encounter-table.dragon-lair")
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.streamers.len() == 2
                    && layout.river.as_ref().is_none_or(|river| {
                        river.deep_terrain_id == "demo.terrain.surface-lava-deep"
                            && river.shallow_terrain_id == "demo.terrain.surface-lava-shallow"
                            && river.chance_one_in == Some(7)
                    })
                    && layout.rooms.as_ref().is_some_and(|rooms| {
                        rooms.shapes.iter().any(|candidate| {
                            candidate.shape == ProceduralRoomShape::Cavern && candidate.weight == 9
                        }) && rooms.shapes.iter().any(|candidate| {
                            candidate.shape == ProceduralRoomShape::Rectangle
                                && candidate.weight == 1
                        })
                    })
            })
    }));
    let lake_depths = floors
        .iter()
        .filter(|floor| {
            floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.lake.is_some())
        })
        .map(|floor| floor.depth)
        .collect::<BTreeSet<_>>();
    assert_eq!(lake_depths, BTreeSet::from([62, 66, 68]));
    let lake_terrain = floors
        .iter()
        .filter_map(|floor| {
            floor
                .layout
                .as_ref()
                .and_then(|layout| layout.lake.as_ref())
                .map(|lake| lake.deep_terrain_id.as_str())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        lake_terrain,
        BTreeSet::from([
            "demo.terrain.rubble",
            "demo.terrain.surface-lava-deep",
            "demo.terrain.surface-tree",
        ])
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.destroyed.is_some()))
            .map(|floor| floor.depth)
            .collect::<Vec<_>>(),
        [64]
    );

    let final_floor = floors.last().expect("depth 72 should exist");
    assert!(final_floor.final_floor);
    assert!(final_floor.next_floor_id.is_none());
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("Tiamat should guard depth 72");
    assert_eq!(guardian.instance_id, "demo.guardian.dragon-lair.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.tiamat-celestial-dragon-of-evil"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.dragon-lair-final-reward")
    );
    assert!(guardian.reward_artifact_item_kind_id.is_none());
}

#[test]
fn p98b_castle_binds_rooms_ecology_guardians_and_representative_layers() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.castle")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Castle should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["H", "g", "h", "p"]);
    assert_eq!(policy.preferred_tags, ["demon"]);
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.castle")
        .expect("Castle should exist");
    assert_eq!(dungeon.legacy_index, Some(12));
    assert_eq!(dungeon.root_floor_id, "demo.floor.castle-depth-40");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.layzark-the-emperor"
    );
    let entrance_guardian = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Anti-paladin should guard the entrance");
    assert_eq!(entrance_guardian.actor_kind_id, "demo.actor.anti-paladin");
    assert_eq!(entrance_guardian.position, ContentPosition { x: 40, y: 16 });
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 88, y: 34 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.castle"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.castle"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 26);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (40..=65).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.castle-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (66, 22)
            && floor.floor_terrain_id == "demo.terrain.floor"
            && floor.encounter_table_id.as_deref() == Some("demo.encounter-table.castle")
            && floor.layout.as_ref().is_some_and(|layout| {
                layout.cavern.is_none()
                    && layout.lake.is_none()
                    && layout.river.is_none()
                    && layout.destroyed.is_none()
                    && layout.streamers.is_empty()
                    && layout.rooms.as_ref().is_some_and(|rooms| {
                        rooms.shapes.len() == 1
                            && rooms.shapes[0].shape == ProceduralRoomShape::Rectangle
                    })
            })
    }));

    let arena = floors
        .iter()
        .find(|floor| floor.depth == 45)
        .expect("depth 45 should represent ARENA");
    let arena_budget = arena
        .generation_budget
        .as_ref()
        .expect("arena should retain a room budget");
    assert_eq!(arena_budget.room_placements, Some(2));
    assert_eq!(arena_budget.room_area_tiles, Some(1200));
    let arena_rooms = arena
        .layout
        .as_ref()
        .and_then(|layout| layout.rooms.as_ref())
        .expect("arena should retain large rectangular rooms");
    assert_eq!(arena_rooms.placement, ProceduralRoomPlacement::Partitioned);
    assert_eq!((arena_rooms.min_width, arena_rooms.min_height), (28, 17));

    let curtain = floors
        .iter()
        .find(|floor| floor.depth == 50)
        .expect("depth 50 should represent CURTAIN");
    assert_eq!(
        curtain.closed_door_terrain_id,
        "demo.terrain.curtain-closed"
    );
    let glass = floors
        .iter()
        .find(|floor| floor.depth == 55)
        .expect("depth 55 should represent GLASS_ROOM");
    assert_eq!(glass.wall_terrain_id, "demo.terrain.glass-wall");

    let closed_curtain = content
        .terrain
        .iter()
        .find(|terrain| terrain.id == "demo.terrain.curtain-closed")
        .expect("closed curtain should exist");
    assert!(!closed_curtain.walkable);
    assert!(closed_curtain.blocks_sight);
    assert_eq!(
        closed_curtain.open_to_terrain_id.as_deref(),
        Some("demo.terrain.curtain-open")
    );
    let open_curtain = content
        .terrain
        .iter()
        .find(|terrain| terrain.id == "demo.terrain.curtain-open")
        .expect("open curtain should exist");
    assert!(open_curtain.walkable);
    assert!(!open_curtain.blocks_sight);
    assert_eq!(
        open_curtain.close_to_terrain_id.as_deref(),
        Some("demo.terrain.curtain-closed")
    );
    let glass_wall = content
        .terrain
        .iter()
        .find(|terrain| terrain.id == "demo.terrain.glass-wall")
        .expect("glass wall should exist");
    assert!(!glass_wall.walkable);
    assert!(!glass_wall.blocks_sight);
    assert!(glass_wall.allows_wall_passage);

    let final_floor = floors.last().expect("depth 65 should exist");
    assert!(final_floor.final_floor);
    assert!(final_floor.next_floor_id.is_none());
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("Layzark should guard depth 65");
    assert_eq!(guardian.instance_id, "demo.guardian.castle.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.layzark-the-emperor");
    assert!(guardian.reward_loot_table_id.is_none());
    assert!(guardian.reward_artifact_item_kind_id.is_none());
}

#[test]
fn p99d_giants_hall_and_snow_castle_bind_substitution_layers_and_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let giants_hall = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.giants-hall")
        .expect("Giant's Hall should exist");
    assert_eq!(giants_hall.legacy_index, Some(24));
    assert_eq!(
        giants_hall
            .substitution
            .as_ref()
            .map(|substitution| substitution.alternate_dungeon_id.as_str()),
        Some("demo.dungeon.snow-castle")
    );
    let snow_castle = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.snow-castle")
        .expect("Snow castle should exist");
    assert_eq!(snow_castle.legacy_index, Some(38));
    assert!(snow_castle.substitution.is_none());
    for (dungeon_id, position) in [
        ("demo.dungeon.giants-hall", ContentPosition { x: 63, y: 44 }),
        ("demo.dungeon.snow-castle", ContentPosition { x: 65, y: 44 }),
    ] {
        assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
            wilderness.locations.iter().any(|location| {
                matches!(
                    location,
                    WildernessLocationDefinition::Dungeon { position: actual, dungeon_id: actual_id }
                        if *actual == position && actual_id == dungeon_id
                )
            })
        }));
    }

    let giant_policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.giants-hall")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Giant's Hall should use global allocation");
    assert_eq!(giant_policy.preferred_tags, ["giant"]);
    assert_eq!(giant_policy.special_div, 2);
    let snow_policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.snow-castle")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Snow castle should use global allocation");
    assert_eq!(snow_policy.preferred_tags, ["giant"]);
    assert_eq!(snow_policy.preferred_habitats, [ActorHabitat::Snow]);
    assert_eq!(
        snow_policy.preferred_damage_immunities,
        [ActorDamageType::Cold]
    );
    assert_eq!(snow_policy.special_div, 2);

    let giant_floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.giants-hall"))
        .collect::<Vec<_>>();
    assert_eq!(giant_floors.len(), 11);
    assert!(giant_floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.floor_terrain_id == "demo.terrain.surface-grass"
            && floor.layout.as_ref().is_some_and(|layout| {
                !layout.place_doors
                    && layout.river.as_ref().and_then(|river| river.chance_one_in) == Some(7)
            })
    }));

    let snow_floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.snow-castle"))
        .collect::<Vec<_>>();
    assert_eq!(snow_floors.len(), 21);
    assert!(snow_floors.iter().all(|floor| {
        (floor.width, floor.height) == (66, 22)
            && floor.floor_terrain_id == "demo.terrain.surface-snow"
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.snow-castle")
            && floor
                .layout
                .as_ref()
                .is_some_and(|layout| !layout.place_doors)
    }));
    assert_eq!(
        snow_floors
            .iter()
            .filter(|floor| floor.wall_terrain_id == "demo.terrain.ice-wall")
            .count(),
        13
    );
    assert!(snow_floors.iter().enumerate().all(|(index, floor)| {
        let expected_shafts = usize::from(index >= 2) + usize::from(index + 2 < snow_floors.len());
        floor
            .connections
            .iter()
            .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
            .count()
            == expected_shafts
    }));

    for final_floor in [giant_floors.last().unwrap(), snow_floors.last().unwrap()] {
        let guardian = final_floor
            .guardian
            .as_ref()
            .expect("Utgard-Loke should guard the end");
        assert_eq!(guardian.actor_kind_id, "demo.actor.utgard-loke");
        assert_eq!(
            guardian.reward_artifact_item_kind_id.as_deref(),
            Some("demo.item.set-of-gauntlets-paurnimmen")
        );
        assert_eq!(
            guardian.reward_loot_table_id.as_deref(),
            Some("demo.loot-table.set-of-gauntlets-final-replacement")
        );
    }
}

#[test]
fn p100d_graveyard_binds_undead_ecology_layers_shafts_and_soulsword() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.graveyard")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Graveyard should use global allocation");
    assert_eq!(policy.preferred_tags, ["nonliving", "undead"]);
    assert_eq!(policy.special_div, 4);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let feature_table = content
        .terrain_feature_tables
        .iter()
        .find(|table| table.id == "demo.terrain-feature-table.graveyard")
        .expect("Graveyard shallow-water feature table");
    assert_eq!(feature_table.rolls, 68);
    assert_eq!(feature_table.entries.len(), 1);
    assert_eq!(
        feature_table.entries[0].terrain_id,
        "demo.terrain.surface-water-shallow"
    );
    assert_eq!(
        feature_table.entries[0].placement,
        TerrainFeaturePlacement::Room
    );

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 85, y: 19 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.graveyard"
            )
        })
    }));
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.graveyard")
        .expect("Graveyard should exist");
    assert_eq!(dungeon.legacy_index, Some(6));
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.vecna-the-emperor-lich"
    );
    let entrance = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Master lich should guard the entrance");
    assert_eq!(entrance.actor_kind_id, "demo.actor.master-lich");

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.graveyard"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 21);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (50..=70).collect::<Vec<_>>()
    );
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.graveyard-entrance")
    );
    assert!(floors.iter().all(|floor| {
        floor.encounter_table_id.as_deref() == Some("demo.encounter-table.graveyard")
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.graveyard")
    }));
    assert!(floors.iter().enumerate().all(|(index, floor)| {
        let expected_shafts = usize::from(index >= 2) + usize::from(index + 2 < floors.len());
        floor
            .connections
            .iter()
            .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
            .count()
            == expected_shafts
    }));
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.destroyed.is_some()))
            .map(|floor| floor.depth)
            .collect::<Vec<_>>(),
        [54]
    );
    assert_eq!(
        floors
            .iter()
            .find(|floor| floor.depth == 58)
            .and_then(|floor| floor.layout.as_ref())
            .and_then(|layout| layout.rooms.as_ref())
            .map(|rooms| rooms.placement),
        Some(ProceduralRoomPlacement::Partitioned)
    );
    let water_lake = floors
        .iter()
        .find(|floor| floor.depth == 62)
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| layout.lake.as_ref())
        .expect("depth 62 should represent LAKE_WATER");
    assert_eq!(
        water_lake.deep_terrain_id,
        "demo.terrain.surface-water-deep"
    );
    let rubble_lake = floors
        .iter()
        .find(|floor| floor.depth == 66)
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| layout.lake.as_ref())
        .expect("depth 66 should represent LAKE_RUBBLE");
    assert_eq!(rubble_lake.deep_terrain_id, "demo.terrain.rubble");

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Vecna should guard depth 70");
    assert_eq!(guardian.actor_kind_id, "demo.actor.vecna-the-emperor-lich");
    assert_eq!(
        guardian.reward_artifact_item_kind_id.as_deref(),
        Some("demo.item.soulsword")
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.graveyard-final-replacement")
    );
}

#[test]
fn p101c_witch_wood_and_plains_of_oz_bind_substitution_ecology_coffee_and_book_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = |id: &str| {
        content
            .encounter_tables
            .iter()
            .find(|table| table.id == id)
            .and_then(|table| table.global_allocation.as_ref())
            .unwrap_or_else(|| panic!("{id} should use global allocation"))
    };
    let witch_policy = policy("demo.encounter-table.witch-wood");
    assert_eq!(witch_policy.preferred_glyphs, ["B", "C"]);
    assert_eq!(witch_policy.preferred_tags, ["animal"]);
    assert_eq!(witch_policy.preferred_habitats, [ActorHabitat::Wood]);
    assert_eq!(witch_policy.special_div, 8);
    assert_eq!(witch_policy.ambient_chance_one_in, 160);
    let oz_policy = policy("demo.encounter-table.plains-of-oz");
    assert_eq!(oz_policy.preferred_tags, ["aussie"]);
    assert_eq!(oz_policy.special_div, 1);
    assert_eq!(oz_policy.ambient_chance_one_in, 120);

    for (actor_id, legacy_index) in [
        ("demo.actor.carnivorous-flying-monkey", 145),
        ("demo.actor.white-crocodile", 1044),
        ("demo.actor.sheep", 1226),
        ("demo.actor.beer-elemental", 1228),
    ] {
        let actor = content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should exist"));
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index)
        );
        assert!(actor.tags.iter().any(|tag| tag == "aussie"));
    }

    let feature_table = |id: &str| {
        content
            .terrain_feature_tables
            .iter()
            .find(|table| table.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    };
    let witch_features = feature_table("demo.terrain-feature-table.witch-wood");
    assert_eq!(witch_features.rolls, 90);
    assert_eq!(
        witch_features
            .entries
            .iter()
            .map(|entry| (entry.terrain_id.as_str(), entry.weight))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("demo.terrain.surface-flower", 1),
            ("demo.terrain.surface-swamp", 2),
        ])
    );
    let oz_features = feature_table("demo.terrain-feature-table.plains-of-oz");
    assert_eq!(oz_features.rolls, 135);
    assert_eq!(
        oz_features
            .entries
            .iter()
            .map(|entry| (entry.terrain_id.as_str(), entry.weight))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([("demo.terrain.dirt", 2), ("demo.terrain.surface-brake", 1),])
    );

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let location = |dungeon_id: &str, expected: ContentPosition| {
        assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
            wilderness.locations.iter().any(|location| {
                matches!(
                    location,
                    WildernessLocationDefinition::Dungeon { position, dungeon_id: id }
                        if *position == expected && id == dungeon_id
                )
            })
        }));
    };
    location("demo.dungeon.witch-wood", ContentPosition { x: 63, y: 53 });
    location(
        "demo.dungeon.plains-of-oz",
        ContentPosition { x: 65, y: 54 },
    );
    let witch = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.witch-wood")
        .expect("Witch Wood should exist");
    assert_eq!(witch.legacy_index, Some(7));
    assert_eq!(
        witch.substitution,
        Some(DungeonSubstitutionDefinition {
            alternate_dungeon_id: "demo.dungeon.plains-of-oz".to_owned(),
            alternate_gate_one_in: None,
        })
    );
    let plains = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.plains-of-oz")
        .expect("Plains of Oz should exist");
    assert_eq!(plains.legacy_index, Some(35));
    assert!(plains.substitution.is_none());

    for (dungeon_id, depths, dimensions, guardian_id, entry_id, shafts) in [
        (
            "demo.dungeon.witch-wood",
            (25..=40).collect::<Vec<_>>(),
            (96, 33),
            "demo.actor.gertrude",
            "demo.terrain.witch-wood-entrance",
            false,
        ),
        (
            "demo.dungeon.plains-of-oz",
            (18..=36).collect::<Vec<_>>(),
            (66, 22),
            "demo.actor.the-wicked-witch-of-the-south-east",
            "demo.terrain.plains-of-oz-entrance",
            true,
        ),
    ] {
        let mut floors = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        floors.sort_by_key(|floor| floor.depth);
        assert_eq!(
            floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
            depths
        );
        assert_eq!(floors[0].entry_terrain_id.as_deref(), Some(entry_id));
        assert!(floors.iter().all(|floor| {
            (floor.width, floor.height) == dimensions
                && floor
                    .layout
                    .as_ref()
                    .is_some_and(|layout| !layout.place_doors)
        }));
        assert!(floors.iter().enumerate().all(|(index, floor)| {
            let shaft_count = floor
                .connections
                .iter()
                .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
                .count();
            shaft_count
                == if shafts {
                    usize::from(index >= 2) + usize::from(index + 2 < floors.len())
                } else {
                    0
                }
        }));
        let guardian = floors
            .last()
            .and_then(|floor| floor.guardian.as_ref())
            .expect("final guardian");
        assert_eq!(guardian.actor_kind_id, guardian_id);
        assert_eq!(guardian.reward_first_realm_book_rank, Some(3));
        assert_eq!(
            guardian.reward_loot_table_id.as_deref(),
            Some("demo.loot-table.first-realm-third-book-fallback")
        );
    }
}

#[test]
fn p94b_mine_binds_mixed_rivers_guardians_rich_veins_and_healing_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.mine")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Mine should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["$"]);
    assert_eq!(policy.special_div, 1);
    assert_eq!(policy.ambient_chance_one_in, 80);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.mine")
        .expect("Mine should exist");
    assert_eq!(dungeon.legacy_index, Some(15));
    assert_eq!(dungeon.root_floor_id, "demo.floor.mine-depth-75");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.polyphemus-the-blind-cyclops"
    );
    let entrance_guardian = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Elder storm giant should guard the entrance");
    assert_eq!(
        entrance_guardian.actor_kind_id,
        "demo.actor.elder-storm-giant"
    );
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 49, y: 23 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.mine"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.mine"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 6);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (75..=80).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.mine-entrance")
    );
    for floor in &floors {
        assert_eq!((floor.width, floor.height), (66, 22));
        assert_eq!(floor.wall_terrain_id, "demo.terrain.wall");
        assert_eq!(floor.floor_terrain_id, "demo.terrain.floor");
        let budget = floor
            .generation_budget
            .as_ref()
            .expect("Mine generation budget");
        assert_eq!(budget.streamer_placements, Some(4));
        assert_eq!(budget.streamer_area_tiles, Some(320));
        let layout = floor.layout.as_ref().expect("Mine layout");
        assert_eq!(layout.streamers.len(), 2);
        assert!(layout.streamers.iter().all(|streamer| {
            streamer
                .treasure
                .as_ref()
                .is_some_and(|treasure| treasure.known_one_in == 2 && treasure.hidden_one_in == 2)
        }));
        let river = layout.river.as_ref().expect("Mine river policy");
        assert_eq!(river.chance_one_in, Some(7));
        assert_eq!(river.deep_terrain_id, "demo.terrain.surface-water-deep");
        assert_eq!(
            river.shallow_terrain_id,
            "demo.terrain.surface-water-shallow"
        );
        let alternative = river
            .alternative
            .as_ref()
            .expect("Mine should choose lava as its alternate river");
        assert_eq!(
            alternative.deep_terrain_id,
            "demo.terrain.surface-lava-deep"
        );
        assert_eq!(
            alternative.shallow_terrain_id,
            "demo.terrain.surface-lava-shallow"
        );
        assert_eq!(alternative.chance_numerator, floor.depth + 1);
        assert_eq!(alternative.chance_denominator, 256);
    }
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor
                .layout
                .as_ref()
                .is_some_and(|layout| layout.destroyed.is_some()))
            .count(),
        1
    );

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Polyphemus should guard depth 80");
    assert_eq!(guardian.instance_id, "demo.guardian.mine.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.polyphemus-the-blind-cyclops"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.mine-final-reward")
    );
    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.mine-final-reward")
        .expect("Mine reward table should exist");
    assert_eq!(
        reward.entries[0].item_kind_id,
        "demo.item.star-healing-potion"
    );

    let mut invalid = artifact.content.clone();
    let alternative = invalid
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .and_then(|world| {
            world
                .procedural_floors
                .iter_mut()
                .find(|floor| floor.id == "demo.floor.mine-depth-75")
        })
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.river.as_mut())
        .and_then(|river| river.alternative.as_mut())
        .expect("Mine alternative river should remain available");
    alternative.chance_numerator = alternative.chance_denominator;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProceduralFloor(id)) if id == "demo.floor.mine-depth-75"
    ));
}

#[test]
fn p95b_battlefield_binds_alignment_shafts_guardians_and_rune_sword() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.battlefield")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Battlefield should use global allocation");
    assert!(policy.preferred_glyphs.is_empty());
    assert_eq!(policy.preferred_tags, ["evil", "good"]);
    assert_eq!(policy.special_div, 0);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.battlefield")
        .expect("Battlefield should exist");
    assert_eq!(dungeon.legacy_index, Some(32));
    assert_eq!(dungeon.root_floor_id, "demo.floor.battlefield-depth-30");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.khamul-the-easterling"
    );
    let entrance_guardian = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Black wraith should guard the entrance");
    assert_eq!(entrance_guardian.actor_kind_id, "demo.actor.black-wraith");
    assert_eq!(entrance_guardian.position, ContentPosition { x: 45, y: 16 });
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 75, y: 57 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.battlefield"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.battlefield"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 21);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (30..=50).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.battlefield-entrance")
    );
    assert_eq!(
        floors[0].entry_connection_id.as_deref(),
        Some("demo.connection.battlefield-depth-30-stairs-up")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.floor_terrain_id == "demo.terrain.floor"
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.battlefield")
    }));
    assert_eq!(
        floors
            .iter()
            .flat_map(|floor| &floor.connections)
            .filter(|connection| connection.kind == FloorConnectionKind::Shaft)
            .count(),
        38
    );
    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Khamul should guard depth 50");
    assert_eq!(guardian.instance_id, "demo.guardian.battlefield.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.khamul-the-easterling");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.battlefield-final-reward")
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.battlefield-final-reward")
        .expect("Battlefield reward table should exist");
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.rune-sword");
    let rune_sword = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.rune-sword")
        .expect("Rune Sword should exist");
    assert_eq!(rune_sword.generation_level, 70);
    assert_eq!(rune_sword.weight_tenths_pound, 150);
    assert_eq!(rune_sword.base_value, 50_000);
    assert_eq!(
        rune_sword.initial_curse,
        Some(ItemCurseSeverityDefinition::Permanent)
    );
    assert!(rune_sword.resists_enchantment);
    assert!(rune_sword.passives.contains(&EquipmentPassive::Vampiric));
    let melee = rune_sword
        .melee_profile
        .as_ref()
        .expect("Rune Sword melee profile");
    assert_eq!((melee.damage_dice, melee.damage_sides), (0, 0));
    assert_eq!((melee.to_hit, melee.to_damage), (-10, -10));
    assert_eq!(
        rune_sword.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
}

#[test]
fn p89d_hideout_binds_depths_ecology_guardian_and_am_quest_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.hideout")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Hideout should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["p"]);
    assert_eq!(policy.preferred_tags, ["thief"]);
    assert_eq!(policy.special_div, 32);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.hideout")
        .expect("Hideout content should exist");
    assert_eq!(dungeon.legacy_index, Some(31));
    assert_eq!(dungeon.root_floor_id, "demo.floor.hideout-depth-8");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.meng-huo-the-king-of-southerings"
    );
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 28, y: 52 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.hideout"
            )
        })
    }));

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.hideout"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 11);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (8..=18).collect::<Vec<_>>()
    );
    assert_eq!((floors[0].width, floors[0].height), (66, 22));
    assert!(
        floors[1..]
            .iter()
            .all(|floor| (floor.width, floor.height) == (96, 33))
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.hideout-entrance")
    );
    for floor in &floors {
        assert_eq!(floor.floor_terrain_id, "demo.terrain.floor");
        assert_eq!(floor.closed_door_terrain_id, "demo.terrain.door-secret");
        let layout = floor.layout.as_ref().expect("Hideout layout");
        assert_eq!(
            layout
                .rooms
                .as_ref()
                .expect("Hideout room geometry")
                .shapes
                .iter()
                .map(|shape| (shape.shape, shape.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([(ProceduralRoomShape::Rectangle, 1)])
        );
        assert_eq!(
            layout
                .streamers
                .iter()
                .map(|streamer| (streamer.terrain_id.as_str(), streamer.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("demo.terrain.magma-vein", 1),
                ("demo.terrain.quartz-vein", 1),
            ])
        );
    }

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Meng Huo should guard depth 18");
    assert_eq!(guardian.instance_id, "demo.guardian.hideout.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.meng-huo-the-king-of-southerings"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.hideout-final-reward")
    );
    assert_eq!(
        floors
            .iter()
            .filter(|floor| floor.guardian.is_some())
            .count(),
        1
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.hideout-final-reward")
        .expect("Hideout should have a final reward table");
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.amulet");
    assert_eq!(reward.quality_weights[0].quality, ItemQuality::Fine);
    assert_eq!(
        reward.affix_weights[0].affix_id.as_deref(),
        Some("rfb-legacy.affix.amulet-am-quest")
    );
    let affix = content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.amulet-am-quest")
        .expect("AM_QUEST amulet mapping should exist");
    let amulet = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.amulet")
        .expect("base amulet should exist");
    assert!(affix_is_compatible_with_item(affix, amulet, 18));
    assert_eq!(affix.roll_groups.len(), 1);
    assert_eq!(affix.roll_groups[0].rolls, 3);
    assert!(
        affix.roll_groups[0]
            .candidates
            .iter()
            .all(|candidate| candidate.properties != AffixPropertyBundleDefinition::default())
    );

    let hideout_exclusives = content
        .actors
        .iter()
        .filter(|actor| {
            actor
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.legacy_dungeon_indices == [31])
        })
        .map(|actor| actor.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        hideout_exclusives,
        BTreeSet::from([
            "demo.actor.dailai-dongzhu-captain-of-southerings",
            "demo.actor.king-duosi-the-chief-of-southerings",
            "demo.actor.king-mulu-the-chief-of-southerings",
            "demo.actor.lady-zhurong-the-avatar-of-flame-spirit",
            "demo.actor.meng-huo-the-king-of-southerings",
            "demo.actor.meng-you-the-brother-of-meng-huo",
            "demo.actor.wutugu-the-chief-of-southerings",
        ])
    );
}

#[test]
fn p89e_man_cave_binds_substitution_depths_guardian_and_lotharang() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.man-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Man cave should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["p"]);
    assert_eq!(policy.preferred_tags, ["thief"]);
    assert_eq!(policy.special_div, 16);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let hideout = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.hideout")
        .expect("Hideout should exist");
    assert_eq!(
        hideout.substitution,
        Some(DungeonSubstitutionDefinition {
            alternate_dungeon_id: "demo.dungeon.man-cave".to_owned(),
            alternate_gate_one_in: Some(32),
        })
    );
    let man_cave = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.man-cave")
        .expect("Man cave should exist");
    assert_eq!(man_cave.legacy_index, Some(40));
    assert_eq!(man_cave.root_floor_id, "demo.floor.man-cave-depth-8");
    assert_eq!(
        man_cave.guardian_actor_kind_id,
        "demo.actor.untamo-the-cruel"
    );

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.man-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 11);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (8..=18).collect::<Vec<_>>()
    );
    assert_eq!((floors[0].width, floors[0].height), (66, 22));
    assert!(
        floors[1..]
            .iter()
            .all(|floor| (floor.width, floor.height) == (96, 33))
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert!(floors.iter().all(|floor| {
        floor
            .layout
            .as_ref()
            .and_then(|layout| layout.rooms.as_ref())
            .is_some_and(|rooms| {
                rooms.shapes
                    == [ProceduralRoomShapeCandidateDefinition {
                        shape: ProceduralRoomShape::Rectangle,
                        weight: 1,
                    }]
            })
    }));
    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Untamo should guard Man cave depth 18");
    assert_eq!(guardian.instance_id, "demo.guardian.man-cave.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.untamo-the-cruel");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.man-cave-final-replacement")
    );
    assert_eq!(
        guardian.reward_artifact_item_kind_id.as_deref(),
        Some("demo.item.lotharang")
    );

    let lotharang = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.lotharang")
        .expect("Lotharang should exist");
    assert_eq!(lotharang.generation_level, 10);
    assert_eq!(lotharang.weight_tenths_pound, 170);
    assert_eq!(lotharang.base_value, 21_000);
    assert_eq!(lotharang.modifiers.strength, 1);
    assert_eq!(lotharang.modifiers.dexterity, 1);
    assert_eq!(
        lotharang.artifact_generation,
        Some(ArtifactGenerationDefinition {
            source_index: 104,
            base_item_kind_id: "demo.item.battle-axe".to_owned(),
            rarity_one_in: 5,
            instant: false,
            affix_ids: Vec::new(),
        })
    );
    let melee = lotharang.melee_profile.as_ref().expect("Lotharang melee");
    assert_eq!((melee.damage_dice, melee.damage_sides), (2, 9));
    assert_eq!((melee.to_hit, melee.to_damage), (9, 8));
    assert_eq!(
        lotharang.slays.get(&SlayTarget::Orc),
        Some(&SlayLevel::Slay)
    );
    assert_eq!(
        lotharang.slays.get(&SlayTarget::Troll),
        Some(&SlayLevel::Slay)
    );
    let generation = lotharang
        .device_generation
        .as_ref()
        .expect("Lotharang should retain its activation");
    assert_eq!(
        generation.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 40,
            energy_per_mille: 1_000,
        })
    );
    assert!(matches!(
        generation.activations.as_slice(),
        [ItemDeviceActivationDefinition {
            device_check_difficulty: 10,
            charges: ItemDeviceChargeRangeDefinition {
                minimum: 1,
                maximum: 1,
                cost: 1,
            },
            effect: ItemUseEffectDefinition::Heal { amount: 30 },
            ..
        }]
    ));
}

#[test]
fn p88c_icky_cave_binds_ecology_terrain_mix_depths_and_guardian() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.icky-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Icky Cave should use global allocation");
    assert_eq!(
        policy
            .preferred_glyphs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["M", "i", "j"].into_iter().collect()
    );
    assert!(policy.preferred_tags.is_empty());
    assert!(policy.preferred_movement_modes.is_empty());
    assert!(policy.preferred_habitats.is_empty());
    assert_eq!(policy.special_div, 32);

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.icky-cave")
        .expect("Icky Cave content should exist");
    assert_eq!(dungeon.legacy_index, Some(21));
    assert_eq!(dungeon.root_floor_id, "demo.floor.icky-cave-depth-10");
    assert_eq!(dungeon.guardian_actor_kind_id, "demo.actor.the-icky-queen");
    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.icky-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 11);
    assert_eq!(
        (floors[0].depth, floors[0].width, floors[0].height),
        (10, 66, 22)
    );
    assert!(
        floors[1..]
            .iter()
            .all(|floor| (floor.width, floor.height) == (96, 33))
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.icky-cave-entrance")
    );

    for floor in &floors {
        assert_eq!(floor.floor_terrain_id, "demo.terrain.surface-grass");
        assert_eq!(
            floor.encounter_table_id.as_deref(),
            Some("demo.encounter-table.icky-cave")
        );
        assert_eq!(
            floor.terrain_feature_table_id.as_deref(),
            Some("demo.terrain-feature-table.icky-cave")
        );
        let budget = floor.generation_budget.as_ref().expect("generation budget");
        assert_eq!(budget.room_area_tiles, Some(800));
        assert_eq!(
            budget.feature_placements,
            Some(if floor.depth == 10 { 186 } else { 320 })
        );
        let layout = floor.layout.as_ref().expect("Icky Cave layout");
        assert_eq!(
            layout
                .rooms
                .as_ref()
                .expect("room geometry")
                .shapes
                .iter()
                .map(|shape| (shape.shape, shape.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                (ProceduralRoomShape::Rectangle, 1),
                (ProceduralRoomShape::Cavern, 9),
            ])
        );
        assert_eq!(
            layout
                .streamers
                .iter()
                .map(|streamer| (streamer.terrain_id.as_str(), streamer.weight))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("demo.terrain.magma-vein", 1),
                ("demo.terrain.quartz-vein", 1),
            ])
        );
    }

    let final_floor = floors.last().expect("depth 20 should exist");
    assert!(final_floor.final_floor);
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("The Icky Queen guardian");
    assert_eq!(guardian.actor_kind_id, "demo.actor.the-icky-queen");

    let feature_table = content
        .terrain_feature_tables
        .iter()
        .find(|table| table.id == "demo.terrain-feature-table.icky-cave")
        .expect("Icky Cave terrain feature table");
    assert_eq!(feature_table.rolls, 320);
    assert_eq!(
        feature_table
            .entries
            .iter()
            .map(|entry| (entry.terrain_id.as_str(), entry.weight, entry.placement))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            (
                "demo.terrain.surface-swamp",
                1,
                TerrainFeaturePlacement::Room,
            ),
            (
                "demo.terrain.surface-water-shallow",
                1,
                TerrainFeaturePlacement::Room,
            ),
        ])
    );
}

#[test]
fn p88d_icky_cave_binds_wilderness_entrance_and_protection_quiver_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");

    assert!(
        world
            .wilderness
            .as_ref()
            .expect("Middle-earth should retain wilderness")
            .locations
            .iter()
            .any(|location| matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 17, y: 29 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.icky-cave"
            ))
    );

    let final_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.icky-cave-depth-20")
        .expect("Icky Cave depth 20 should exist");
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("Icky Cave depth 20 should contain The Icky Queen");
    assert_eq!(guardian.instance_id, "demo.guardian.icky-cave.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.the-icky-queen");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.icky-cave-final-reward")
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.icky-cave-final-reward")
        .expect("The Icky Queen should have a fixed reward table");
    assert_eq!(reward.rolls, 1);
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.quiver");
    assert_eq!(reward.entries[0].quantity, 1);
    assert_eq!(reward.quality_weights.len(), 1);
    assert_eq!(reward.quality_weights[0].quality, ItemQuality::Ordinary);
    assert_eq!(reward.affix_weights.len(), 1);
    assert_eq!(
        reward.affix_weights[0].affix_id.as_deref(),
        Some("rfb-legacy.affix.quiver-protection")
    );
}

#[test]
fn p87d_tidal_cave_binds_wilderness_entrance_and_fixed_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");

    assert!(
        world
            .wilderness
            .as_ref()
            .expect("Middle-earth should retain wilderness")
            .locations
            .iter()
            .any(|location| matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 47, y: 53 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.tidal-cave"
            ))
    );

    let final_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.tidal-cave-depth-27")
        .expect("Tidal Cave depth 27 should exist");
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("Tidal Cave depth 27 should contain Grendel");
    assert_eq!(guardian.instance_id, "demo.guardian.tidal-cave.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.grendel");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.tidal-cave-final-reward")
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.tidal-cave-final-reward")
        .expect("Grendel should have a fixed reward table");
    assert_eq!(reward.rolls, 1);
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(
        reward.entries[0].item_kind_id,
        "demo.item.giant-strength-potion"
    );
    assert_eq!(reward.entries[0].quantity, 1);
    assert_eq!(reward.quality_weights.len(), 1);
    assert_eq!(reward.quality_weights[0].quality, ItemQuality::Ordinary);
    assert_eq!(reward.affix_weights.len(), 1);
    assert_eq!(reward.affix_weights[0].affix_id, None);
}

#[test]
fn p86c_camelot_binds_depths_ecology_layout_and_mirror_shield_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.camelot")
        .expect("Camelot should be active");
    assert_eq!(dungeon.legacy_index, Some(2));
    assert_eq!(dungeon.root_floor_id, "demo.floor.camelot-depth-20");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.arthur-pendragon"
    );
    assert!(
        world
            .wilderness
            .as_ref()
            .expect("Middle-earth should retain wilderness")
            .locations
            .iter()
            .any(|location| matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 7, y: 59 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.camelot"
            ))
    );

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.camelot"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 16);
    assert_eq!(floors.first().map(|floor| floor.depth), Some(20));
    assert_eq!(floors.last().map(|floor| floor.depth), Some(35));
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.camelot-entrance")
    );
    for floor in &floors {
        assert_eq!((floor.width, floor.height), (96, 33));
        assert_eq!(floor.wall_terrain_id, "demo.terrain.wall");
        assert_eq!(floor.closed_door_terrain_id, "demo.terrain.door-secret");
        assert_eq!(floor.trap_terrain_id, "demo.terrain.warren-snare");
        let layout = floor.layout.as_ref().expect("Camelot should use a layout");
        let rooms = layout
            .rooms
            .as_ref()
            .expect("Camelot should generate rooms");
        assert_eq!(rooms.shapes.len(), 1);
        assert_eq!(rooms.shapes[0].shape, ProceduralRoomShape::Rectangle);
        assert_eq!(
            layout
                .streamers
                .iter()
                .map(|streamer| streamer.terrain_id.as_str())
                .collect::<BTreeSet<_>>(),
            ["demo.terrain.magma-vein", "demo.terrain.quartz-vein"]
                .into_iter()
                .collect()
        );
    }

    let final_floor = floors.last().expect("Camelot should have a final floor");
    assert!(final_floor.final_floor);
    let guardian = final_floor
        .guardian
        .as_ref()
        .expect("depth 35 should contain Arthur");
    assert_eq!(guardian.instance_id, "demo.guardian.camelot.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.arthur-pendragon");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.camelot-final-reward")
    );

    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.camelot")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Camelot should use global allocation");
    assert_eq!(policy.special_div, 32);
    assert_eq!(policy.ambient_chance_one_in, 160);
    assert_eq!(
        policy
            .preferred_tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["knight"].into_iter().collect()
    );
    assert_eq!(
        policy
            .preferred_glyphs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["p", "H", "g", "d"].into_iter().collect()
    );

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.camelot-final-reward")
        .expect("Arthur should have a fixed reward table");
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.mirror-shield");
    assert_eq!(reward.affix_weights[0].affix_id, None);
}

#[test]
fn p40_chameleon_retains_the_authoritative_form_change_marker() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let chameleon = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.chameleon")
        .expect("Chameleon should be imported");

    assert_eq!(chameleon.level, 20);
    assert_eq!(chameleon.hit_point_dice.unwrap().dice, 10);
    assert_eq!(chameleon.hit_point_dice.unwrap().sides, 100);
    assert!(chameleon.tags.iter().any(|tag| tag == "chameleon"));
    assert!(chameleon.movement.modes.contains(&ActorMovementMode::Fly));
    assert!(
        chameleon
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.is_empty())
    );
    for damage_type in [
        ActorDamageType::Acid,
        ActorDamageType::Cold,
        ActorDamageType::Electricity,
        ActorDamageType::Fire,
        ActorDamageType::Poison,
    ] {
        assert_eq!(
            chameleon.resistances.get(&damage_type),
            Some(&ActorResistanceLevel::Resistant)
        );
    }
}

#[test]
fn p41_ghast_retains_the_authoritative_eldritch_horror_marker() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ghast = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.ghast")
        .expect("Ghast should be imported");

    assert_eq!(ghast.level, 19);
    assert_eq!(ghast.hit_point_dice.unwrap().dice, 12);
    assert_eq!(ghast.hit_point_dice.unwrap().sides, 10);
    assert!(ghast.tags.iter().any(|tag| tag == "eldritch-horror"));
    assert!(ghast.tags.iter().any(|tag| tag == "undead"));
    assert!(ghast.movement.modes.contains(&ActorMovementMode::Swim));
    assert_eq!(
        ghast.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Vulnerable)
    );
    assert_eq!(
        ghast
            .melee_routine
            .as_ref()
            .expect("Ghast should retain its melee routine")
            .blows
            .len(),
        3
    );
}

#[test]
fn p42a_direct_monsters_keep_source_identity_without_new_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("half-troll", 491, 33),
        ("cave-troll", 496, 33),
        ("giant-skeleton-troll", 500, 33),
        ("water-troll", 509, 33),
        ("triceratops", 1217, 33),
        ("night-stalker", 514, 34),
        ("velociraptor", 515, 34),
        ("bloodletter-of-khorne", 523, 34),
        ("giant-grey-scorpion", 524, 34),
        ("pegasus", 990, 34),
        ("ulfang-the-black", 1046, 34),
        ("diplodocus", 1218, 34),
        ("sheer-heart-attack-the-bomb-hand", 531, 35),
        ("dagashi", 532, 35),
        ("leng-spider", 535, 35),
        ("star-vampire", 536, 35),
        ("acidic-cytoplasm", 541, 35),
        ("wooly-rhinoceros", 547, 35),
        ("giant-fire-ant", 548, 35),
        ("the-minotaur-of-the-labyrinth", 1034, 35),
        ("ankylosaur", 1219, 35),
        ("xorn", 550, 36),
        ("rogrog-the-black-troll", 551, 36),
        ("mist-giant", 552, 36),
        ("trapper", 565, 36),
        ("chaos-spawn", 574, 36),
        ("exploding-ant", 1109, 36),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
        assert!(
            actor.monster_casting.is_none(),
            "{actor_id} should not gain monster casting"
        );
    }
}

#[test]
fn p42b_direct_monsters_complete_the_ability_free_harvest() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("black-pudding", 585, 37),
        ("killer-iridescent-beetle", 586, 37),
        ("night-mare", 622, 39),
        ("spawn-of-ubbo-sathla", 626, 40),
        ("morgenstern-julian-s-steed", 629, 40),
        ("spirit-troll", 630, 40),
        ("war-troll", 631, 40),
        ("disenchanter-worm-mass", 632, 40),
        ("xaren", 639, 40),
        ("jubjub-bird", 640, 40),
        ("minotaur", 641, 40),
        ("mumak", 673, 40),
        ("ashram-the-ebony-knight", 974, 40),
        ("unicorn", 985, 39),
        ("kokuo-raou-s-steed", 1019, 40),
        ("narwhal", 1042, 40),
        ("pteranodon", 1220, 40),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
        assert!(
            actor.monster_casting.is_none(),
            "{actor_id} should not gain monster casting"
        );
    }
}

#[test]
fn p43_monsters_reuse_the_existing_ability_catalog() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ability_ids = artifact
        .content
        .abilities
        .iter()
        .map(|ability| ability.id.as_str())
        .collect::<BTreeSet<_>>();

    for (id, legacy_index, level) in [
        ("shadow-drake", 471, 33),
        ("barrow-wight", 499, 33),
        ("chaos-drake", 501, 33),
        ("law-drake", 502, 33),
        ("balance-drake", 503, 33),
        ("ethereal-drake", 504, 33),
        ("logrus-ghost", 507, 33),
        ("multi-hued-hound", 513, 33),
        ("lich", 518, 34),
        ("oriental-vampire", 521, 34),
        ("doom-drake", 527, 34),
        ("gargoyle", 528, 34),
        ("malicious-leprechaun", 529, 35),
        ("shard-vortex", 897, 35),
        ("topaz-monk", 1047, 35),
        ("beer-elemental", 1228, 35),
        ("booze-hound", 1341, 35),
        ("pattern-ghost", 553, 36),
        ("young-gold-dragon", 559, 36),
        ("mature-bronze-dragon", 562, 36),
        ("mezzodaemon", 568, 36),
        ("ebony-monk", 870, 36),
        ("botei-building-the-emperor", 963, 36),
        ("demonologist", 1008, 36),
        ("chaos-butterfly", 578, 37),
        ("time-ghost", 579, 37),
        ("will-o-the-wisp", 582, 37),
        ("shan", 583, 37),
        ("nexus-vortex", 587, 37),
        ("mature-gold-dragon", 590, 37),
        ("crystal-drake", 591, 37),
        ("sky-whale", 594, 38),
        ("time-vortex", 599, 38),
        ("emperor-wight", 604, 38),
        ("scylla", 610, 39),
        ("7-headed-hydra", 614, 39),
        ("clubber-demon", 648, 40),
        ("eol-the-dark-elven-smith", 976, 40),
        ("vrock", 1158, 40),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
        let casting = actor
            .monster_casting
            .as_ref()
            .unwrap_or_else(|| panic!("{actor_id} should retain monster casting"));
        assert!(!casting.abilities.is_empty(), "{actor_id} ability set");
        assert!(
            casting
                .abilities
                .iter()
                .all(|candidate| ability_ids.contains(candidate.ability_id.as_str())),
            "{actor_id} should only reference existing abilities"
        );
    }
}

#[test]
fn p44a_monsters_generate_only_parameterized_existing_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ability_ids = artifact
        .content
        .abilities
        .iter()
        .map(|ability| ability.id.as_str())
        .collect::<BTreeSet<_>>();

    for (id, legacy_index, level) in [
        ("bokrug", 489, 33),
        ("biclops", 490, 33),
        ("ivory-monk", 492, 33),
        ("logrus-master", 498, 33),
        ("fasolt-the-giant", 506, 33),
        ("fire-elemental", 510, 33),
        ("cherub", 511, 33),
        ("water-elemental", 512, 33),
        ("mystic", 915, 33),
        ("that-black-bat", 975, 33),
        ("rusalka", 1143, 33),
        ("nidaros-the-clairvoyant", 1240, 33),
        ("earth-elemental", 525, 34),
        ("king-koopa", 928, 34),
        ("storm-giant", 487, 35),
        ("headless-ghost", 533, 35),
        ("dread-knight", 534, 35),
        ("smoke-elemental", 537, 35),
        ("halfling-slinger", 539, 35),
        ("gravity-hound", 540, 35),
        ("inertia-hound", 542, 35),
        ("impact-hound", 543, 35),
        ("ooze-elemental", 545, 35),
        ("mature-white-dragon", 549, 35),
        ("bazooker", 896, 35),
        ("pip-the-braver-from-another-world", 1004, 35),
        ("young-multi-hued-dragon", 556, 36),
        ("mature-blue-dragon", 560, 36),
        ("mature-green-dragon", 561, 36),
        ("young-red-dragon", 563, 36),
        ("bodak", 566, 36),
        ("elder-thing", 569, 36),
        ("ice-elemental", 570, 36),
        ("the-greater-hell-magic-mushroom-were-quylthulg", 572, 36),
        ("lord-borel-of-hendrake", 573, 36),
        ("sky-golem", 895, 36),
        ("young-silver-dragon", 1208, 36),
        ("implorington-iii", 1231, 36),
        ("lorgan-chief-of-the-easterlings", 1232, 36),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
        assert!(
            actor.monster_casting.as_ref().is_some_and(|casting| {
                !casting.abilities.is_empty()
                    && casting
                        .abilities
                        .iter()
                        .all(|candidate| ability_ids.contains(candidate.ability_id.as_str()))
            }),
            "{actor_id} should bind generated parameter records"
        );
    }
}

#[test]
fn p44b_monsters_complete_the_parameterized_existing_effect_harvest() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ability_ids = artifact
        .content
        .abilities
        .iter()
        .map(|ability| ability.id.as_str())
        .collect::<BTreeSet<_>>();

    for (id, legacy_index, level) in [
        ("fire-angel", 576, 37),
        ("flying-polyp", 580, 37),
        ("the-queen-ant", 581, 37),
        ("magma-elemental", 584, 37),
        ("plasma-vortex", 588, 37),
        ("mature-red-dragon", 589, 37),
        ("mature-black-dragon", 592, 37),
        ("bone-dragon", 941, 37),
        ("mature-silver-dragon", 1209, 37),
        ("mature-multi-hued-dragon", 593, 38),
        ("father-dagon", 595, 38),
        ("mother-hydra", 596, 38),
        ("mandor-master-of-the-logrus", 598, 38),
        ("ancient-blue-dragon", 601, 38),
        ("ancient-bronze-dragon", 602, 38),
        ("seraph", 605, 38),
        ("loge-spirit-of-fire", 606, 38),
        ("nightgaunt", 608, 38),
        ("baron-of-hell", 609, 38),
        ("ar-pharazon-the-golden", 978, 38),
        ("gazhak-the-ogre-tyrant", 1255, 38),
        ("castamir-the-usurper", 1286, 38),
        ("fire-vampire", 613, 39),
        ("kavlax-the-many-headed", 616, 39),
        ("ancient-white-dragon", 617, 39),
        ("ancient-green-dragon", 618, 39),
        ("ettin", 621, 39),
        ("ancient-black-dragon", 624, 39),
        ("eldrak", 620, 40),
        ("rotting-quylthulg", 633, 40),
        ("9-headed-hydra", 635, 40),
        ("archpriest", 637, 40),
        ("jasra-brand-s-mistress", 642, 40),
        ("ancient-gold-dragon", 645, 40),
        ("great-crystal-drake", 646, 40),
        ("wyrd-sister", 647, 40),
        ("death-quasit", 649, 40),
        ("dread", 667, 40),
        ("djinni", 892, 40),
        ("efreeti", 893, 40),
        ("troll-king", 894, 40),
        ("dao", 993, 40),
        ("high-elven-ranger", 1006, 40),
        ("m-bison", 1014, 40),
        ("master-mindcrafter", 1056, 40),
        ("flame-spider", 1171, 40),
        ("venom-spider", 1174, 40),
        ("the-marquis-de-la-tour", 1206, 40),
        ("ancient-silver-dragon", 1210, 40),
        ("spulga-the-troll-priestess", 1304, 40),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
        assert!(
            actor.monster_casting.as_ref().is_some_and(|casting| {
                !casting.abilities.is_empty()
                    && casting
                        .abilities
                        .iter()
                        .all(|candidate| ability_ids.contains(candidate.ability_id.as_str()))
            }),
            "{actor_id} should bind generated parameter records"
        );
    }
}

#[test]
fn p45_monsters_bind_low_risk_shared_mappings() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability_ids = artifact
        .content
        .abilities
        .iter()
        .map(|ability| ability.id.as_str())
        .collect::<BTreeSet<_>>();

    for (id, legacy_index, level) in [
        ("bert-the-stone-troll", 493, 33),
        ("bill-the-stone-troll", 494, 33),
        ("tom-the-stone-troll", 495, 33),
        ("ishikawa-goemon", 505, 33),
        ("master-thief", 516, 34),
        ("nightblade", 564, 36),
        ("ratatosk-the-world-tree-squirrel", 1357, 37),
        ("rolento", 1013, 38),
        ("devil-s-huntsman", 1147, 38),
        ("malekith-the-accursed", 628, 40),
        ("marilith", 1130, 40),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    for task_actor in ["anti-paladin", "revenant", "death-knight", "sorcerer"] {
        let task_actor = actor(task_actor);
        assert!(task_actor.tags.iter().any(|tag| tag == "outpost-quest"));
    }
    for table_id in [
        "demo.loot-table.evil-paladin",
        "demo.loot-table.rogue",
        "demo.loot-table.samurai",
    ] {
        assert!(
            artifact
                .content
                .loot_tables
                .iter()
                .any(|table| table.id == table_id),
            "{table_id} should be formal content"
        );
    }

    let bert_has_poison_damage = actor("bert-the-stone-troll")
        .melee_routine
        .as_ref()
        .into_iter()
        .flat_map(|routine| &routine.blows)
        .flat_map(|blow| &blow.effects)
        .any(|effect| {
            matches!(
                effect,
                MeleeBlowEffectDefinition::Damage {
                    damage_type: ActorDamageType::Poison,
                    ..
                }
            )
        });
    assert!(bert_has_poison_damage);
    assert!(
        actor("malekith-the-accursed")
            .contact_auras
            .iter()
            .any(|aura| aura.damage_type == ActorDamageType::Curse)
    );
    assert!(
        actor("hand-grenade")
            .tags
            .iter()
            .any(|tag| tag == "hand-grenade")
    );
    assert_eq!(actor("hand-grenade").level, 38);
    assert!(actor("hand-grenade").allocation.is_none());
    for hound in artifact
        .content
        .actors
        .iter()
        .filter(|candidate| matches!(candidate.glyph.as_str(), "C" | "Z"))
    {
        assert!(
            hound.tags.iter().any(|tag| tag == "hound"),
            "{} should match the original C/Z hound summon predicate",
            hound.id
        );
    }

    for ability_id in [
        "rfb-legacy.ability.beam-hell-fire-1d1-75",
        "rfb-legacy.ability.summon-hand-grenade-l38-1d3-1",
        "rfb-legacy.ability.summon-hound-l38-1d2-1",
    ] {
        assert!(ability_ids.contains(ability_id), "missing {ability_id}");
    }
    for id in ["rolento", "devil-s-huntsman", "malekith-the-accursed"] {
        assert!(actor(id).monster_casting.as_ref().is_some_and(|casting| {
            casting
                .abilities
                .iter()
                .all(|candidate| ability_ids.contains(candidate.ability_id.as_str()))
        }));
    }
}

#[test]
fn p46_trump_monster_keeps_its_source_identity_and_turn_tag() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let jurt = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.jurt-the-living-trump")
        .expect("Jurt should be imported");

    assert_eq!(jurt.level, 34);
    assert_eq!(
        jurt.allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(517)
    );
    assert!(jurt.tags.iter().any(|tag| tag == "trump"));
}

#[test]
fn p46_quantum_monster_keeps_its_source_identity_and_turn_tag() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let quantum = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.quantum-dot")
        .expect("Quantum dot should be imported");

    assert_eq!(quantum.level, 35);
    assert_eq!(
        quantum
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(863)
    );
    assert!(quantum.tags.iter().any(|tag| tag == "quantum"));
}

#[test]
fn p46_shatter_monsters_share_the_authoritative_melee_effect() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for (actor_id, legacy_index) in [
        ("demo.actor.colossus", 558),
        ("demo.actor.chthonian", 619),
        ("demo.actor.rock-giant", 1124),
    ] {
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .expect("SHATTER actor should be imported");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index)
        );
        assert!(actor.melee_routine.as_ref().is_some_and(|routine| {
            routine.blows.iter().any(|blow| {
                blow.effects
                    .iter()
                    .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Shatter { .. }))
            })
        }));
    }
}

#[test]
fn p46_beholder_keeps_gaze_sleep_and_amnesia() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let beholder = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.beholder")
        .expect("Beholder should be imported");
    let routine = beholder
        .melee_routine
        .as_ref()
        .expect("Beholder should retain its gaze routine");

    assert_eq!(
        beholder
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(603)
    );
    assert!(routine.blows.iter().any(|blow| {
        blow.effects
            .iter()
            .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Paralysis { .. }))
    }));
    assert!(routine.blows.iter().any(|blow| {
        blow.effects
            .iter()
            .any(|effect| matches!(effect, MeleeBlowEffectDefinition::Amnesia { .. }))
    }));
    assert!(beholder.monster_casting.as_ref().is_some_and(|casting| {
        casting
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id == "rfb-legacy.ability.gaze")
    }));
}

#[test]
fn p46_chronomage_keeps_dice_less_time_as_a_rider() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let chronomage = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.chronomage")
        .expect("Chronomage should be imported");

    assert_eq!(
        chronomage
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1092)
    );
    let routine = chronomage
        .melee_routine
        .as_ref()
        .expect("Chronomage should retain its melee routine");
    assert_eq!(routine.blows.len(), 3);
    assert!(
        routine
            .blows
            .iter()
            .all(|blow| blow.effects.iter().any(|effect| matches!(
                effect,
                MeleeBlowEffectDefinition::Time {
                    chance_percent: Some(25)
                }
            )))
    );
}

#[test]
fn p47a_level_41_42_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("strygalldwir", 651, 41),
        ("giant-headless", 653, 41),
        ("judge-fire", 654, 41),
        ("ubbo-sathla-the-unbegotten-source", 655, 41),
        ("judge-mortis", 656, 41),
        ("dark-elven-sorcerer", 657, 41),
        ("byakhee", 659, 41),
        ("formless-spawn-of-tsathoggua", 662, 41),
        ("gorlim-betrayer-of-barahir", 891, 41),
        ("hezrou", 1153, 41),
        ("hunting-horror", 663, 42),
        ("greater-basilisk", 668, 42),
        ("jack-of-shadows", 670, 42),
        ("the-lurking-horror", 1196, 42),
        ("bloodfreezer", 1388, 42),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }

    let bast_kin = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.kin-bast-goddess-of-cats")
        .expect("Bast should retain her summon-kin spell");
    assert!(matches!(
        bast_kin.effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 62,
            count_dice: 1,
            count_sides: 1,
            count_bonus: 1,
            ..
        } if category == "kin-glyph-102"
    ));
}

#[test]
fn p47b_level_43_44_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("zephyr-lord", 671, 43),
        ("juggernaut-of-khorne", 672, 43),
        ("judge-fear", 674, 43),
        ("ancient-multi-hued-dragon", 675, 43),
        ("ethereal-dragon", 676, 43),
        ("dark-young-of-shub-niggurath", 677, 43),
        ("colour-out-of-space", 678, 43),
        ("devu-chiokovo", 999, 43),
        ("glabrezu", 1155, 43),
        ("eskimo", 1262, 43),
        ("death-leprechaun", 680, 44),
        ("chaugnar-faugn-horror-from-the-hills", 681, 44),
        ("lloigor", 682, 44),
        ("quachil-uttaus-treader-of-the-dust", 684, 44),
        ("shoggoth", 685, 44),
        ("judge-death", 686, 44),
        ("ariel-queen-of-air", 687, 44),
        ("11-headed-hydra", 688, 44),
        ("scatha-the-worm", 692, 44),
        ("vore", 888, 44),
        ("kamikaze-hound", 952, 44),
        ("dwar-dog-lord-of-waw", 1009, 44),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p48a_level_45_46_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("warrior-of-the-dawn", 693, 45),
        ("zoth-ommog", 695, 45),
        ("smaug-the-golden", 697, 45),
        ("the-stormbringer", 698, 45),
        ("durahan", 982, 45),
        ("marid", 994, 45),
        ("chameleon-lord", 1041, 45),
        ("stone-dragon", 1048, 45),
        ("death-beast", 1082, 45),
        ("nalfeshnee", 1156, 45),
        ("ruby-serpent", 1211, 45),
        ("death-pumpkin", 1300, 45),
        ("dracolisk", 703, 46),
        ("yibb-tstll-the-patient-one", 706, 46),
        ("ghatanothoa", 707, 46),
        ("ent", 708, 46),
        ("grand-master-thief", 1024, 46),
        ("emerald-serpent", 1212, 46),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p48b_level_47_48_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("itangast-the-fire-drake", 710, 47),
        ("death-mold", 711, 47),
        ("fafner-the-dragon", 712, 47),
        ("fangorn", 713, 47),
        ("zhar-the-twin-obscenity", 714, 47),
        ("shuten-douji-king-ogre-of-ooe-mountain", 979, 47),
        ("charon-boatman-of-the-styx", 1025, 47),
        ("amethyst-serpent", 1213, 47),
        ("ice-weasel", 1237, 47),
        ("mummified-sorcerer", 1268, 47),
        ("drolem", 691, 48),
        ("glaurung-father-of-the-dragons", 715, 48),
        ("beld-ruler-of-marmo", 973, 48),
        ("winged-horror", 1216, 48),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p49_shared_mappings_import_six_monsters_and_update_fallen_angel() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("demo.actor.{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("archon", 661, 41),
        ("undead-beholder", 664, 42),
        ("quaker-master-of-earth", 679, 43),
        ("high-priest", 689, 44),
        ("ultra-elite-paladin", 699, 45),
        ("nexus-spider", 1172, 45),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    let casting_ids = |id: &str| {
        actor(id)
            .monster_casting
            .as_ref()
            .expect("P49 caster should retain monster casting")
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<Vec<_>>()
    };
    for id in ["archon", "fallen-angel"] {
        assert!(
            casting_ids(id).contains(&"rfb-legacy.ability.invulnerability-self"),
            "{id} should cast invulnerability"
        );
    }
    assert!(
        casting_ids("ultra-elite-paladin").contains(&"rfb-legacy.ability.beam-holy-fire-1d1-76")
    );
    assert!(casting_ids("nexus-spider").contains(&"rfb-legacy.ability.jump-nexus-l45"));

    let mind_blast = &actor("undead-beholder")
        .melee_routine
        .as_ref()
        .unwrap()
        .blows[0]
        .effects;
    assert!(matches!(
        mind_blast.first(),
        Some(MeleeBlowEffectDefinition::Damage {
            damage_dice: 2,
            damage_sides: 6,
            damage_type: ActorDamageType::Psi,
            ..
        })
    ));
    assert!(matches!(
        mind_blast.get(1),
        Some(MeleeBlowEffectDefinition::Confusion {
            damage_dice: 0,
            damage_sides: 0,
            ..
        })
    ));
    let aura = &actor("quaker-master-of-earth").contact_auras[0];
    assert_eq!(aura.damage_type, ActorDamageType::Shards);
    assert_eq!((aura.damage_dice, aura.damage_sides), (3, 3));

    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should be generated"))
    };
    assert!(matches!(
        ability("rfb-legacy.ability.invulnerability-self").effect,
        AbilityEffectDefinition::ApplyStatus {
            ref status_kind_id,
            duration_ticks: 4,
            duration_dice: 1,
            duration_sides: 4,
            incoming_damage_percent: 0,
            ..
        } if status_kind_id == "rfb.status.invulnerability"
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.beam-holy-fire-1d1-76").effect,
        AbilityEffectDefinition::BeamDamage {
            damage_type: ActorDamageType::HolyFire,
            ..
        }
    ));
    assert!(matches!(
        ability("rfb-legacy.ability.jump-nexus-l45").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_type: ActorDamageType::Nexus,
            ..
        }
    ));
}

#[test]
fn p50_clear_head_imports_utgard_loke() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.utgard-loke")
        .expect("Utgard-Loke should be imported");

    assert_eq!(actor.level, 44);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(683)
    );
    assert!(actor.tags.iter().any(|tag| tag == "clear-head"));
}

#[test]
fn p50_inertia_imports_baba_yaga() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.baba-yaga")
        .expect("Baba Yaga should be imported");

    assert_eq!(actor.level, 43);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1256)
    );
    assert!(matches!(
        actor
            .melee_routine
            .as_ref()
            .expect("Baba Yaga should retain melee")
            .blows[0]
            .effects[1],
        MeleeBlowEffectDefinition::Inertia { .. }
    ));
}

#[test]
fn p50_amberite_imports_rinaldo() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.rinaldo-son-of-brand")
        .expect("Rinaldo should be imported");

    assert_eq!(actor.level, 41);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(660)
    );
    assert!(actor.tags.iter().any(|tag| tag == "amberite"));
}

#[test]
fn p50_bomb_imports_leprechaun_fanatic() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.leprechaun-fanatic")
        .expect("Leprechaun fanatic should be imported");

    assert_eq!(actor.level, 46);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(700)
    );
    assert!(matches!(
        actor
            .melee_routine
            .as_ref()
            .expect("Leprechaun fanatic should retain melee")
            .blows[0]
            .effects[0],
        MeleeBlowEffectDefinition::Bomb {
            damage_dice: 12,
            damage_sides: 12,
            ..
        }
    ));
}

#[test]
fn p50_hand_doom_imports_shadow_fiend() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.the-shadow-fiend")
        .expect("The Shadow Fiend should be imported");

    assert_eq!(actor.level, 48);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1197)
    );
    let ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.hand-of-doom")
        .expect("Hand of Doom should be generated");
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::CurseDamage {
            damage_dice: 1,
            damage_sides: 20,
            damage_bonus: 40,
            damage_is_current_hp_percent: true,
            nonlethal: true,
        }
    ));
}

#[test]
fn p50_percent_mana_drain_imports_draugr() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.draugr")
        .expect("Draugr should be imported");

    assert_eq!(actor.level, 48);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1356)
    );
    assert!(matches!(
        actor
            .melee_routine
            .as_ref()
            .expect("Draugr should retain melee")
            .blows[0]
            .effects[2],
        MeleeBlowEffectDefinition::DrainResource {
            chance_percent: Some(25),
            amount_dice: 1,
            amount_sides: 25,
        }
    ));
}

#[test]
fn p51a_level_49_50_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("charybdis", 669, 49),
        ("garm-guardian-of-hel", 717, 49),
        ("lesser-balrog", 940, 49),
        ("nine-tailed-fox", 987, 49),
        ("moe-the-cyclops", 1236, 49),
        ("atomic-elemental", 1336, 49),
        ("greater-wall-monster", 718, 50),
        ("nycadaemon", 719, 50),
        ("goat-of-mendes", 721, 50),
        ("nightwing", 722, 50),
        ("maulotaur", 723, 50),
        ("master-mystic", 916, 50),
        ("golden-angel", 1010, 50),
        ("taepodong", 1015, 50),
        ("nappa-the-saiyan", 1022, 50),
        ("reindeer", 1081, 50),
        ("ice-giant", 1125, 50),
        ("elder-fire-giant", 1126, 50),
        ("the-grand-inquisitor", 1203, 50),
        ("the-midnight-dragon", 1214, 50),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p51b_level_51_52_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("nether-hound", 724, 51),
        ("time-hound", 725, 51),
        ("plasma-hound", 726, 51),
        ("demonic-quylthulg", 727, 51),
        ("ulik-the-troll", 729, 51),
        ("baphomet-the-minotaur-lord", 730, 51),
        ("storm-troll", 875, 51),
        ("dark-elven-shade", 886, 51),
        ("mana-hound", 887, 51),
        ("sleipnir-the-odin-s-steed", 991, 51),
        ("madame-debby", 1032, 51),
        ("hell-knight", 731, 52),
        ("hoarmurath-of-dir", 939, 52),
        ("arachnotron", 1290, 52),
        ("tracking-pixel", 1291, 52),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p52a_level_53_54_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("eihort-the-thing-in-the-labyrinth", 734, 53),
        ("the-king-in-yellow", 735, 53),
        ("khamul-the-easterling", 738, 53),
        ("hound-of-tindalos", 739, 54),
        ("great-ice-wyrm", 741, 54),
        ("the-phoenix", 743, 54),
        ("nightcrawler", 744, 54),
        ("shudde-m-ell", 747, 54),
        ("petshop", 1043, 54),
        ("elder-vampire", 1058, 54),
        ("great-bile-wyrm", 1066, 54),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p52b_level_55_56_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("death-vortex", 751, 55),
        ("aether-vortex", 752, 55),
        ("nidhogg-the-hel-drake", 753, 55),
        ("the-lernean-hydra", 754, 55),
        ("thuringwethil-the-vampire-messenger", 755, 55),
        ("great-hell-wyrm", 756, 55),
        ("hastur-the-unspeakable", 757, 55),
        ("bloodthirster", 758, 55),
        ("draconic-quylthulg", 759, 55),
        ("great-venom-wyrm", 890, 55),
        ("ansalom-the-dark-wizard", 1005, 55),
        ("steel-dragon", 1049, 55),
        ("dionysus-the-munchkin-of-chaos", 1068, 55),
        ("walken", 1069, 55),
        ("barbazu", 1157, 55),
        ("greater-naga", 1162, 55),
        ("hru", 709, 56),
        ("nyogtha-the-thing-that-should-not-be", 760, 56),
        ("ahtu-avatar-of-nyarlathotep", 761, 56),
        ("fundin-bluecloak", 762, 56),
        ("dworkin-barimen", 763, 56),
        ("maeglin-betrayer-of-gondolin", 977, 56),
        ("elder-storm-giant", 1128, 56),
        ("takeminakata-drastic-measures", 1133, 56),
        ("mummy-king", 1267, 56),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p57a_level_57_59_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("lutogaida-the-spellmistress-of-rarfus", 1261, 57),
        ("great-storm-wyrm", 728, 58),
        ("ancalagon-the-black", 766, 58),
        ("daoloth-the-render-of-the-veils", 767, 58),
        ("the-defiler", 1033, 58),
        ("countess-bathory", 1201, 58),
        ("serpopard", 1269, 58),
        ("nightwalker", 768, 59),
        ("habu-the-champion-of-chaos", 770, 59),
        ("grand-master-mystic", 917, 59),
        ("druaga", 935, 59),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p57b_level_60_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index) in [
        ("lord-of-chaos", 737),
        ("saruman-of-many-colours", 771),
        ("gandalf-the-grey", 772),
        ("brand-mad-visionary-of-amber", 773),
        ("ohmu", 879),
        ("bramd-the-ice-dragon", 968),
        ("eibra-the-water-dragon", 969),
        ("narse-the-black-dragon", 970),
        ("mycen-the-gold-dragon", 971),
        ("shooting-star-the-red-dragon", 972),
        ("temporal-champion", 1093),
        ("kotoshiro-the-oracle", 1134),
        ("cave-gorm", 1159),
        ("nagaraja", 1163),
        ("the-diamond-dragon", 1167),
        ("huitzilonevada-the-feathered-boa", 1248),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, 60, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p58_level_61_63_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("keeper-of-secrets", 746, 61),
        ("lems-the-cyborg", 937, 61),
        ("angelic-quylthulg", 1287, 61),
        ("bast-goddess-of-cats", 777, 62),
        ("kenshirou-the-fist-of-the-north-star", 936, 62),
        ("raou-the-conqueror", 1018, 62),
        ("iku-turso", 1288, 62),
        ("great-unclean-one", 736, 63),
        ("the-yamata-no-orochi", 872, 63),
        ("scrupiox-the-nightcrawler", 1238, 63),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p63a_level_64_65_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("wadjet-the-protector", 1257, 64),
        ("jabberwock", 778, 65),
        ("chaos-hound", 779, 65),
        ("locke-the-superman", 881, 65),
        ("layzark-the-emperor", 882, 65),
        ("clone-of-locke-the-superman", 930, 65),
        ("disintegrate-vortex", 1045, 65),
        ("osyluth", 1151, 65),
        ("bronze-golem", 1198, 65),
        ("bone-golem", 1199, 65),
        ("sha", 1270, 65),
        ("valkyrie", 1345, 65),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p63b_level_67_68_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("great-wyrm-of-chaos", 783, 67),
        ("great-wyrm-of-law", 784, 67),
        ("shambler", 786, 67),
        ("glaaki", 788, 67),
        ("bleys-master-of-manipulation", 789, 67),
        ("jisisl-of-ice", 946, 67),
        ("invisible-pink-unicorn", 1079, 67),
        ("okuninushi-the-conqueror", 1135, 67),
        ("great-wyrm-of-many-colours", 790, 68),
        ("fiona-the-sorceress", 791, 68),
        ("omoikane-spirit-of-wisdom", 1136, 68),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p64a_level_69_70_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("julian-master-of-arden-forest", 794, 69),
        ("old-sorcerer", 1026, 69),
        ("takemikazuchi-the-thunder", 1137, 69),
        ("gelugon", 1152, 69),
        ("steam-powered-mechanical-dragon", 1202, 69),
        ("kundry-queen-of-the-lost-haven", 1254, 69),
        ("tiamat-celestial-dragon-of-evil", 795, 70),
        ("the-norsa", 796, 70),
        ("rhan-tegoth", 797, 70),
        ("bouncing-mine", 889, 70),
        ("iketa-the-brave", 949, 70),
        ("flying-spaghetti-monster", 1080, 70),
        ("death-scythe", 1084, 70),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p66_level_71_76_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("caine-the-conspirator", 799, 71),
        ("master-quylthulg", 800, 71),
        ("greater-draconic-quylthulg", 801, 71),
        ("greater-rotting-quylthulg", 802, 71),
        ("greater-demonic-quylthulg", 1123, 71),
        ("muru-siloch-the-spinning-abomination", 1233, 71),
        ("great-wyrm-of-balance", 785, 72),
        ("null-the-living-void", 803, 72),
        ("vecna-the-emperor-lich", 804, 72),
        ("huan-the-hound-of-valinor", 992, 72),
        ("great-wyrm-of-space-time", 1064, 72),
        ("akuvalt-the-deceiver", 1274, 72),
        ("omarax-the-eye-tyrant", 805, 73),
        ("biketal-of-fire", 945, 73),
        ("ammit-the-devourer", 1243, 73),
        ("flying-fishrooster", 1272, 73),
        ("sky-drake", 793, 74),
        ("tsathoggua-the-sleeper-of-n-kai", 806, 74),
        ("gerard-strongman-of-amber", 807, 74),
        ("izanagi-the-spirit", 1139, 74),
        ("ungoliant-the-unlight", 808, 75),
        ("atlach-nacha-the-spider-god", 809, 75),
        ("y-golonac", 810, 75),
        ("aether-hound", 811, 75),
        ("the-demogorgon", 920, 75),
        ("aboleth", 929, 75),
        ("warp-demon", 812, 76),
        ("yig-father-of-serpents", 814, 76),
        ("fenghuang", 988, 76),
        ("kirin", 989, 76),
        ("atlas-the-titan", 1050, 76),
        ("susanoo-the-angry", 1140, 76),
        ("mahishasura-the-buffalo-demon", 1362, 76),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p67_level_77_80_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("unmaker", 815, 77),
        ("cyberdemon", 816, 77),
        ("spectral-wyrm", 874, 77),
        ("pit-fiend", 1154, 77),
        ("hela-queen-of-the-dead", 817, 78),
        ("the-mouth-of-sauron", 818, 78),
        ("klingsor-evil-master-of-magic", 819, 78),
        ("corwin-lord-of-avalon", 820, 78),
        ("the-emperor-quylthulg", 821, 78),
        ("cthugha-the-living-flame", 823, 78),
        ("tsukuyomi-spirit-of-moon", 1141, 78),
        ("greater-balrog", 720, 79),
        ("benedict-the-ideal-warrior", 824, 79),
        ("lourph", 865, 79),
        ("ultimate-magus", 1083, 79),
        ("amaterasu-spirit-of-sun", 1142, 79),
        ("azriel-angel-of-death", 765, 80),
        ("the-witch-king-of-angmar", 825, 80),
        ("cyaegha", 826, 80),
        ("spellwarp-automaton", 1085, 80),
        ("tonberry", 1087, 80),
        ("ninja-tonberry", 1088, 80),
        ("master-tonberry", 1089, 80),
        ("mothra", 1164, 80),
        ("polyphemus-the-blind-cyclops", 1250, 80),
        ("shesha-the-infinite", 1361, 80),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p73_level_81_90_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("richard-wong-master-of-time", 880, 81),
        ("pazuzu-lord-of-air", 827, 82),
        ("ithaqua-the-windwalker", 828, 82),
        ("ullur-the-archer-god", 1358, 82),
        ("hell-hound-of-julian", 829, 83),
        ("tyr-the-one-armed-god", 1350, 83),
        ("cantoras-the-skeletal-lord", 830, 84),
        ("the-tarrasque", 838, 84),
        ("njord-lord-of-the-vanir", 1353, 84),
        ("abhoth-source-of-uncleanness", 833, 85),
        ("ymir-the-ice-giant", 834, 85),
        ("lungorthin-the-balrog-of-white-fire", 839, 85),
        ("wahha-man-the-golden", 1031, 85),
        ("typhoeus-the-storm-giant", 1127, 85),
        ("nebiros-the-marquis", 1160, 85),
        ("king-ghidora", 1165, 85),
        ("pryftidustulyx-the-communist-dragon", 1305, 85),
        ("freyr-lord-of-plenty", 1342, 85),
        ("hanuman-the-monkey-god", 1364, 85),
        ("star-spawn-of-cthulhu", 836, 86),
        ("bahamut-celestial-dragon-of-good", 1000, 86),
        ("thor-the-thunderer", 1349, 86),
        ("rama-the-exiled-prince", 1370, 86),
        ("krishna-avatar-of-vishnu", 1371, 86),
        ("draugluin-sire-of-all-werewolves", 840, 87),
        ("kronos-lord-of-the-titans", 1051, 87),
        ("vidarr-the-silent-avenger", 1351, 87),
        ("karthikeya-the-six-headed-warrior", 1366, 87),
        ("thanatos-god-of-death", 1166, 88),
        ("anubis-keeper-of-the-balance", 1242, 88),
        ("isis-the-great-goddess", 1263, 88),
        ("yama-lord-of-the-dead", 1379, 88),
        ("tulzscha-the-green-flame", 842, 89),
        ("chitauli", 1169, 89),
        ("apophis-the-primordial-chaos", 1245, 89),
        ("fenris-wolf", 846, 90),
        ("great-wyrm-of-power", 847, 90),
        ("caaws", 866, 90),
        ("star-blade", 1178, 90),
        ("the-storm-of-unmagic", 1193, 90),
        ("beelzebub-lord-of-the-flies", 1194, 90),
        ("jack-the-ripper", 1204, 90),
        ("kali-mother-of-rage", 1384, 90),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }

    let sky_drake = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.sky-drake")
        .expect("Sky Drake should remain imported");
    let evolution = sky_drake
        .evolution
        .as_ref()
        .expect("P73 should complete Sky Drake's evolution target");
    assert_eq!(evolution.required_experience, 600_000);
    assert_eq!(
        evolution.next_actor_kind_id,
        "demo.actor.great-wyrm-of-power"
    );
}

#[test]
fn p74_level_91_127_direct_monsters_keep_source_identity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("wiruin-the-maelstrom", 1192, 91),
        ("festivus-the-teenage-offscreen-ninja-drolem", 1230, 91),
        ("seth-the-disintegrator", 1260, 91),
        ("carcharoth-the-jaws-of-thirst", 850, 92),
        ("umbaba-samahongo", 1170, 92),
        ("horus-the-ancient", 1244, 92),
        ("azathoth-seething-nuclear-chaos", 852, 93),
        ("the-maw-of-hell", 1191, 93),
        ("c-blue", 1395, 93),
        ("cerberus-guardian-of-hades", 853, 94),
        ("the-destroyer", 855, 94),
        ("the-babbage-analytical-engine", 1205, 96),
        ("the-etheric-dimensional-phase-automaton", 1207, 96),
        ("dor", 1181, 97),
        ("sekhmet-the-mistress-of-fury", 1247, 97),
        ("michael-the-guardian-overlord", 1179, 98),
        ("lucifer-father-of-lies", 1195, 100),
        ("metatron-the-high-angel", 1253, 100),
        ("monkey-clone", 1095, 127),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }
}

#[test]
fn p75a_category_summoners_and_no_summon_actor_keep_source_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (id, legacy_index, level) in [
        ("hathor-the-heavenly-cow", 1271, 82),
        ("freyja-lady-of-the-slain", 1352, 83),
        ("mephistopheles-lord-of-hell", 831, 84),
        ("surtur-the-giant-fire-demon", 837, 85),
        ("frigg-queen-of-asgard", 1354, 86),
        ("oremorj-the-cyberdemon-lord", 843, 89),
        ("yog-sothoth-the-all-in-one", 845, 90),
        ("durga-the-goddess-of-war", 1383, 90),
        ("indra-the-heavenly-king-of-meru", 1391, 92),
        ("nyarlathotep-the-crawling-chaos", 851, 93),
        ("gothmog-the-high-captain-of-balrogs", 856, 95),
        ("great-cthulhu", 857, 96),
        ("amun-the-mysterious", 1266, 97),
        ("a-plain-gold-ring", 864, 110),
    ] {
        let actor_id = format!("demo.actor.{id}");
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("{actor_id} should be imported"));
        assert_eq!(actor.level, level, "{actor_id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{actor_id} source index"
        );
    }

    for (ability_id, category, sides, bonus) in [
        ("summon-cyber-l84-1d3", "cyber", 3, 0),
        ("summon-cat-l83-1d3-1", "cat", 3, 1),
        ("summon-egyptian-l82-1d2", "egyptian", 2, 0),
        ("summon-hindu-l92-1d2", "hindu", 2, 0),
        ("summon-norse-l83-1d2", "norse", 2, 0),
    ] {
        let ability = artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{ability_id}"))
            .unwrap_or_else(|| panic!("{ability_id} should be imported"));
        let AbilityEffectDefinition::SummonCategory {
            category: actual_category,
            count_dice,
            count_sides,
            count_bonus,
            ..
        } = &ability.effect
        else {
            panic!("{ability_id} should remain a category summon");
        };
        assert_eq!(actual_category, category);
        assert_eq!((*count_dice, *count_sides, *count_bonus), (1, sides, bonus));
    }

    let ring = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.a-plain-gold-ring")
        .expect("Plain Gold Ring should be imported");
    assert!(ring.tags.iter().any(|tag| tag == "no-summon"));
    assert!(
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == "demo.actor.cyberdemon")
            .is_some_and(|actor| actor.tags.iter().any(|tag| tag == "cyber"))
    );
}

#[test]
fn p75b_fixed_summoners_and_combat_mappings_keep_source_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("varuna-lord-of-water", 1376, 81),
        ("combat-echizen-because-it-s-time", 873, 85),
        ("demeter-the-goddess-of-nature", 1106, 86),
        ("justshorn-sorcerer-king-of-the-sheeple", 1225, 86),
        ("poseidon-lord-of-seas-and-storm", 1097, 88),
        ("raphael-the-messenger", 769, 89),
        ("talos-masterwork-spellwarp-automaton", 1086, 90),
        ("saraswati-goddess-of-knowledge", 1390, 90),
        ("brahma-the-creating-spirit", 1389, 92),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    for (id, ability_id) in [
        ("combat-echizen-because-it-s-time", "jump-shards-l85"),
        ("raphael-the-messenger", "breath-holy-fire-17-250-r3"),
    ] {
        assert!(
            actor(id)
                .monster_casting
                .as_ref()
                .expect("P75B combat mapper should preserve casting")
                .abilities
                .iter()
                .any(|candidate| candidate.ability_id
                    == format!("rfb-legacy.ability.{ability_id}")),
            "{id} should use {ability_id}"
        );
    }

    for (ability_id, category, maximum_level, sides, bonus, target, water_flow) in [
        (
            "summon-makara-l50-1d2-2",
            "mount-meru",
            50,
            2,
            2,
            "demo.actor.makara",
            true,
        ),
        (
            "summon-ent-l46-1d4",
            "giant",
            46,
            4,
            0,
            "demo.actor.ent",
            false,
        ),
        (
            "summon-sheep-l3-1d4",
            "sheep",
            3,
            4,
            0,
            "demo.actor.sheep",
            false,
        ),
        (
            "summon-greater-kraken-l63-1d4",
            "ocean",
            63,
            4,
            0,
            "demo.actor.greater-kraken",
            true,
        ),
        (
            "summon-spellwarp-automaton-l80-1d3",
            "nonliving",
            80,
            3,
            0,
            "demo.actor.spellwarp-automaton",
            false,
        ),
        (
            "summon-saraswati-l90-1d1",
            "hindu",
            90,
            1,
            0,
            "demo.actor.saraswati-goddess-of-knowledge",
            false,
        ),
        (
            "summon-brahma-l92-1d1",
            "hindu",
            92,
            1,
            0,
            "demo.actor.brahma-the-creating-spirit",
            false,
        ),
    ] {
        let ability = artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{ability_id}"))
            .unwrap_or_else(|| panic!("{ability_id} should be imported"));
        let AbilityEffectDefinition::SummonCategory {
            category: actual_category,
            maximum_level: actual_maximum_level,
            count_dice,
            count_sides,
            count_bonus,
            batch_candidates,
            ..
        } = &ability.effect
        else {
            panic!("{ability_id} should remain a category summon");
        };
        assert_eq!(actual_category, category);
        assert_eq!(
            (
                *actual_maximum_level,
                *count_dice,
                *count_sides,
                *count_bonus
            ),
            (maximum_level, 1, sides, bonus)
        );
        assert_eq!(batch_candidates.len(), 1);
        assert_eq!(batch_candidates[0].actor_kind_id, target);
        assert_eq!(batch_candidates[0].weight, 1);
        assert_eq!(
            ability.tags.iter().any(|tag| tag == "monster-water-flow"),
            water_flow
        );
    }
}

#[test]
fn p68_low_risk_mappings_keep_source_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("izanami-spirit-of-yomi", 1138, 71),
        ("uriel-angel-of-fire", 764, 73),
        ("nephthys-lady-of-the-night", 1264, 77),
        ("caldarm-the-third", 931, 79),
        ("hell-spider", 1177, 80),
        ("metal-babble", 871, 80),
        ("metal-babble-unique", 1110, 80),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    assert!(matches!(
        ability("ball-poison-12d2").effect,
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 12,
            damage_sides: 2,
            damage_bonus: 0,
            damage_type: ActorDamageType::Poison,
            radius: 2,
            ..
        }
    ));
    assert!(matches!(
        ability("ball-nexus-10d10-158").effect,
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 10,
            damage_sides: 10,
            damage_bonus: 158,
            damage_type: ActorDamageType::Nexus,
            radius: 2,
            ..
        }
    ));

    let uriel_aura = &actor("uriel-angel-of-fire").contact_auras[0];
    assert_eq!(uriel_aura.damage_type, ActorDamageType::Plasma);
    assert_eq!((uriel_aura.damage_dice, uriel_aura.damage_sides), (4, 5));

    let hell_spider_aura = &actor("hell-spider").contact_auras[0];
    assert_eq!(hell_spider_aura.damage_type, ActorDamageType::HellFire);
    assert_eq!(
        (hell_spider_aura.damage_dice, hell_spider_aura.damage_sides),
        (2, 6)
    );
    assert!(matches!(
        ability("jump-hell-fire-1d1-65").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 65,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: ActorDamageType::HellFire,
            radius: 5,
            blink_radius: 10,
        }
    ));

    assert!(
        actor("clone-of-locke-the-superman")
            .tags
            .iter()
            .any(|tag| tag == "clone-of-locke")
    );
    assert!(matches!(
        ability("summon-clone-of-locke-l65-1d3").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 65,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 0,
            ..
        } if category == "clone-of-locke"
    ));

    let normal_babble = actor("metal-babble");
    let unique_babble = actor("metal-babble-unique");
    assert_eq!(normal_babble.experience_value, 400_000);
    assert_eq!(unique_babble.experience_value, 400_000);
    assert!(!normal_babble.tags.iter().any(|tag| tag == "unique"));
    assert!(unique_babble.tags.iter().any(|tag| tag == "unique"));
}

#[test]
fn p69_pantheon_monsters_keep_source_identity_and_norse_summoning() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level, pantheon) in [
        ("skadi-the-huntress", 1355, 72, "norse"),
        ("heimdall-guardian-of-bifrost", 1348, 77, "norse"),
        ("magni-son-of-thor", 1359, 78, "norse"),
        ("ganesha-the-elephant-god", 1378, 78, "hindu"),
        ("agni-the-threefold-fire", 1363, 79, "hindu"),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
        assert!(actor.tags.iter().any(|tag| tag == pantheon), "{id}");
        assert!(actor.tags.iter().any(|tag| tag == "unique"), "{id}");
    }

    let norse_ids = artifact
        .content
        .actors
        .iter()
        .filter(|actor| actor.tags.iter().any(|tag| tag == "norse"))
        .map(|actor| actor.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        norse_ids,
        [
            "demo.actor.aegir-god-king-of-the-sea-giants",
            "demo.actor.freyja-lady-of-the-slain",
            "demo.actor.freyr-lord-of-plenty",
            "demo.actor.frigg-queen-of-asgard",
            "demo.actor.heimdall-guardian-of-bifrost",
            "demo.actor.loki-the-trickster",
            "demo.actor.magni-son-of-thor",
            "demo.actor.njord-lord-of-the-vanir",
            "demo.actor.odin-the-all-father",
            "demo.actor.skadi-the-huntress",
            "demo.actor.thor-the-thunderer",
            "demo.actor.tyr-the-one-armed-god",
            "demo.actor.ullur-the-archer-god",
            "demo.actor.vidarr-the-silent-avenger",
        ]
        .into_iter()
        .collect()
    );

    let summon = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.summon-norse-l77-1d2")
        .expect("Heimdall pantheon summon should be imported");
    assert!(matches!(
        summon.effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 77,
            count_dice: 1,
            count_sides: 2,
            count_bonus: 0,
            ..
        } if category == "norse"
    ));
}

#[test]
fn p70_aegir_and_sea_giant_keep_ocean_and_special_summon_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("P70 should contain {id}"))
    };

    let sea_giant = actor("demo.actor.sea-giant");
    let sea_allocation = sea_giant
        .allocation
        .as_ref()
        .expect("Sea giant should retain ocean allocation");
    assert_eq!(sea_giant.level, 45);
    assert_eq!(sea_allocation.legacy_index, 1276);
    assert!(sea_allocation.wild_only);
    assert_eq!(sea_allocation.habitats, vec![ActorHabitat::Ocean]);
    assert!(sea_giant.tags.iter().any(|tag| tag == "ocean"));
    assert!(!sea_giant.tags.iter().any(|tag| tag == "orc-cave"));

    let aegir = actor("demo.actor.aegir-god-king-of-the-sea-giants");
    assert_eq!(aegir.level, 77);
    assert_eq!(
        aegir
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1277)
    );
    for tag in ["norse", "orc-cave", "unique"] {
        assert!(aegir.tags.iter().any(|candidate| candidate == tag));
    }

    let summon = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.summon-aegir-retinue-1d4")
        .expect("Aegir special summon should be imported");
    assert!(summon.tags.iter().any(|tag| tag == "monster-water-flow"));
    let AbilityEffectDefinition::SummonCategory {
        category,
        maximum_level,
        count_dice,
        count_sides,
        count_bonus,
        batch_candidates,
        ..
    } = &summon.effect
    else {
        panic!("Aegir special should remain a category summon");
    };
    assert_eq!(category, "ocean");
    assert_eq!(
        (*maximum_level, *count_dice, *count_sides, *count_bonus),
        (77, 1, 4, 0)
    );
    assert_eq!(
        batch_candidates
            .iter()
            .map(|candidate| (candidate.actor_kind_id.as_str(), candidate.weight))
            .collect::<Vec<_>>(),
        vec![("demo.actor.sea-giant", 1), ("demo.actor.lesser-kraken", 1),]
    );
}

#[test]
fn p71_banor_rupart_forms_keep_source_identity_and_shared_transform() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("P71 should contain {id}"))
    };
    for (id, legacy_index, rarity, max_hp) in [
        ("demo.actor.banor-rupart", 932, 2, 7_000),
        ("demo.actor.banor-the-prince-regent", 933, 255, 3_500),
        ("demo.actor.rupart-the-general", 934, 255, 3_500),
    ] {
        let definition = actor(id);
        let allocation = definition
            .allocation
            .as_ref()
            .expect("P71 forms should retain source allocation metadata");
        assert_eq!(definition.level, 71);
        assert_eq!(definition.max_hp, max_hp);
        assert_eq!(
            (allocation.legacy_index, allocation.rarity),
            (legacy_index, rarity)
        );
        assert!(definition.tags.iter().any(|tag| tag == "unique"));
        assert!(
            definition
                .monster_casting
                .as_ref()
                .expect("P71 forms should cast")
                .abilities
                .iter()
                .any(|candidate| {
                    candidate.ability_id == "rfb-legacy.ability.banor-rupart-transform"
                })
        );
    }

    let transform = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.banor-rupart-transform")
        .expect("P71 transform ability should compile");
    assert!(matches!(
        &transform.effect,
        AbilityEffectDefinition::NoOp { reason } if reason == "banor-rupart-transform"
    ));
    assert!(
        transform
            .tags
            .iter()
            .any(|tag| tag == "monster-banor-rupart-transform")
    );
}

#[test]
fn p72_location_bound_monsters_keep_their_allocation_boundaries() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P72 should contain {id}"))
    };

    for (id, legacy_index, level) in [
        ("mathmag-the-prince-of-whales", 1251, 74),
        ("jormungand-the-midgard-serpent", 854, 75),
        ("leviathan", 782, 76),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("ocean monsters should retain wilderness allocation");
        assert_eq!(
            (actor.level, allocation.legacy_index),
            (level, legacy_index)
        );
        assert!(allocation.wild_only);
        assert_eq!(allocation.habitats, vec![ActorHabitat::Ocean]);
        assert!(actor.tags.iter().any(|tag| tag == "ocean"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }

    for (id, legacy_index, level) in [
        ("jambavan-king-of-the-beasts", 1385, 74),
        ("vali-king-of-the-vanaras", 1369, 76),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("Mount Meru monsters should retain dungeon allocation");
        assert_eq!(
            (actor.level, allocation.legacy_index),
            (level, legacy_index)
        );
        assert_eq!(allocation.legacy_dungeon_indices, vec![43]);
        assert!(actor.tags.iter().any(|tag| tag == "mount-meru"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }

    for (id, level) in [("basement-cat", 74), ("eric-the-usurper", 76)] {
        let actor = actor(id);
        assert_eq!(actor.level, level);
        assert!(actor.allocation.is_none());
        assert!(actor.tags.iter().any(|tag| tag == "fixed-placement"));
        assert!(actor.tags.iter().any(|tag| tag == "fixed-unique"));
    }

    for id in ["vanara", "vanara-sage", "vali-king-of-the-vanaras"] {
        assert!(actor(id).tags.iter().any(|tag| tag == "vanara"), "{id}");
    }
    let summon = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.summon-vanara-l76-1d3-1")
        .expect("Vali's vanara summon should compile");
    assert!(matches!(
        summon.effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 76,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 1,
            ..
        } if category == "vanara"
    ));
}

#[test]
fn p64b_low_risk_mappings_keep_source_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("grand-fearlord", 1121, 65),
        ("ultimate-beholder", 781, 66),
        ("the-nightmare-dragon", 1215, 66),
        ("hypnos-lord-of-sleep", 787, 67),
        ("tselakus-the-dreadlord", 792, 68),
        ("disintegrate-spider", 1175, 70),
        ("vasuki-the-serpent-king", 1360, 70),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    assert!(
        actor("night-mare")
            .tags
            .iter()
            .any(|tag| tag == "night-mare")
    );
    for (id, expected_category, maximum_level, count_sides, count_bonus) in [
        ("summon-night-mare-l65-1d3-1", "night-mare", 65, 3, 1),
        ("summon-night-mare-l67-1d3-1", "night-mare", 67, 3, 1),
        ("summon-amberite-l68-1d2", "amberite", 68, 2, 0),
        ("summon-kin-glyph-110-l70-1d3-1", "kin-glyph-110", 70, 3, 1),
    ] {
        assert!(
            matches!(
                ability(id).effect,
                AbilityEffectDefinition::SummonCategory {
                    ref category,
                    maximum_level: actual_level,
                    count_dice: 1,
                    count_sides: actual_sides,
                    count_bonus: actual_bonus,
                    ..
                } if category == expected_category
                    && actual_level == maximum_level
                    && actual_sides == count_sides
                    && actual_bonus == count_bonus
            ),
            "{id}"
        );
    }
    assert!(matches!(
        ability("summon-night-mare-l39-1d3-2").effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 39,
            count_dice: 1,
            count_sides: 3,
            count_bonus: 2,
            ..
        } if category == "night-mare"
    ));
    assert!(matches!(
        ability("jump-disintegrate-l70").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 70,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: ActorDamageType::Disintegrate,
            radius: 5,
            blink_radius: 10,
        }
    ));
    for (id, damage_type, dice, sides) in [
        ("tselakus-the-dreadlord", ActorDamageType::Dark, 3, 4),
        ("disintegrate-spider", ActorDamageType::Disintegrate, 3, 3),
    ] {
        let aura = &actor(id).contact_auras[0];
        assert_eq!(aura.damage_type, damage_type, "{id} aura type");
        assert_eq!((aura.damage_dice, aura.damage_sides), (dice, sides));
    }

    let brain_smash = &actor("ultimate-beholder")
        .melee_routine
        .as_ref()
        .expect("Ultimate beholder should retain melee")
        .blows[0]
        .effects;
    assert!(matches!(
        brain_smash[0],
        MeleeBlowEffectDefinition::Damage {
            damage_type: ActorDamageType::Psi,
            damage_dice: 5,
            damage_sides: 5,
            ..
        }
    ));
    assert!(matches!(
        brain_smash[1],
        MeleeBlowEffectDefinition::Blind { .. }
    ));
    assert!(matches!(
        brain_smash[2],
        MeleeBlowEffectDefinition::Confusion { .. }
    ));
    assert!(matches!(
        brain_smash[3],
        MeleeBlowEffectDefinition::Paralysis { .. }
    ));
    assert!(matches!(
        brain_smash[4],
        MeleeBlowEffectDefinition::Slow { .. }
    ));
}

#[test]
fn p65_dio_brando_binds_world_to_the_extra_action_marker() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.dio-brando")
        .expect("Dio Brando should be imported");
    assert_eq!(actor.level, 66);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(878)
    );
    assert!(actor.monster_casting.as_ref().is_some_and(|casting| {
        casting
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id == "rfb-legacy.ability.world")
    }));
    assert!(matches!(
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == "rfb-legacy.ability.kin-dio-brando")
            .map(|ability| &ability.effect),
        Some(AbilityEffectDefinition::SummonCategory {
            category,
            maximum_level: 66,
            count_dice: 1,
            count_sides: 1,
            count_bonus: 1,
            ..
        }) if category == "kin-glyph-86"
    ));

    let world = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.world")
        .expect("WORLD should be imported");
    assert!(world.tags.iter().any(|tag| tag == "monster-world"));
    assert!(matches!(
        &world.effect,
        AbilityEffectDefinition::NoOp { reason } if reason == "monster-world"
    ));
}

#[test]
fn p59a_nether_jump_and_contact_auras_keep_source_semantics() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("undead-spider", 1176, 60),
        ("vlad-dracula-prince-of-darkness", 780, 63),
        ("solar", 943, 64),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    for (id, damage_type, dice, sides) in [
        ("undead-spider", ActorDamageType::Nether, 3, 3),
        (
            "vlad-dracula-prince-of-darkness",
            ActorDamageType::Nether,
            3,
            6,
        ),
        ("solar", ActorDamageType::HolyFire, 3, 3),
    ] {
        let aura = &actor(id).contact_auras[0];
        assert_eq!(aura.damage_type, damage_type, "{id} aura type");
        assert_eq!((aura.damage_dice, aura.damage_sides), (dice, sides));
    }

    let undead_spider = actor("undead-spider");
    assert!(
        undead_spider
            .monster_casting
            .as_ref()
            .expect("Undead spider should cast")
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id == "rfb-legacy.ability.jump-nether-l60")
    );
    let jump = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.jump-nether-l60")
        .expect("Nether jump should be imported");
    assert!(matches!(
        jump.effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 60,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: ActorDamageType::Nether,
            radius: 5,
            blink_radius: 10,
        }
    ));
}

#[test]
fn p59bc_location_restricted_monsters_keep_source_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("greater-kraken", 775, 63),
        ("behemoth", 716, 64),
        ("hugin-the-scheming-raven", 1346, 58),
        ("asura", 1374, 58),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }

    for id in ["greater-kraken", "behemoth"] {
        let actor = actor(id);
        let allocation = actor.allocation.as_ref().expect("ocean allocation");
        assert!(allocation.wild_only);
        assert_eq!(allocation.habitats, vec![ActorHabitat::Ocean]);
        assert!(allocation.legacy_dungeon_indices.is_empty());
        assert!(actor.tags.iter().any(|tag| tag == "ocean"));
    }

    for (id, dungeon_index, tag) in [
        ("hugin-the-scheming-raven", 39, "asgard"),
        ("asura", 43, "mount-meru"),
    ] {
        let actor = actor(id);
        let allocation = actor.allocation.as_ref().expect("dungeon allocation");
        assert!(!allocation.wild_only);
        assert_eq!(allocation.legacy_dungeon_indices, vec![dungeon_index]);
        assert!(actor.tags.iter().any(|actor_tag| actor_tag == tag));
    }
}

#[test]
fn p60_gragomani_keeps_curse_melee_and_weighted_batch_followers() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.gragomani-the-leprechaun-prophet")
        .expect("Gragomani should be imported");
    assert_eq!(actor.level, 61);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1285)
    );
    let curse = &actor
        .melee_routine
        .as_ref()
        .expect("Gragomani should retain melee")
        .blows[3]
        .effects[0];
    assert!(matches!(
        curse,
        MeleeBlowEffectDefinition::Damage {
            damage_dice: 6,
            damage_sides: 6,
            damage_type: ActorDamageType::Curse,
            armor_mitigated: false,
            ..
        }
    ));
    assert!(
        actor
            .monster_casting
            .as_ref()
            .expect("Gragomani should cast")
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id
                == "rfb-legacy.ability.summon-gragomani-followers-1d4-4")
    );
    let ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.summon-gragomani-followers-1d4-4")
        .expect("Gragomani special summon should be imported");
    let AbilityEffectDefinition::SummonCategory {
        category,
        count_dice,
        count_sides,
        count_bonus,
        batch_candidates,
        ..
    } = &ability.effect
    else {
        panic!("Gragomani special should remain a category summon");
    };
    assert_eq!(category, "kin-glyph-104");
    assert_eq!((*count_dice, *count_sides, *count_bonus), (1, 4, 4));
    assert_eq!(
        batch_candidates,
        &[
            AbilitySummonCandidateDefinition {
                actor_kind_id: "demo.actor.malicious-leprechaun".to_owned(),
                weight: 1,
            },
            AbilitySummonCandidateDefinition {
                actor_kind_id: "demo.actor.leprechaun-fanatic".to_owned(),
                weight: 3,
            },
        ]
    );
}

#[test]
fn p53a_ice_jump_and_angel_summons_reuse_shared_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index, level) in [
        ("planetar", 942, 50),
        ("ice-spider", 1173, 50),
        ("knight-templar", 1037, 52),
        ("greater-dokkaebi", 1394, 55),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    assert_eq!(
        actor("ice-spider")
            .contact_auras
            .iter()
            .map(|aura| (aura.damage_type, aura.damage_dice, aura.damage_sides))
            .collect::<Vec<_>>(),
        vec![(ActorDamageType::Ice, 3, 3)]
    );
    assert!(matches!(
        ability("jump-ice-l50").effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 50,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: ActorDamageType::Ice,
            radius: 5,
            blink_radius: 10,
        }
    ));

    for level in [50, 52, 55] {
        assert!(matches!(
            ability(&format!("summon-angel-l{level}-1d3-1")).effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level,
                count_dice: 1,
                count_sides: 3,
                count_bonus: 1,
                ..
            } if category == "angel" && maximum_level == level
        ));
    }

    let angel_ids = artifact
        .content
        .actors
        .iter()
        .filter(|actor| actor.tags.iter().any(|tag| tag == "angel"))
        .map(|actor| actor.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        angel_ids,
        [
            "demo.actor.angel",
            "demo.actor.archangel",
            "demo.actor.archon",
            "demo.actor.azriel-angel-of-death",
            "demo.actor.cherub",
            "demo.actor.fallen-angel",
            "demo.actor.lucifer-father-of-lies",
            "demo.actor.metatron-the-high-angel",
            "demo.actor.michael-the-guardian-overlord",
            "demo.actor.planetar",
            "demo.actor.raphael-the-messenger",
            "demo.actor.seraph",
            "demo.actor.solar",
            "demo.actor.star-blade",
            "demo.actor.uriel-angel-of-fire",
        ]
        .into_iter()
        .collect()
    );
    assert!(artifact.content.actors.iter().all(|actor| {
        !actor.tags.iter().any(|tag| tag == "angel")
            || (actor.glyph == "A"
                && actor
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "evil" | "good")))
    }));
}

#[test]
fn p53b_fixed_special_summons_target_reindeer_and_death_pumpkins() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };
    let ability = |id: &str| {
        artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{id}"))
            .unwrap_or_else(|| panic!("{id} should be imported"))
    };

    for (id, legacy_index) in [("santa-claus", 733), ("jack-of-lanterns", 1302)] {
        let actor = actor(id);
        assert_eq!(actor.level, 52, "{id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} source index"
        );
    }

    for (caster_id, target_id, category, ability_id) in [
        (
            "santa-claus",
            "reindeer",
            "reindeer",
            "summon-reindeer-l52-1d4",
        ),
        (
            "jack-of-lanterns",
            "death-pumpkin",
            "death-pumpkin",
            "summon-death-pumpkin-l52-1d4",
        ),
    ] {
        assert!(
            actor(target_id).tags.iter().any(|tag| tag == category),
            "{target_id} should belong to its fixed summon category"
        );
        assert!(
            actor(caster_id)
                .monster_casting
                .as_ref()
                .expect("special summoner should retain monster casting")
                .abilities
                .iter()
                .any(|candidate| {
                    candidate.ability_id == format!("rfb-legacy.ability.{ability_id}")
                }),
            "{caster_id} should cast {ability_id}"
        );
        assert!(matches!(
            ability(ability_id).effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level: 52,
                count_dice: 1,
                count_sides: 4,
                count_bonus: 0,
                ..
            } if category == target_id
        ));
    }

    assert_eq!(
        actor("jack-of-lanterns")
            .contact_auras
            .iter()
            .map(|aura| (aura.damage_type, aura.damage_dice, aura.damage_sides))
            .collect::<Vec<_>>(),
        vec![(ActorDamageType::Light, 3, 3)]
    );
}

#[test]
fn p54_ancient_roc_uses_the_dedicated_bird_drop_effect() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.the-ancient-roc-of-okeldad")
        .expect("the Ancient Roc should be imported");
    assert_eq!(actor.level, 52);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1239)
    );
    assert!(actor.movement.modes.contains(&ActorMovementMode::Fly));
    assert!(
        actor
            .monster_casting
            .as_ref()
            .expect("the Ancient Roc should retain monster casting")
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id == "rfb-legacy.ability.bird-drop")
    );
    let ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.bird-drop")
        .expect("bird drop should be imported");
    assert!(matches!(ability.effect, AbilityEffectDefinition::BirdDrop));
    assert_eq!(
        ability.target.modes,
        vec![
            AbilityTargetModeDefinition::Position,
            AbilityTargetModeDefinition::Entity,
        ]
    );
    assert_eq!(ability.target.range, 8);
    assert!(ability.target.requires_line_of_effect);
}

#[test]
fn p55a_monsters_retain_their_dedicated_location_boundaries() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("P55A should contain {id}"))
    };

    for (id, source_index) in [
        ("demo.actor.vanara", 1367),
        ("demo.actor.makara", 1377),
        ("demo.actor.rakshasa", 1386),
        ("demo.actor.vanara-sage", 1375),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("Mount Meru monsters should retain allocation metadata");
        assert_eq!(allocation.legacy_index, source_index);
        assert_eq!(allocation.legacy_dungeon_indices, vec![43]);
        assert!(!allocation.wild_only);
        assert!(actor.tags.iter().any(|tag| tag == "mount-meru"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }

    for id in ["demo.actor.fastitocalon", "demo.actor.lesser-kraken"] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("ocean monsters should retain allocation metadata");
        assert!(allocation.wild_only);
        assert_eq!(allocation.habitats, vec![ActorHabitat::Ocean]);
        assert!(allocation.legacy_dungeon_indices.is_empty());
        assert!(actor.tags.iter().any(|tag| tag == "ocean"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }
}

#[test]
fn p55b_eagles_keep_their_wilderness_and_summon_boundaries() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P55B should contain {id}"))
    };

    assert_eq!(
        artifact
            .content
            .actors
            .iter()
            .filter(|actor| actor.tags.iter().any(|tag| tag == "eagle"))
            .map(|actor| actor.id.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "demo.actor.eagle",
            "demo.actor.great-eagle",
            "demo.actor.gwaihir-the-windlord",
            "demo.actor.meneldor-the-swift",
            "demo.actor.thorondor",
        ]
        .into_iter()
        .collect()
    );

    for (id, source_index, level) in [
        ("great-eagle", 335, 35),
        ("meneldor-the-swift", 384, 38),
        ("gwaihir-the-windlord", 410, 40),
        ("thorondor", 468, 55),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("P55B eagles should retain allocation metadata");
        assert_eq!(allocation.legacy_index, source_index);
        assert!(allocation.wild_only);
        assert!(allocation.legacy_dungeon_indices.is_empty());
        assert_eq!(
            allocation.habitats.iter().copied().collect::<BTreeSet<_>>(),
            [
                ActorHabitat::Mountain,
                ActorHabitat::Snow,
                ActorHabitat::Volcano,
            ]
            .into_iter()
            .collect()
        );
        assert!(actor.monster_casting.as_ref().is_some_and(|casting| {
            casting
                .abilities
                .iter()
                .any(|candidate| candidate.ability_id == "rfb-legacy.ability.bird-drop")
        }));
    }

    for level in [38, 40, 55] {
        let ability = artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.summon-eagle-l{level}-1d3-1"))
            .unwrap_or_else(|| panic!("P55B should contain the level {level} eagle summon"));
        assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::SummonCategory {
                ref category,
                maximum_level,
                count_dice: 1,
                count_sides: 3,
                count_bonus: 1,
                ..
            } if category == "eagle" && maximum_level == level
        ));
    }
}

#[test]
fn p56a_internet_exploder_keeps_its_slow_time_death_explosion() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.internet-exploder")
        .expect("P56A should contain Internet Exploder");
    assert_eq!(actor.level, 50);
    let allocation = actor
        .allocation
        .as_ref()
        .expect("Internet Exploder should retain allocation metadata");
    assert_eq!(allocation.legacy_index, 921);
    assert_eq!(allocation.rarity, 4);
    assert_eq!(allocation.max_depth, 999);
    let blow = actor
        .melee_routine
        .as_ref()
        .and_then(|routine| routine.blows.first())
        .expect("Internet Exploder should retain its explosion");
    assert!(blow.self_destructs);
    assert!(matches!(
        blow.effects.as_slice(),
        [
            MeleeBlowEffectDefinition::Damage {
                damage_dice: 10,
                damage_sides: 20,
                damage_type: ActorDamageType::Time,
                ..
            },
            MeleeBlowEffectDefinition::Slow { .. }
        ]
    ));
}

#[test]
fn p56b_task_monsters_stay_fixed_and_keep_exact_special_summons() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P56B should contain {id}"))
    };

    for (id, level, task_tag) in [
        ("bull-gates", 52, "angwil-quest"),
        ("lord-bovin-of-the-high-tower", 52, "anambar-quest"),
        ("the-gospel-of-mug", 56, "angwil-quest"),
    ] {
        let actor = actor(id);
        assert_eq!(actor.level, level);
        assert!(actor.allocation.is_none());
        assert!(actor.tags.iter().any(|tag| tag == "fixed-placement"));
        assert!(actor.tags.iter().any(|tag| tag == task_tag));
    }

    for (actor_id, category, ability_id) in [
        (
            "internet-exploder",
            "internet-exploder",
            "summon-internet-exploder-l52-1d4",
        ),
        (
            "tracking-pixel",
            "tracking-pixel",
            "summon-tracking-pixel-l56-1d4-max3",
        ),
    ] {
        assert!(actor(actor_id).tags.iter().any(|tag| tag == category));
        let ability = artifact
            .content
            .abilities
            .iter()
            .find(|ability| ability.id == format!("rfb-legacy.ability.{ability_id}"))
            .unwrap_or_else(|| panic!("P56B should contain {ability_id}"));
        assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::SummonCategory {
                category: ref actual_category,
                count_dice: 1,
                count_sides: 4,
                ..
            } if actual_category == category
        ));
    }

    let gospel = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb-legacy.ability.summon-tracking-pixel-l56-1d4-max3")
        .expect("P56B Gospel summon should compile");
    assert!(matches!(
        gospel.effect,
        AbilityEffectDefinition::SummonCategory {
            maximum_count: Some(3),
            ..
        }
    ));
}

#[test]
fn outpost_has_walls_inner_shops_and_an_exterior_warrens_entrance() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens");
    assert_eq!(world.town_id.as_deref(), Some("demo.town.outpost"));
    let wilderness = world
        .wilderness
        .as_ref()
        .expect("Warrens journey should contain the authoritative wilderness map");
    assert_eq!((wilderness.width, wilderness.height), (99, 66));
    assert_eq!(wilderness.start_position, ContentPosition { x: 28, y: 52 });
    assert_eq!(wilderness.rows.len(), 66);
    assert!(wilderness.rows.iter().all(|row| row.len() == 99));
    assert_eq!(wilderness.legend.len(), 30);
    assert_eq!(
        wilderness.legend.iter().filter(|entry| entry.road).count(),
        10
    );
    assert_eq!(
        wilderness.locations,
        [
            WildernessLocationDefinition::Town {
                position: ContentPosition { x: 26, y: 39 },
                map_origin: ContentPosition { x: 27, y: 6 },
                town_id: "demo.town.anambar".to_owned(),
            },
            WildernessLocationDefinition::Town {
                position: ContentPosition { x: 28, y: 52 },
                map_origin: ContentPosition { x: 0, y: 0 },
                town_id: "demo.town.outpost".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 5, y: 48 },
                dungeon_id: "demo.dungeon.labyrinth".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 7, y: 59 },
                dungeon_id: "demo.dungeon.camelot".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 13, y: 53 },
                dungeon_id: "demo.dungeon.volcano".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 17, y: 29 },
                dungeon_id: "demo.dungeon.icky-cave".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 28, y: 52 },
                dungeon_id: "demo.dungeon.hideout".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 28, y: 52 },
                dungeon_id: "demo.dungeon.man-cave".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 28, y: 52 },
                dungeon_id: "demo.dungeon.warrens".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 30, y: 27 },
                dungeon_id: "demo.dungeon.atlantis".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 30, y: 27 },
                dungeon_id: "demo.dungeon.numenor".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 30, y: 45 },
                dungeon_id: "demo.dungeon.orc-cave".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 30, y: 45 },
                dungeon_id: "demo.dungeon.troll-cave".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 42, y: 58 },
                dungeon_id: "demo.dungeon.lonely-mountain".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 47, y: 53 },
                dungeon_id: "demo.dungeon.tidal-cave".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 49, y: 23 },
                dungeon_id: "demo.dungeon.mine".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 63, y: 44 },
                dungeon_id: "demo.dungeon.giants-hall".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 63, y: 53 },
                dungeon_id: "demo.dungeon.witch-wood".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 65, y: 44 },
                dungeon_id: "demo.dungeon.snow-castle".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 65, y: 54 },
                dungeon_id: "demo.dungeon.plains-of-oz".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 74, y: 28 },
                dungeon_id: "demo.dungeon.dragon-lair".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 75, y: 57 },
                dungeon_id: "demo.dungeon.battlefield".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 76, y: 46 },
                dungeon_id: "demo.dungeon.eyrie".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 85, y: 19 },
                dungeon_id: "demo.dungeon.graveyard".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 88, y: 34 },
                dungeon_id: "demo.dungeon.castle".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 94, y: 52 },
                dungeon_id: "demo.dungeon.chameleon-cave".to_owned(),
            },
        ]
    );
    let anambar = artifact
        .content
        .towns
        .iter()
        .find(|town| town.id == "demo.town.anambar")
        .expect("fixture should contain Anambar");
    assert_eq!(anambar.floor_id, "demo.floor.anambar");
    assert_eq!(anambar.shop_ids.len(), 10);
    let anambar_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == anambar.floor_id)
        .expect("Anambar should use a fixed town floor");
    assert_eq!(anambar_floor.lifecycle, FloorLifecycle::Town);
    assert_eq!((anambar_floor.width, anambar_floor.height), (23, 11));
    assert!(anambar_floor.inline_map.is_some());
    let anambar_home = artifact
        .content
        .town_facilities
        .iter()
        .find(|facility| facility.id == "demo.town-facility.anambar-home")
        .expect("Anambar should contain Home");
    assert_eq!(
        anambar_home.storage_id.as_deref(),
        Some("demo.town-facility.outpost-home")
    );
    let entrances = [
        (
            "demo.terrain.general-store-entrance",
            ContentPosition { x: 32, y: 13 },
        ),
        (
            "demo.terrain.temple-entrance",
            ContentPosition { x: 45, y: 19 },
        ),
        (
            "demo.terrain.alchemist-entrance",
            ContentPosition { x: 53, y: 13 },
        ),
        (
            "demo.terrain.magic-shop-entrance",
            ContentPosition { x: 57, y: 13 },
        ),
        (
            "demo.terrain.bookstore-entrance",
            ContentPosition { x: 55, y: 13 },
        ),
        (
            "demo.terrain.armoury-entrance",
            ContentPosition { x: 30, y: 19 },
        ),
        (
            "demo.terrain.weaponsmith-entrance",
            ContentPosition { x: 34, y: 19 },
        ),
        (
            "demo.terrain.black-market-entrance",
            ContentPosition { x: 55, y: 19 },
        ),
        (
            "demo.terrain.shroomery-entrance",
            ContentPosition { x: 61, y: 19 },
        ),
    ];
    for (terrain_id, entrance) in entrances {
        assert!(world.terrain_overrides.iter().any(|terrain| {
            terrain.terrain_id == terrain_id && terrain.positions == [entrance]
        }));
    }

    let fortifications = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.outpost-fortification")
        .expect("fixture should contain town fortifications");
    assert_eq!((world.width, world.height), (96, 32));
    assert_eq!(world.border_terrain_id, "demo.terrain.surface-grass");
    assert!(
        world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.warrens"))
            .all(|floor| (floor.width, floor.height) == (66, 22))
    );
    let expected_fortifications = (22..=66)
        .flat_map(|x| [ContentPosition { x, y: 6 }, ContentPosition { x, y: 25 }])
        .chain((7..=24).flat_map(|y| [ContentPosition { x: 22, y }, ContentPosition { x: 66, y }]))
        .filter(|position| position.y != 16)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fortifications
            .positions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected_fortifications,
        "the Outpost should have one continuous perimeter interrupted only by its gates"
    );
    let gates = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.outpost-gate")
        .expect("fixture should contain town gates");
    assert_eq!(
        gates.positions,
        [
            ContentPosition { x: 22, y: 16 },
            ContentPosition { x: 66, y: 16 }
        ]
    );
    assert!(entrances.iter().all(|(_, position)| {
        position.x > 22 && position.x < 66 && position.y > 6 && position.y < 25
    }));
    let warrens_entrance = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.stairs-down")
        .expect("fixture should contain the Warrens entrance");
    assert_eq!(
        warrens_entrance.positions,
        [ContentPosition { x: 74, y: 16 }]
    );
    assert!(warrens_entrance.positions[0].x > 66);

    let mut wrong_entrance = artifact.content.clone();
    wrong_entrance
        .shops
        .iter_mut()
        .find(|shop| shop.id == "demo.shop.outpost-general-store")
        .unwrap()
        .entrance_position = ContentPosition { x: 18, y: 8 };
    assert!(matches!(
        validate_and_normalize(&mut wrong_entrance),
        Err(ContentError::InvalidShop(id)) if id == "demo.shop.outpost-general-store"
    ));

    let mut unowned_shop = artifact.content.clone();
    unowned_shop
        .towns
        .iter_mut()
        .find(|town| town.id == "demo.town.outpost")
        .expect("fixture should contain Outpost")
        .shop_ids
        .retain(|shop_id| shop_id != "demo.shop.outpost-general-store");
    assert!(matches!(
        validate_and_normalize(&mut unowned_shop),
        Err(ContentError::InvalidShop(id)) if id == "demo.shop.outpost-general-store"
    ));

    let mut malformed_wilderness = artifact.content.clone();
    malformed_wilderness
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens")
        .wilderness
        .as_mut()
        .expect("fixture should contain wilderness")
        .rows[0]
        .pop();
    assert!(matches!(
        validate_and_normalize(&mut malformed_wilderness),
        Err(ContentError::InvalidWilderness(_))
    ));
}

#[test]
fn p89c_outpost_has_a_distinct_public_hideout_entrance() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let entrance = artifact
        .content
        .terrain
        .iter()
        .find(|terrain| terrain.id == "demo.terrain.hideout-entrance")
        .expect("Hideout entrance terrain should compile");
    assert_eq!(entrance.glyph, "<");
    assert!(entrance.tags.iter().any(|tag| tag == "stairs-down"));

    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should remain available");
    let hideout_entrance = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == entrance.id)
        .expect("Outpost should place the public Hideout entrance");
    assert_eq!(
        hideout_entrance.positions,
        [ContentPosition { x: 93, y: 29 }]
    );
    assert_eq!(
        world
            .terrain_overrides
            .iter()
            .find(|terrain| terrain.terrain_id == "demo.terrain.stairs-down")
            .expect("Warrens entrance should remain available")
            .positions,
        [ContentPosition { x: 74, y: 16 }]
    );
    let task_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("Thieves' Hideout task floor should remain available");
    assert_eq!(
        task_floor.task_id.as_deref(),
        Some("demo.task.thieves-hideout")
    );
    assert_ne!(
        task_floor.entry_terrain_id.as_deref(),
        Some(entrance.id.as_str())
    );
}

#[test]
fn thieves_hideout_uses_the_original_fixed_map_and_formation_contract() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.thieves-hideout")
        .expect("fixture should contain the thieves' hideout task");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-count")
    );
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.broad-sword"
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("fixture should contain the thieves' hideout floor");
    assert_eq!((floor.width, floor.height, floor.depth), (21, 8, 5));
    assert_eq!(
        floor.available_entry_terrain_id.as_deref(),
        Some("demo.terrain.thieves-hideout-entry-available")
    );
    let inline_map = floor
        .inline_map
        .as_ref()
        .expect("thieves' hideout should retain its inline map");
    assert_eq!(inline_map.loot_spawns.len(), 4);
    let formation = inline_map
        .monster_formation
        .as_ref()
        .expect("thieves' hideout should retain its formation");
    assert_eq!(formation.draw_count, 10);
    assert_eq!(formation.placement_indices, [0, 2, 4, 5, 7, 8]);
    assert_eq!(formation.positions.len(), 6);
    assert_eq!(formation.candidate_actor_kind_ids.len(), 7);

    let door_count = inline_map
        .terrain_overrides
        .iter()
        .find(|override_| override_.terrain_id == "demo.terrain.door-closed")
        .expect("fixed map should contain closed doors")
        .positions
        .len();
    let trap_count = inline_map
        .terrain_overrides
        .iter()
        .find(|override_| override_.terrain_id == "demo.terrain.warren-snare")
        .expect("fixed map should contain traps")
        .positions
        .len();
    assert_eq!((door_count, trap_count), (5, 8));
    let trap_override = inline_map
        .terrain_overrides
        .iter()
        .find(|override_| override_.terrain_id == "demo.terrain.warren-snare")
        .expect("fixed map should contain trap candidates");
    assert_eq!(trap_override.chance_percent, 50);
    assert_eq!(
        trap_override.otherwise_terrain_id.as_deref(),
        Some("demo.terrain.floor")
    );
    assert_eq!(floor.wall_terrain_id, "demo.terrain.permanent-wall");
}

#[test]
fn trouble_at_home_uses_the_original_map_targets_items_and_warrior_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Middle-earth");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.trouble-at-home")
        .expect("fixture should contain Trouble at Home");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-white-horse")
    );
    assert_eq!(task.objectives.len(), 1);
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::KillActorKind);
    assert_eq!(task.objectives[0].required, 5);
    assert_eq!(
        task.objectives[0].actor_kind_id.as_deref(),
        Some("demo.actor.mean-looking-mercenary")
    );
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.hard-studded-leather"
    );
    assert_eq!(
        task.reward.as_ref().unwrap().class_overrides[0].class_id,
        "demo.class.warrior"
    );
    assert_eq!(
        task.reward.as_ref().unwrap().class_overrides[0].entries[0].item_kind_id,
        "demo.item.set-of-studded-leather-gloves"
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.trouble-at-home")
        .expect("fixture should contain the White Horse back room");
    assert_eq!((floor.width, floor.height, floor.depth), (38, 17, 5));
    let inline_map = floor
        .inline_map
        .as_ref()
        .expect("Trouble at Home should retain its inline map");
    assert_eq!(inline_map.player_position, ContentPosition { x: 25, y: 15 });
    assert_eq!(inline_map.actor_spawns.len(), 12);
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .filter(|spawn| spawn.kind_id == "demo.actor.mean-looking-mercenary")
            .count(),
        5
    );
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .filter(|spawn| spawn.kind_id == "demo.actor.singing-happy-drunk")
            .count(),
        7
    );
    for actor_id in [
        "demo.actor.mean-looking-mercenary",
        "demo.actor.singing-happy-drunk",
    ] {
        assert_eq!(
            artifact
                .content
                .actors
                .iter()
                .find(|actor| actor.id == actor_id)
                .unwrap_or_else(|| panic!("{actor_id} should be imported"))
                .level,
            0,
            "{actor_id} should retain its source level"
        );
    }
    assert_eq!(inline_map.item_spawns.len(), 4);
    assert!(inline_map.item_spawns.iter().all(|spawn| {
        spawn.kind_id == "demo.item.piece-of-elvish-waybread" && spawn.quantity == 1
    }));
    let pair = inline_map
        .scrambled_item_pair
        .as_ref()
        .expect("boldness and booze should retain their two-position scramble");
    assert_eq!(
        pair.iter()
            .map(|spawn| spawn.kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["demo.item.boldness-potion", "demo.item.booze-potion"])
    );
    assert_eq!(inline_map.loot_spawns.len(), 3);
    assert_eq!(
        inline_map
            .terrain_overrides
            .iter()
            .find(|override_| override_.terrain_id == "demo.terrain.door-closed")
            .expect("fixed map should contain closed doors")
            .positions
            .len(),
        9
    );

    let formation = inline_map
        .monster_formation
        .as_ref()
        .expect("the original random monster cell should be retained");
    assert_eq!(formation.draw_count, 1);
    assert_eq!(formation.placement_indices, [0]);
    assert_eq!(formation.positions, [ContentPosition { x: 6, y: 10 }]);
    let expected_candidates = artifact
        .content
        .actors
        .iter()
        .filter(|actor| {
            let Some(allocation) = &actor.allocation else {
                return false;
            };
            let has_tag = |tag| actor.tags.iter().any(|candidate| candidate == tag);
            let aquatic = actor.movement.modes.contains(&ActorMovementMode::Aquatic);
            let flies = actor.movement.modes.contains(&ActorMovementMode::Fly);
            actor.role == ActorRole::Monster
                && actor.level <= 10
                && !has_tag("unique")
                && !has_tag("guardian")
                && !has_tag("no-quest")
                && (!has_tag("outpost-quest") || has_tag("trouble-at-home"))
                && !allocation.wild_only
                && (allocation.max_depth == 0 || allocation.max_depth >= 10)
                && (!allocation.force_depth || actor.level <= 5)
                && allocation
                    .task_id
                    .as_deref()
                    .is_none_or(|task_id| task_id == "demo.task.trouble-at-home")
                && allocation.legacy_dungeon_indices.is_empty()
                && (!aquatic || flies)
        })
        .map(|actor| actor.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_candidates.len(), 161);
    assert_eq!(
        formation
            .candidate_actor_kind_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected_candidates
    );

    let booze = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.booze-potion")
        .expect("the original booze potion should be imported");
    assert_eq!((booze.generation_level, booze.weight_tenths_pound), (0, 4));
    assert_eq!(booze.base_value, 1);
    assert!(matches!(
        booze.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::ApplyBooze)
    ));
}

#[test]
fn crows_nest_uses_the_original_map_clear_goal_birds_scramble_and_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Middle-earth");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.crows-nest")
        .expect("fixture should contain Crow's Nest");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-white-horse")
    );
    assert_eq!(
        task.prerequisite_task_id.as_deref(),
        Some("demo.task.trouble-at-home")
    );
    assert_eq!(task.objectives.len(), 1);
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::ClearFloor);
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.enlightenment-staff"
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.crows-nest")
        .expect("fixture should contain Crow's Nest");
    assert_eq!((floor.width, floor.height, floor.depth), (38, 17, 15));
    let inline_map = floor
        .inline_map
        .as_ref()
        .expect("Crow's Nest should retain its inline map");
    assert_eq!(inline_map.player_position, ContentPosition { x: 2, y: 14 });
    assert_eq!(inline_map.actor_spawns.len(), 9);
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .map(|spawn| spawn.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            }),
        BTreeMap::from([
            ("demo.actor.carrion", 1),
            ("demo.actor.crow", 6),
            ("demo.actor.crow-of-durthang", 2),
        ])
    );
    assert_eq!(inline_map.item_spawns.len(), 5);
    assert!(
        inline_map
            .item_spawns
            .iter()
            .all(|spawn| { spawn.kind_id == "demo.item.human-skeleton" && spawn.quantity == 1 })
    );
    let pair = inline_map
        .scrambled_item_loot_pair
        .as_ref()
        .expect("skeleton and random-loot glyphs should retain their group scramble");
    assert_eq!((pair.item_spawns.len(), pair.loot_spawns.len()), (10, 10));
    assert!(
        pair.item_spawns
            .iter()
            .all(|spawn| spawn.kind_id == "demo.item.human-skeleton")
    );
    assert!(
        pair.loot_spawns
            .iter()
            .all(|spawn| spawn.loot_table_id == "demo.loot-table.base-items")
    );
    let terrain_counts = inline_map
        .terrain_overrides
        .iter()
        .map(|override_| (override_.terrain_id.as_str(), override_.positions.len()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(terrain_counts["demo.terrain.floor"], 220);
    assert_eq!(terrain_counts["demo.terrain.dirt"], 42);
    assert_eq!(terrain_counts["demo.terrain.stairs-up"], 1);

    let staff = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.enlightenment-staff")
        .expect("the fixed quest reward should exist");
    let activation = &staff
        .device_generation
        .as_ref()
        .expect("the reward should be a device")
        .activations[0];
    assert_eq!(activation.device_check_difficulty, 20);
    assert_eq!(
        (activation.charges.minimum, activation.charges.maximum),
        (60, 60)
    );
    assert_eq!(activation.charges.cost, 10);
}

#[test]
fn old_man_willow_uses_the_original_map_formation_target_and_elemental_ring() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Middle-earth");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.old-man-willow")
        .expect("fixture should contain Old Man Willow");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-white-horse")
    );
    assert_eq!(
        task.prerequisite_task_id.as_deref(),
        Some("demo.task.crows-nest")
    );
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::KillActorKind);
    assert_eq!(task.objectives[0].required, 1);
    assert_eq!(
        task.objectives[0].actor_kind_id.as_deref(),
        Some("demo.actor.old-man-willow")
    );
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.ring"
    );
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].affix_ids,
        ["rfb-legacy.affix.elemental-jewelry"]
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.old-man-willow")
        .expect("fixture should contain Old Man Willow's grove");
    assert_eq!((floor.width, floor.height, floor.depth), (31, 20, 22));
    let inline_map = floor
        .inline_map
        .as_ref()
        .expect("Old Man Willow should retain its inline map");
    assert_eq!(inline_map.player_position, ContentPosition { x: 1, y: 18 });
    assert_eq!(inline_map.actor_spawns.len(), 23);
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .map(|spawn| spawn.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            }),
        BTreeMap::from([
            ("demo.actor.huorn", 8),
            ("demo.actor.old-man-willow", 1),
            ("demo.actor.sabre-tooth-tiger", 2),
            ("demo.actor.sasquatch", 6),
            ("demo.actor.vorpal-bunny", 6),
        ])
    );
    let terrain_counts = inline_map
        .terrain_overrides
        .iter()
        .map(|override_| (override_.terrain_id.as_str(), override_.positions.len()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(terrain_counts["demo.terrain.permanent-wall"], 98);
    assert_eq!(terrain_counts["demo.terrain.surface-grass"], 271);
    assert_eq!(terrain_counts["demo.terrain.stairs-up"], 1);

    let willow = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.old-man-willow")
        .expect("Old Man Willow should be imported");
    assert_eq!((willow.level, willow.max_hp), (22, 529));
    assert!(willow.force_sleep);
    assert!(willow.movement.never_moves);
    assert_eq!(
        willow.monster_casting.as_ref().map(|casting| (
            casting.frequency_percent,
            casting.abilities[0].ability_id.as_str()
        )),
        Some((16, "rfb-legacy.ability.drag"))
    );
    assert_eq!(
        willow.resistances.get(&ActorDamageType::Fire),
        Some(&ActorResistanceLevel::Vulnerable)
    );

    let elemental = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.elemental-jewelry")
        .expect("the Elemental jewelry ego should exist");
    assert_eq!(elemental.generation_level, 22);
    assert_eq!(elemental.roll_groups.len(), 2);
    assert_eq!(
        elemental.roll_groups[0]
            .candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<u32>(),
        4
    );
    assert_eq!(
        elemental.roll_groups[1]
            .candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<u32>(),
        12
    );
}

#[test]
fn vapor_quest_uses_the_original_map_formation_jewelry_and_detection_rod() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Middle-earth");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.vapor-quest")
        .expect("fixture should contain the Vapor Quest");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-white-horse")
    );
    assert_eq!(
        task.prerequisite_task_id.as_deref(),
        Some("demo.task.old-man-willow")
    );
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::ClearFloor);
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.detection-rod"
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.vapor-quest")
        .expect("fixture should contain the Vapor Quest cellar");
    assert_eq!((floor.width, floor.height, floor.depth), (25, 22, 25));
    let inline_map = floor
        .inline_map
        .as_ref()
        .expect("Vapor Quest should retain its inline map");
    assert_eq!(inline_map.player_position, ContentPosition { x: 12, y: 20 });
    assert_eq!(inline_map.actor_spawns.len(), 18);
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .map(|spawn| spawn.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            }),
        BTreeMap::from([
            ("demo.actor.air-elemental", 6),
            ("demo.actor.gas-spore", 1),
            ("demo.actor.radiation-eye", 8),
            ("demo.actor.shimmering-vortex", 1),
            ("demo.actor.weird-fume", 2),
        ])
    );
    assert_eq!(
        inline_map
            .item_spawns
            .iter()
            .map(|spawn| spawn.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            }),
        BTreeMap::from([("demo.item.amulet", 6), ("demo.item.ring", 6)])
    );
    let terrain_counts = inline_map
        .terrain_overrides
        .iter()
        .map(|override_| (override_.terrain_id.as_str(), override_.positions.len()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(terrain_counts["demo.terrain.floor"], 207);
    assert_eq!(terrain_counts["demo.terrain.stairs-up"], 1);

    let rod = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.detection-rod")
        .expect("the fixed quest reward should exist");
    let activation = &rod
        .device_generation
        .as_ref()
        .expect("the reward should be a device")
        .activations[0];
    assert_eq!(activation.device_check_difficulty, 30);
    assert_eq!(
        (activation.charges.minimum, activation.charges.maximum),
        (45, 45)
    );
    assert_eq!(activation.charges.cost, 17);
    let ItemUseEffectDefinition::Sequence { effects } = &activation.effect else {
        panic!("Detection should retain its ordered five-part effect");
    };
    assert_eq!(effects.len(), 5);
    for (effect, subject, category, persistent) in [
        (
            &effects[0],
            AbilityDetectSubjectDefinition::Terrain,
            "trap",
            true,
        ),
        (
            &effects[1],
            AbilityDetectSubjectDefinition::Terrain,
            "passage",
            true,
        ),
        (
            &effects[2],
            AbilityDetectSubjectDefinition::Gold,
            "gold",
            false,
        ),
        (
            &effects[3],
            AbilityDetectSubjectDefinition::Item,
            "item",
            false,
        ),
    ] {
        assert!(matches!(
            effect,
            ItemUseEffectDefinition::Detect {
                subject: actual_subject,
                category: actual_category,
                radius: 30,
                persistent: actual_persistent,
                through_walls: true,
            } if *actual_subject == subject
                && actual_category == category
                && *actual_persistent == persistent
        ));
    }
    assert!(matches!(
        &effects[4],
        ItemUseEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Actor,
            category,
            radius: 30,
            persistent: false,
            through_walls: true,
        } if category == "any-monster"
    ));
}

#[test]
fn old_castle_uses_the_original_map_fixed_formation_and_warrior_artifact_pool() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Middle-earth");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.old-castle")
        .expect("fixture should contain the Old Castle");
    assert_eq!(
        task.source_facility_id.as_deref(),
        Some("demo.town-facility.outpost-white-horse")
    );
    assert_eq!(
        task.prerequisite_task_id.as_deref(),
        Some("demo.task.vapor-quest")
    );
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::ClearFloor);
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.crisdurian"
    );
    assert_eq!(task.reward.as_ref().unwrap().class_overrides.len(), 1);
    assert_eq!(
        task.reward.as_ref().unwrap().class_overrides[0]
            .entries
            .iter()
            .map(|entry| (entry.item_kind_id.as_str(), entry.weight))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([("demo.item.pain", 4), ("demo.item.slayer", 1)])
    );

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.old-castle")
        .expect("fixture should contain the Old Castle floor");
    assert_eq!((floor.width, floor.height, floor.depth), (71, 28, 50));
    let inline_map = floor.inline_map.as_ref().expect("map should stay inline");
    assert_eq!(inline_map.player_position, ContentPosition { x: 31, y: 1 });
    assert_eq!(inline_map.actor_spawns.len(), 68);
    assert_eq!(inline_map.item_spawns.len(), 13);
    assert_eq!(inline_map.loot_spawns.len(), 72);
    assert_eq!(
        inline_map
            .actor_spawns
            .iter()
            .map(|spawn| spawn.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            })["demo.actor.anti-paladin"],
        5
    );
    assert_eq!(
        inline_map
            .monster_formation
            .as_ref()
            .expect("seven source-random cells should use the formation roller")
            .positions
            .len(),
        7
    );

    for (id, dice, sides, to_hit, to_damage) in [
        ("demo.item.crisdurian", 4, 6, 23, 24),
        ("demo.item.slayer", 5, 6, 15, 15),
        ("demo.item.pain", 9, 7, 0, 30),
    ] {
        let item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .expect("Old Castle artifact should exist");
        let melee = item
            .melee_profile
            .as_ref()
            .expect("artifact should be a weapon");
        assert_eq!(
            (
                melee.damage_dice,
                melee.damage_sides,
                melee.to_hit,
                melee.to_damage
            ),
            (dice, sides, to_hit, to_damage)
        );
        assert!(item.tags.iter().any(|tag| tag == "artifact"));
    }
}

#[test]
fn inline_floor_items_reject_duplicate_or_blocked_placements() {
    fn trouble_inline(content: &mut CompiledContentV1) -> &mut InlineFloorMapDefinition {
        content
            .worlds
            .iter_mut()
            .find(|world| world.id == "demo.world.middle-earth")
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter_mut()
                    .find(|floor| floor.id == "demo.floor.trouble-at-home")
            })
            .and_then(|floor| floor.inline_map.as_mut())
            .expect("Trouble at Home should retain its inline map")
    }

    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut duplicate_id = artifact.content.clone();
    let inline_map = trouble_inline(&mut duplicate_id);
    inline_map.scrambled_item_pair.as_mut().unwrap()[0].instance_id =
        inline_map.item_spawns[0].instance_id.clone();
    assert!(matches!(
        validate_and_normalize(&mut duplicate_id),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_position = artifact.content.clone();
    let pair = trouble_inline(&mut duplicate_position)
        .scrambled_item_pair
        .as_mut()
        .unwrap();
    pair[1].position = pair[0].position;
    assert!(matches!(
        validate_and_normalize(&mut duplicate_position),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_position = artifact.content.clone();
    trouble_inline(&mut blocked_position).item_spawns[0].position = ContentPosition { x: 0, y: 0 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_position),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn scrambled_item_loot_pair_requires_equal_nonempty_disjoint_groups() {
    fn crows_pair(content: &mut CompiledContentV1) -> &mut InlineScrambledItemLootPairDefinition {
        content
            .worlds
            .iter_mut()
            .find(|world| world.id == "demo.world.middle-earth")
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter_mut()
                    .find(|floor| floor.id == "demo.floor.crows-nest")
            })
            .and_then(|floor| floor.inline_map.as_mut())
            .and_then(|inline_map| inline_map.scrambled_item_loot_pair.as_mut())
            .expect("Crow's Nest should retain its item/loot scramble")
    }

    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut mismatched = artifact.content.clone();
    crows_pair(&mut mismatched).loot_spawns.pop();
    assert!(matches!(
        validate_and_normalize(&mut mismatched),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut overlapping = artifact.content.clone();
    let pair = crows_pair(&mut overlapping);
    pair.loot_spawns[0].position = pair.item_spawns[0].position;
    assert!(matches!(
        validate_and_normalize(&mut overlapping),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn general_store_economy_content_enforces_generic_stock_rules() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let store_id = "demo.shop.outpost-general-store";

    let mutations: [fn(&mut CompiledContentV1); 4] = [
        |content: &mut CompiledContentV1| {
            content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .owner
                .greed_percent = 99;
        },
        |content: &mut CompiledContentV1| {
            let store = content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap();
            store.stock.push(store.stock[0].clone());
        },
        |content: &mut CompiledContentV1| {
            let shop = content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap();
            shop.stock[0].initial_minimum = shop.stock[0].initial_maximum + 1;
        },
        |content: &mut CompiledContentV1| {
            let mut item = content
                .shops
                .iter()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .stock[0]
                .item_kind_id
                .clone();
            let mut definition = content
                .items
                .iter()
                .find(|candidate| candidate.id == item)
                .expect("stock kind should exist")
                .clone();
            item = "demo.item.invalid-zero-value-stock".to_owned();
            definition.id = item.clone();
            definition.base_value = 0;
            content.items.push(definition);
            content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .stock[0]
                .item_kind_id = item;
        },
    ];
    for mutate in mutations {
        let mut invalid = artifact.content.clone();
        mutate(&mut invalid);
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidShop(id)) if id == store_id
        ));
    }

    let mut extended = artifact.content.clone();
    let template = extended
        .shops
        .iter()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .stock[0]
        .clone();
    let mut added = template;
    added.item_kind_id = "demo.item.leather-pouch".to_owned();
    extended
        .shops
        .iter_mut()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .stock
        .push(added);
    validate_and_normalize(&mut extended).expect("a valid new stock item should be data-only");

    let mut invalid_owner = artifact.content.clone();
    invalid_owner
        .shops
        .iter_mut()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .owner
        .race_id = "demo.race.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_owner),
        Err(ContentError::InvalidShop(id)) if id == store_id
    ));

    let mut invalid_value = artifact.content.clone();
    invalid_value
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.ration-of-food")
        .expect("ration should exist")
        .base_value = 1_000_000_000;
    assert!(matches!(
        validate_and_normalize(&mut invalid_value),
        Err(ContentError::InvalidItemValue(id)) if id == "demo.item.ration-of-food"
    ));

    let mut invalid_race_factor = artifact.content.clone();
    invalid_race_factor
        .races
        .iter_mut()
        .find(|race| race.id == "demo.race.rfb-human")
        .expect("Human race should exist")
        .shop_adjust_percent = 49;
    assert!(matches!(
        validate_and_normalize(&mut invalid_race_factor),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));
}

#[test]
fn temple_and_alchemist_stock_are_strictly_separated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let expected = [
        (
            "demo.shop.outpost-temple",
            BTreeSet::from([
                "demo.item.light-healing-potion",
                "demo.item.valor-tonic",
                "demo.item.homeward-scroll",
                "demo.item.cleansing-scroll",
                "demo.item.greater-cleansing-scroll",
                "demo.item.holy-prayer-scroll",
                "demo.item.boldness-potion",
                "demo.item.cure-serious-wounds-potion",
                "demo.item.cure-critical-wounds-potion",
            ]),
        ),
        (
            "demo.shop.outpost-alchemist",
            BTreeSet::from([
                "demo.item.flicker-scroll",
                "demo.item.farstep-scroll",
                "demo.item.seeking-scroll",
                "demo.item.trapfinding-scroll",
                "demo.item.treasure-detection-scroll",
                "demo.item.temperate-tonic",
                "demo.item.appraisal-scroll",
                "demo.item.revelation-scroll",
                "demo.item.recharging-scroll",
                "demo.item.cartography-scroll",
                "demo.item.door-stair-location-scroll",
                "demo.item.detect-invisible-scroll",
                "demo.item.confusing-touch-scroll",
                "demo.item.detect-monsters-scroll",
                "demo.item.fury-draught",
                "demo.item.renewal-tonic",
                "demo.item.strength-renewal-tonic",
                "demo.item.restore-intelligence-potion",
                "demo.item.restore-wisdom-potion",
                "demo.item.restore-dexterity-potion",
                "demo.item.restore-constitution-potion",
                "demo.item.restore-charisma-potion",
                "demo.item.sight-potion",
                "demo.item.antidote-potion",
                "demo.item.curing-potion",
                "demo.item.light-scroll",
                "demo.item.rumour-scroll",
            ]),
        ),
    ];
    for (shop_id, item_ids) in expected {
        let shop = artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == shop_id)
            .expect("shop should exist");
        assert_eq!(
            shop.stock
                .iter()
                .map(|stock| stock.item_kind_id.as_str())
                .collect::<BTreeSet<_>>(),
            item_ids
        );
    }
}

#[test]
fn supported_legacy_consumables_use_their_source_allocations() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let base_items = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");
    let expected = [
        ("demo.item.seeking-scroll", 0, 30),
        ("demo.item.veil-draught", 0, 20),
        ("demo.item.light-healing-potion", 0, 20),
        ("demo.item.summoning-scroll", 1, 30),
        ("demo.item.flicker-scroll", 1, 50),
        ("demo.item.appraisal-scroll", 1, u16::MAX),
        ("demo.item.detect-invisible-scroll", 1, 30),
        ("demo.item.slowness-potion", 1, 20),
        ("demo.item.boldness-potion", 1, u16::MAX),
        ("demo.item.swiftstep-tonic", 1, u16::MAX),
        ("demo.item.temperate-tonic", 1, 60),
        ("demo.item.vigor-potion", 15, u16::MAX),
        ("demo.item.valor-tonic", 1, u16::MAX),
        ("demo.item.venom-draught", 3, 30),
        ("demo.item.frailty-tonic", 3, 30),
        ("demo.item.clamor-scroll", 5, 30),
        ("demo.item.cartography-scroll", 5, 50),
        ("demo.item.trapfinding-scroll", 5, 30),
        ("demo.item.door-stair-location-scroll", 5, 30),
        ("demo.item.confusing-touch-scroll", 5, 40),
        ("demo.item.clumsiness-potion", 5, 20),
        ("demo.item.hallucination-mushroom", 10, 30),
        ("demo.item.weakness-mushroom", 10, 30),
        ("demo.item.sickness-mushroom", 10, 30),
        ("demo.item.lose-memories-potion", 10, 30),
        ("demo.item.stupidity-mushroom", 15, 30),
        ("demo.item.naivety-mushroom", 15, 30),
        ("demo.item.paralysis-mushroom", 20, 40),
        ("demo.item.ruination-potion", 40, 80),
    ];
    for (item_id, min_depth, max_depth) in expected {
        let entry = base_items
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == item_id && entry.min_depth == min_depth)
            .unwrap_or_else(|| panic!("{item_id} should have its source allocation"));
        assert_eq!(entry.max_depth, max_depth, "{item_id}");
    }
    assert!(!base_items.entries.iter().any(|entry| {
        matches!(
            entry.item_kind_id.as_str(),
            "demo.item.benediction-scroll" | "demo.item.satisfy-hunger-scroll"
        )
    }));
    assert_eq!(
        base_items
            .entries
            .iter()
            .filter(|entry| entry.item_kind_id == "demo.item.star-healing-potion")
            .map(|entry| (entry.min_depth, entry.weight))
            .collect::<Vec<_>>(),
        vec![(40, 12), (60, 25), (80, 50)]
    );
}

#[test]
fn base_item_pool_is_shared_without_absorbing_fixed_rewards() {
    let pack_path = original_pack_path();
    let artifact = compile_pack_dir(&pack_path).expect("original pack should compile");
    let base_items = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");

    assert_eq!(base_items.entries.len(), 342);

    let selection: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack_path.join("legacy-item-selection.json"))
            .expect("item selection should be readable"),
    )
    .expect("item selection should be valid JSON");
    let adaptations: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack_path.join("legacy-item-adaptations.json"))
            .expect("item adaptations should be readable"),
    )
    .expect("item adaptations should be valid JSON");
    let mut active_source_items = BTreeMap::new();
    for item in selection["items"]
        .as_array()
        .expect("item selection should contain items")
    {
        active_source_items.insert(
            item["sourceIndex"]
                .as_u64()
                .expect("selected item should have a source index"),
            format!(
                "demo.item.{}",
                item["id"]
                    .as_str()
                    .expect("selected item should have an id")
            ),
        );
    }
    for item in adaptations["items"]
        .as_array()
        .expect("item adaptations should contain items")
        .iter()
        .filter(|item| item["status"] == "active")
    {
        active_source_items
            .entry(
                item["sourceIndex"]
                    .as_u64()
                    .expect("adapted item should have a source index"),
            )
            .or_insert_with(|| {
                item["itemId"]
                    .as_str()
                    .expect("adapted item should have an item id")
                    .to_owned()
            });
    }
    assert_eq!(active_source_items.len(), 318);

    let source_items_without_allocations =
        BTreeSet::from([33, 34, 36, 37, 345, 346, 347, 400, 401, 460]);
    let expected_item_ids = active_source_items
        .iter()
        .filter(|(source_index, _)| !source_items_without_allocations.contains(source_index))
        .map(|(_, item_id)| item_id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_item_ids = base_items
        .entries
        .iter()
        .map(|entry| entry.item_kind_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_item_ids.len(), 308);
    assert_eq!(actual_item_ids, expected_item_ids);

    // Source 313 is one Staff allocation split into two formal adaptations.
    assert!(actual_item_ids.contains("demo.item.detect-objects-staff"));
    assert!(!actual_item_ids.contains("demo.item.identify-staff"));
    assert_eq!(
        base_items.quality_policy,
        Some(LootQualityPolicyDefinition::RfbDepth {
            good_cap_percent: 75,
            great_cap_percent: 20,
        })
    );
    assert_eq!(
        base_items
            .affix_weights
            .iter()
            .map(|affix| (affix.affix_id.as_deref(), affix.weight))
            .collect::<Vec<_>>(),
        vec![
            (None, 9),
            (Some("rfb-legacy.affix.protection"), 1),
            (Some("rfb-legacy.affix.slaying"), 1),
        ]
    );
    assert!(!artifact.content.loot_tables.iter().any(|table| {
        matches!(
            table.id.as_str(),
            "demo.loot-table.warrens" | "demo.loot-table.orc-cave"
        )
    }));
    assert!(
        artifact
            .content
            .loot_tables
            .iter()
            .any(|table| table.id == "demo.loot-table.warrens-final-reward")
    );
    assert!(
        artifact
            .content
            .loot_tables
            .iter()
            .any(|table| table.id == "demo.loot-table.orc-cave-final-reward")
    );

    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    for (dungeon_id, expected_floor_count) in
        [("demo.dungeon.warrens", 9), ("demo.dungeon.orc-cave", 18)]
    {
        let floors = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        assert_eq!(floors.len(), expected_floor_count, "{dungeon_id}");
        assert!(
            floors.iter().all(|floor| {
                floor.loot_table_id.as_deref() == Some("demo.loot-table.base-items")
            })
        );
    }
    let hideout = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("Thieves Hideout should exist");
    let hideout_loot = &hideout
        .inline_map
        .as_ref()
        .expect("Thieves Hideout should keep its inline map")
        .loot_spawns;
    assert_eq!(hideout_loot.len(), 4);
    assert!(
        hideout_loot
            .iter()
            .all(|spawn| spawn.loot_table_id == "demo.loot-table.base-items")
    );

    assert_eq!(
        artifact
            .content
            .actors
            .iter()
            .filter(|actor| {
                actor
                    .death_drop
                    .as_ref()
                    .and_then(|drop| drop.item_table_id.as_deref())
                    == Some("demo.loot-table.base-items")
            })
            .count(),
        797
    );
}

#[test]
fn formal_drop_themes_use_source_allocations_and_rfb_depth_quality() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let policy = Some(LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent: 75,
        great_cap_percent: 20,
    });

    for (table_id, expected_entries) in [
        ("demo.loot-table.warrior", 59),
        ("demo.loot-table.archer", 13),
        ("demo.loot-table.mage", 53),
        ("demo.loot-table.priest", 39),
        ("demo.loot-table.evil-priest", 18),
        ("demo.loot-table.paladin", 73),
        ("demo.loot-table.dwarf", 6),
        ("demo.loot-table.ninja", 3),
        ("demo.loot-table.hobbit", 32),
    ] {
        let table = artifact
            .content
            .loot_tables
            .iter()
            .find(|table| table.id == table_id)
            .unwrap_or_else(|| panic!("{table_id} should exist"));
        assert_eq!(table.entries.len(), expected_entries, "{table_id}");
        assert_eq!(table.quality_policy, policy, "{table_id}");
        assert!(table.quality_weights.is_empty(), "{table_id}");
        assert!(
            table
                .entries
                .iter()
                .all(|entry| !matches!(entry.max_depth, 9 | 32)),
            "{table_id} should not retain a dungeon depth cap"
        );
    }

    for table_id in [
        "demo.loot-table.warrior",
        "demo.loot-table.paladin",
        "demo.loot-table.dwarf",
        "demo.loot-table.mage",
    ] {
        let table = artifact
            .content
            .loot_tables
            .iter()
            .find(|table| table.id == table_id)
            .expect("Protection theme table should exist");
        let expected = if table_id == "demo.loot-table.warrior" {
            vec![
                (None, 9),
                (Some("rfb-legacy.affix.protection"), 1),
                (Some("rfb-legacy.affix.slaying"), 1),
            ]
        } else {
            vec![(None, 9), (Some("rfb-legacy.affix.protection"), 1)]
        };
        assert_eq!(
            table
                .affix_weights
                .iter()
                .map(|entry| (entry.affix_id.as_deref(), entry.weight))
                .collect::<Vec<_>>(),
            expected,
            "{table_id}"
        );
    }

    let retired = [
        "demo.loot-table.large-kobold",
        "demo.loot-table.small-kobold",
        "demo.loot-table.kobold",
        "demo.loot-table.warrens-keeper",
        "demo.loot-table.orc-cave-warrior",
    ];
    assert!(
        artifact
            .content
            .loot_tables
            .iter()
            .all(|table| !retired.contains(&table.id.as_str()))
    );
    let warrior_drops = artifact
        .content
        .actors
        .iter()
        .filter_map(|actor| {
            actor
                .death_drop
                .as_ref()
                .filter(|drop| drop.theme_table_id.as_deref() == Some("demo.loot-table.warrior"))
        })
        .collect::<Vec<_>>();
    assert_eq!(warrior_drops.len(), 100);
    assert!(
        warrior_drops
            .iter()
            .all(|drop| drop.theme_chance_percent == 50)
    );

    let warrior = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrior")
        .expect("Warrior drop table should exist");
    assert!(warrior.entries.iter().any(|entry| {
        entry.item_kind_id == "demo.item.bastard-sword"
            && entry.weight == 100
            && entry.min_depth == 15
            && entry.max_depth == u16::MAX
    }));
    assert_eq!(
        warrior
            .entries
            .iter()
            .filter(|entry| entry.item_kind_id == "demo.item.pointy-hat")
            .map(|entry| (entry.min_depth, entry.weight))
            .collect::<Vec<_>>(),
        vec![(10, 20), (40, 33)]
    );

    let hobbit = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.hobbit")
        .expect("Hobbit drop table should exist");
    assert_eq!(
        hobbit
            .entries
            .iter()
            .map(|entry| entry.item_kind_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        26
    );
    assert_eq!(
        hobbit
            .affix_weights
            .iter()
            .map(|entry| (entry.affix_id.as_deref(), entry.weight))
            .collect::<Vec<_>>(),
        vec![(None, 1)]
    );
    assert_eq!(
        hobbit
            .entries
            .iter()
            .filter(|entry| entry.item_kind_id == "demo.item.sixfold-provision")
            .map(|entry| (entry.min_depth, entry.weight))
            .collect::<Vec<_>>(),
        vec![(20, 12), (30, 25), (40, 100)]
    );
    assert_eq!(
        hobbit
            .entries
            .iter()
            .filter(|entry| entry.item_kind_id == "demo.item.ration-of-food")
            .map(|entry| (entry.min_depth, entry.weight))
            .collect::<Vec<_>>(),
        vec![(0, 100), (5, 100), (10, 100), (20, 100)]
    );
    for excluded in [
        "demo.item.hard-biscuit",
        "demo.item.strip-of-venison",
        "demo.item.pint-of-fine-ale",
        "demo.item.pint-of-fine-wine",
        "demo.item.iron-shot",
        "demo.item.mithril-shot",
    ] {
        assert!(
            hobbit
                .entries
                .iter()
                .all(|entry| entry.item_kind_id != excluded),
            "{excluded} should not be in the Hobbit theme"
        );
    }

    let scruffy = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.scruffy-looking-hobbit")
        .expect("Scruffy looking hobbit should exist");
    let drop = scruffy
        .death_drop
        .as_ref()
        .expect("Scruffy looking hobbit should drop items");
    assert_eq!(drop.kind, MonsterDropKindDefinition::Items);
    assert_eq!(
        drop.item_table_id.as_deref(),
        Some("demo.loot-table.base-items")
    );
    assert_eq!(
        drop.theme_table_id.as_deref(),
        Some("demo.loot-table.hobbit")
    );
    assert_eq!(drop.theme_chance_percent, 50);
    assert_eq!(
        drop.count_dice,
        vec![MonsterDropDiceDefinition { dice: 1, sides: 2 }]
    );
    assert!(scruffy.terrain_interaction.picks_up_items);
    assert!(
        scruffy
            .melee_routine
            .as_ref()
            .is_some_and(|routine| routine.blows.iter().any(|blow| blow
                .effects
                .iter()
                .any(|effect| matches!(effect, MeleeBlowEffectDefinition::EatGold { .. }))))
    );
}

#[test]
fn base_pool_and_global_warrior_theme_use_source_depths() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let normal = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");
    let warrior = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrior")
        .expect("Warrior loot should exist");

    for (item_id, min_depth, warrior_item) in [
        ("demo.item.leather-scale-mail", 15, false),
        ("demo.item.jingasa", 16, true),
        ("demo.item.pair-of-metal-shod-boots", 20, true),
        ("demo.item.iron-helm", 20, true),
        ("demo.item.set-of-spiked-gauntlets", 20, true),
        ("demo.item.halberd", 25, true),
        ("demo.item.orcish-pick", 30, false),
        ("demo.item.elven-cloak", 30, false),
        ("demo.item.large-metal-shield", 30, true),
        ("demo.item.augmented-chain-mail", 30, true),
    ] {
        let entry = normal
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == item_id)
            .unwrap_or_else(|| panic!("{item_id} should be in the base item pool"));
        assert_eq!((entry.min_depth, entry.max_depth), (min_depth, u16::MAX));

        let warrior_entry = warrior
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == item_id);
        assert_eq!(warrior_entry.is_some(), warrior_item, "{item_id}");
        if let Some(entry) = warrior_entry {
            assert_eq!((entry.min_depth, entry.max_depth), (min_depth, u16::MAX));
        }
    }
}

#[test]
fn low_mid_equipment_uses_source_allocations_and_theme_predicates() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let table = |id: &str| {
        artifact
            .content
            .loot_tables
            .iter()
            .find(|table| table.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    };
    let base_items = table("demo.loot-table.base-items");
    let expected_allocations = [
        ("demo.item.trident", 5, 100),
        ("demo.item.fauchard", 18, 50),
        ("demo.item.broad-spear", 14, 100),
        ("demo.item.pike", 15, 100),
        ("demo.item.beaked-axe", 15, 100),
        ("demo.item.broad-axe", 15, 100),
        ("demo.item.glaive", 20, 100),
        ("demo.item.heavy-lance", 43, 25),
        ("demo.item.lance", 10, 100),
        ("demo.item.battle-axe", 15, 100),
        ("demo.item.nunchaku", 16, 50),
        ("demo.item.ball-and-chain", 20, 100),
        ("demo.item.jo-staff", 11, 50),
        ("demo.item.war-hammer", 5, 100),
        ("demo.item.three-piece-rod", 20, 33),
        ("demo.item.flail", 10, 100),
        ("demo.item.bo-staff", 20, 100),
        ("demo.item.lead-filled-mace", 15, 100),
        ("demo.item.gnomish-shovel", 20, 25),
        ("demo.item.rhino-hide-armour", 15, 100),
        ("demo.item.leather-jacket", 20, 33),
        ("demo.item.ring-mail", 20, 100),
    ];

    for (item_id, min_depth, weight) in expected_allocations {
        let allocations = base_items
            .entries
            .iter()
            .filter(|entry| entry.item_kind_id == item_id)
            .map(|entry| (entry.min_depth, entry.max_depth, entry.weight))
            .collect::<Vec<_>>();
        assert_eq!(
            allocations,
            vec![(min_depth, u16::MAX, weight)],
            "{item_id}"
        );
    }

    let new_item_ids = expected_allocations
        .iter()
        .map(|(item_id, _, _)| *item_id)
        .collect::<BTreeSet<_>>();
    for (table_id, expected_item_ids) in [
        (
            "demo.loot-table.warrior",
            &[
                "demo.item.battle-axe",
                "demo.item.beaked-axe",
                "demo.item.broad-axe",
                "demo.item.broad-spear",
                "demo.item.fauchard",
                "demo.item.glaive",
                "demo.item.lance",
                "demo.item.pike",
                "demo.item.ring-mail",
                "demo.item.trident",
            ][..],
        ),
        (
            "demo.loot-table.paladin",
            &[
                "demo.item.battle-axe",
                "demo.item.beaked-axe",
                "demo.item.broad-axe",
                "demo.item.broad-spear",
                "demo.item.fauchard",
                "demo.item.glaive",
                "demo.item.lance",
                "demo.item.pike",
                "demo.item.ring-mail",
                "demo.item.trident",
            ][..],
        ),
        (
            "demo.loot-table.priest",
            &[
                "demo.item.ball-and-chain",
                "demo.item.bo-staff",
                "demo.item.flail",
                "demo.item.jo-staff",
                "demo.item.lead-filled-mace",
                "demo.item.nunchaku",
                "demo.item.three-piece-rod",
                "demo.item.war-hammer",
            ][..],
        ),
        (
            "demo.loot-table.evil-priest",
            &[
                "demo.item.ball-and-chain",
                "demo.item.bo-staff",
                "demo.item.flail",
                "demo.item.jo-staff",
                "demo.item.lead-filled-mace",
                "demo.item.nunchaku",
                "demo.item.three-piece-rod",
                "demo.item.war-hammer",
            ][..],
        ),
        (
            "demo.loot-table.dwarf",
            &[
                "demo.item.battle-axe",
                "demo.item.beaked-axe",
                "demo.item.broad-axe",
            ][..],
        ),
    ] {
        let actual_item_ids = table(table_id)
            .entries
            .iter()
            .map(|entry| entry.item_kind_id.as_str())
            .filter(|item_id| new_item_ids.contains(item_id))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_item_ids,
            expected_item_ids.iter().copied().collect(),
            "{table_id}"
        );
    }
}

#[test]
fn selected_legacy_equipment_uses_its_shop_and_source_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_stock = |id: &str| {
        artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == id)
            .expect("shop should exist")
            .stock
            .iter()
            .map(|entry| entry.item_kind_id.as_str())
            .collect::<BTreeSet<_>>()
    };
    assert!(
        BTreeSet::from([
            "demo.item.club",
            "demo.item.dagger",
            "demo.item.main-gauche",
            "demo.item.tanto",
            "demo.item.whip",
            "demo.item.rapier",
            "demo.item.small-sword",
            "demo.item.cutlass",
            "demo.item.mace",
            "demo.item.khopesh",
            "demo.item.scimitar",
            "demo.item.hatchet",
            "demo.item.sickle",
            "demo.item.awl-pike",
            "demo.item.lucerne-hammer",
            "demo.item.quarterstaff",
            "demo.item.morning-star",
            "demo.item.shovel",
            "demo.item.pick",
        ])
        .is_subset(&shop_stock("demo.shop.outpost-weaponsmith"))
    );
    assert!(
        BTreeSet::from([
            "demo.item.cloak",
            "demo.item.robe",
            "demo.item.padded-armour",
            "demo.item.knit-cap",
            "demo.item.soft-leather-armour",
            "demo.item.soft-studded-leather",
            "demo.item.hard-leather-armour",
            "demo.item.hard-studded-leather",
            "demo.item.pair-of-hard-leather-boots",
            "demo.item.cord-armour",
            "demo.item.metal-cap",
            "demo.item.small-metal-shield",
            "demo.item.large-leather-shield",
            "demo.item.set-of-studded-leather-gloves",
            "demo.item.set-of-gauntlets",
        ])
        .is_subset(&shop_stock("demo.shop.outpost-armoury"))
    );

    let base_items = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");
    let depth = |id: &str| {
        base_items
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == id)
            .map(|entry| (entry.min_depth, entry.max_depth))
    };
    assert_eq!(depth("demo.item.club"), Some((0, u16::MAX)));
    assert_eq!(depth("demo.item.broken-dagger"), Some((0, u16::MAX)));
    assert_eq!(depth("demo.item.broken-sword"), Some((0, u16::MAX)));
    assert_eq!(depth("demo.item.dagger"), Some((0, u16::MAX)));
    assert_eq!(depth("demo.item.filthy-rag"), Some((0, u16::MAX)));
    assert_eq!(depth("demo.item.cloak"), Some((1, u16::MAX)));
    assert_eq!(depth("demo.item.robe"), Some((1, u16::MAX)));
    assert_eq!(depth("demo.item.shovel"), Some((5, 10)));
    assert_eq!(depth("demo.item.padded-armour"), Some((5, u16::MAX)));
    assert_eq!(depth("demo.item.knit-cap"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.main-gauche"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.pointy-hat"), Some((10, u16::MAX)));
    assert_eq!(depth("demo.item.soft-leather-armour"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.soft-studded-leather"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.tanto"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.whip"), Some((3, u16::MAX)));
    assert_eq!(depth("demo.item.cord-armour"), Some((7, u16::MAX)));
    assert_eq!(depth("demo.item.cutlass"), Some((5, u16::MAX)));
    assert_eq!(depth("demo.item.hard-leather-armour"), Some((5, u16::MAX)));
    assert_eq!(depth("demo.item.rapier"), Some((5, u16::MAX)));
    assert_eq!(depth("demo.item.mace"), Some((5, u16::MAX)));
    assert_eq!(
        depth("demo.item.pair-of-hard-leather-boots"),
        Some((5, u16::MAX))
    );
    assert_eq!(depth("demo.item.paper-armour"), Some((7, u16::MAX)));
    assert_eq!(depth("demo.item.pick"), Some((10, 30)));
    assert_eq!(depth("demo.item.small-sword"), Some((5, u16::MAX)));
    assert_eq!(
        depth("demo.item.set-of-studded-leather-gloves"),
        Some((5, u16::MAX))
    );
    assert_eq!(depth("demo.item.metal-cap"), Some((10, u16::MAX)));
    assert_eq!(depth("demo.item.small-metal-shield"), Some((10, u16::MAX)));
    assert_eq!(
        depth("demo.item.large-leather-shield"),
        Some((15, u16::MAX))
    );
    assert_eq!(
        depth("demo.item.hard-studded-leather"),
        Some((10, u16::MAX))
    );
    assert_eq!(depth("demo.item.set-of-gauntlets"), Some((10, u16::MAX)));
}

#[test]
fn bookstore_stocks_original_town_books() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_id = "demo.shop.outpost-bookstore";
    let shop = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("bookstore should exist");
    assert_eq!(shop.category, ShopCategory::Bookstore);
    assert_eq!(
        shop.stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "demo.item.beginners-handbook",
            "demo.item.black-mass",
            "demo.item.black-prayers",
            "demo.item.book-of-common-prayer",
            "demo.item.book-of-elements",
            "demo.item.call-of-the-wild",
            "demo.item.cantrips-for-beginners",
            "demo.item.dark-incantations",
            "demo.item.earth-wind-and-fire",
            "demo.item.high-mass",
            "demo.item.immortal-rituals",
            "demo.item.major-arcana",
            "demo.item.manual-of-mastery",
            "demo.item.master-sorcerers-handbook",
            "demo.item.minor-arcana",
            "demo.item.nature-mastery",
            "demo.item.rites-of-initiation",
            "demo.item.ways-of-war",
        ])
    );
    let values = artifact
        .content
        .items
        .iter()
        .filter(|item| shop.stock.iter().any(|stock| stock.item_kind_id == item.id))
        .map(|item| (item.id.as_str(), item.base_value))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values["demo.item.black-prayers"], 100);
    assert_eq!(values["demo.item.black-mass"], 1_000);
    assert_eq!(values["demo.item.book-of-elements"], 100);
    assert_eq!(values["demo.item.beginners-handbook"], 100);
    assert_eq!(values["demo.item.master-sorcerers-handbook"], 1_000);
    assert_eq!(values["demo.item.cantrips-for-beginners"], 100);
    assert_eq!(values["demo.item.earth-wind-and-fire"], 1_000);
    assert_eq!(values["demo.item.minor-arcana"], 250);
    assert_eq!(values["demo.item.manual-of-mastery"], 2_500);
    assert_eq!(values["demo.item.rites-of-initiation"], 100);
}

#[test]
fn black_market_stocks_original_non_town_books_and_priced_p3_consumables() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_id = "demo.shop.outpost-black-market";
    let shop = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("Black Market should exist");
    assert_eq!(shop.category, ShopCategory::BlackMarket);
    assert_eq!(shop.owner.greed_percent, 150);
    assert_eq!(shop.owner.purchase_price_cap, 30_000);
    assert_eq!(
        shop.stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "demo.item.black-channels",
            "demo.item.blessings-of-the-grail",
            "demo.item.book-of-the-unicorn",
            "demo.item.day-of-ragnarok",
            "demo.item.demonthoughts",
            "demo.item.disease-mushroom",
            "demo.item.necronomicon",
            "demo.item.natures-gifts",
            "demo.item.natures-wrath",
            "demo.item.path-of-destruction",
            "demo.item.pattern-sorcery",
            "demo.item.restore-constitution-mushroom",
            "demo.item.restore-strength-mushroom",
            "demo.item.unhealth-mushroom",
            "demo.item.invulnerability-potion",
            "demo.item.giant-strength-potion",
            "demo.item.great-clarity-potion",
            "demo.item.grimoire-of-power",
            "demo.item.hellfire-tome",
            "demo.item.wrath-of-god",
            "demo.item.understanding-scroll",
            "demo.item.inventory-protection-scroll",
            "demo.item.enlightenment-potion",
            "demo.item.exorcism-and-dispelling",
            "demo.item.star-enlightenment-potion",
            "demo.item.self-knowledge-potion",
            "demo.item.experience-potion",
            "demo.item.neo-tsuyoshi-special",
            "demo.item.rune-of-protection-scroll",
            "demo.item.destruction-scroll",
            "demo.item.mundanity-scroll",
            "demo.item.acquirement-scroll",
            "demo.item.star-acquirement-scroll",
            "demo.item.crafting-scroll",
            "demo.item.new-life-potion",
            "demo.item.polymorph-potion",
        ])
    );
    let values = artifact
        .content
        .items
        .iter()
        .filter(|item| shop.stock.iter().any(|stock| stock.item_kind_id == item.id))
        .map(|item| (item.id.as_str(), item.base_value))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values["demo.item.black-channels"], 15_000);
    assert_eq!(values["demo.item.necronomicon"], 100_000);
    assert_eq!(values["demo.item.day-of-ragnarok"], 100_000);
    assert_eq!(values["demo.item.path-of-destruction"], 15_000);
    assert_eq!(values["demo.item.unhealth-mushroom"], 50);
    assert_eq!(values["demo.item.disease-mushroom"], 50);
    assert_eq!(values["demo.item.restore-constitution-mushroom"], 350);
    assert_eq!(values["demo.item.restore-strength-mushroom"], 350);
    assert_eq!(values["demo.item.invulnerability-potion"], 100_000);
    assert_eq!(values["demo.item.giant-strength-potion"], 10_000);
    assert_eq!(values["demo.item.great-clarity-potion"], 1_000);
    assert_eq!(values["demo.item.understanding-scroll"], 2_500);
    assert_eq!(values["demo.item.inventory-protection-scroll"], 2_500);
    assert_eq!(values["demo.item.enlightenment-potion"], 800);
    assert_eq!(values["demo.item.star-enlightenment-potion"], 120_000);
    assert_eq!(values["demo.item.self-knowledge-potion"], 2_000);
    assert_eq!(values["demo.item.rune-of-protection-scroll"], 500);
    assert_eq!(values["demo.item.destruction-scroll"], 250);
    assert_eq!(values["demo.item.polymorph-potion"], 5_000);
}

#[test]
fn p3_1_allocated_items_all_have_a_shop_or_base_pool_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_items = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()));
    let loot_items = artifact
        .content
        .loot_tables
        .iter()
        .filter(|table| matches!(table.id.as_str(), "demo.loot-table.base-items"))
        .flat_map(|table| {
            table
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.as_str())
        });
    let available = shop_items.chain(loot_items).collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.poison-mushroom",
        "demo.item.blindness-mushroom",
        "demo.item.paranoia-mushroom",
        "demo.item.confusion-mushroom",
        "demo.item.hallucination-mushroom",
        "demo.item.paralysis-mushroom",
        "demo.item.weakness-mushroom",
        "demo.item.sickness-mushroom",
        "demo.item.stupidity-mushroom",
        "demo.item.naivety-mushroom",
        "demo.item.unhealth-mushroom",
        "demo.item.disease-mushroom",
        "demo.item.cure-poison-mushroom",
        "demo.item.cure-blindness-mushroom",
        "demo.item.cure-paranoia-mushroom",
        "demo.item.cure-confusion-mushroom",
        "demo.item.restore-constitution-mushroom",
        "demo.item.restore-strength-mushroom",
        "demo.item.hard-biscuit",
        "demo.item.strip-of-venison",
        "demo.item.slime-mold",
        "demo.item.sleep-potion",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn p3_2_items_all_have_a_shop_or_warrens_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_items = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()));
    let loot_items = artifact
        .content
        .loot_tables
        .iter()
        .filter(|table| matches!(table.id.as_str(), "demo.loot-table.base-items"))
        .flat_map(|table| {
            table
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.as_str())
        });
    let available = shop_items.chain(loot_items).collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.water-potion",
        "demo.item.apple-juice",
        "demo.item.slime-mold-juice",
        "demo.item.lose-memories-potion",
        "demo.item.ruination-potion",
        "demo.item.sight-potion",
        "demo.item.antidote-potion",
        "demo.item.curing-potion",
        "demo.item.invulnerability-potion",
        "demo.item.giant-strength-potion",
        "demo.item.great-clarity-potion",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn p3_3_items_all_have_a_shop_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let available = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()))
        .collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.treasure-detection-scroll",
        "demo.item.understanding-scroll",
        "demo.item.inventory-protection-scroll",
        "demo.item.enlightenment-potion",
        "demo.item.star-enlightenment-potion",
        "demo.item.self-knowledge-potion",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn p3_4_items_all_have_a_shop_or_warrens_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_items = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()));
    let loot_items = artifact
        .content
        .loot_tables
        .iter()
        .filter(|table| table.id == "demo.loot-table.base-items")
        .flat_map(|table| {
            table
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.as_str())
        });
    let available = shop_items.chain(loot_items).collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.darkness-scroll",
        "demo.item.trap-creation-scroll",
        "demo.item.light-scroll",
        "demo.item.rune-of-protection-scroll",
        "demo.item.destruction-scroll",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn p3_5_items_all_have_a_shop_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let available = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()))
        .collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.mundanity-scroll",
        "demo.item.acquirement-scroll",
        "demo.item.star-acquirement-scroll",
        "demo.item.rumour-scroll",
        "demo.item.crafting-scroll",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn archer_theme_uses_only_implemented_source_predicate_members() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let archer_items = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.archer")
        .expect("Archer drop table should exist")
        .entries
        .iter()
        .map(|entry| entry.item_kind_id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        archer_items,
        BTreeSet::from([
            "demo.item.arrow",
            "demo.item.dwarven-backpack",
            "demo.item.fabric-bag",
            "demo.item.heavy-crossbow",
            "demo.item.leather-pouch",
            "demo.item.light-crossbow",
            "demo.item.long-bow",
            "demo.item.mithril-arrow",
            "demo.item.ring",
            "demo.item.seeker-arrow",
            "demo.item.sheaf-arrow",
            "demo.item.short-bow",
            "demo.item.sling",
        ])
    );
}

#[test]
fn p3_7_active_potions_all_have_an_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_items = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()));
    let warrens_items = artifact
        .content
        .loot_tables
        .iter()
        .filter(|table| table.id == "demo.loot-table.base-items")
        .flat_map(|table| {
            table
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.as_str())
        });
    let available = shop_items.chain(warrens_items).collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.experience-potion",
        "demo.item.neo-tsuyoshi-special",
        "demo.item.tsuyoshi-special",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
}

#[test]
fn guaranteed_floor_supplies_require_rfb_chance_and_supported_items() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let warrens = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens");
    assert!(
        warrens
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.warrens"))
            .all(|floor| {
                floor.guaranteed_items.as_slice()
                    == [
                        ProceduralGuaranteedItemDefinition {
                            id: "demo.guaranteed.warrens-food".to_owned(),
                            chance_one_in: 2,
                            entries: vec![ProceduralGuaranteedItemEntryDefinition {
                                item_kind_id: "demo.item.ration-of-food".to_owned(),
                                weight: 1,
                            }],
                        },
                        ProceduralGuaranteedItemDefinition {
                            id: "demo.guaranteed.warrens-light".to_owned(),
                            chance_one_in: 2,
                            entries: vec![
                                ProceduralGuaranteedItemEntryDefinition {
                                    item_kind_id: "demo.item.flask-of-oil".to_owned(),
                                    weight: 1,
                                },
                                ProceduralGuaranteedItemEntryDefinition {
                                    item_kind_id: "demo.item.brass-lantern".to_owned(),
                                    weight: 2,
                                },
                            ],
                        },
                    ]
            })
    );

    let mut invalid_chance = artifact.content.clone();
    invalid_chance
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .expect("Warrens first floor should remain available")
        .guaranteed_items[0]
        .chance_one_in = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_chance),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut invalid_item = artifact.content.clone();
    invalid_item
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain Warrens")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .expect("Warrens first floor should remain available")
        .guaranteed_items[0]
        .entries[0]
        .item_kind_id = "demo.item.broad-sword".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_item),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn warrens_stair_ranges_match_floor_topology_and_stay_bounded() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_up = artifact.content.clone();
    zero_up
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .and_then(|world| {
            world
                .procedural_floors
                .iter_mut()
                .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        })
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens first floor should retain stair ranges")
        .up
        .minimum = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_up),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_down = artifact.content.clone();
    missing_down
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .and_then(|world| {
            world
                .procedural_floors
                .iter_mut()
                .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        })
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens first floor should retain stair ranges")
        .down = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_down),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut final_down = artifact.content.clone();
    final_down
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .and_then(|world| {
            world
                .procedural_floors
                .iter_mut()
                .find(|floor| floor.id == "demo.floor.warrens-depth-9")
        })
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens final floor should retain its up stair range")
        .down = Some(ProceduralCountRangeDefinition {
        minimum: 4,
        maximum: 5,
    });
    assert!(matches!(
        validate_and_normalize(&mut final_down),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn streamer_treasure_candidates_are_complete_and_validated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let magma = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .and_then(|world| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        })
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| {
            layout
                .streamers
                .iter()
                .find(|candidate| candidate.terrain_id == "demo.terrain.magma-vein")
        })
        .and_then(|candidate| candidate.treasure.as_ref())
        .expect("magma streamer treasure should remain available");
    assert_eq!(magma.known_terrain_id, "demo.terrain.magma-treasure");
    assert_eq!(
        magma.hidden_terrain_id,
        "demo.terrain.magma-hidden-treasure"
    );
    assert_eq!((magma.known_one_in, magma.hidden_one_in), (60, 20));

    let mut invalid = artifact.content;
    invalid.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| {
            layout
                .streamers
                .iter_mut()
                .find(|candidate| candidate.terrain_id == "demo.terrain.magma-vein")
        })
        .and_then(|candidate| candidate.treasure.as_mut())
        .expect("magma streamer treasure should remain available")
        .known_one_in = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn pest_control_matches_the_original_warrens_contract() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("fixture should contain the Warrens journey");
    let task = world
        .tasks
        .iter()
        .find(|task| task.id == "demo.task.pest-control")
        .expect("fixture should contain Pest Control");

    assert_eq!(
        task.prerequisite_task_id.as_deref(),
        Some("demo.task.thieves-hideout")
    );
    assert!(matches!(
        &task.location,
        TaskLocationDefinition::DungeonDepth { dungeon_id, depth }
            if dungeon_id == "demo.dungeon.warrens" && *depth == 5
    ));
    assert_eq!(task.objectives.len(), 1);
    assert_eq!(task.objectives[0].kind, TaskObjectiveKind::KillActorKind);
    assert_eq!(task.objectives[0].required, 8);
    assert_eq!(
        task.objectives[0].actor_kind_id.as_deref(),
        Some("demo.actor.warg")
    );
    assert_eq!(task.target_placements.len(), 1);
    assert_eq!(task.target_placements[0].spawn_count, 8);
    assert_eq!(
        task.completion_exit_terrain_id.as_deref(),
        Some("demo.terrain.stairs-down")
    );
    assert_eq!(
        task.reward.as_ref().unwrap().entries[0].item_kind_id,
        "demo.item.fur-cloak"
    );

    let reward = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.fur-cloak")
        .expect("fixture should contain the Fur Cloak reward");
    assert_eq!(reward.weight_tenths_pound, 30);
    assert_eq!(reward.base_value, 200);
    assert_eq!(reward.equipment_slot.as_deref(), Some("cloak"));
    assert_eq!(reward.modifiers.defense, 3);
}

#[test]
fn outpost_count_services_and_follow_up_tasks_match_the_original_sequence() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let count = artifact
        .content
        .town_facilities
        .iter()
        .find(|facility| facility.id == "demo.town-facility.outpost-count")
        .expect("Outpost should contain the Count's residence");
    assert_eq!(count.identify_item_cost, Some(50));
    assert_eq!(count.legal_name_change_cost, Some(10));
    assert_eq!(
        count.task_ids,
        [
            "demo.task.thieves-hideout",
            "demo.task.pest-control",
            "demo.task.the-sewer",
            "demo.task.haunted-house",
            "demo.task.royal-crypt",
        ]
    );
    assert_eq!(
        artifact
            .content
            .town_facilities
            .iter()
            .find(|facility| facility.id == "demo.town-facility.outpost-white-horse")
            .expect("Outpost should contain the White Horse task service")
            .task_ids,
        [
            "demo.task.trouble-at-home",
            "demo.task.crows-nest",
            "demo.task.old-man-willow",
            "demo.task.vapor-quest",
            "demo.task.old-castle"
        ]
    );

    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should remain available");
    let expected = [
        (
            "demo.task.the-sewer",
            "demo.task.pest-control",
            "demo.floor.outpost-sewer",
            15,
        ),
        (
            "demo.task.haunted-house",
            "demo.task.the-sewer",
            "demo.floor.outpost-haunted-house",
            45,
        ),
        (
            "demo.task.royal-crypt",
            "demo.task.haunted-house",
            "demo.floor.outpost-royal-crypt",
            70,
        ),
    ];
    for (task_id, prerequisite, floor_id, depth) in expected {
        let task = world
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .expect("follow-up task should remain available");
        assert_eq!(
            task.source_facility_id.as_deref(),
            Some("demo.town-facility.outpost-count")
        );
        assert_eq!(task.prerequisite_task_id.as_deref(), Some(prerequisite));
        assert!(matches!(
            &task.location,
            TaskLocationDefinition::DedicatedFloors { floor_ids }
                if floor_ids == &[floor_id.to_owned()]
        ));
        assert_eq!(
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == floor_id)
                .map(|floor| floor.depth),
            Some(depth)
        );
    }
}

#[test]
fn terrain_glyphs_do_not_consume_letters_reserved_for_actors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    assert!(artifact.content.terrain.iter().all(|terrain| {
        !terrain
            .glyph
            .chars()
            .next()
            .is_some_and(|glyph| glyph.is_ascii_alphabetic())
    }));

    let mut letter_terrain = artifact.content.clone();
    letter_terrain.terrain[0].glyph = "T".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut letter_terrain),
        Err(ContentError::InvalidTerrainGlyph(_))
    ));
}

#[test]
fn wilderness_towns_accept_fixed_town_floors_and_derive_world_ownership() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut content = artifact.content;
    let town_id = "demo.town.second";
    let floor_id = "demo.floor.second-town";
    let home_id = "demo.town-facility.second-home";

    let mut town = content
        .towns
        .iter()
        .find(|town| town.id == "demo.town.outpost")
        .expect("Outpost should remain available")
        .clone();
    town.id = town_id.to_owned();
    town.floor_id = floor_id.to_owned();
    town.facility_ids = vec![home_id.to_owned()];
    town.shop_ids.clear();
    content.towns.push(town);

    let mut home = content
        .town_facilities
        .iter()
        .find(|facility| facility.id == "demo.town-facility.outpost-home")
        .expect("Outpost Home should remain available")
        .clone();
    home.id = home_id.to_owned();
    home.town_id = town_id.to_owned();
    home.entrance_position = ContentPosition { x: 2, y: 1 };
    content.town_facilities.push(home);

    let world = content
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth world should remain available");
    let mut floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("inline floor template should remain available")
        .clone();
    floor.id = floor_id.to_owned();
    floor.name_key = "floor-demo-second-town-name".to_owned();
    floor.lifecycle = FloorLifecycle::Town;
    floor.depth = 0;
    floor.width = 4;
    floor.height = 3;
    floor.entry_terrain_id = None;
    floor.available_entry_terrain_id = None;
    floor.completed_entry_terrain_id = None;
    floor.failed_entry_terrain_id = None;
    floor.abandoned_entry_terrain_id = None;
    floor.task_id = None;
    floor.inline_map = Some(InlineFloorMapDefinition {
        player_position: ContentPosition { x: 1, y: 1 },
        terrain_overrides: vec![
            InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.floor".to_owned(),
                positions: vec![
                    ContentPosition { x: 0, y: 1 },
                    ContentPosition { x: 1, y: 1 },
                ],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
            InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.home-entrance".to_owned(),
                positions: vec![ContentPosition { x: 2, y: 1 }],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
            InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.outpost-gate".to_owned(),
                positions: vec![ContentPosition { x: 3, y: 1 }],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
        ],
        actor_spawns: Vec::new(),
        item_spawns: Vec::new(),
        scrambled_item_pair: None,
        scrambled_item_loot_pair: None,
        loot_spawns: Vec::new(),
        monster_formation: None,
    });
    world.procedural_floors.push(floor);
    let wilderness = world
        .wilderness
        .as_mut()
        .expect("Middle-earth world should retain wilderness");
    wilderness
        .locations
        .push(WildernessLocationDefinition::Town {
            position: ContentPosition {
                x: wilderness.start_position.x + 1,
                y: wilderness.start_position.y,
            },
            map_origin: ContentPosition { x: 45, y: 15 },
            town_id: town_id.to_owned(),
        });

    let set_second_town_origin = |content: &mut CompiledContentV1, origin: ContentPosition| {
        let location = content
            .worlds
            .iter_mut()
            .find(|world| world.id == "demo.world.middle-earth")
            .expect("Middle-earth world should remain available")
            .wilderness
            .as_mut()
            .expect("Middle-earth world should retain wilderness")
            .locations
            .iter_mut()
            .find(|location| {
                matches!(
                    location,
                    WildernessLocationDefinition::Town {
                        town_id: location_town_id,
                        ..
                    } if location_town_id == town_id
                )
            })
            .expect("second town location should remain available");
        let WildernessLocationDefinition::Town { map_origin, .. } = location else {
            unreachable!("second town location must remain a town");
        };
        *map_origin = origin;
    };

    let mut invalid_origin = content.clone();
    set_second_town_origin(&mut invalid_origin, ContentPosition { x: 93, y: 31 });
    assert!(matches!(
        validate_and_normalize(&mut invalid_origin),
        Err(ContentError::InvalidTown(id)) if id == town_id
    ));

    let mut disconnected = content.clone();
    set_second_town_origin(&mut disconnected, ContentPosition { x: 10, y: 10 });
    assert!(matches!(
        validate_and_normalize(&mut disconnected),
        Err(ContentError::InvalidTown(id)) if id == town_id
    ));

    let mut random_terrain = content.clone();
    let random_override = random_terrain
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == floor_id)
        .expect("second town floor should remain available")
        .inline_map
        .as_mut()
        .expect("second town floor should retain its inline map")
        .terrain_overrides
        .first_mut()
        .expect("second town floor should retain terrain overrides");
    random_override.chance_percent = 50;
    random_override.otherwise_terrain_id = Some(random_override.terrain_id.clone());
    assert!(matches!(
        validate_and_normalize(&mut random_terrain),
        Err(ContentError::InvalidTown(id)) if id == town_id
    ));

    validate_and_normalize(&mut content).expect("formal second town should validate");
}

#[test]
fn p82a_ocean_monsters_keep_macro_habitat_boundaries() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P82A should contain {id}"))
    };

    for (id, source_index, level, habitats) in [
        ("killer-whale", 363, 22, vec![ActorHabitat::Ocean]),
        ("shrieking-eel", 1252, 23, vec![ActorHabitat::Ocean]),
        (
            "boadile",
            869,
            25,
            vec![ActorHabitat::Ocean, ActorHabitat::Shore],
        ),
        ("jaws", 467, 30, vec![ActorHabitat::Ocean]),
        ("giant-squid", 482, 32, vec![ActorHabitat::Ocean]),
        ("seahorse", 443, 36, vec![ActorHabitat::Ocean]),
        ("giganto-the-gargantuan", 650, 38, vec![ActorHabitat::Ocean]),
        (
            "moire-queen-of-rebma",
            615,
            39,
            vec![
                ActorHabitat::Ocean,
                ActorHabitat::Shore,
                ActorHabitat::Swamp,
            ],
        ),
        ("mutant-manta-ray", 1333, 40, vec![ActorHabitat::Ocean]),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("P82A ocean monster should retain allocation metadata");
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(allocation.legacy_index, source_index, "{id} source index");
        assert!(allocation.wild_only, "{id} should remain wilderness-only");
        assert_eq!(allocation.habitats, habitats, "{id} habitats");
        assert!(
            allocation.legacy_dungeon_indices.is_empty(),
            "{id} should not enter a dungeon allocation"
        );
        assert!(actor.tags.iter().any(|tag| tag == "ocean"));
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"));
    }

    assert!(actor("giant-squid").rideable);
    assert!(actor("giganto-the-gargantuan").rideable);
    assert!(actor("jaws").terrain_interaction.picks_up_items);
}

#[test]
fn p82b_fixed_unique_and_polar_cat_keep_random_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P82B should contain {id}"))
    };

    let polar_cat = actor("polar-cat");
    let polar_allocation = polar_cat
        .allocation
        .as_ref()
        .expect("Polar cat should retain snow allocation");
    assert_eq!(polar_cat.level, 40);
    assert_eq!(polar_allocation.legacy_index, 1392);
    assert!(polar_allocation.wild_only);
    assert_eq!(polar_allocation.habitats, vec![ActorHabitat::Snow]);
    assert!(polar_allocation.legacy_dungeon_indices.is_empty());
    assert!(polar_cat.tags.iter().any(|tag| tag == "snow"));
    assert!(!polar_cat.tags.iter().any(|tag| tag == "orc-cave"));
    assert!(polar_cat.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Cold && aura.damage_dice == 2 && aura.damage_sides == 3
    }));
    assert!(
        polar_cat
            .monster_casting
            .as_ref()
            .expect("Polar cat should retain blinking")
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id == "rfb-legacy.ability.blink")
    );

    for (id, source_index, level) in [
        ("barney-the-dinosaur", 1061, 29),
        ("groo-the-wanderer", 1062, 33),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("FIXED_UNIQUE should not remove random allocation");
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(allocation.legacy_index, source_index, "{id} source index");
        assert_eq!(allocation.rarity, 255, "{id} rarity");
        assert!(actor.tags.iter().any(|tag| tag == "unique"), "{id}");
        assert!(actor.tags.iter().any(|tag| tag == "fixed-unique"), "{id}");
        assert!(
            !actor.tags.iter().any(|tag| tag == "fixed-placement"),
            "{id}"
        );
    }

    assert!(
        actor("groo-the-wanderer")
            .allocation
            .as_ref()
            .expect("Groo allocation")
            .habitats
            .contains(&ActorHabitat::All)
    );
}

#[test]
fn p83_location_monsters_keep_their_legacy_dungeon_boundaries() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P83 should contain {id}"))
    };

    for (id, source_index, level, dungeon_index) in [
        ("dailai-dongzhu-captain-of-southerings", 1075, 10, 31),
        ("lady-zhurong-the-avatar-of-flame-spirit", 1074, 14, 31),
        ("trapdoor-spider", 1314, 10, 35),
        ("dingo", 1320, 10, 35),
        ("burning-bush", 1307, 26, 35),
        ("giant-wombat", 1332, 29, 35),
        ("garkain", 1328, 30, 35),
        ("the-wicked-witch-of-the-south-east", 1306, 40, 35),
        ("sadie-the-rainbow-serpent", 1331, 40, 35),
        ("aude", 1148, 38, 7),
        ("helga", 1149, 39, 7),
        ("gertrude", 1150, 40, 7),
        ("sugriva-lord-of-kishkindha", 1368, 69, 43),
        ("nandi-the-bull-of-shiva", 1381, 70, 43),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("P83 location monster should retain allocation metadata");
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(allocation.legacy_index, source_index, "{id} source index");
        assert_eq!(
            allocation.legacy_dungeon_indices,
            [dungeon_index],
            "{id} dungeon index"
        );
        assert!(!allocation.wild_only, "{id} should remain dungeon-only");
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"), "{id}");
    }

    for id in ["aude", "helga"] {
        assert!(
            actor(id).tags.iter().any(|tag| tag == "witch-sister"),
            "{id} should be a Gertrude summon candidate"
        );
    }
    let gertrude = actor("gertrude");
    assert!(
        gertrude
            .monster_casting
            .as_ref()
            .expect("Gertrude should retain spellcasting")
            .abilities
            .iter()
            .any(|candidate| candidate.ability_id
                == "rfb-legacy.ability.summon-gertrude-sisters-l40-1d1-1")
    );
}

#[test]
fn p84a_knight_summon_candidates_have_the_formal_category_tag() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for id in [
        "novice-paladin",
        "paladin",
        "white-knight",
        "ultra-elite-paladin",
        "knight-templar",
    ] {
        let actor = artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P84A should retain {id}"));
        assert!(
            actor.tags.iter().any(|tag| tag == "knight"),
            "{id} should be eligible for the exact knight summon"
        );
    }
}

#[test]
fn p84b_camelot_roster_stays_bound_to_legacy_dungeon_two() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P84B should contain {id}"))
    };

    for (id, source_index, level) in [
        ("sir-kay", 1116, 22),
        ("camelot-knight", 1117, 26),
        ("sir-galahad", 1114, 28),
        ("sir-gareth", 1115, 28),
        ("sir-gawain", 1113, 29),
        ("sir-lancelot", 1112, 30),
        ("arthur-pendragon", 1111, 32),
        ("mordred", 1119, 32),
        ("morgana-le-fay", 1118, 35),
        ("the-questing-beast", 1122, 35),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("Camelot actors should retain dormant allocation metadata");
        assert_eq!(actor.level, level, "{id} level");
        assert_eq!(allocation.legacy_index, source_index, "{id} source index");
        assert_eq!(allocation.legacy_dungeon_indices, [2], "{id} dungeon");
        assert!(!allocation.wild_only, "{id} should remain dungeon-only");
        assert!(actor.tags.iter().any(|tag| tag == "camelot"), "{id}");
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"), "{id}");
    }

    for (id, summon_ability_id) in [
        ("sir-kay", "summon-camelot-knight-l22-1d2"),
        ("camelot-knight", "summon-knight-l26-1d2"),
        ("sir-galahad", "summon-camelot-knight-l28-1d2"),
        ("sir-gareth", "summon-camelot-knight-l28-1d2"),
        ("sir-gawain", "summon-camelot-knight-l29-1d2"),
        ("sir-lancelot", "summon-camelot-knight-l30-1d2"),
        ("arthur-pendragon", "summon-camelot-knight-l32-1d2"),
    ] {
        let actor = actor(id);
        assert!(actor.tags.iter().any(|tag| tag == "knight"), "{id}");
        assert!(actor.tags.iter().any(|tag| tag == "camelot-knight"), "{id}");
        assert!(
            actor
                .monster_casting
                .as_ref()
                .expect("Camelot knight should retain spellcasting")
                .abilities
                .iter()
                .any(|candidate| candidate.ability_id
                    == format!("rfb-legacy.ability.{summon_ability_id}")),
            "{id} summon"
        );
    }
}

#[test]
fn p102b_chameleon_cave_binds_ecology_layout_guardian_and_polymorph_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.chameleon-cave")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Chameleon cave should use global allocation");
    assert_eq!(policy.preferred_tags, ["chameleon"]);
    assert_eq!(policy.special_div, 0);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.chameleon-cave-final-reward")
        .expect("Chameleon cave reward should exist");
    assert_eq!(reward.rolls, 1);
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.polymorph-potion");

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 94, y: 52 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.chameleon-cave"
            )
        })
    }));
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.chameleon-cave")
        .expect("Chameleon cave should exist");
    assert_eq!(dungeon.legacy_index, Some(18));
    assert_eq!(dungeon.root_floor_id, "demo.floor.chameleon-cave-depth-30");
    assert_eq!(dungeon.guardian_actor_kind_id, "demo.actor.chameleon-lord");

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.chameleon-cave"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (30..=45).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.chameleon-cave-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.wall_terrain_id == "demo.terrain.wall"
            && floor.floor_terrain_id == "demo.terrain.floor"
    }));

    for floor in floors
        .iter()
        .filter(|floor| !matches!(floor.depth, 33 | 41))
    {
        let river = floor
            .layout
            .as_ref()
            .and_then(|layout| layout.river.as_ref())
            .expect("ordinary Chameleon cave layer should retain a river policy");
        assert_eq!(river.chance_one_in, Some(7));
        assert_eq!(river.deep_terrain_id, "demo.terrain.surface-water-deep");
        let alternative = river
            .alternative
            .as_ref()
            .expect("lava should be the alternate river");
        assert_eq!(
            alternative.deep_terrain_id,
            "demo.terrain.surface-lava-deep"
        );
        assert_eq!(alternative.chance_numerator, floor.depth + 1);
        assert_eq!(alternative.chance_denominator, 256);
    }
    let layer = |depth| {
        floors
            .iter()
            .find(|floor| floor.depth == depth)
            .and_then(|floor| floor.layout.as_ref())
            .unwrap_or_else(|| panic!("depth {depth} layout"))
    };
    assert_eq!(
        layer(33).lake.as_ref().map(|lake| (
            lake.deep_terrain_id.as_str(),
            lake.shallow_terrain_id.as_str()
        )),
        Some(("demo.terrain.surface-tree", "demo.terrain.surface-grass"))
    );
    assert_eq!(
        layer(35).rooms.as_ref().map(|rooms| rooms.placement),
        Some(ProceduralRoomPlacement::Partitioned)
    );
    assert!(layer(39).destroyed.is_some());
    assert_eq!(
        layer(41).lake.as_ref().map(|lake| (
            lake.deep_terrain_id.as_str(),
            lake.shallow_terrain_id.as_str()
        )),
        Some(("demo.terrain.rubble", "demo.terrain.floor"))
    );

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Chameleon Lord should guard depth 45");
    assert_eq!(guardian.instance_id, "demo.guardian.chameleon-cave.1");
    assert_eq!(guardian.actor_kind_id, "demo.actor.chameleon-lord");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.chameleon-cave-final-reward")
    );
}

#[test]
fn p85_level_zero_roster_stays_in_its_wilderness_habitats() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P85 should contain {id}"))
    };

    for (id, source_index, habitats) in [
        (
            "scrawny-cat",
            2,
            vec![ActorHabitat::Grass, ActorHabitat::Town],
        ),
        ("sparrow", 3, vec![ActorHabitat::Town]),
        (
            "chaffinch",
            4,
            vec![ActorHabitat::Grass, ActorHabitat::Wood],
        ),
        (
            "wild-rabbit",
            5,
            vec![ActorHabitat::Grass, ActorHabitat::Wood],
        ),
        ("woodsman", 6, vec![ActorHabitat::Wood]),
        ("scruffy-little-dog", 7, vec![ActorHabitat::Town]),
        ("farmer-maggot", 8, vec![ActorHabitat::Town]),
        ("blubbering-idiot", 9, vec![ActorHabitat::Town]),
        ("hobo", 10, vec![ActorHabitat::Town]),
        ("raving-lunatic", 11, vec![ActorHabitat::Town]),
        ("pitiful-looking-beggar", 12, vec![ActorHabitat::Town]),
        ("mangy-looking-leper", 13, vec![ActorHabitat::Town]),
        ("aimless-looking-merchant", 16, vec![ActorHabitat::Town]),
        ("battle-scarred-veteran", 18, vec![ActorHabitat::Town]),
        ("nick-the-butcher", 19, vec![ActorHabitat::Town]),
        ("scrawny-horse", 955, vec![ActorHabitat::Town]),
        (
            "noborta-kesyta-the-yeek-president",
            1059,
            vec![ActorHabitat::Town],
        ),
        ("mori-troll", 1060, vec![ActorHabitat::Town]),
    ] {
        let actor = actor(id);
        let allocation = actor
            .allocation
            .as_ref()
            .expect("P85 actor should retain wilderness allocation");
        assert_eq!(actor.level, 0, "{id} level");
        assert_eq!(allocation.legacy_index, source_index, "{id} source index");
        assert!(allocation.wild_only, "{id} should be wilderness-only");
        assert_eq!(allocation.habitats, habitats, "{id} habitats");
        assert!(allocation.legacy_dungeon_indices.is_empty(), "{id}");
        assert!(!actor.tags.iter().any(|tag| tag == "orc-cave"), "{id}");
    }

    for id in ["farmer-maggot", "nick-the-butcher"] {
        let actor = actor(id);
        assert!(actor.friendly, "{id} should retain FRIENDLY");
        assert!(actor.tags.iter().any(|tag| tag == "unique"), "{id}");
    }
    for id in ["noborta-kesyta-the-yeek-president", "mori-troll"] {
        let actor = actor(id);
        assert!(actor.tags.iter().any(|tag| tag == "unique"), "{id}");
        assert!(actor.tags.iter().any(|tag| tag == "fixed-unique"), "{id}");
    }
    assert!(actor("scrawny-horse").rideable);
}

#[test]
fn p96c_numenor_atlantis_share_entry_substitution_and_aquatic_ecology() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.numenor-atlantis")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Numenor and Atlantis should share global allocation");
    assert!(policy.preferred_glyphs.is_empty());
    assert!(policy.preferred_tags.is_empty());
    assert_eq!(
        policy.preferred_movement_modes,
        [
            ActorMovementMode::Aquatic,
            ActorMovementMode::Fly,
            ActorMovementMode::Swim,
        ]
    );
    assert_eq!(policy.preferred_habitats, [ActorHabitat::Ocean]);
    assert_eq!(policy.special_div, 0);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let terrain = content
        .terrain
        .iter()
        .find(|terrain| terrain.id == "demo.terrain.numenor-atlantis-entrance")
        .expect("shared underwater entrance terrain");
    assert_eq!(terrain.glyph, ">");
    assert!(terrain.tags.iter().any(|tag| tag == "stairs-down"));

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let locations = world
        .wilderness
        .as_ref()
        .expect("Middle-earth wilderness")
        .locations
        .iter()
        .filter_map(|location| match location {
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 30, y: 27 },
                dungeon_id,
            } if matches!(
                dungeon_id.as_str(),
                "demo.dungeon.numenor" | "demo.dungeon.atlantis"
            ) =>
            {
                Some(dungeon_id.as_str())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        locations,
        BTreeSet::from(["demo.dungeon.atlantis", "demo.dungeon.numenor"])
    );

    let numenor = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.numenor")
        .expect("Numenor should exist");
    let atlantis = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.atlantis")
        .expect("Atlantis should exist");
    assert_eq!(numenor.legacy_index, Some(11));
    assert_eq!(atlantis.legacy_index, Some(41));
    assert_eq!(
        numenor.substitution,
        Some(DungeonSubstitutionDefinition {
            alternate_dungeon_id: "demo.dungeon.atlantis".to_owned(),
            alternate_gate_one_in: None,
        })
    );
    assert!(atlantis.substitution.is_none());
    assert_eq!(
        numenor.guardian_actor_kind_id,
        "demo.actor.jormungand-the-midgard-serpent"
    );
    assert_eq!(
        atlantis.guardian_actor_kind_id,
        "demo.actor.kundry-queen-of-the-lost-haven"
    );
    let numenor_entrance = numenor
        .entrance_guardian
        .as_ref()
        .expect("Numenor entrance guardian");
    let atlantis_entrance = atlantis
        .entrance_guardian
        .as_ref()
        .expect("Atlantis entrance guardian");
    assert_ne!(numenor_entrance.instance_id, atlantis_entrance.instance_id);
    assert_ne!(numenor_entrance.position, atlantis_entrance.position);
    assert_eq!(numenor_entrance.actor_kind_id, "demo.actor.lesser-kraken");
    assert_eq!(atlantis_entrance.actor_kind_id, "demo.actor.lesser-kraken");

    for (dungeon_id, depths, final_guardian) in [
        (
            "demo.dungeon.numenor",
            (55..=75).collect::<Vec<_>>(),
            "demo.actor.jormungand-the-midgard-serpent",
        ),
        (
            "demo.dungeon.atlantis",
            (55..=65).collect::<Vec<_>>(),
            "demo.actor.kundry-queen-of-the-lost-haven",
        ),
    ] {
        let mut floors = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        floors.sort_by_key(|floor| floor.depth);
        assert_eq!(
            floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
            depths
        );
        assert_eq!(
            floors[0].entry_terrain_id.as_deref(),
            Some("demo.terrain.numenor-atlantis-entrance")
        );
        assert!(floors.iter().all(|floor| {
            floor.encounter_table_id.as_deref() == Some("demo.encounter-table.numenor-atlantis")
        }));
        assert!(floors.windows(2).all(|pair| {
            pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
                && pair[1].return_floor_id == pair[0].id
        }));
        let guardian = floors
            .last()
            .and_then(|floor| floor.guardian.as_ref())
            .expect("final guardian");
        assert_eq!(guardian.actor_kind_id, final_guardian);
        assert_eq!(
            guardian.reward_artifact_item_kind_id.as_deref(),
            Some("demo.item.trifurcate-spear-of-wrath")
        );
        assert_eq!(
            guardian.reward_loot_table_id.as_deref(),
            Some("demo.loot-table.trifurcate-spear-final-replacement")
        );
    }
}

#[test]
fn p96d_numenor_atlantis_bind_water_mix_veins_stairs_and_representative_layers() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let feature_table = |id: &str| {
        content
            .terrain_feature_tables
            .iter()
            .find(|table| table.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    };
    let numenor_features = feature_table("demo.terrain-feature-table.numenor");
    assert_eq!(numenor_features.rolls, 160);
    assert_eq!(numenor_features.entries.len(), 1);
    assert_eq!(
        numenor_features.entries[0].terrain_id,
        "demo.terrain.surface-water-shallow"
    );
    assert_eq!(
        numenor_features.entries[0].placement,
        TerrainFeaturePlacement::Room
    );
    let atlantis_features = feature_table("demo.terrain-feature-table.atlantis");
    assert_eq!(atlantis_features.rolls, 320);
    assert_eq!(
        atlantis_features
            .entries
            .iter()
            .map(|entry| (entry.terrain_id.as_str(), entry.weight, entry.placement))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            (
                "demo.terrain.surface-water-deep",
                1,
                TerrainFeaturePlacement::Room,
            ),
            (
                "demo.terrain.surface-water-shallow",
                1,
                TerrainFeaturePlacement::Room,
            ),
        ])
    );

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    for (
        dungeon_id,
        feature_table_id,
        feature_placements,
        expected_lake_depths,
        expected_destroyed_depths,
        has_river,
    ) in [
        (
            "demo.dungeon.numenor",
            "demo.terrain-feature-table.numenor",
            160,
            vec![65],
            vec![70],
            true,
        ),
        (
            "demo.dungeon.atlantis",
            "demo.terrain-feature-table.atlantis",
            320,
            vec![60],
            vec![],
            false,
        ),
    ] {
        let mut floors = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        floors.sort_by_key(|floor| floor.depth);
        assert!(floors.iter().all(|floor| floor.connections.is_empty()));
        assert_eq!(
            floors
                .iter()
                .filter(|floor| floor
                    .layout
                    .as_ref()
                    .is_some_and(|layout| layout.lake.is_some()))
                .map(|floor| floor.depth)
                .collect::<Vec<_>>(),
            expected_lake_depths
        );
        assert_eq!(
            floors
                .iter()
                .filter(|floor| floor
                    .layout
                    .as_ref()
                    .is_some_and(|layout| layout.destroyed.is_some()))
                .map(|floor| floor.depth)
                .collect::<Vec<_>>(),
            expected_destroyed_depths
        );

        for floor in floors {
            assert_eq!(
                floor.loot_table_id.as_deref(),
                Some("demo.loot-table.base-items")
            );
            assert_eq!(
                floor.terrain_feature_table_id.as_deref(),
                Some(feature_table_id)
            );
            let budget = floor.generation_budget.as_ref().expect("generation budget");
            assert_eq!(budget.room_area_tiles, Some(800));
            assert_eq!(budget.streamer_placements, Some(2));
            assert_eq!(budget.streamer_area_tiles, Some(32));
            assert_eq!(budget.feature_placements, Some(feature_placements));
            assert_eq!(budget.river_area_tiles, has_river.then_some(160));
            assert_eq!(
                (budget.lake_area_tiles, budget.lake_deep_area_tiles),
                if floor.depth == expected_lake_depths[0] {
                    (Some(360), Some(120))
                } else {
                    (None, None)
                }
            );
            assert_eq!(
                (budget.destruction_centers, budget.destroyed_area_tiles),
                if expected_destroyed_depths.contains(&floor.depth) {
                    (Some(1), Some(96))
                } else {
                    (None, None)
                }
            );

            let layout = floor.layout.as_ref().expect("standard room layout");
            assert_eq!(
                layout
                    .rooms
                    .as_ref()
                    .expect("room geometry")
                    .shapes
                    .iter()
                    .map(|shape| (shape.shape, shape.weight))
                    .collect::<Vec<_>>(),
                vec![(ProceduralRoomShape::Rectangle, 1)]
            );
            assert_eq!(
                layout
                    .streamers
                    .iter()
                    .map(|streamer| (streamer.terrain_id.as_str(), streamer.weight))
                    .collect::<BTreeMap<_, _>>(),
                BTreeMap::from([
                    ("demo.terrain.magma-vein", 1),
                    ("demo.terrain.quartz-vein", 1),
                ])
            );
            assert_eq!(layout.river.is_some(), has_river);
            if let Some(river) = &layout.river {
                assert_eq!(river.deep_terrain_id, "demo.terrain.surface-water-deep");
                assert_eq!(
                    river.shallow_terrain_id,
                    "demo.terrain.surface-water-shallow"
                );
                assert_eq!(river.chance_one_in, Some(7));
            }
            let stairs = layout.stairs.as_ref().expect("ordinary stair ranges");
            assert_eq!((stairs.up.minimum, stairs.up.maximum), (1, 2));
            assert_eq!(
                stairs.down.map(|range| (range.minimum, range.maximum)),
                (!floor.final_floor).then_some((4, 5))
            );
        }
    }
}

#[test]
fn p103c_volcano_binds_lava_ecology_guardians_layers_and_reward() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.volcano")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Volcano should use global allocation");
    assert_eq!(policy.preferred_movement_modes, [ActorMovementMode::Fly]);
    assert_eq!(policy.preferred_habitats, [ActorHabitat::Volcano]);
    assert_eq!(policy.preferred_damage_immunities, [ActorDamageType::Fire]);
    assert_eq!((policy.special_div, policy.ambient_chance_one_in), (0, 160));

    let features = content
        .terrain_feature_tables
        .iter()
        .find(|table| table.id == "demo.terrain-feature-table.volcano")
        .expect("Volcano lava distribution should exist");
    assert_eq!(features.rolls, 480);
    assert_eq!(
        features
            .entries
            .iter()
            .map(|entry| (entry.terrain_id.as_str(), entry.weight))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([
            ("demo.terrain.surface-lava-deep", 1),
            ("demo.terrain.surface-lava-shallow", 2),
        ])
    );

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 13, y: 53 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.volcano"
            )
        })
    }));
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.volcano")
        .expect("Volcano should exist");
    assert_eq!(dungeon.legacy_index, Some(8));
    assert_eq!(dungeon.root_floor_id, "demo.floor.volcano-depth-50");
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.shooting-star-the-red-dragon"
    );
    let entrance = dungeon
        .entrance_guardian
        .as_ref()
        .expect("Lesser Balrog should guard the entrance");
    assert_eq!(entrance.instance_id, "demo.guardian.volcano-entrance.1");
    assert_eq!(entrance.actor_kind_id, "demo.actor.lesser-balrog");

    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.volcano"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (50..=60).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.volcano-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (96, 33)
            && floor.wall_terrain_id == "demo.terrain.wall"
            && floor.floor_terrain_id == "demo.terrain.dirt"
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.volcano")
    }));
    for floor in floors.iter().filter(|floor| floor.depth != 55) {
        let river = floor
            .layout
            .as_ref()
            .and_then(|layout| layout.river.as_ref())
            .expect("ordinary Volcano layer should retain lava rivers");
        assert_eq!(river.chance_one_in, Some(7));
        assert_eq!(river.deep_terrain_id, "demo.terrain.surface-lava-deep");
        assert_eq!(
            river.shallow_terrain_id,
            "demo.terrain.surface-lava-shallow"
        );
    }
    let layout = |depth| {
        floors
            .iter()
            .find(|floor| floor.depth == depth)
            .and_then(|floor| floor.layout.as_ref())
            .unwrap_or_else(|| panic!("depth {depth} layout"))
    };
    assert!(layout(55).lake.is_some());
    assert!(layout(57).destroyed.is_some());
    assert!(floors.iter().all(|floor| {
        floor.layout.as_ref().is_some_and(|layout| {
            layout.rooms.as_ref().is_some_and(|rooms| {
                rooms
                    .shapes
                    .iter()
                    .any(|shape| shape.shape == ProceduralRoomShape::Cavern && shape.weight == 9)
            })
        })
    }));

    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Shooting Star should guard depth 60");
    assert_eq!(guardian.instance_id, "demo.guardian.volcano.1");
    assert_eq!(
        guardian.actor_kind_id,
        "demo.actor.shooting-star-the-red-dragon"
    );
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.volcano-final-reward")
    );
    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.volcano-final-reward")
        .expect("Volcano reward should exist");
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.mana-storm-staff");
}

#[test]
fn p104b_anambar_library_and_shroomery_are_fully_bound() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let town = artifact
        .content
        .towns
        .iter()
        .find(|town| town.id == "demo.town.anambar")
        .expect("Anambar should exist");
    assert!(
        town.facility_ids
            .contains(&"demo.town-facility.anambar-library".to_owned())
    );
    assert!(
        town.shop_ids
            .contains(&"demo.shop.anambar-shroomery".to_owned())
    );

    let library = artifact
        .content
        .town_facilities
        .iter()
        .find(|facility| facility.id == "demo.town-facility.anambar-library")
        .expect("Anambar library should exist");
    assert_eq!(library.category, TownFacilityCategory::Service);
    assert_eq!(
        library.owner_name_key.as_deref(),
        Some("town-facility-demo-anambar-library-owner-name")
    );
    assert_eq!(library.identify_item_cost, Some(50));
    assert_eq!(library.research_item_cost, Some(1_300));
    assert_eq!(library.identify_all_items_cost, Some(350));
    assert_eq!(
        library.overview_message_key.as_deref(),
        Some("town-facility-demo-anambar-library-overview")
    );
    assert_eq!(library.entrance_position, ContentPosition { x: 16, y: 1 });

    let shroomery = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == "demo.shop.anambar-shroomery")
        .expect("Anambar shroomery should exist");
    assert_eq!(shroomery.category, ShopCategory::Shroomery);
    assert_eq!(shroomery.entrance_position, ContentPosition { x: 4, y: 9 });
    assert_eq!(shroomery.stock.len(), 5);

    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == town.floor_id)
        .expect("Anambar floor should exist");
    let inline = floor
        .inline_map
        .as_ref()
        .expect("Anambar should use a fixed map");
    for (terrain_id, position) in [
        ("demo.terrain.library-entrance", library.entrance_position),
        (
            "demo.terrain.shroomery-entrance",
            shroomery.entrance_position,
        ),
    ] {
        assert!(inline.terrain_overrides.iter().any(|override_| {
            override_.terrain_id == terrain_id && override_.positions == [position]
        }));
    }
}

#[test]
fn p105b_anambar_recovery_enchantment_and_recall_facilities_are_fully_bound() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let town = artifact
        .content
        .towns
        .iter()
        .find(|town| town.id == "demo.town.anambar")
        .expect("Anambar should exist");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    let floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == town.floor_id)
        .and_then(|floor| floor.inline_map.as_ref())
        .expect("Anambar should use a fixed map");
    let expected = [
        (
            "demo.town-facility.anambar-weapon-master",
            ContentPosition { x: 4, y: 1 },
            "demo.terrain.weapon-master-entrance",
        ),
        (
            "demo.town-facility.anambar-warrior-guild",
            ContentPosition { x: 8, y: 1 },
            "demo.terrain.warrior-guild-entrance",
        ),
        (
            "demo.town-facility.anambar-mammon-temple",
            ContentPosition { x: 12, y: 1 },
            "demo.terrain.mammon-temple-entrance",
        ),
        (
            "demo.town-facility.anambar-archer-guild",
            ContentPosition { x: 20, y: 1 },
            "demo.terrain.archer-guild-entrance",
        ),
        (
            "demo.town-facility.anambar-trump-tower",
            ContentPosition { x: 8, y: 9 },
            "demo.terrain.trump-tower-entrance",
        ),
    ];
    for (facility_id, position, terrain_id) in expected {
        assert!(town.facility_ids.contains(&facility_id.to_owned()));
        let facility = artifact
            .content
            .town_facilities
            .iter()
            .find(|facility| facility.id == facility_id)
            .expect("P105 facility should exist");
        assert_eq!(facility.category, TownFacilityCategory::Service);
        assert_eq!(facility.entrance_position, position);
        assert_eq!(facility.entrance_terrain_id, terrain_id);
        assert!(!facility.service_actions.is_empty());
        assert!(floor.terrain_overrides.iter().any(|override_| {
            override_.terrain_id == terrain_id && override_.positions == [position]
        }));
    }

    let facility = |id: &str| {
        artifact
            .content
            .town_facilities
            .iter()
            .find(|facility| facility.id == id)
            .expect("P105 facility should exist")
    };
    assert_eq!(
        facility("demo.town-facility.anambar-warrior-guild").owner_class_ids,
        ["demo.class.cavalry", "demo.class.warrior"]
    );
    assert_eq!(
        facility("demo.town-facility.anambar-mammon-temple").member_class_ids,
        ["demo.class.paladin"]
    );
    assert_eq!(
        facility("demo.town-facility.anambar-trump-tower").member_race_ids,
        ["rfb-legacy.race.amberite"]
    );
    assert_eq!(
        facility("demo.town-facility.anambar-trump-tower").owner_realm_ids,
        ["trump"]
    );
    assert_eq!(
        facility("demo.town-facility.anambar-mammon-temple")
            .service_actions
            .iter()
            .map(|service| (service.kind, service.owner_cost, service.other_cost))
            .collect::<Vec<_>>(),
        [
            (TownFacilityServiceKind::Heal, 0, 500),
            (TownFacilityServiceKind::RestoreVitality, 500, 2_500),
            (TownFacilityServiceKind::CureMutation, 10_000, 100_000),
        ]
    );
}

#[test]
fn p106_dual_town_bounty_offices_share_the_wanted_reward_contract() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let facility = |id: &str| {
        artifact
            .content
            .town_facilities
            .iter()
            .find(|facility| facility.id == id)
            .expect("P106 bounty facility should exist")
    };
    let outpost = facility("demo.town-facility.outpost-bounty-office");
    let anambar = facility("demo.town-facility.anambar-police-station");
    assert_eq!(outpost.category, TownFacilityCategory::QuestGiver);
    assert_eq!(anambar.category, TownFacilityCategory::QuestGiver);
    assert_eq!(outpost.entrance_position, ContentPosition { x: 57, y: 19 });
    assert_eq!(anambar.entrance_position, ContentPosition { x: 12, y: 9 });
    assert_eq!(
        outpost.entrance_terrain_id,
        "demo.terrain.bounty-office-entrance"
    );
    let outpost_rewards = &outpost
        .bounty_office
        .as_ref()
        .expect("Outpost should expose bounty rules")
        .wanted_reward_item_kind_ids;
    let anambar_rewards = &anambar
        .bounty_office
        .as_ref()
        .expect("Anambar should expose bounty rules")
        .wanted_reward_item_kind_ids;
    assert_eq!(outpost_rewards.len(), 20);
    assert_eq!(outpost_rewards, anambar_rewards);
    assert_eq!(outpost_rewards[7], "demo.item.destruction-scroll");
    assert_eq!(outpost_rewards[9], "demo.item.crafting-scroll");
    assert_eq!(outpost_rewards[13], "demo.item.new-life-potion");
    assert_eq!(outpost_rewards[18], "demo.item.invulnerability-potion");

    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    assert!(world.terrain_overrides.iter().any(|override_| {
        override_.terrain_id == "demo.terrain.bounty-office-entrance"
            && override_
                .positions
                .contains(&ContentPosition { x: 57, y: 19 })
    }));
    let anambar_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.anambar")
        .and_then(|floor| floor.inline_map.as_ref())
        .expect("Anambar fixed map should exist");
    assert!(anambar_floor.terrain_overrides.iter().any(|override_| {
        override_.terrain_id == "demo.terrain.bounty-office-entrance"
            && override_.positions == [ContentPosition { x: 12, y: 9 }]
    }));
}

#[test]
fn p107c_anambar_rewards_bind_their_authoritative_effects_and_sacred_ego() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let item = |id: &str| {
        content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    };
    let activation_effect = |id: &str| {
        &item(id)
            .device_generation
            .as_ref()
            .expect("task device should have generated activation")
            .activations[0]
            .effect
    };

    assert!(matches!(
        activation_effect("demo.item.frost-ball-wand"),
        ItemUseEffectDefinition::AreaDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 50,
            damage_type: ActorDamageType::Cold,
            radius: 2,
        }
    ));
    let ItemUseEffectDefinition::Sequence { effects } =
        activation_effect("demo.item.confusing-light-staff")
    else {
        panic!("Confusing Lights should use an ordered status sequence");
    };
    assert_eq!(effects.len(), 5);
    assert_eq!(
        effects
            .iter()
            .map(|effect| match effect {
                ItemUseEffectDefinition::VisibleApplyStatus {
                    status_kind_id,
                    power,
                    ..
                } => (status_kind_id.as_str(), *power),
                _ => panic!("Confusing Lights steps should target visible actors"),
            })
            .collect::<Vec<_>>(),
        [
            ("rfb.status.slow", Some(110)),
            ("rfb.status.stun", Some(110)),
            ("rfb.status.confusion", Some(110)),
            ("rfb.status.fear", Some(110)),
            ("rfb.status.paralysis", Some(36)),
        ]
    );
    assert!(matches!(
        activation_effect("demo.item.destruction-staff"),
        ItemUseEffectDefinition::AreaDestruction {
            minimum_radius: 13,
            maximum_radius: 17,
            ..
        }
    ));
    let restoring = item("demo.item.sixfold-provision");
    assert_eq!(restoring.base_value, 1000);
    assert!(restoring.tags.contains(&"task-reward".to_owned()));
    assert!(matches!(
        restoring.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::RestoreAllAttributes)
    ));

    let sacred = content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.sacred-pendant")
        .expect("Sacred Pendant should exist");
    let ego = sacred
        .rfb_ego
        .as_ref()
        .expect("Sacred Pendant should retain EGO identity");
    assert_eq!((ego.source_index, ego.rarity), (221, 2));
    assert_eq!(ego.types, [RfbEgoTypeDefinition::Amulet]);
    assert_eq!(sacred.roll_groups.len(), 3);
    assert_eq!(sacred.roll_groups[2].rolls, 5);
    assert!(sacred.tags.contains(&"blessed-weapon".to_owned()));
}

#[test]
fn p107d_crystal_castle_binds_glass_layers_guardians_and_diamond_edge() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = &artifact.content;
    let diamond = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.diamond-edge")
        .expect("Diamond Edge should exist");
    assert!(diamond.vorpal);
    assert_eq!(diamond.tunneling_pval, 4);
    assert_eq!(
        diamond.melee_profile.as_ref().map(|profile| (
            profile.damage_dice,
            profile.damage_sides,
            profile.to_hit,
            profile.to_damage
        )),
        Some((6, 6, 10, 10))
    );
    for (id, walkable, blocks_sight) in [
        ("demo.terrain.glass-floor", true, false),
        ("demo.terrain.glass-door-open", true, false),
        ("demo.terrain.glass-door-closed", false, false),
    ] {
        let terrain = content
            .terrain
            .iter()
            .find(|terrain| terrain.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"));
        assert_eq!(
            (terrain.walkable, terrain.blocks_sight),
            (walkable, blocks_sight)
        );
    }
    let policy = content
        .encounter_tables
        .iter()
        .find(|table| table.id == "demo.encounter-table.crystal-castle")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Crystal Castle should use global allocation");
    assert_eq!(policy.preferred_glyphs, ["D"]);
    assert_eq!(policy.preferred_tags, ["invisible"]);
    assert_eq!((policy.special_div, policy.ambient_chance_one_in), (0, 160));

    let world = content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.middle-earth")
        .expect("Middle-earth should exist");
    assert!(world.wilderness.as_ref().is_some_and(|wilderness| {
        wilderness.locations.iter().any(|location| {
            matches!(
                location,
                WildernessLocationDefinition::Dungeon {
                    position: ContentPosition { x: 40, y: 37 },
                    dungeon_id,
                } if dungeon_id == "demo.dungeon.crystal-castle"
            )
        })
    }));
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.crystal-castle")
        .expect("Crystal Castle should exist");
    assert_eq!(dungeon.legacy_index, Some(20));
    assert_eq!(
        dungeon.guardian_actor_kind_id,
        "demo.actor.the-diamond-dragon"
    );
    assert_eq!(
        dungeon
            .entrance_guardian
            .as_ref()
            .map(|guardian| guardian.actor_kind_id.as_str()),
        Some("demo.actor.ethereal-dragon")
    );
    let mut floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.crystal-castle"))
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(
        floors.iter().map(|floor| floor.depth).collect::<Vec<_>>(),
        (40..=60).collect::<Vec<_>>()
    );
    assert!(floors.windows(2).all(|pair| {
        pair[0].next_floor_id.as_deref() == Some(pair[1].id.as_str())
            && pair[1].return_floor_id == pair[0].id
    }));
    assert_eq!(
        floors[0].entry_terrain_id.as_deref(),
        Some("demo.terrain.crystal-castle-entrance")
    );
    assert!(floors.iter().all(|floor| {
        (floor.width, floor.height) == (66, 22)
            && floor.floor_terrain_id == "demo.terrain.floor"
            && floor.terrain_feature_table_id.as_deref()
                == Some("demo.terrain-feature-table.crystal-castle")
    }));
    let floor = |depth| {
        floors
            .iter()
            .find(|floor| floor.depth == depth)
            .copied()
            .unwrap()
    };
    assert_eq!(
        floor(45)
            .generation_budget
            .as_ref()
            .unwrap()
            .room_placements,
        Some(2)
    );
    assert_eq!(
        floor(50).closed_door_terrain_id,
        "demo.terrain.curtain-closed"
    );
    assert_eq!(floor(55).wall_terrain_id, "demo.terrain.glass-wall");
    let guardian = floors
        .last()
        .and_then(|floor| floor.guardian.as_ref())
        .expect("Diamond Dragon should guard depth 60");
    assert_eq!(guardian.actor_kind_id, "demo.actor.the-diamond-dragon");
    assert_eq!(
        guardian.reward_loot_table_id.as_deref(),
        Some("demo.loot-table.crystal-castle-final-reward")
    );
    let reward = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.crystal-castle-final-reward")
        .expect("Crystal Castle reward should exist");
    assert_eq!(reward.entries[0].item_kind_id, "demo.item.diamond-edge");
}
