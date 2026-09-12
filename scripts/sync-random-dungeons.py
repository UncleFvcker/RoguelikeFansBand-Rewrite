#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Sync the four DF1_RANDOM content profiles from RFB master Git objects.

This writes source content only. It does not compile, test, refresh the content
lock, generate schemas, or place entrances in the playable wilderness.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1"
KINDS = {26: "random-forest", 27: "random-volcano", 28: "random-mountain", 29: "random-sea"}
TERRAINS = {
    "FLOOR": "floor", "GRANITE": "wall", "GRASS": "surface-grass",
    "FLOWER": "surface-flower", "TREE": "surface-tree", "BRAKE": "surface-brake",
    "DIRT": "dirt", "SHALLOW_WATER": "surface-water-shallow",
    "DEEP_WATER": "surface-water-deep", "SHALLOW_LAVA": "surface-lava-shallow",
    "DEEP_LAVA": "surface-lava-deep", "MOUNTAIN_WALL": "mountain-wall",
    "MOUNTAIN": "surface-mountain", "DARK_PIT": "dark-pit", "ICE_FLOOR": "ice-floor",
    "MAGMA_VEIN": "magma-vein", "QUARTZ_VEIN": "quartz-vein",
}
WILD_TERRAINS = {
    "LAVA": "deep-lava", "MOUNTAIN": "mountain", "TREES": "trees",
    "WATER": "deep-water", "SNOW": "snow",
}


def terrain_id(tag):
    return "demo.terrain." + TERRAINS[tag]


def source_records(text):
    records = []
    current = None
    for line_number, line in enumerate(text.splitlines(), 1):
        if line.startswith("N:"):
            current = {"name": line[2:], "line": line_number, "fields": {}}
            records.append(current)
        elif current is not None and len(line) >= 2 and line[1] == ":":
            current["fields"].setdefault(line[0], []).append(line[2:])
    return records


def one(record, field):
    values = record["fields"][field]
    if len(values) != 1:
        raise ValueError(f"{record['name']}: expected exactly one {field} line")
    return values[0]


def flags(record, field):
    return [flag.strip() for line in record["fields"].get(field, [])
            for flag in line.split("|") if flag.strip()]


def material_mix(line):
    fields = line.split(":")
    pairs = [(terrain_id(fields[i]), int(fields[i + 1])) for i in range(0, 6, 2)]
    if sum(weight for _, weight in pairs) != 100:
        raise ValueError("Source material percentages must sum to 100")
    base = pairs[0][0]
    return base, [{"terrainId": terrain, "percent": weight}
                  for terrain, weight in pairs if weight and terrain != base]


def monster_policy(record, world_info, dungeon_flags):
    policy = {"preferredGlyphs": [], "preferredTags": [], "preferredMovementModes": [],
              "preferredHabitats": [], "preferredDamageImmunities": []}
    for flag in flags(record, "M"):
        if flag.startswith("R_CHAR_"):
            policy["preferredGlyphs"].extend(flag[7:])
        elif flag.startswith("WILD_"):
            policy["preferredHabitats"].append(flag[5:].lower())
        elif flag in {"ANIMAL", "TROLL", "GIANT"}:
            policy["preferredTags"].append(flag.lower())
        elif flag in {"CAN_FLY", "CAN_SWIM", "AQUATIC"}:
            policy["preferredMovementModes"].append(
                {"CAN_FLY": "fly", "CAN_SWIM": "swim", "AQUATIC": "aquatic"}[flag])
        elif flag == "IM_FIRE":
            policy["preferredDamageImmunities"].append("fire")
        else:
            raise ValueError(f"Unmapped monster flag: {flag}")
    policy = {key: sorted(set(values)) for key, values in policy.items()
              if values or key == "preferredGlyphs"}
    policy["specialDiv"] = int(next(flag[12:] for flag in dungeon_flags
                                    if flag.startswith("MONSTER_DIV_")))
    policy["ambientChanceOneIn"] = world_info[5]
    return policy


