use std::collections::BTreeSet;

use super::*;

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
    assert_eq!(policy.special_div, 16);
    assert_eq!(policy.ambient_chance_one_in, 160);

    let mut allocation = artifact
        .content
        .actors
        .iter()
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
            ("demo.actor.abyss-worm-mass", 214, 4, 40),
            ("demo.actor.agent-of-black-market", 14, 1, 0),
            ("demo.actor.air-spirit", 227, 2, 50),
            ("demo.actor.baby-black-dragon", 166, 2, 40),
            ("demo.actor.baby-blue-dragon", 163, 2, 40),
            ("demo.actor.baby-green-dragon", 165, 2, 40),
            ("demo.actor.baby-multi-hued-dragon", 204, 2, 40),
            ("demo.actor.baby-red-dragon", 167, 2, 40),
            ("demo.actor.baby-white-dragon", 164, 2, 40),
            ("demo.actor.balcmeg-the-relentless", 1182, 2, 999),
            ("demo.actor.bandit", 150, 2, 40),
            ("demo.actor.barracuda", 96, 2, 40),
            ("demo.actor.black-harpy", 157, 1, 60),
            ("demo.actor.black-mamba", 210, 3, 40),
            ("demo.actor.black-naga", 71, 1, 30),
            ("demo.actor.black-ogre", 262, 2, 60),
            ("demo.actor.black-orc", 244, 2, 50),
            ("demo.actor.blinking-dot", 22, 1, 10),
            ("demo.actor.bloodfang-the-wolf", 170, 1, 999),
            ("demo.actor.bloodshot-eye", 129, 3, 40),
            ("demo.actor.bloodshot-icky-thing", 155, 3, 40),
            ("demo.actor.blubbering-icky-thing", 41, 1, 20),
            ("demo.actor.blue-horror", 189, 3, 40),
            ("demo.actor.blue-icky-thing", 252, 4, 50),
            ("demo.actor.blue-yeek", 52, 1, 20),
            ("demo.actor.boldor-king-of-the-yeeks", 237, 3, 999),
            ("demo.actor.bomb-mosquito", 1017, 3, 20),
            ("demo.actor.brodda-the-easterling", 169, 2, 999),
            ("demo.actor.broken-death-sword", 953, 5, 40),
            ("demo.actor.brown-mold", 113, 1, 40),
            ("demo.actor.brown-yeek", 141, 1, 40),
            ("demo.actor.bullroarer-the-hobbit", 914, 3, 999),
            ("demo.actor.buzzy-beetle", 951, 4, 60),
            ("demo.actor.carnivorous-flying-monkey", 145, 2, 40),
            ("demo.actor.carrion", 361, 1, 70),
            ("demo.actor.caustic-icky-thing", 132, 2, 40),
            ("demo.actor.cave-lizard", 82, 1, 30),
            ("demo.actor.cave-orc", 126, 1, 40),
            ("demo.actor.cave-spider", 60, 1, 30),
            ("demo.actor.chaos-shapechanger", 203, 2, 40),
            ("demo.actor.cheerful-leprechaun", 258, 2, 50),
            ("demo.actor.chiokovo", 997, 3, 30),
            ("demo.actor.clear-hound", 282, 3, 50),
            ("demo.actor.clear-icky-thing", 26, 1, 10),
            ("demo.actor.clear-mushroom-patch", 184, 2, 40),
            ("demo.actor.clear-worm-mass", 79, 2, 30),
            ("demo.actor.cloaker", 243, 5, 50),
            ("demo.actor.copperhead-snake", 106, 1, 40),
            ("demo.actor.creeping-copper-coins", 85, 2, 40),
            ("demo.actor.creeping-gold-coins", 195, 3, 40),
            ("demo.actor.creeping-mithril-coins", 239, 4, 50),
            ("demo.actor.creeping-silver-coins", 117, 2, 40),
            ("demo.actor.crow", 61, 2, 20),
            ("demo.actor.crow-of-durthang", 1224, 2, 40),
            ("demo.actor.crypt-creep", 124, 2, 40),
            ("demo.actor.culverin", 867, 2, 50),
            ("demo.actor.dark-elf", 122, 2, 40),
            ("demo.actor.dark-elven-mage", 178, 1, 40),
            ("demo.actor.dark-elven-priest", 226, 1, 50),
            ("demo.actor.dark-elven-warrior", 182, 1, 40),
            ("demo.actor.dark-naga", 265, 2, 50),
            ("demo.actor.death-sword", 107, 5, 40),
            ("demo.actor.devilfish", 918, 4, 50),
            ("demo.actor.dimetrodon", 1223, 3, 90),
            (
                "demo.actor.disembodied-hand-that-strangled-people",
                112,
                2,
                40,
            ),
            ("demo.actor.disenchanter-eye", 104, 2, 40),
            ("demo.actor.disenchanter-mold", 192, 2, 40),
            ("demo.actor.drider", 234, 2, 50),
            ("demo.actor.druid", 241, 2, 50),
            ("demo.actor.duck", 1241, 1, 25),
            ("demo.actor.duck-quacked-platypus", 1325, 1, 36),
            ("demo.actor.dweller-on-the-threshold", 263, 5, 50),
            ("demo.actor.eagle", 172, 2, 40),
            ("demo.actor.ewok", 92, 2, 40),
            ("demo.actor.fang-farmer-maggots-dog", 55, 2, 999),
            ("demo.actor.filthy-street-urchin", 1, 2, 0),
            ("demo.actor.flesh-golem", 256, 1, 50),
            ("demo.actor.floating-eye", 32, 1, 10),
            ("demo.actor.floating-orb", 912, 2, 50),
            ("demo.actor.flying-skull", 273, 3, 50),
            ("demo.actor.freesia", 57, 1, 999),
            ("demo.actor.frosty-jelly", 84, 1, 40),
            ("demo.actor.fruit-bat", 37, 1, 10),
            ("demo.actor.frumious-bandersnatch", 232, 2, 50),
            ("demo.actor.gazer", 218, 1, 50),
            ("demo.actor.giant-black-ant", 49, 1, 20),
            ("demo.actor.giant-brown-bat", 114, 1, 40),
            ("demo.actor.giant-clear-centipede", 276, 2, 30),
            ("demo.actor.giant-cockroach", 1007, 2, 40),
            ("demo.actor.giant-flea", 259, 1, 50),
            ("demo.actor.giant-fruit-fly", 197, 6, 40),
            ("demo.actor.giant-green-frog", 56, 1, 20),
            ("demo.actor.giant-grey-rat", 156, 1, 40),
            ("demo.actor.giant-leech", 95, 1, 40),
            ("demo.actor.giant-moth", 1273, 2, 12),
            ("demo.actor.giant-octopus", 266, 2, 50),
            ("demo.actor.giant-pink-ant", 168, 2, 80),
            ("demo.actor.giant-pink-frog", 121, 1, 40),
            ("demo.actor.giant-piranha", 187, 2, 90),
            ("demo.actor.giant-salamander", 143, 1, 40),
            ("demo.actor.giant-slug", 120, 1, 40),
            ("demo.actor.giant-spider", 175, 2, 40),
            ("demo.actor.giant-tarantula", 275, 3, 60),
            ("demo.actor.giant-white-ant", 75, 1, 30),
            ("demo.actor.giant-white-centipede", 24, 1, 10),
            ("demo.actor.giant-white-dragon-fly", 250, 3, 50),
            ("demo.actor.giant-white-louse", 69, 1, 30),
            ("demo.actor.giant-white-mouse", 27, 1, 10),
            ("demo.actor.giant-white-rat", 86, 1, 40),
            ("demo.actor.giant-white-tick", 176, 2, 40),
            ("demo.actor.giant-yellow-toad", 1329, 6, 40),
            ("demo.actor.gibbering-mouther", 253, 4, 50),
            ("demo.actor.gnome-mage", 281, 2, 60),
            ("demo.actor.goblin", 87, 1, 40),
            ("demo.actor.golfimbul-the-hill-orc-chief", 215, 3, 999),
            ("demo.actor.goomba", 924, 1, 20),
            ("demo.actor.grape-jelly", 212, 3, 40),
            ("demo.actor.greater-hell-beast", 39, 6, 999),
            ("demo.actor.green-glutton-ghost", 100, 1, 40),
            ("demo.actor.green-jelly", 66, 1, 30),
            ("demo.actor.green-mold", 146, 2, 40),
            ("demo.actor.green-naga", 94, 1, 40),
            ("demo.actor.green-worm-mass", 31, 1, 10),
            ("demo.actor.gremlin", 153, 4, 40),
            ("demo.actor.grey-icky-thing", 103, 1, 40),
            ("demo.actor.grey-mold", 20, 1, 10),
            ("demo.actor.grid-bug", 34, 3, 20),
            ("demo.actor.griffon", 279, 1, 50),
            ("demo.actor.grip-farmer-maggots-dog", 53, 2, 999),
            ("demo.actor.grishnakh-the-hill-orc", 186, 3, 999),
            ("demo.actor.grizzly-bear", 191, 1, 45),
            ("demo.actor.guardian-naga", 269, 2, 50),
            ("demo.actor.hairy-mold", 190, 2, 40),
            ("demo.actor.half-orc", 264, 3, 50),
            ("demo.actor.hellcat", 222, 1, 50),
            ("demo.actor.hibagon", 983, 10, 30),
            ("demo.actor.hill-orc", 149, 1, 40),
            ("demo.actor.hippocampus", 207, 1, 40),
            ("demo.actor.hippogriff", 209, 1, 40),
            ("demo.actor.hobbes-the-tiger", 200, 2, 999),
            ("demo.actor.homonculus", 280, 3, 50),
            ("demo.actor.horse", 956, 1, 20),
            ("demo.actor.hunting-hawk-of-julian", 151, 2, 40),
            ("demo.actor.illusionist", 240, 2, 50),
            ("demo.actor.insect-swarm", 38, 1, 10),
            ("demo.actor.irish-wolfhound-of-flora", 254, 2, 50),
            ("demo.actor.ixitxachitl", 220, 1, 50),
            ("demo.actor.jackal", 35, 1, 5),
            ("demo.actor.jibaku-ghost", 1012, 2, 40),
            ("demo.actor.kamikaze-yeek", 179, 1, 40),
            ("demo.actor.killer-bee", 174, 2, 40),
            ("demo.actor.killer-brown-beetle", 236, 2, 50),
            ("demo.actor.king-cobra", 171, 2, 40),
            (
                "demo.actor.king-duosi-the-chief-of-southerings",
                1076,
                2,
                999,
            ),
            ("demo.actor.knight-archer", 219, 1, 50),
            ("demo.actor.kobold", 30, 1, 30),
            ("demo.actor.kutar", 1020, 4, 30),
            ("demo.actor.lagduf-the-snaga", 140, 2, 999),
            ("demo.actor.large-brown-snake", 28, 1, 10),
            ("demo.actor.large-grey-snake", 90, 1, 40),
            ("demo.actor.large-kobold", 102, 1, 40),
            ("demo.actor.large-white-snake", 21, 1, 10),
            ("demo.actor.large-yellow-snake", 59, 1, 20),
            ("demo.actor.lemure", 148, 3, 40),
            ("demo.actor.light-hound", 271, 2, 60),
            ("demo.actor.lion", 1321, 2, 50),
            ("demo.actor.lost-soul", 133, 2, 40),
            ("demo.actor.lousy-the-king-of-louses", 1063, 3, 999),
            ("demo.actor.lug-the-grotesque", 1183, 3, 999),
            ("demo.actor.lurker", 247, 3, 50),
            ("demo.actor.lynx", 1347, 2, 40),
            ("demo.actor.mad-bear", 1028, 1, 40),
            ("demo.actor.manes", 128, 2, 40),
            ("demo.actor.master-yeek", 224, 2, 40),
            ("demo.actor.mauhur-the-orc-captain", 1072, 3, 999),
            ("demo.actor.meng-you-the-brother-of-meng-huo", 1073, 2, 999),
            ("demo.actor.metallic-blue-centipede", 67, 1, 30),
            ("demo.actor.metallic-green-centipede", 42, 1, 20),
            ("demo.actor.metallic-red-centipede", 77, 1, 30),
            ("demo.actor.mi-go", 274, 2, 50),
            ("demo.actor.mine-dog", 221, 4, 50),
            ("demo.actor.mirkwood-spider", 277, 2, 50),
            ("demo.actor.moaning-spirit", 231, 2, 50),
            ("demo.actor.mongbat", 235, 3, 50),
            ("demo.actor.moon-beast", 223, 1, 50),
            ("demo.actor.nami-the-mate", 1021, 4, 999),
            ("demo.actor.nether-worm-mass", 213, 4, 40),
            ("demo.actor.newt", 23, 1, 10),
            ("demo.actor.nibelung", 111, 1, 40),
            ("demo.actor.night-lizard", 134, 2, 40),
            ("demo.actor.nixie", 248, 1, 50),
            ("demo.actor.novice-archaeologist", 45, 3, 30),
            ("demo.actor.novice-archer", 116, 2, 40),
            ("demo.actor.novice-mage", 93, 2, 40),
            ("demo.actor.novice-mindcrafter", 1054, 1, 50),
            ("demo.actor.novice-paladin", 147, 2, 40),
            ("demo.actor.novice-priest", 109, 2, 40),
            ("demo.actor.novice-ranger", 142, 1, 40),
            ("demo.actor.novice-rogue", 44, 1, 30),
            ("demo.actor.novice-warrior", 110, 2, 40),
            ("demo.actor.nurgling", 139, 2, 40),
            ("demo.actor.ochre-jelly", 245, 3, 50),
            ("demo.actor.ogre", 238, 2, 50),
            ("demo.actor.orc-shaman", 162, 1, 40),
            ("demo.actor.orcish-artillery", 954, 3, 40),
            ("demo.actor.orfax-son-of-boldor", 180, 3, 999),
            ("demo.actor.owlbear", 188, 1, 40),
            ("demo.actor.panther", 198, 2, 40),
            ("demo.actor.phantom-warrior", 152, 1, 40),
            ("demo.actor.pink-jelly", 131, 1, 40),
            ("demo.actor.pink-naga", 130, 2, 40),
            ("demo.actor.piranha", 70, 1, 60),
            ("demo.actor.plague-rat", 1298, 2, 40),
            ("demo.actor.plaguebearer-of-nurgle", 268, 2, 50),
            ("demo.actor.poltergeist", 65, 1, 30),
            ("demo.actor.portuguese-man-o-war", 160, 2, 40),
            ("demo.actor.priest", 225, 1, 50),
            ("demo.actor.pseudo-dragon", 193, 2, 50),
            ("demo.actor.purple-mushroom-patch", 108, 2, 40),
            ("demo.actor.quiver-slot", 185, 2, 40),
            ("demo.actor.radiant-kavu", 1071, 1, 50),
            ("demo.actor.radiation-eye", 80, 1, 30),
            ("demo.actor.rat-thing", 115, 1, 40),
            ("demo.actor.rattlesnake", 119, 1, 40),
            ("demo.actor.raven", 68, 2, 30),
            ("demo.actor.red-worm-mass", 105, 1, 40),
            ("demo.actor.robin-hood-the-outlaw", 138, 2, 999),
            ("demo.actor.rock-lizard", 33, 1, 10),
            ("demo.actor.rock-mole", 161, 2, 40),
            ("demo.actor.rotting-corpse", 125, 1, 40),
            ("demo.actor.salamander", 50, 1, 20),
            ("demo.actor.sand-dweller", 183, 1, 40),
            ("demo.actor.scruffy-looking-hobbit", 74, 1, 30),
            ("demo.actor.servant-of-glaaki", 181, 1, 40),
            ("demo.actor.shadow-creature-of-fiona", 201, 2, 40),
            ("demo.actor.shadow-hound", 272, 2, 60),
            ("demo.actor.shallow-puddle", 885, 6, 30),
            ("demo.actor.sheep", 1226, 4, 20),
            ("demo.actor.shrieker-mushroom-patch", 40, 1, 50),
            ("demo.actor.silver-jelly", 73, 2, 30),
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
            ("demo.actor.snow-leopard", 1338, 2, 50),
            ("demo.actor.software-bug", 246, 2, 90),
            ("demo.actor.soldier-ant", 36, 1, 10),
            ("demo.actor.space-monster", 144, 2, 40),
            ("demo.actor.spotted-jelly", 233, 3, 50),
            ("demo.actor.spotted-mushroom-patch", 72, 1, 30),
            ("demo.actor.swamp-rabbit", 1387, 7, 42),
            ("demo.actor.swordfish", 88, 2, 40),
            ("demo.actor.swordsman", 216, 1, 50),
            ("demo.actor.tax-collector", 199, 3, 40),
            ("demo.actor.tengu", 194, 1, 40),
            ("demo.actor.the-borshin", 177, 2, 999),
            ("demo.actor.the-ghost-q", 1003, 3, 999),
            ("demo.actor.tiger", 230, 2, 50),
            ("demo.actor.time-initiate", 1091, 3, 40),
            ("demo.actor.trench-wurm", 1070, 1, 50),
            ("demo.actor.ufthak-of-cirith-ungol", 260, 3, 999),
            ("demo.actor.undead-devilfish", 913, 4, 50),
            ("demo.actor.undead-mass", 202, 2, 40),
            ("demo.actor.unruly-horse", 957, 2, 30),
            ("demo.actor.unstable-worm-mass", 876, 4, 50),
            ("demo.actor.vlasta", 249, 3, 50),
            ("demo.actor.vorpal-bunny", 205, 3, 40),
            ("demo.actor.wallaby", 1316, 2, 30),
            ("demo.actor.war-bear", 173, 1, 40),
            ("demo.actor.warg", 257, 2, 50),
            ("demo.actor.warrens-keeper", 135, 3, 999),
            ("demo.actor.wererat", 270, 2, 50),
            ("demo.actor.white-harpy", 51, 1, 20),
            ("demo.actor.white-icky-thing", 25, 1, 10),
            ("demo.actor.white-wolf", 211, 1, 40),
            ("demo.actor.white-worm-mass", 89, 1, 40),
            ("demo.actor.wild-cat", 62, 2, 20),
            ("demo.actor.wolf", 196, 1, 40),
            ("demo.actor.wolf-farmer-maggots-dog", 54, 2, 999),
            ("demo.actor.wood-spider", 127, 3, 40),
            ("demo.actor.wormtongue-agent-of-saruman", 137, 2, 999),
            ("demo.actor.wounded-bear", 159, 1, 999),
            ("demo.actor.yellow-jelly", 48, 1, 20),
            ("demo.actor.yellow-light", 81, 1, 30),
            ("demo.actor.yellow-mold", 76, 1, 30),
            ("demo.actor.yellow-mushroom-patch", 47, 1, 20),
            ("demo.actor.yellow-worm-mass", 78, 2, 30),
            ("demo.actor.yeti", 154, 3, 40),
            ("demo.actor.zog", 98, 2, 40),
            ("demo.actor.zombified-human", 229, 1, 50),
            ("demo.actor.zombified-kobold", 123, 1, 40),
            ("demo.actor.zombified-orc", 208, 1, 40),
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
            Some("demo.loot-table.warrens")
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
        actor("demo.actor.plague-rat").contact_aura,
        Some(ActorContactAuraDefinition {
            damage_type: ActorDamageType::Poison,
            damage_dice: 1,
            damage_sides: 2,
            chance_percent: None,
        })
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
        Some("demo.loot-table.large-kobold")
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
fn outpost_has_walls_inner_shops_and_an_exterior_warrens_entrance() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.warrens-journey")
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
                position: ContentPosition { x: 28, y: 52 },
                town_id: "demo.town.outpost".to_owned(),
            },
            WildernessLocationDefinition::Dungeon {
                position: ContentPosition { x: 28, y: 52 },
                dungeon_id: "demo.dungeon.warrens".to_owned(),
            },
        ]
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
    assert!(
        world
            .procedural_floors
            .iter()
            .filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
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
    unowned_shop.towns[0]
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
        .find(|world| world.id == "demo.world.warrens-journey")
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
fn thieves_hideout_uses_the_original_fixed_map_and_formation_contract() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.warrens-journey")
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
    assert_eq!(task.reward.item_kind_id, "demo.item.broad-sword");

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
            let kind_id = content
                .shops
                .iter()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .stock[0]
                .item_kind_id
                .clone();
            content
                .items
                .iter_mut()
                .find(|item| item.id == kind_id)
                .expect("stock kind should exist")
                .base_value = 0;
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
fn supported_legacy_consumables_are_available_at_their_source_depths() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let warrens = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrens")
        .expect("Warrens loot table should exist");
    let expected = [
        ("demo.item.seeking-scroll", 0),
        ("demo.item.veil-draught", 0),
        ("demo.item.light-healing-potion", 0),
        ("demo.item.summoning-scroll", 1),
        ("demo.item.flicker-scroll", 1),
        ("demo.item.appraisal-scroll", 1),
        ("demo.item.detect-invisible-scroll", 1),
        ("demo.item.benediction-scroll", 1),
        ("demo.item.slowness-potion", 1),
        ("demo.item.boldness-potion", 1),
        ("demo.item.swiftstep-tonic", 1),
        ("demo.item.temperate-tonic", 1),
        ("demo.item.vigor-potion", 1),
        ("demo.item.valor-tonic", 1),
        ("demo.item.venom-draught", 3),
        ("demo.item.frailty-tonic", 3),
        ("demo.item.clamor-scroll", 5),
        ("demo.item.cartography-scroll", 5),
        ("demo.item.trapfinding-scroll", 5),
        ("demo.item.door-stair-location-scroll", 5),
        ("demo.item.confusing-touch-scroll", 5),
        ("demo.item.clumsiness-potion", 5),
    ];
    for (item_id, min_depth) in expected {
        let entry = warrens
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == item_id)
            .unwrap_or_else(|| panic!("{item_id} should be available in the Warrens"));
        assert_eq!((entry.min_depth, entry.max_depth), (min_depth, 9));
    }
}

