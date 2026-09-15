#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Generate Angband source content from RFB master; --check writes nothing.

Does not compile Rust, update the content lock, or generate protocol/schema files.
"""
import argparse
import copy
import json
from pathlib import Path
import re
import runpy
import subprocess


ROOT = Path(__file__).resolve().parents[1]
SHARED = runpy.run_path(str(ROOT / "scripts/sync-random-dungeons.py"))
DUNGEON = "demo.dungeon.angband"
ENTRANCE = "demo.terrain.angband-entrance"


def encoded(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def floor_id(depth):
    return f"demo.floor.angband-depth-{depth}"


def connection_id(depth, kind):
    return f"demo.connection.angband-{depth}-{kind}"


def birth_tasks(git, commit, pack):
    records = SHARED["source_records"]
    one, flags = SHARED["one"], SHARED["flags"]
    selection = json.loads((pack / "legacy-warrens-monster-selection.json").read_text(encoding="utf-8"))
    ids = {entry["sourceIndex"]: "demo.actor." + entry["id"] for entry in selection["monsters"]}
    # init2.c also adds GUARDIAN from dungeon definitions after parsing r_info.
    guardians = {int(tag.removeprefix("FINAL_GUARDIAN_"))
                 for dungeon in records(git("show", f"{commit}:lib/edit/d_info.txt"))
                 if "SUPPRESSED" not in flags(dungeon, "F")
                 for tag in flags(dungeon, "F") if tag.startswith("FINAL_GUARDIAN_")}
    candidates = []
    for record in records(git("show", f"{commit}:lib/edit/r_info.txt")):
        index = int(record["name"].split(":", 1)[0])
        tags = set(flags(record, "F"))
        if "UNIQUE" not in tags:
            continue
        level, rarity, maximum, *_ = map(int, one(record, "W").split(":"))
        # quest_unique=TRUE, wilderness enabled, dungeon_type=dun_level=0.
        # get_mon_num_prep excludes these before drawing; NO_QUEST is checked after.
        if not 1 <= rarity <= 100 or tags & {
            "WILD_ONLY", "AQUATIC", "MULTIPLY", "FRIENDLY", "GUARDIAN",
            "OLYMPIAN", "EGYPTIAN", "NORSE", "HINDU", "COMPOST",
        } or ("FORCE_DEPTH" in tags and level > 0) or any(tag.startswith("DUNGEON_") for tag in tags):
            continue
        if index in guardians or index in {883, 1110, 1303, 1337}:  # dead serpent, metal babble, final-depth machine, postgame
            continue
        if maximum == 0:
            continue  # get_mon_num_aux rejects max_level < selection level, including zero.
        actor_id = ids[index]  # Missing source actors are errors, never silently removed.
        actor_path = pack / "actors" / (actor_id.removeprefix("demo.actor.") + ".json")
        actor = json.loads(actor_path.read_text(encoding="utf-8"))
        if actor["id"] != actor_id or actor["level"] != level or "unique" not in actor["tags"]:
            raise ValueError(f"Source task actor identity/level changed: {actor_id}")
        candidates.append({"actorKindId": actor_id, "legacyIndex": index, "rarity": rarity,
                           "maxDepth": maximum, "canBeTarget": index != 683 and not tags & {"NO_QUEST", "UNIQUE2", "NAZGUL"}})
    candidates.sort(key=lambda entry: entry["legacyIndex"])
    names = [None if match[1] == "NULL" else json.loads(match[1]) for match in re.finditer(
        r'^\s*(NULL|"(?:[^"\\]|\\.)*")\s*,?\s*$',
        git("show", f"{commit}:src/quest_name_zh.inc"), re.MULTILINE)]
    tasks, messages, sources = [], {"zh-CN": {}, "en-US": {}}, []
    quest_records = {int(record["name"].split(":", 1)[0]): record
                     for record in records(git("show", f"{commit}:lib/edit/q_info.txt"))}
    for index in [8, 9, *range(40, 50)]:
        record = quest_records[index]
        _, depth, english = record["name"].split(":", 2)
        random = index >= 40
        expected = {"RANDOM", "RETAKE"} if random else {"RETAKE"}
        goal = "KILL(*)" if random else {8: "KILL(oberon)", 9: "KILL(serpent of chaos)"}[index]
        if set(flags(record, "T")) != expected or one(record, "W") != "angband" or one(record, "G") != goal or names[index] is None:
            raise ValueError(f"Angband task source changed or Chinese name unresolved: {index}")
        suffix = f"random-{index - 39}" if random else {8: "oberon", 9: "serpent-of-chaos"}[index]
        key = f"task-demo-angband-{suffix}"
        location = {"kind": "random-dungeon-depth" if random else "dungeon-depth", "dungeonId": DUNGEON,
                    "baseDepth" if random else "depth": int(depth)}
        objective = {"kind": "kill-actor-kind", "required": 1}
        if not random:
            objective["actorKindId"] = ids[{8: 860, 9: 862}[index]]
        tasks.append({"id": f"demo.task.angband-{suffix}", "nameKey": key + "-name",
                      "descriptionKey": key + "-description", "location": location, "objectives": [objective]})
        # Random quests have no separate source description; retain their source name.
        description = names[index]
        if not random:
            description = "".join(line[2:] for line in git("show", f"{commit}:lib/edit/{one(record, 'F')}").splitlines()
                                  if line.startswith("D:"))
        for locale, name in [("zh-CN", names[index]), ("en-US", english)]:
            messages[locale][key + "-name"] = name
            messages[locale][key + "-description"] = (name if random else description)
        sources.append({"sourceIndex": index, "sourceLine": record["line"], "sourceFields": record["fields"],
                        "taskId": tasks[-1]["id"], "chineseName": names[index]})
    return tasks, candidates, messages, sources


def boss_artifacts(git, commit, pack):
    records, one, flags = SHARED["source_records"], SHARED["one"], SHARED["flags"]
    def names(path):
        return [None if m[1] == "NULL" else json.loads(m[1]) for m in re.finditer(
            r'^\s*(NULL|"(?:[^"\\]|\\.)*")\s*,?\s*$', git("show", f"{commit}:{path}"), re.MULTILINE)]
    kind_names, artifact_names = names("src/kind_name_zh.inc"), names("src/artifact_name_zh.inc")
    kinds, index = {}, -1
    for record in records(git("show", f"{commit}:lib/edit/k_info.txt")):
        number = record["name"].split(":", 1)[0]
        index = index + 1 if number == "*" else int(number)
        kinds[index] = record
    artifacts = {int(r["name"].split(":", 1)[0]): r for r in records(git("show", f"{commit}:lib/edit/a_info.txt"))}
    outputs, sources = {}, []
    messages = {"zh-CN": {}, "en-US": {}}
    for base_index, art_index, base_id, art_id, slot, glyph, tag in [
        (607, 3, "jewel", "jewel-of-judgement", "light", "*", "light-source"),
        (223, 34, "massive-iron-crown", "crown-of-chaos", "head", "]", "armor"),
        (138, 111, "mighty-hammer", "grond", "weapon", "\\", "weapon"),
    ]:
        base, art = kinds[base_index], artifacts[art_index]
        tval, sval, pval = map(int, one(base, "I").split(":"))
        art_tval, art_sval, art_pval = map(int, one(art, "I").split(":"))
        if (tval, sval) != (art_tval, art_sval) or "INSTA_ART" not in flags(base, "F"):
            raise ValueError(f"Angband artifact base changed: {art_index}")
        base_zh = re.sub(r'^& .*?~', '', kind_names[base_index])
        art_zh = artifact_names[art_index]
        if not base_zh or not art_zh:
            raise ValueError(f"Unresolved Angband artifact name: {art_index}")
        full_zh = art_zh + base_zh if art_zh.endswith("之") else base_zh + " " + art_zh
        for source, suffix, artifact in [(base, base_id, False), (art, art_id, True)]:
            weight = list(map(int, one(source, "W").split(":")))
            armor, dice, to_hit, to_damage, to_armor = one(source, "P").split(":")
            source_flags = set(flags(source, "F"))
            if artifact:
                source_flags.update(["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"])
            item = {"$schema": f"{SHARED['SCHEMA']}/item.schema.json", "formatVersion": 1,
                    "id": f"demo.item.{suffix}", "nameKey": f"item-demo-{suffix}-name",
                    "descriptionKey": f"item-demo-{suffix}-description", "glyph": glyph,
                    "generationLevel": weight[0], "weightTenthsPound": weight[-2], "baseValue": weight[-1],
                    "maxStack": 1, "equipmentSlot": slot,
                    "tags": ["equipment", tag, "artifact" if artifact else "instant-artifact-base"],
                    "rfbValue": {"pval": art_pval if artifact else pval, "toArmor": int(to_armor), "flags": sorted(source_flags)}}
            if slot == "weapon":
                count, sides = map(int, dice.split("d"))
                item.update(meleeProfile={"attacks": 1, "damageDice": count, "damageSides": sides,
                                          "toHit": int(to_hit), "toDamage": int(to_damage)}, ridingWeaponKind="compatible")
            if artifact:
                item.update(artifactGeneration={"sourceIndex": art_index, "baseItemKindId": f"demo.item.{base_id}",
                                                "instant": True, "rarityOneIn": weight[1]},
                            resistsMonsterDestruction=True, resistsProjectionDestruction=True,
                            elementalDestructionImmunities=["acid", "electricity", "fire", "cold"])
                if art_index == 3:
                    item.update(modifiers={"intelligence": art_pval, "wisdom": art_pval, "speed": art_pval},
                                passives=["see-invisible", "hold-life"],
                                resistances={"confusion": "resistant", "chaos": "resistant"},
                                equipmentBonuses={"lightRadius": 3},
                                deviceGeneration={"recovery": {"intervalTicks": 300, "energyPerMille": 1000},
                                    "activations": [{"id": "demo.item-activation.jewel", "nameKey": "item-activation-demo-jewel-name",
                                        "effectProgramId": "demo.effect.jewel", "deviceCheckDifficulty": 50, "rfbValue": 10000,
                                        "weight": 1, "minDepth": 1, "maxDepth": 100,
                                        "charges": {"minimum": 1, "maximum": 1, "cost": 1},
                                        "target": {"modes": ["self"], "range": 0, "requiresLineOfEffect": False}}]})
                elif art_index == 34:
                    item.update(modifiers={stat: art_pval for stat in ["strength", "intelligence", "wisdom", "dexterity", "constitution", "charisma"]},
                                equipmentBonuses={"infravision": art_pval, "lightRadius": 1},
                                initialCurse="permanent", passives=["see-invisible", "telepathy", "anti-teleport"],
                                resistances={element: "resistant" for element in ["acid", "electricity", "fire", "cold", "poison", "light", "dark", "confusion", "nexus"]})
                else:
                    item.update(modifiers={"defense": int(armor) + int(to_armor)},
                                passives=["anti-magic", "see-invisible", "telepathy"],
                                resistances={element: "resistant" for element in ["acid", "electricity", "fire", "cold"]},
                                slays={**{target: "slay" for target in ["animal", "evil", "undead", "demon", "troll", "orc", "human"]}, "dragon": "kill"})
            else:
                item["rfbBaseKind"] = {"sourceIndex": base_index, "tval": tval, "sval": sval}
            outputs[pack / f"items/{suffix}.json"] = encoded(item)
            english = source["name"].split(":", 1)[1].replace("& ", "").replace("~", "")
            if artifact:
                english = base["name"].split(":", 1)[1].replace("& ", "").replace("~", "") + " " + english
            for locale, name in [("zh-CN", full_zh if artifact else base_zh), ("en-US", english)]:
                messages[locale][item["nameKey"]] = name
                messages[locale][item["descriptionKey"]] = "".join(source["fields"].get("D", [])) or name
        sources.append({"sourceFile": "lib/edit/a_info.txt", "sourceIndex": art_index, "sourceLine": art["line"], "sourceFields": art["fields"],
                        "itemKindId": f"demo.item.{art_id}", "chineseName": full_zh,
                        "baseSourceFile": "lib/edit/k_info.txt", "baseSourceIndex": base_index, "baseSourceFields": base["fields"]})
    outputs[pack / "effectPrograms/jewel.json"] = encoded({"$schema": f"{SHARED['SCHEMA']}/effect-program.schema.json",
        "formatVersion": 1, "id": "demo.effect.jewel", "input": "self", "steps": [{"type": "jewel"}]})
    for locale in messages:
        messages[locale]["item-activation-demo-jewel-name"] = "透视与召回" if locale == "zh-CN" else "Clairvoyance and Recall"
    sources.append({"sourceIndex": 42, "itemKindId": "demo.item.amber", "sourceFields": artifacts[42]["fields"], "reused": True})
    # Source monster summons do not time out. Zero uses the shared persistent
    # summon identity, preserving the caster and source ability through saves.
    for actor_name in ["oberon-king-of-amber", "the-serpent-of-chaos"]:
        actor = json.loads((pack / f"actors/{actor_name}.json").read_text(encoding="utf-8"))
        for entry in actor["monsterCasting"]["abilities"]:
            ability_path = pack / "abilities" / (entry["abilityId"].split(".ability.")[1] + ".json")
            ability = json.loads(ability_path.read_text(encoding="utf-8"))
            program_path = pack / "abilityPrograms" / (ability["abilityProgramId"].split(".ability-program.")[1] + ".json")
            program = json.loads(program_path.read_text(encoding="utf-8"))
            if len(program["steps"]) == 1 and program["steps"][0]["type"] == "summon-category":
                program["steps"][0]["durationTurns"] = 0
                outputs[program_path] = encoded(program)
            if entry["abilityId"] == "rfb-legacy.ability.ball-shards-1d1-599":
                program["steps"][0]["damageType"] = "rocket"
                outputs[program_path] = encoded(program)
    selection_path = pack / "legacy-item-selection.json"
    selection_text = selection_path.read_text(encoding="utf-8")
    selected = {entry["sourceIndex"]: entry["id"] for entry in json.loads(selection_text)["items"]}
    for index, suffix in [(607, "jewel"), (223, "massive-iron-crown"), (138, "mighty-hammer")]:
        if index not in selected:
            selection_text = selection_text.replace('"items": [', '"items": [\n    ' + json.dumps({"sourceIndex": index, "id": suffix}) + ',', 1)
        elif selected[index] != suffix:
            raise ValueError(f"Angband artifact base ID conflicts with source selection: {index}")
    outputs[selection_path] = selection_text
    return outputs, messages, sources


def sync(source, root, check):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(source), *args]).decode("utf-8")

    commit = git("rev-parse", "master").strip()
    record = next(record for record in SHARED["source_records"](git("show", f"{commit}:lib/edit/d_info.txt"))
                  if record["name"].split(":", 1)[0] == "1")
    one = SHARED["one"]
    info = [int(value, 0) for value in one(record, "W").split(":")]
    flags = SHARED["flags"](record, "F")
    if len(info) != 10 or info[:2] != [1, 127] or info[3] != 0 or set(flags) != {"MONSTER_DIV_64", "COFFEE"}:
        raise ValueError("Angband source profile changed; review the new source before importing")
    names = [json.loads(match[1]) for match in re.finditer(
        r'^\s*("(?:[^"\\]|\\.)*")\s*,?\s*$',
        git("show", f"{commit}:src/dungeon_name_zh.inc"), re.MULTILINE)]
    chinese = names[1]
    y, x = map(int, one(record, "P").split(":"))
    floor_terrain, floor_mix = SHARED["material_mix"](one(record, "L"))
    wall_terrain, wall_mix = SHARED["material_mix"](one(record, "A"))
    outer_wall = SHARED["terrain_id"](one(record, "A").split(":")[6])
    pack = root / "packs/rfb-demo-original"
    world_path = pack / "worlds/middle-earth.json"
    world_text = world_path.read_text(encoding="utf-8")
    world = json.loads(world_text)
    common = next(floor for floor in world["proceduralFloors"] if floor["id"] == "demo.floor.asgard-depth-64")
    rooms = copy.deepcopy(common["layout"]["rooms"])
    rooms["shapes"] = [{"shape": "rectangle", "weight": 1}]
    streamer_ids = [SHARED["terrain_id"](tag) for tag in one(record, "A").split(":")[8:10]]
    streamers = [copy.deepcopy(next(streamer for streamer in common["layout"]["streamers"]
                                   if streamer["terrainId"] == terrain)) for terrain in streamer_ids]
    floors = []
    for depth in range(info[0], info[1] + 1):
        returning = floor_id(depth - 1) if depth > info[0] else "demo.floor.surface"
        connections = [{"id": connection_id(depth, "stairs-up"), "kind": "stairs",
                        "terrainId": "demo.terrain.stairs-up", "targetFloorId": returning}]
        if depth > info[0]:
            connections[0]["targetConnectionId"] = connection_id(depth - 1, "stairs-down")
        if depth < info[1]:
            connections.append({"id": connection_id(depth, "stairs-down"), "kind": "stairs",
                                "terrainId": "demo.terrain.stairs-down", "targetFloorId": floor_id(depth + 1),
                                "targetConnectionId": connection_id(depth + 1, "stairs-up")})
        for delta, direction, opposite in [(-2, "up", "down"), (2, "down", "up")]:
            target = depth + delta
            if info[0] <= target <= info[1]:
                connections.append({"id": connection_id(depth, f"shaft-{direction}"), "kind": "shaft",
                                    "terrainId": f"demo.terrain.shaft-{direction}", "targetFloorId": floor_id(target),
                                    "targetConnectionId": connection_id(target, f"shaft-{opposite}")})
        floor = {
            "id": floor_id(depth), "nameKey": "floor-demo-angband-depth-name",
            "returnFloorId": returning, "lifecycle": "dungeon", "dungeonId": DUNGEON,
            "finalFloor": depth == info[1], "encounterTableId": "demo.encounter-table.angband",
            "lootTableId": "demo.loot-table.base-items", "lootAllocation": common["lootAllocation"],
            "goldAllocation": common["goldAllocation"],
            "generationBudget": {"actorSlots": info[4], "lootPlacements": 8, "roomPlacements": 6,
                                 "roomAreaTiles": 1100, "streamerPlacements": 2, "streamerAreaTiles": 32},
            "layout": {"rooms": rooms, "floorMix": floor_mix, "wallMix": wall_mix,
                       "streamers": streamers, "placeDoors": True},
            "depth": depth, "width": 96, "height": 33, "wallTerrainId": wall_terrain,
            "floorTerrainId": floor_terrain, "upStairTerrainId": "demo.terrain.stairs-up",
            "closedDoorTerrainId": "demo.terrain.door-secret", "trapTerrainId": "demo.terrain.warren-snare",
            "actorSpawns": [], "lootSpawns": [], "entryTerrainId": ENTRANCE, "connections": connections,
        }
        if depth == info[0]:
            floor["entryConnectionId"] = connection_id(depth, "stairs-up")
        if depth < info[1]:
            floor.update(nextFloorId=floor_id(depth + 1), downStairTerrainId="demo.terrain.stairs-down")
        floors.append(floor)
    dungeon = {"id": DUNGEON, "legacyIndex": 1, "rootFloorId": floor_id(info[0]),
               "tunnelPercent": int(one(record, "L").split(":")[6]), "outerWallTerrainId": outer_wall,
               "lootQualityPolicy": {"kind": "rfb-depth", "goodCapPercent": info[6], "greatCapPercent": info[7]}}
    old_floors = {floor["id"] for floor in world["proceduralFloors"] if floor.get("dungeonId") == DUNGEON}
    world_text = SHARED["merge_world_array"](world_text, "dungeons", {DUNGEON}, [dungeon])
    world_text = SHARED["merge_world_array"](world_text, "proceduralFloors", old_floors, floors)
    tasks, candidates, task_messages, task_sources = birth_tasks(git, commit, pack)
    world_text = SHARED["merge_world_array"](world_text, "tasks", {task["id"] for task in tasks}, tasks)
    candidate_json = encoded(candidates).rstrip().replace("\n", "\n  ")
    match = re.search(r'^  "randomTaskCandidates": ', world_text, re.MULTILINE)
    if match:
        _, end = json.JSONDecoder().raw_decode(world_text, match.end())
        world_text = world_text[:match.end()] + candidate_json + world_text[end:]
    else:
        world_text = world_text.replace('  "tasks": [', '  "randomTaskCandidates": ' + candidate_json + ',\n  "tasks": [', 1)
    locations = [location for location in world["wilderness"]["locations"] if location.get("dungeonId") != DUNGEON]
    locations.append({"dungeonId": DUNGEON, "kind": "dungeon", "position": {"x": x, "y": y}})
    start = re.search(r'^    "locations": ', world_text, re.MULTILINE).end()
    _, end = json.JSONDecoder().raw_decode(world_text, start)
    world_text = world_text[:start] + encoded(locations).rstrip().replace("\n", "\n    ") + world_text[end:]
    outputs, artifact_messages, artifact_sources = boss_artifacts(git, commit, pack)
    world_text = re.sub(r'"victoryTaskIds": \[.*?\]', '"victoryTaskIds": ["demo.task.angband-serpent-of-chaos"]', world_text)
    outputs[world_path] = world_text
    schema = SHARED["SCHEMA"]
    outputs[pack / "terrain/angband-entrance.json"] = encoded({
        "$schema": f"{schema}/terrain.schema.json", "formatVersion": 1, "id": ENTRANCE,
        "nameKey": "terrain-demo-angband-entrance-name", "descriptionKey": "terrain-demo-angband-entrance-description",
        "glyph": ">", "walkable": True, "blocksSight": False, "tags": ["stairs-down", "passage"]})
    outputs[pack / "encounterTables/angband.json"] = encoded({
        "$schema": f"{schema}/encounter-table.schema.json", "formatVersion": 1,
        "id": "demo.encounter-table.angband", "rolls": max(1, info[4] * 96 * 33 // (198 * 66)), "entries": [],
        "globalAllocation": {"preferredGlyphs": [], "specialDiv": 64, "ambientChanceOneIn": info[5]}})
    selection_path = pack / "legacy-wilderness-selection.json"
    selection_text = selection_path.read_text(encoding="utf-8")
    selection = json.loads(selection_text)
    chosen = {"sourceIndex": 1, "sourceName": record["name"].split(":", 1)[1], "id": DUNGEON}
    existing = next((entry for entry in selection["dungeons"] if entry["sourceIndex"] == 1), None)
    if existing is None:
        selection_text = selection_text.replace('"dungeons": [', '"dungeons": [\n    ' + json.dumps(chosen) + ',', 1)
    elif existing != chosen:
        raise ValueError("Angband source selection conflicts with its stable ID")
    outputs[selection_path] = selection_text
    for locale, name in [("zh-CN", chinese), ("en-US", "Angband")]:
        path = root / f"locales/{locale}/content.ftl"
        text = path.read_text(encoding="utf-8")
        for key, value in [("floor-demo-angband-depth-name", name), ("terrain-demo-angband-entrance-name", name),
                           ("terrain-demo-angband-entrance-description", one(record, "D")), *task_messages[locale].items(), *artifact_messages[locale].items()]:
            entry = f"{key} = {value}"
            pattern = rf"^{re.escape(key)} = .*?$"
            text = re.sub(pattern, lambda _: entry, text, flags=re.MULTILINE) if re.search(pattern, text, re.MULTILINE) else text.rstrip() + "\n" + entry + "\n"
        outputs[path] = text
    outputs[pack / "legacy-angband-source.json"] = encoded({
        "schemaVersion": 1, "sourceCommit": commit, "sourceFile": "lib/edit/d_info.txt", "sourceLine": record["line"],
        "sourceIndex": 1, "sourceName": "Angband", "chineseName": chinese, "position": {"x": x, "y": y},
        "sourceFields": record["fields"], "dungeonId": DUNGEON, "depthCount": len(floors),
        "tasks": task_sources, "birthCandidateCount": len(candidates),
        "artifacts": artifact_sources,
        "campaignSourceFiles": ["src/quest.c:quest_complete (MON_SERPENT victory and fame +50)", "src/files.c:do_cmd_suicide (winner retirement)"],
        "bossSourceFiles": ["lib/edit/r_info.txt:860,862", "src/xtra2.c:monster_death", "src/monspell.c:_ball", "src/devices.c:EFFECT_JEWEL", "src/dungeon.c:process_world"],
        "bossAdaptations": [
            "Existing bounded monster combat, area geometry, speed/armor and resistance tiers remain in use; these are not a bit-identical upstream simulation.",
            "ROCKET(600) retains its stable legacy ability ID but uses Rocket damage, creature collision, shards resistance and sound/shards status saves.",
            "The twelve shared summon programs used by the bosses have no lifetime timeout. Existing bounded count/category/maximum-level selection and friendly-risk policy remain; source weighted summon_specific allocation and escorts are not newly reproduced here.",
            "Fixed task targets are no_pet. Chosen drops reject current pets, player-owned summons and dead-unique resurrections. Source town arena/battle and clone modes are unavailable; there is no general WASPET history for arbitrary externally prepared actors.",
            "Jewel recall choice is carried in UseJewel before activation; failed or declined recall preserves clairvoyance and life loss. Generic UseItem declines recall. Source superstealth is unavailable.",
            "Jewel periodic drain is checked every ten local world ticks while equipped and outside anti-magic; the otherwise unused global one_in_999 draw is not consumed when it cannot affect the player."],
        "birthAllocationSource": ["lib/edit/r_info.txt", "src/init2.c:d_info guardian initialization", "src/quest.c:_get_questor/quests_on_birth",
                                  "src/monster2.c:get_mon_num_prep/get_mon_num_aux", "src/tables.c:quest_unique"],
        "adaptations": [
            "96x33 maps and six bounded rectangle rooms reuse existing finite geometry; full source room/pit/nest templates are not reproduced.",
            "The source minimum allocation count is scaled from 198x66 to 96x33 (14 to 3 initial rolls), with a 14 actor-slot budget; source ambient chance and MONSTER_DIV_64 use existing global allocation.",
            "Magma/quartz use the shared finite two-streamer budget and existing hidden/known treasure rates. Full source streamer length/density is not reproduced.",
            "Ordinary stairs and explicit two-depth shafts use the existing saved return-connection system; source random shaft frequency is not reproduced.",
            "min_plev=30 and COFFEE are retained as source metadata, not a new entry level gate or new game mode.",
            "AG3 imports the unique-only birth pool including fixed-placement actors; NO_QUEST and Utgard-Loke stay weighted and are rejected after drawing. FORCE_DEPTH, guardian, wilderness, dungeon and pantheon birth exclusions follow source dungeon_type=dun_level=0.",
            "Source draws use the shared RfbRng algorithm, not upstream's RNG bitstream. Ten random assignments precede fixed boss reservations.",
            "Formal victory requires the completed Serpent quest and successful objective progress. Existing post-victory level/attribute extensions and surface/town-only retirement are retained adaptations; upstream allows winner retirement through its suicide command.",
            "Upstream terms retained by NOTICE and LICENSES/RFB-UPSTREAM-NOTICE.txt."]})
    terrain_ids = {json.loads(path.read_text(encoding="utf-8"))["id"] for path in (pack / "terrain").glob("*.json")} | {ENTRANCE}
    required = {floor_terrain, wall_terrain, outer_wall, *streamer_ids}
    if not required <= terrain_ids:
        raise ValueError(f"Missing source terrain: {required - terrain_ids}")
    changed = [path for path, text in outputs.items() if not path.exists() or path.read_text(encoding="utf-8") != text]
    if check and changed:
        raise SystemExit("Angband content is stale: " + ", ".join(str(path.relative_to(root)) for path in changed))
    if not check:
        for path in changed:
            path.write_text(outputs[path], encoding="utf-8", newline="\n")
    print(f"{'Checked' if check else 'Synced'} Angband: {len(floors)} depths, entry ({x},{y}), source {commit}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    sync(args.source, args.root, args.check)