def dungeon_content(record, names, common_floor):
    index, english = record["name"].split(":", 1)
    index = int(index)
    kind = KINDS[index]
    info = [int(value, 0) for value in one(record, "W").split(":")]
    dungeon_flags = flags(record, "F")
    supported = {"RANDOM", "CAVE", "CAVERN", "BIG", "NO_DOORS", "WATER_RIVER",
                 "LAVA_RIVER", "LAKE_LAVA", "LAKE_WATER", "DESTROY"}
    if len(info) != 10 or info[3] != 3 or "RANDOM" not in dungeon_flags:
        raise ValueError(f"{kind}: expected DF1_RANDOM and MODE_OR")
    for flag in dungeon_flags:
        if flag not in supported and not flag.startswith(("MONSTER_DIV_", "WILD_TYPE_")):
            raise ValueError(f"{kind}: unmapped dungeon flag {flag}")
    chinese = names[index]
    wild_types = [int(flag[10:]) for flag in dungeon_flags if flag.startswith("WILD_TYPE_")]
    if len(wild_types) != 1:
        raise ValueError(f"{kind}: expected one WILD_TYPE")
    wild_terrain = {2: "deep-water", 7: "trees", 10: "deep-lava", 11: "mountain"}[wild_types[0]]
    floor_base, floor_mix = material_mix(one(record, "L"))
    wall_base, wall_mix = material_mix(one(record, "A"))
    floor = {
        "nameKey": f"floor-demo-{kind}-depth-name", "returnFloorId": "demo.floor.surface",
        "lifecycle": "dungeon", "dungeonId": f"demo.dungeon.{kind}",
        "encounterTableId": f"demo.encounter-table.{kind}",
        "lootTableId": "demo.loot-table.base-items",
        "lootAllocation": common_floor["lootAllocation"],
        "goldAllocation": common_floor["goldAllocation"],
        "width": 198 if "BIG" in dungeon_flags else 96,
        "height": 66 if "BIG" in dungeon_flags else 33,
        "wallTerrainId": wall_base, "floorTerrainId": floor_base,
        "upStairTerrainId": "demo.terrain.stairs-up",
        "closedDoorTerrainId": "demo.terrain.door-secret",
        "trapTerrainId": "demo.terrain.warren-snare",
        "entryTerrainId": f"demo.terrain.{kind}-entrance",
        "actorSpawns": [], "lootSpawns": [],
    }
    # Reuse the formal room geometry and finite budgets. These are Rewrite
    # adaptations, not the original MAX_HGT/MAX_WID population algorithm.
    rooms = json.loads(json.dumps(common_floor["layout"]["rooms"]))
    if "CAVE" not in dungeon_flags:
        rooms["shapes"] = [{"shape": "rectangle", "weight": 1}]
    if "WILD_TYPE_7" in dungeon_flags:
        rooms.update(minWidth=7, maxWidth=15, minHeight=7, maxHeight=15)
        rooms["shapes"] = [{"shape": "circle", "weight": 5}, {"shape": "cavern", "weight": 1}]
    budget = {"actorSlots": info[4], "lootPlacements": 8,
              "roomPlacements": 6, "roomAreaTiles": 1100}
    layout = {"rooms": rooms, "floorMix": floor_mix, "wallMix": wall_mix,
              "placeDoors": "NO_DOORS" not in dungeon_flags,
              "stairs": {"up": {"minimum": 1, "maximum": 2}}}
    if "CAVERN" in dungeon_flags:
        layout["cavern"] = {"terrainId": floor_base, "rfbDepthChance": True}
        budget["cavernAreaTiles"] = 600
    if "WATER_RIVER" in dungeon_flags or "LAVA_RIVER" in dungeon_flags:
        water = "WATER_RIVER" in dungeon_flags
        layout["river"] = {"deepTerrainId": terrain_id("DEEP_WATER" if water else "DEEP_LAVA"),
                           "shallowTerrainId": terrain_id("SHALLOW_WATER" if water else "SHALLOW_LAVA"),
                           "chanceOneIn": 7}
        if water:
            layout["river"]["rfbDepthChance"] = True
        budget["riverAreaTiles"] = 240
    if "LAKE_WATER" in dungeon_flags or "LAKE_LAVA" in dungeon_flags:
        water = "LAKE_WATER" in dungeon_flags
        layout["lake"] = {"deepTerrainId": terrain_id("DEEP_WATER" if water else "DEEP_LAVA"),
                          "shallowTerrainId": terrain_id("SHALLOW_WATER" if water else "SHALLOW_LAVA")}
        budget.update(lakeAreaTiles=400, lakeDeepAreaTiles=120)
    if "DESTROY" in dungeon_flags:
        layout["destroyed"] = {"terrainId": "demo.terrain.rubble"}
        budget.update(destructionCenters=2, destroyedAreaTiles=160)
    streams = one(record, "A").split(":")[8:]
    layout["streamers"] = []
    for tag in streams:
        if tag == "NONE":
            continue
        candidate = {"terrainId": terrain_id(tag), "weight": 1}
        existing = next((item for item in common_floor["layout"]["streamers"]
                         if item["terrainId"] == candidate["terrainId"]), None)
        if existing:
            candidate = existing.copy()
        layout["streamers"].append(candidate)
    if layout["streamers"]:
        budget.update(streamerPlacements=2, streamerAreaTiles=32)
    floor.update(generationBudget=budget, layout=layout)
    floors = [dict(floor, id=f"demo.floor.{kind}-depth-{depth}", depth=depth)
              for depth in range(info[0], info[1] + 1)]
    dungeon = {"id": floor["dungeonId"], "random": True, "legacyIndex": index,
               "rootFloorId": floors[0]["id"], "instanceLifecycle": {"kind": "reset-on-surface"},
               "lootQualityPolicy": {"kind": "rfb-depth", "goodCapPercent": info[6], "greatCapPercent": info[7]},
               "tunnelPercent": int(one(record, "L").split(":")[6]),
               "outerWallTerrainId": terrain_id(one(record, "A").split(":")[6]),
               "wildernessTerrain": wild_terrain}
    encounter = {"$schema": f"{SCHEMA}/encounter-table.schema.json", "formatVersion": 1,
                 "id": floor["encounterTableId"], "rolls": info[4],
                 "globalAllocation": monster_policy(record, info, dungeon_flags), "entries": []}
    entrance = {"$schema": f"{SCHEMA}/terrain.schema.json", "formatVersion": 1,
                "id": floor["entryTerrainId"], "nameKey": f"terrain-demo-{kind}-entrance-name",
                "descriptionKey": f"terrain-demo-{kind}-entrance-description", "glyph": ">",
                "walkable": True, "blocksSight": False, "tags": ["stairs-down", "passage"]}
    audit = {"sourceIndex": index, "sourceName": english, "chineseName": chinese,
             "sourceLine": record["line"], "sourceFields": record["fields"],
             "dungeonId": dungeon["id"], "floorCount": len(floors),
             "parameterStatus": {
                 "W.depth": "formal member depths", "W.mode": "existing MODE_OR allocation",
                 "W.minPlayerLevel": "no active entry consumer in master; not an entry level gate",
                 "W.minAlloc": "fixed population budget adaptation", "W.ambient": "existing allocation",
                 "W.goodGreat": "dungeon lootQualityPolicy applied through LootContext floor identity",
                 "W.pitNest": "template monster-filter masks; full source room templates are outside the finite geometry adaptation, no pit/nest rooms declared",
                 "L.materials": "existing floorMix", "L.tunnelPercent": "dungeon tunnelPercent chooses existing L or cave-like corridor",
                 "A.materials": "existing wallMix", "A.outer": "outerWallTerrainId paints room perimeters before tunnels",
                 "A.inner": "no inner-wall room template in the supported circle/cavern/rectangle geometry; source value retained, no runtime flag claimed",
                 "A.streamers": "existing streamers", "F.WILD_TYPE": "forest 5:1 circle/cavern room mix and dungeon terrain-change consumers",
                 "F.generation": "random runtime destruction 1/36, lake 1/18 with depth thresholds, then cavern depth chance; river depth gate and lake-vault exclusion",
                 "M": "existing OR preferences; duplicate WILD_WOOD normalized",
             }}
    return dungeon, floors, encounter, entrance, audit