#[test]
fn selected_legacy_equipment_is_exposed_by_its_shop_and_warrens_depth() {
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

    let warrens = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrens")
        .expect("Warrens loot table should exist");
    let depth = |id: &str| {
        warrens
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == id)
            .map(|entry| (entry.min_depth, entry.max_depth))
    };
    assert_eq!(depth("demo.item.club"), Some((0, 9)));
    assert_eq!(depth("demo.item.broken-dagger"), Some((0, 9)));
    assert_eq!(depth("demo.item.broken-sword"), Some((0, 9)));
    assert_eq!(depth("demo.item.dagger"), Some((0, 9)));
    assert_eq!(depth("demo.item.filthy-rag"), Some((0, 9)));
    assert_eq!(depth("demo.item.cloak"), Some((1, 9)));
    assert_eq!(depth("demo.item.robe"), Some((1, 9)));
    assert_eq!(depth("demo.item.shovel"), Some((1, 9)));
    assert_eq!(depth("demo.item.padded-armour"), Some((2, 9)));
    assert_eq!(depth("demo.item.knit-cap"), Some((3, 9)));
    assert_eq!(depth("demo.item.main-gauche"), Some((3, 9)));
    assert_eq!(depth("demo.item.pointy-hat"), Some((3, 9)));
    assert_eq!(depth("demo.item.soft-leather-armour"), Some((3, 9)));
    assert_eq!(depth("demo.item.soft-studded-leather"), Some((3, 9)));
    assert_eq!(depth("demo.item.tanto"), Some((3, 9)));
    assert_eq!(depth("demo.item.whip"), Some((3, 9)));
    assert_eq!(depth("demo.item.cord-armour"), Some((5, 9)));
    assert_eq!(depth("demo.item.cutlass"), Some((5, 9)));
    assert_eq!(depth("demo.item.hard-leather-armour"), Some((5, 9)));
    assert_eq!(depth("demo.item.rapier"), Some((5, 9)));
    assert_eq!(depth("demo.item.mace"), Some((5, 9)));
    assert_eq!(depth("demo.item.pair-of-hard-leather-boots"), Some((5, 9)));
    assert_eq!(depth("demo.item.paper-armour"), Some((5, 9)));
    assert_eq!(depth("demo.item.pick"), Some((5, 9)));
    assert_eq!(depth("demo.item.small-sword"), Some((5, 9)));
    assert_eq!(
        depth("demo.item.set-of-studded-leather-gloves"),
        Some((5, 9))
    );
    assert_eq!(depth("demo.item.metal-cap"), None);
    assert_eq!(depth("demo.item.small-metal-shield"), None);
    assert_eq!(depth("demo.item.large-leather-shield"), None);
    assert_eq!(depth("demo.item.hard-studded-leather"), None);
    assert_eq!(depth("demo.item.set-of-gauntlets"), None);
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
        BTreeSet::from(["demo.item.stench-of-death", "demo.item.sepulchral-ways",])
    );
    let values = artifact
        .content
        .items
        .iter()
        .filter(|item| shop.stock.iter().any(|stock| stock.item_kind_id == item.id))
        .map(|item| (item.id.as_str(), item.base_value))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values["demo.item.stench-of-death"], 100);
    assert_eq!(values["demo.item.sepulchral-ways"], 1_000);
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
            "demo.item.disease-mushroom",
            "demo.item.necronomicon",
            "demo.item.restore-constitution-mushroom",
            "demo.item.restore-strength-mushroom",
            "demo.item.unhealth-mushroom",
            "demo.item.invulnerability-potion",
            "demo.item.giant-strength-potion",
            "demo.item.great-clarity-potion",
            "demo.item.understanding-scroll",
            "demo.item.inventory-protection-scroll",
            "demo.item.enlightenment-potion",
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
}

#[test]
fn p3_1_items_all_have_a_shop_or_warrens_acquisition_path() {
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
        .filter(|table| {
            matches!(
                table.id.as_str(),
                "demo.loot-table.warrens" | "demo.loot-table.kobold"
            )
        })
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
        "demo.item.satisfy-hunger-scroll",
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
        .filter(|table| {
            matches!(
                table.id.as_str(),
                "demo.loot-table.warrens" | "demo.loot-table.kobold"
            )
        })
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
        .filter(|table| table.id == "demo.loot-table.warrens")
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
fn p3_6_launchers_and_ammunition_all_have_an_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_items = artifact
        .content
        .shops
        .iter()
        .flat_map(|shop| shop.stock.iter().map(|stock| stock.item_kind_id.as_str()));
    let archer_ammunition = artifact
        .content
        .loot_tables
        .iter()
        .filter(|table| table.id == "demo.loot-table.archer")
        .flat_map(|table| {
            table
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.as_str())
        });
    let available = shop_items.chain(archer_ammunition).collect::<BTreeSet<_>>();

    for item_id in [
        "demo.item.sling",
        "demo.item.long-bow",
        "demo.item.light-crossbow",
        "demo.item.heavy-crossbow",
        "demo.item.sheaf-arrow",
        "demo.item.mithril-arrow",
        "demo.item.seeker-arrow",
        "demo.item.bolt",
        "demo.item.steel-bolt",
        "demo.item.mithril-bolt",
        "demo.item.seeker-bolt",
        "demo.item.adamantine-bolt",
        "demo.item.rounded-pebble",
        "demo.item.iron-shot",
        "demo.item.mithril-shot",
    ] {
        assert!(
            available.contains(item_id),
            "{item_id} should be obtainable"
        );
    }
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
        .filter(|table| table.id == "demo.loot-table.warrens")
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
        .find(|world| world.id == "demo.world.warrens-journey")
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
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens")
        .procedural_floors[0]
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
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens")
        .procedural_floors[0]
        .guaranteed_items[0]
        .entries[0]
        .item_kind_id = "demo.item.broad-sword".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_item),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn loot_tables_require_valid_weights_references_and_instance_shapes() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_weight = artifact.content.clone();
    zero_weight
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut dangling_affix = artifact.content.clone();
    dangling_affix
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .affix_weights[1]
        .affix_id = Some("demo.affix.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_affix),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut stackable_quality = artifact.content.clone();
    stackable_quality
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .item_kind_id = "demo.item.luminous-shard".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut stackable_quality),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut player_drop = artifact.content.clone();
    let player = player_drop
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.loot_table_id = Some("demo.loot-table.ember-mote".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_drop),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut player_carry = artifact.content.clone();
    let player = player_carry
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.carried_loot_table_id = Some("demo.loot-table.ember-mote-carried".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_carry),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut invalid_chance = artifact.content.clone();
    invalid_chance
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.small-kobold")
        .expect("fixture should contain the probabilistic loot table")
        .roll_chance_percent = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid_chance),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid_dice = artifact.content.clone();
    invalid_dice
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.warrens-keeper")
        .expect("fixture should contain the Warrens keeper")
        .death_drop
        .as_mut()
        .expect("Warrens keeper should have a death drop")
        .count_dice[0]
        .sides = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_dice),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut inverted_depth = artifact.content.clone();
    let entry = &mut inverted_depth
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens")
        .expect("fixture should contain the floor loot table")
        .entries[0];
    entry.min_depth = 2;
    entry.max_depth = 1;
    assert!(matches!(
        validate_and_normalize(&mut inverted_depth),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut dangling_guardian_reward = artifact.content.clone();
    dangling_guardian_reward
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain the Warrens journey")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.final_floor)
        .and_then(|floor| floor.guardian.as_mut())
        .expect("Warrens should contain a final guardian")
        .reward_loot_table_id = Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_guardian_reward),
        Err(ContentError::DanglingReference { .. })
    ));
}