def wilderness_encounters(text):
    result, audit = [], []
    for record in source_records(text):
        kind = one(record, "T").split(":")
        if kind[0] != "WILD" or kind[1] not in WILD_TERRAINS:
            continue
        options = [flag.strip() for flag in "|".join(kind[2:]).split("|") if flag.strip()]
        if set(options) - {"FORMATION", "SHOP", "NIGHT", "DAY", "GOOD", "EVIL", "FRIENDLY",
                           "NO_ROTATE", "DEBUG", "THEME_OBJECT"}:
            raise ValueError(f"{record['name']}: unknown template flags {options}")
        if "?" in record["fields"] or {"DAY", "NIGHT"} <= set(options):
            raise ValueError(f"{record['name']}: unsupported conditional template")
        when = one(record, "W").split(":")
        slug = re.sub(r"[^a-z0-9]+", "-", record["name"].lower()).strip("-")
        encounter = {"id": f"demo.wilderness-encounter.{kind[1].lower()}-{slug}",
                     "terrain": WILD_TERRAINS[kind[1]], "minLevel": int(when[0]),
                     "maxLevel": None if when[1] == "*" else int(when[1]),
                     "rarity": int(when[2])}
        for option, field in [("SHOP", "requiresShop"), ("DEBUG", "debug")]:
            if option in options:
                encounter[field] = True
        if "DAY" in options or "NIGHT" in options:
            encounter["time"] = "day" if "DAY" in options else "night"
        rows = record["fields"].get("M", [])
        directives = dict(line.split(":", 1) for line in record["fields"].get("L", []))
        used = set("".join(rows)) - {" "}
        entrance_symbols = {symbol for symbol in used
                            if directives.get(symbol, "").startswith("ENTRANCE(")}
        if entrance_symbols:
            if len(entrance_symbols) != 1:
                raise ValueError("Expected one entrance symbol")
            entrance_symbol = next(iter(entrance_symbols))
            match = re.fullmatch(r"ENTRANCE\(GLOW\s*\|\s*MARK,\s*(\d+)\)", directives[entrance_symbol])
            if not match:
                raise ValueError("Unmapped entrance directive")
            dungeon_kind = KINDS[int(match[1])]
            legend = []
            for symbol in sorted(used):
                if symbol == entrance_symbol:
                    entry = {"symbol": symbol, "terrainId": f"demo.terrain.{dungeon_kind}-entrance",
                             "glow": True, "mark": True}
                else:
                    directive = re.fullmatch(r"([A-Z_]+)(?:\((GLOW)\))?", directives[symbol])
                    if not directive:
                        raise ValueError(f"Unmapped entrance map directive: {directives[symbol]}")
                    entry = {"symbol": symbol, "terrainId": terrain_id(directive[1])}
                    if directive[2]:
                        entry["glow"] = True
                legend.append(entry)
            encounter["entranceMap"] = {"dungeonId": f"demo.dungeon.{dungeon_kind}",
                                        "rows": rows, "legend": legend,
                                        "noRotate": "NO_ROTATE" in options}
        result.append(encounter)
        audit.append({"id": encounter["id"], "sourceName": record["name"],
                      "sourceLine": record["line"], "sourceType": one(record, "T"),
                      "sourceWhen": one(record, "W"), "mapImplemented": bool(entrance_symbols)})
    if len({entry["id"] for entry in result}) != len(result):
        raise ValueError("Duplicate wilderness template identity")
    return result, audit