#[test]
fn procedural_floor_tables_require_valid_depth_roles_and_references() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_depth = artifact.content.clone();
    zero_depth.worlds[0].procedural_floors[0].depth = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_candidate = artifact.content.clone();
    player_candidate.encounter_tables[0].entries[0].actor_kind_id =
        "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_candidate),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut dangling_loot = artifact.content.clone();
    dangling_loot.worlds[0].procedural_floors[0].loot_table_id =
        Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_loot),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut duplicate_actor = artifact.content.clone();
    duplicate_actor.worlds[0].procedural_floors[0].encounter_table_id = None;
    duplicate_actor.worlds[0].procedural_floors[0].generation_budget = None;
    duplicate_actor.worlds[0].procedural_floors[0].nest = None;
    duplicate_actor.worlds[0].procedural_floors[0]
        .actor_spawns
        .push(ProceduralActorSpawnDefinition {
            instance_id: "demo.monster.ember-mote.1".to_owned(),
            room_id: "remote".to_owned(),
            actor_kind_ids: vec!["demo.actor.echo-hound".to_owned()],
        });
    assert!(matches!(
        validate_and_normalize(&mut duplicate_actor),
        Err(ContentError::DuplicateInstanceId(_))
    ));

    let mut zero_weight = artifact.content.clone();
    zero_weight.encounter_tables[0].entries[0].weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut missing_theme = artifact.content.clone();
    missing_theme.worlds[0].procedural_floors[0].theme_table_id =
        Some("demo.theme-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut exhausted_actor_budget = artifact.content.clone();
    exhausted_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain the nest floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .actor_slots = 3;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_loot_budget = artifact.content.clone();
    exhausted_loot_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2")
        .expect("fixture should contain the vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .loot_placements = 1;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_loot_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_spatial_budget = artifact.content.clone();
    incomplete_spatial_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-8")
        .expect("fixture should contain the spatial Vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .vault_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_spatial_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_group_budget = artifact.content.clone();
    undersized_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = Some(1);
    assert!(matches!(
        validate_and_normalize(&mut undersized_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_escort = artifact.content.clone();
    player_escort
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .and_then(|group| group.escort.as_mut())
        .expect("fixture should contain an escort table")
        .entries[0]
        .actor_kind_id = "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_escort),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut self_guarding_leader = artifact.content.clone();
    self_guarding_leader
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .expect("fixture should contain a dynamic group")
        .pack_ai
        .leader = MonsterPackBehavior::GuardLeader;
    assert!(matches!(
        validate_and_normalize(&mut self_guarding_leader),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut invalid_feature_terrain = artifact.content.clone();
    invalid_feature_terrain.terrain_feature_tables[0].entries[0].terrain_id =
        "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_feature_terrain),
        Err(ContentError::InvalidTerrainFeatureTable(_))
    ));

    let mut incomplete_feature_budget = artifact.content.clone();
    incomplete_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_feature_budget = artifact.content.clone();
    oversized_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = Some(5);
    assert!(matches!(
        validate_and_normalize(&mut oversized_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_room_budget = artifact.content.clone();
    incomplete_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_room_budget = artifact.content.clone();
    undersized_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = Some(35);
    assert!(matches!(
        validate_and_normalize(&mut undersized_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_cavern = artifact.content.clone();
    blocked_cavern.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .cavern
        .as_mut()
        .expect("fixture should contain a cavern")
        .terrain_id = "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut blocked_cavern),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_cavern_budget = artifact.content.clone();
    incomplete_cavern_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .cavern_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_cavern_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_lake_budget = artifact.content.clone();
    incomplete_lake_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the lake floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .lake_deep_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_lake_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_deep_water = artifact.content.clone();
    walkable_deep_water
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-water-deep")
        .expect("fixture should contain deep water")
        .walkable = true;
    assert!(matches!(
        validate_and_normalize(&mut walkable_deep_water),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incompatible_river = artifact.content.clone();
    incompatible_river.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the river floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .river
        .as_mut()
        .expect("fixture should contain a river")
        .shallow_terrain_id = "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut incompatible_river),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_maze_budget = artifact.content.clone();
    mismatched_maze_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .expect("fixture should contain the maze floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(126);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_maze_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut maze_with_rooms = artifact.content.clone();
    let room_geometry = maze_with_rooms.worlds[0]
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| layout.rooms.clone())
        .expect("fixture should contain room geometry");
    maze_with_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .and_then(|floor| floor.layout.as_mut())
        .expect("fixture should contain the maze-only layout")
        .rooms = Some(room_geometry);
    assert!(matches!(
        validate_and_normalize(&mut maze_with_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut room_overlay_maze = artifact.content.clone();
    let final_floor = room_overlay_maze.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the rooms floor");
    final_floor
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(127);
    final_floor
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .maze = Some(ProceduralMazeDefinition {
        width: 15,
        height: 15,
    });
    assert!(matches!(
        validate_and_normalize(&mut room_overlay_maze),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_pit_budget = artifact.content.clone();
    mismatched_pit_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .pit_actor_slots = Some(24);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_pit_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_pit_table = artifact.content.clone();
    dangling_pit_table.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .layout
        .as_mut()
        .and_then(|layout| layout.pit.as_mut())
        .expect("fixture should contain a pit")
        .encounter_table_id = "demo.encounter-table.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_pit_table),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut incomplete_destroyed_budget = artifact.content.clone();
    incomplete_destroyed_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the destroyed floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .destruction_centers = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_destroyed_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_streamer = artifact.content.clone();
    walkable_streamer
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-vein")
        .expect("fixture should contain the streamer terrain")
        .walkable = true;
    assert!(validate_and_normalize(&mut walkable_streamer).is_err());

    let mut duplicate_room_shape = artifact.content.clone();
    let shapes = &mut duplicate_room_shape.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-layout floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .rooms
        .as_mut()
        .expect("fixture should contain room geometry")
        .shapes;
    shapes[1].shape = shapes[0].shape;
    assert!(matches!(
        validate_and_normalize(&mut duplicate_room_shape),
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
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.first_mut())
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
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.first_mut())
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
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.last_mut())
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
fn region_tables_require_depth_eligible_candidates_and_composable_budgets() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    fn regional_floor(content: &mut CompiledContentV1) -> &mut ProceduralFloorDefinition {
        content.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-2")
            .expect("fixture should contain the regional floor")
    }

    let mut exhausted_depth = artifact.content.clone();
    regional_floor(&mut exhausted_depth).depth = 11;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_budget = artifact.content.clone();
    regional_floor(&mut missing_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_budget = artifact.content.clone();
    regional_floor(&mut oversized_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = Some(3);
    assert!(matches!(
        validate_and_normalize(&mut oversized_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mixed_floor_tables = artifact.content.clone();
    regional_floor(&mut mixed_floor_tables).encounter_table_id =
        Some("demo.encounter-table.resonance-descent".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut mixed_floor_tables),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut composable_features = artifact.content.clone();
    composable_features.terrain_feature_tables[0].entries[0].min_depth = 2;
    let floor = regional_floor(&mut composable_features);
    floor.terrain_feature_table_id =
        Some("demo.terrain-feature-table.resonance-hazards".to_owned());
    floor
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .feature_placements = Some(1);
    validate_and_normalize(&mut composable_features)
        .expect("regional feature, theme, vault, and connections should compose");

    let mut missing_theme = artifact.content.clone();
    missing_theme.region_tables[0].entries[0].theme_id = "demo.theme.resonance-missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::InvalidRegionTable(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    let budget = incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional group budget");
    budget.group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_special_actor_budget = artifact.content.clone();
    exhausted_special_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .actor_slots = 27;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_special_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut pit_consumes_too_many_rooms = artifact.content.clone();
    pit_consumes_too_many_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .room_placements = Some(2);
    assert!(matches!(
        validate_and_normalize(&mut pit_consumes_too_many_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn vaults_require_walkable_unique_positions_and_depth_eligible_encounters() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut blocked_member = artifact.content.clone();
    blocked_member.vaults[0].encounter_groups[0].member_positions[0] =
        ContentPosition { x: 0, y: 0 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_member),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_transform = artifact.content.clone();
    let transform = duplicate_transform.vaults[0]
        .transforms
        .first()
        .copied()
        .unwrap_or(VaultTransform::Identity);
    duplicate_transform.vaults[0].transforms = vec![transform, transform];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_transform),
        Err(ContentError::InvalidVault(_))
    ));

    let mut interior_entrance = artifact.content.clone();
    let vault = interior_entrance
        .vaults
        .iter_mut()
        .find(|vault| vault.width >= 4 && vault.height >= 4)
        .expect("fixture should contain a large Vault");
    vault.entrance_positions = vec![ContentPosition { x: 1, y: 1 }];
    assert!(matches!(
        validate_and_normalize(&mut interior_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_entrance = artifact.content.clone();
    let entrance = duplicate_entrance.vaults[0].entrance_positions[0];
    duplicate_entrance.vaults[0].entrance_positions = vec![entrance, entrance];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut disconnected_interior = artifact.content.clone();
    let vault = disconnected_interior
        .vaults
        .iter_mut()
        .find(|vault| vault.id == "demo.vault.harmonic-sepulcher")
        .expect("fixture should contain the sepulcher Vault");
    vault
        .terrain_overrides
        .iter_mut()
        .find(|terrain| terrain.terrain_id == "demo.terrain.wall")
        .expect("fixture should contain Vault walls")
        .positions
        .extend((1..5).map(|x| ContentPosition { x, y: 2 }));
    assert!(matches!(
        validate_and_normalize(&mut disconnected_interior),
        Err(ContentError::InvalidVault(_))
    ));

    let mut legacy_entrance = artifact.content.clone();
    let entrance = legacy_entrance.vaults[0].entrance_positions[0];
    legacy_entrance.vaults[0].entrance_positions.clear();
    legacy_entrance.vaults[0].entrance_position = Some(entrance);
    validate_and_normalize(&mut legacy_entrance)
        .expect("legacy single Vault entrance should normalize");
    assert_eq!(legacy_entrance.vaults[0].entrance_position, None);
    assert_eq!(legacy_entrance.vaults[0].entrance_positions, [entrance]);

    let mut theme_mismatch = artifact.content.clone();
    theme_mismatch.vaults[0].theme_id = "demo.theme.other".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut theme_mismatch),
        Err(ContentError::InvalidThemeTable(_))
    ));

    let mut no_depth_candidate = artifact.content.clone();
    for entry in &mut no_depth_candidate.vaults[0].encounter_groups[0].entries {
        entry.min_depth = 1;
        entry.max_depth = 1;
    }
    assert!(matches!(
        validate_and_normalize(&mut no_depth_candidate),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn task_definitions_require_owned_locations_and_valid_target_placements() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut outside_member = artifact.content.clone();
    outside_member.worlds[0]
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.echo-chain")
        .expect("fixture should contain the staged task")
        .objectives[1]
        .floor_id = Some("demo.floor.echo-bounty-rift".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut outside_member),
        Err(ContentError::InvalidTask(_))
    ));

    let mut non_kill_placement = artifact.content.clone();
    non_kill_placement.worlds[0]
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.echo-chain")
        .expect("fixture should contain the staged task")
        .target_placements[0]
        .objective_index = 0;
    assert!(matches!(
        validate_and_normalize(&mut non_kill_placement),
        Err(ContentError::InvalidTask(_))
    ));

    let mut wrong_owner = artifact.content.clone();
    wrong_owner.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-chain-vault-rift")
        .expect("fixture should contain the staged task floor")
        .task_id = Some("demo.task.echo-bounty".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut wrong_owner),
        Err(ContentError::InvalidTask(_))
    ));

    let mut zero_limit = artifact.content.clone();
    zero_limit.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-rift")
        .expect("fixture should contain the retakeable bounty")
        .max_retakes = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut zero_limit),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_policy = artifact.content.clone();
    mismatched_policy.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-annex-rift")
        .expect("fixture should contain the shared bounty member")
        .retake_floor_policy = RetakeFloorPolicy::PreserveFloor;
    assert!(matches!(
        validate_and_normalize(&mut mismatched_policy),
        Err(ContentError::InvalidTask(_))
    ));

    let mut dedicated_completion_exit = artifact.content.clone();
    dedicated_completion_exit
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain the Warrens journey")
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.thieves-hideout")
        .expect("fixture should contain the thieves task")
        .completion_exit_terrain_id = Some("demo.terrain.stairs-down".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dedicated_completion_exit),
        Err(ContentError::InvalidTask(_))
    ));
}

#[test]
fn pest_control_matches_the_original_warrens_contract() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.warrens-journey")
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
    assert_eq!(task.reward.item_kind_id, "demo.item.fur-cloak");

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
fn tasks_can_bind_an_existing_dungeon_depth_without_owning_the_floor() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut content = artifact.content.clone();
    content.worlds[0].tasks.push(TaskDefinition {
        id: "demo.task.depth-binding".to_owned(),
        name_key: "floor-demo-echo-depth-1-name".to_owned(),
        description_key: "terrain-demo-task-rift-description".to_owned(),
        source_facility_id: None,
        prerequisite_task_id: None,
        location: TaskLocationDefinition::DungeonDepth {
            dungeon_id: "demo.dungeon.echo-depths".to_owned(),
            depth: 2,
        },
        objectives: vec![TaskObjectiveDefinition {
            kind: TaskObjectiveKind::ClearFloor,
            floor_id: Some("demo.floor.echo-depth-2".to_owned()),
            required: 1,
            item_instance_id: None,
            item_kind_id: None,
            actor_instance_id: None,
            actor_kind_id: None,
        }],
        target_placements: Vec::new(),
        completion_exit_terrain_id: None,
        reward: TaskRewardDefinition {
            item_instance_id: "demo.task.depth-binding.reward.1".to_owned(),
            item_kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 1,
        },
    });
    validate_and_normalize(&mut content)
        .expect("existing dungeon depth should be task-addressable");

    let task = content.worlds[0]
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.depth-binding")
        .expect("test task should remain available");
    task.location = TaskLocationDefinition::DungeonDepth {
        dungeon_id: "demo.dungeon.echo-depths".to_owned(),
        depth: 99,
    };
    assert!(matches!(
        validate_and_normalize(&mut content),
        Err(ContentError::InvalidTask(_))
    ));
}

#[test]
fn dungeon_trees_require_shared_guardian_mirrors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_guardian = artifact.content.clone();
    missing_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .guardian = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut broken_chain = artifact.content.clone();
    broken_chain.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .dungeon_id = Some("demo.dungeon.other".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_chain),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut final_with_descent = artifact.content.clone();
    let final_floor = final_with_descent.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor");
    final_floor.next_floor_id = Some("demo.floor.echo-depth-1".to_owned());
    final_floor.down_stair_terrain_id = Some("demo.terrain.stairs-down".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut final_with_descent),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_guardian = artifact.content.clone();
    mismatched_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain a guardian mirror")
        .guardian
        .as_mut()
        .expect("mirror should retain a guardian")
        .actor_kind_id = "demo.actor.echo-hound".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut mismatched_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut converging_tree = artifact.content.clone();
    let child_parent = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2-mirror")
        .expect("fixture should contain the mirror branch");
    child_parent
        .connections
        .push(ProceduralFloorConnectionDefinition {
            id: "demo.connection.test.second-parent-down".to_owned(),
            kind: FloorConnectionKind::Stairs,
            terrain_id: "demo.terrain.stairs-down".to_owned(),
            target_floor_id: "demo.floor.echo-depth-3-mirror".to_owned(),
            target_connection_id: Some("demo.connection.test.second-parent-up".to_owned()),
            target_candidates: Vec::new(),
        });
    let child = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain the existing mirror final");
    child.connections.push(ProceduralFloorConnectionDefinition {
        id: "demo.connection.test.second-parent-up".to_owned(),
        kind: FloorConnectionKind::Stairs,
        terrain_id: "demo.terrain.stairs-up".to_owned(),
        target_floor_id: "demo.floor.echo-depth-2-mirror".to_owned(),
        target_connection_id: Some("demo.connection.test.second-parent-down".to_owned()),
        target_candidates: Vec::new(),
    });
    assert!(matches!(
        validate_and_normalize(&mut converging_tree),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn dungeon_entrance_guardians_and_entry_requirements_are_validated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = &artifact.content.worlds[0];
    let resonance = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.resonance-descent")
        .expect("demo should contain the resonance dungeon");
    let entrance = resonance
        .entrance_guardian
        .as_ref()
        .expect("resonance should declare an entrance guardian");
    assert_eq!(entrance.position, ContentPosition { x: 2, y: 1 });
    assert!(resonance.entry_requirements.is_empty());

    let mut zero_ttl = artifact.content.clone();
    zero_ttl.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.archive-depths")
        .expect("archive dungeon should remain available")
        .instance_lifecycle = DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 };
    assert!(matches!(
        validate_and_normalize(&mut zero_ttl),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut zero_legacy_index = artifact.content.clone();
    zero_legacy_index
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("Warrens journey should remain available")
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.warrens")
        .expect("Warrens should remain available")
        .legacy_index = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut zero_legacy_index),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_legacy_index = artifact.content.clone();
    duplicate_legacy_index.worlds[0].dungeons[0].legacy_index = Some(30);
    duplicate_legacy_index.worlds[0].dungeons[1].legacy_index = Some(30);
    assert!(matches!(
        validate_and_normalize(&mut duplicate_legacy_index),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_guardian = artifact.content.clone();
    blocked_guardian.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == resonance.id)
        .expect("resonance should remain available")
        .entrance_guardian
        .as_mut()
        .expect("entrance guardian should remain available")
        .position = ContentPosition { x: 3, y: 2 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_requirement = artifact.content.clone();
    let dungeon = duplicate_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available");
    let requirement = DungeonEntryRequirementDefinition::CarriedItem {
        item_kind_id: "demo.item.luminous-shard".to_owned(),
        quantity: 1,
    };
    dungeon.entry_requirements = vec![requirement.clone(), requirement];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_requirement = artifact.content.clone();
    dangling_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available")
        .entry_requirements = vec![DungeonEntryRequirementDefinition::TaskStatus {
        task_id: "demo.task.missing".to_owned(),
        status: DungeonEntryTaskStatus::Completed,
    }];
    assert!(matches!(
        validate_and_normalize(&mut dangling_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn floor_connections_require_reciprocal_targets_and_matching_terrain_roles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut broken_pair = artifact.content.clone();
    broken_pair.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
        .expect("fixture should contain the first downward connection")
        .target_connection_id = Some("demo.connection.echo-depth-2.up-b".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_pair),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut wrong_shaft_kind = artifact.content.clone();
    wrong_shaft_kind.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.shaft-down")
        .expect("fixture should contain the downward shaft")
        .kind = FloorConnectionKind::Stairs;
    assert!(matches!(
        validate_and_normalize(&mut wrong_shaft_kind),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_entry = artifact.content.clone();
    missing_entry.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .entry_connection_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_entry),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn door_terrain_transitions_are_reciprocal_and_match_collision() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_reciprocal = artifact.content.clone();
    missing_reciprocal
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-closed")
        .expect("fixture should contain the closed door")
        .open_to_terrain_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_reciprocal),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut blocked_open_door = artifact.content.clone();
    blocked_open_door
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-open")
        .expect("fixture should contain the open door")
        .walkable = false;
    assert!(matches!(
        validate_and_normalize(&mut blocked_open_door),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_bash = artifact.content.clone();
    incomplete_bash
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .bash_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_bash),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut invalid_lock = artifact.content.clone();
    invalid_lock
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .open_check_difficulty = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid_lock),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_concealment = artifact.content.clone();
    incomplete_concealment
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-secret")
        .expect("fixture should contain the secret door")
        .search_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_concealment),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut non_door_generator = artifact.content.clone();
    non_door_generator.worlds[0].procedural_floors[0].closed_door_terrain_id =
        "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut non_door_generator),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
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