def merge_world_array(text, key, owned_ids, additions):
    """Keep other content's exact JSON text, replacing only these four profiles."""
    start = re.search(rf'^  "{key}": \[', text, re.MULTILINE).end()
    decoder = json.JSONDecoder()
    cursor, values = start, []
    while True:
        token = cursor
        while text[token].isspace():
            token += 1
        if text[token] == "]":
            end = token + 1
            break
        value, after = decoder.raw_decode(text, token)
        if value["id"] not in owned_ids:
            values.append(text[cursor:after])
        token = after
        while text[token].isspace():
            token += 1
        cursor = token + 1 if text[token] == "," else token
    values.extend("\n    " + json.dumps(value, ensure_ascii=False, indent=2).replace("\n", "\n    ")
                  for value in additions)
    return text[:start] + ",".join(values) + "\n  ]" + text[end:]


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def sync(source, root):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(source), *args]).decode("utf-8")

    commit = git("rev-parse", "master").strip()
    read = lambda path: git("show", f"{commit}:{path}")
    pack = root / "packs/rfb-demo-original"
    world_path = pack / "worlds/middle-earth.json"
    world_text = world_path.read_text(encoding="utf-8")
    world = json.loads(world_text)
    # The existing formal Asgard profile supplies only shared room/budget and
    # treasure parameters; all dungeon materials, flags and ecology come from RFB.
    common = next(floor for floor in world["proceduralFloors"]
                  if floor["id"] == "demo.floor.asgard-depth-64")
    names = [json.loads(match[1]) for match in re.finditer(r'^\s*("(?:[^"\\]|\\.)*")\s*,?\s*$',
                                                         read("src/dungeon_name_zh.inc"), re.MULTILINE)]
    dungeons, floors, profiles, outputs, locale_entries = [], [], [], {}, {"en-US": [], "zh-CN": []}
    for record in source_records(read("lib/edit/d_info.txt")):
        if int(record["name"].split(":", 1)[0]) not in KINDS:
            continue
        dungeon, members, monsters, entrance, audit = dungeon_content(record, names, common)
        kind = KINDS[audit["sourceIndex"]]
        dungeons.append(dungeon)
        floors.extend(members)
        profiles.append(audit)
        outputs[pack / f"encounterTables/{kind}.json"] = monsters
        outputs[pack / f"terrain/{kind}-entrance.json"] = entrance
        for locale, name in [("zh-CN", audit["chineseName"]), ("en-US", audit["sourceName"])]:
            locale_entries[locale].extend([
                f"floor-demo-{kind}-depth-name = {name}",
                f"terrain-demo-{kind}-entrance-name = {name}",
                f"terrain-demo-{kind}-entrance-description = {one(record, 'D')}",
            ])
    if len(dungeons) != 4:
        raise ValueError("Source does not contain all four random dungeons")
    encounters, encounter_audit = wilderness_encounters(read("lib/edit/v_info.txt"))
    owned = {dungeon["id"] for dungeon in dungeons}
    floor_ids = {floor["id"] for floor in world["proceduralFloors"] if floor.get("dungeonId") in owned}
    world_text = merge_world_array(world_text, "dungeons", owned, dungeons)
    world_text = merge_world_array(world_text, "proceduralFloors", floor_ids, floors)
    if "encounters" in world["wilderness"]:
        match = re.search(r'^    "encounters": ', world_text, re.MULTILINE)
        _, end = json.JSONDecoder().raw_decode(world_text, match.end())
        world_text = (world_text[:match.end()] + json.dumps(encounters, ensure_ascii=False, indent=2)
                      .replace("\n", "\n    ") + world_text[end:])
    else:
        start = re.search(r'^  "wilderness": ', world_text, re.MULTILINE).end()
        _, end = json.JSONDecoder().raw_decode(world_text, start)
        before = world_text[:end - 1].rstrip()
        world_text = (before + ',\n    "encounters": ' + json.dumps(encounters, ensure_ascii=False, indent=2)
                      .replace("\n", "\n    ") + "\n  }" + world_text[end:])
    # Validate all new terrain references before writing any source content.
    terrain_ids = {json.loads(path.read_text(encoding="utf-8"))["id"]
                   for path in (pack / "terrain").glob("*.json")}
    for key, value in TERRAINS.items():
        if f"demo.terrain.{value}" not in terrain_ids:
            raise ValueError(f"Missing formal terrain for {key}: {value}")
    outputs[pack / "legacy-random-dungeon-source.json"] = {
        "schemaVersion": 1, "sourceRef": "master", "sourceCommit": commit,
        "sources": ["lib/edit/d_info.txt", "src/dungeon_name_zh.inc", "lib/edit/v_info.txt",
                    "src/init1.c", "src/rooms.c", "src/wild.c", "src/floors.c", "src/generate.c",
                    "src/cmd2.c", "src/spells3.c", "src/dungeon.c", "src/object2.c",
                    "src/spells2.c", "src/generate.h"],
        "profiles": profiles, "encounters": encounter_audit,
        "adaptations": [
            "119 independent formal depths; gameplay entry and travel are not implemented by this generator.",
            "RD2 runtime uniformly selects a member depth on entry; ascent exits, level teleport descends one depth or exits, and surface return resets the instance.",
            "Natural down stairs are absent; source floors.c stair_creation still permits magical down stairs below maximum depth.",
            "Outward recall may have no ordinary destination. Unlike source recall_dungeon assignment on departure, Rewrite preserves the last ordinary destination and never records random dungeons for surface recall or reset.",
            "Random entry stages depth RNG and generation with the existing floor transition; failed entry commits neither. RD3 binds only actual map entrances to normal stair traversal.",
            "RD3 attempts special encounters per newly generated 66x22 chunk using a stream derived from Game wilderness_seed and absolute chunk coordinates, not per original RFB generation region. Frequencies and RNG sequences are not claimed identical to upstream.",
            "Source terrain normalization, level/day/night/shop eligibility, integer 1000/rarity weights, eligible DEBUG precedence, rotation constraints and up to 100 placement attempts are retained. Wilderness shop eligibility uses the upstream reset-to-true default; unimplemented shop/formation outcomes do not reroll.",
            "Cache saves retain both no-placement decisions and selected template ID/origin/transform. Base terrain and template parameters are derived from content. Bounded eviction and wilderness seed updates retain the existing refresh policy; reload and cached visits do not redraw encounters.",
            "Actual entrance connections retain template placement and absolute chunk identity; local positions translate with scrolling. Placement excludes roads, towns, fixed locations, ambush generation and the arrival cell, and preserves GLOW/MARK on mapped cells.",
            "30 source-ordered candidates from the five entrance terrains; only six entrance maps materialized.",
            "Swimming Hole declares ENTRANCE but does not place it; retains selection metadata only.",
            "Unimplemented encounter selection must not reroll or renormalize the six entrance templates.",
            "Non-BIG maps use 96x33; BIG uses 198x66. Existing finite room/loot/population budgets are reused.",
            "RD5 uses source random-floor feature eligibility and mutual exclusion. Water/fire vault lake variants share the existing finite lake geometry, while retaining their river exclusion. Destruction cave/earth-vault variants share bounded rubble geometry; fractal generation and embedded source vault populations are not reproduced.",
            "Source pit/nest masks only affect monster-bearing room templates. Those full source templates are outside the supported finite room geometry; no unimplemented pit/nest feature is emitted. Inner-wall materials therefore remain source metadata. min_plev has no active entry consumer in master.",
            "Forest earthquakes and destruction preserve water and use tree/brake/grass materials. Existing spell radius, affected-cell and actor/item resolution remain Rewrite rules; source destruction power reduction and weighted floor/wall substitutions for other wilderness dungeon types are not reproduced.",
            "Schema, content lock and behavior validation are pending the agreed final validation phase.",
            "Upstream terms retained by NOTICE and LICENSES/RFB-UPSTREAM-NOTICE.txt.",
        ],
    }
    for path, value in outputs.items():
        write_json(path, value)
    world_path.write_text(world_text, encoding="utf-8", newline="\n")
    for locale, entries in locale_entries.items():
        path = root / f"locales/{locale}/content.ftl"
        text = path.read_text(encoding="utf-8")
        for entry in entries:
            key = entry.split(" = ", 1)[0]
            pattern = rf"^{re.escape(key)} = .*?$"
            if re.search(pattern, text, re.MULTILINE):
                text = re.sub(pattern, lambda _: entry, text, flags=re.MULTILINE)
            else:
                text = text.rstrip() + "\n" + entry + "\n"
        path.write_text(text, encoding="utf-8", newline="\n")
    print(f"Wrote {len(dungeons)} dungeons, {len(floors)} depths, {len(encounters)} encounter candidates at {commit}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="RFB Git repository (reads master objects only)")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    sync(args.source, args.root)
