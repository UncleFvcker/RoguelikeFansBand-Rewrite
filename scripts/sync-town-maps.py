#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Generate Anambar, Thalos or Zul from RFB master Git objects; --check is read-only."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "packs/rfb-demo-original"


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def dump(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def replace_property(text, key, value):
    token = f'"{key}"'
    if token not in text:
        if not value:
            return text
        end = text.rindex("}")
        return text[:end].rstrip() + f',\n  {token}: {dump(value)}\n' + text[end:]
    start = text.index(":", text.index(token)) + 1
    while text[start].isspace():
        start += 1
    old, end = json.JSONDecoder().raw_decode(text, start)
    return text if old == value else text[:start] + dump(value) + text[end:]


def replace_members(text, key, update):
    """Keep unrelated source formatting, including large compact floor definitions."""
    cursor = text.index("[", text.index(f'"{key}"')) + 1
    decoder = json.JSONDecoder()
    changes = []
    while True:
        while text[cursor] in " \r\n\t,":
            cursor += 1
        if text[cursor] == "]":
            break
        member, end = decoder.raw_decode(text, cursor)
        before = dump(member)
        update(member)
        if dump(member) != before:
            changes.append((cursor, end, dump(member)))
        cursor = end
    for start, end, value in reversed(changes):
        text = text[:start] + value + text[end:]
    return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--town", required=True, choices=["anambar", "thalos", "zul"])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    commit = subprocess.check_output(["git", "-C", str(args.source), "rev-parse", "master"], text=True).strip()

    def source(path):
        return subprocess.check_output(["git", "-C", str(args.source), "show", f"{commit}:{path}"]).decode("utf-8")

    name = args.town
    town_id, floor_id = f"demo.town.{name}", f"demo.floor.{name}"
    source_name = {"anambar": "t_ana", "thalos": "t_thalos", "zul": "t_zul"}[name]
    town_source = source(f"lib/edit/{source_name}.txt")
    rows = [line[2:] for line in town_source.splitlines() if line.startswith("M:")]
    width, height = (94, 57) if name == "zul" else (198, 66)
    assert len(rows) == height and all(len(row) == width for row in rows)
    positions = defaultdict(list)
    for y, row in enumerate(rows):
        for x, symbol in enumerate(row):
            positions[symbol].append({"x": x, "y": y})

    # Unconditional legend, before the task expressions. Preserve the source's blank cells.
    tags = {}
    for text in (source("lib/edit/t_pref.txt"), town_source.split("?:")[0]):
        for line in text.splitlines():
            if line.startswith("L:"):
                tags[line[2]] = line[4:]
    materials = {
        "FLOOR": "floor", "TREE": "surface-tree", "SHALLOW_WATER": "surface-water-shallow",
        "DEEP_WATER": "surface-water-deep", "MOUNTAIN": "surface-mountain", "DIRT": "dirt",
        "SHALLOW_LAVA": "surface-lava-shallow", "DEEP_LAVA": "surface-lava-deep",
        "GRASS": "surface-grass", "BRAKE": "surface-brake", "FLOWER": "surface-flower",
        "RUBBLE": "rubble", "PERMANENT": "permanent-wall", "CLOSED_DOOR": "door-closed",
        "HOME": "home-entrance", "MUSEUM": "museum-entrance",
        "BUILDING_3": "casino-entrance", "BUILDING_5": "beastmaster-entrance",
        "BUILDING_8": "reporters-guild-entrance", "BUILDING_10": "thieves-guild-entrance",
        "BUILDING_12": "cornucopia-bank-entrance",
    }
    shop_symbols = dict(zip("012345679e", ["shroomery", "general-store", "armoury", "weaponsmith", "temple", "alchemist", "magic-shop", "black-market", "bookstore", "inn"]))
    facility_symbols = {"a": "library", "b": "mayor-office", "g": "weapon-master", "h": "warrior-guild", "j": "mammon-temple", "l": "archer-guild", "n": "police-station", "o": "trump-tower", "8": "home", "M": "museum"}
    if name == "thalos":
        facility_symbols = {"a": "library", "b": "palace", "f": "bounty-office", "g": "weapon-master", "h": "warrior-guild", "i": "sorcery-tower", "j": "life-temple", "m": "paladin-guild", "n": "royal-academy", "8": "home", "M": "museum"}
        materials.update({"BUILDING_2": "town-arena-entrance", "ENTRANCE(GLOW | MARK, 21)": "icky-cave-entrance"})
        # Undeclared ! has the default FLOOR; rooms.c has no ! object/monster case.
        tags["!"] = "FLOOR"
        assert not positions.get("l") and positions["!"] == [{"x": 22, "y": 39}]
    elif name == "zul":
        shop_symbols = {symbol: shop_symbols[symbol] for symbol in "1345679"}
        shop_symbols.update({"A": "jeweler", "D": "dragonskin"})
        facility_symbols = {}
        # Tower services are registered in Z3; shop doors already use their live definitions.
        materials.update({"BUILDING_8": "sorcery-tower-entrance", "BUILDING_14": "chaos-tower-entrance",
                          "BUILDING_15": "nature-tower-entrance"})
    outputs = {}
    for directory, mapping in (("shops", shop_symbols), ("townFacilities", facility_symbols)):
        for symbol, facility in mapping.items():
            path = PACK / directory / f"{name}-{facility}.json"
            value = read_json(path)
            materials[tags[symbol]] = value["entranceTerrainId"].removeprefix("demo.terrain.")
            text = replace_property(path.read_text(encoding="utf-8"), "entrancePosition", positions[symbol][0])
            outputs[path] = replace_property(text, "additionalEntrancePositions", positions[symbol][1:])

    terrain = defaultdict(list)
    for symbol, cells in positions.items():
        if symbol != " ":
            terrain["demo.terrain." + materials[tags[symbol]]].extend(cells)
    explicit = sum(map(len, terrain.values()))
    assert explicit == {"anambar": 7671, "thalos": 13068, "zul": 2158}[name]
    task_symbols = {"v": "orc-camp", "u": "clear-tunnels", "U": "scary-rock-treasure", "z": "dinosaur-quest", "w": "apina-island", "W": "lord-bovin-treachery", "8": "cop-quest", "y": "smugglers-den", "x": "cellar-killer"}
    if name == "thalos":
        task_symbols = {"w": "shadow-fairies", "x": "djinnis-cavern", "L": "cyclops-lair", "z": "old-watchtower", "q": "cloning-pits", "y": "clear-wreckage", "r": "tidy-laboratory", "p": "basilisk-cave", "M": "dark-academy", "s": "staff-recovery", "F": "renegade-sorcerer"}
    elif name == "zul":
        task_symbols = {}
    rules = []
    for symbol, task in task_symbols.items():
        task_id = f"demo.task.{name}-{task}"
        cases = [{"taskId": task_id, "statuses": ["taken", "active"], "terrainId": f"demo.terrain.{name}-{task}-entry"}]
        default = "demo.terrain." + materials[tags[symbol]]
        if symbol == "8":
            default = "demo.terrain.permanent-wall"
            cases.append({"taskId": task_id, "statuses": ["completed"], "terrainId": "demo.terrain.home-entrance"})
        if name == "thalos" and symbol == "M":
            cases.append({"taskId": task_id, "statuses": ["reward-available", "failed", "abandoned"], "terrainId": "demo.terrain.permanent-wall"})
        if name == "thalos" and symbol == "s":
            cases.append({"taskId": "demo.task.thalos-staff-recovery-first", "statuses": ["taken", "active"], "terrainId": "demo.terrain.thalos-staff-recovery-first-entry"})
        rules.append({"positions": positions[symbol], "defaultTerrainId": default, "cases": cases})
    if name == "thalos":
        for symbol, natural in (("D", "surface-grass"), ("E", "surface-brake"), ("F", "surface-flower")):
            rule = next((rule for rule in rules if rule["positions"] == positions[symbol]), None)
            if rule is None:
                rule = {"positions": positions[symbol], "defaultTerrainId": "demo.terrain.permanent-wall", "cases": [{"taskId": "demo.task.thalos-renegade-sorcerer", "statuses": ["taken", "active"], "terrainId": "demo.terrain.permanent-wall"}]}
                rules.append(rule)
            # Later source assignments win: quest 64 Taken/Finished precede quest 71's early state.
            rule["cases"].extend([
                {"taskId": "demo.task.thalos-renegade-sorcerer", "statuses": ["completed"], "terrainId": f"demo.terrain.{natural}"},
                {"taskId": "demo.task.thalos-shadow-fairies", "statuses": ["locked", "available", "taken", "active", "reward-available"], "terrainId": f"demo.terrain.{natural}"},
            ])

    world_path = PACK / "worlds/middle-earth.json"
    text = world_path.read_text(encoding="utf-8")

    def update_floor(floor):
        if floor["id"] == floor_id:
            floor.update(width=width, height=height)
            # Source blank cells inherit wilderness; Thalos covers every underlying cell.
            arrival = {"x": 53, "y": 32} if name == "zul" else {"x": 99, "y": 33}
            floor["inlineMap"] = {"inheritWildernessTerrain": True, "playerPosition": arrival, "terrainOverrides": [{"terrainId": terrain_id, "positions": sorted(cells, key=lambda p: (p["y"], p["x"]))} for terrain_id, cells in sorted(terrain.items())], "taskTerrainOverrides": rules}
        elif floor.get("taskId", "").startswith(f"demo.task.{name}-"):
            floor["returnFloorId"] = floor_id

    def update_task(task):
        if name == "anambar" and task["id"] == "demo.task.anambar-dinosaur-quest":
            task["failureReturnSpawn"] = {"actorKindId": "demo.actor.triceratops", "position": positions["t"][0], "chancePercent": 33}

    def update_location(location):
        if location.get("townId") == town_id:
            location["mapOrigin"] = {"x": 0, "y": 0}

    text = replace_members(text, "proceduralFloors", update_floor)
    text = replace_members(text, "tasks", update_task)
    outputs[world_path] = replace_members(text, "locations", update_location)
    changed = [path for path, value in outputs.items() if path.read_text(encoding="utf-8") != value]
    if args.check and changed:
        raise SystemExit(f"{name} source drift: " + ", ".join(str(p.relative_to(ROOT)) for p in changed))
    for path in changed:
        path.write_text(outputs[path], encoding="utf-8", newline="\n")
    print(f"{name} {commit}: {width}x{height}, {explicit} explicit cells, {width * height - explicit} inherited cells; {len(changed)} files {'differ' if args.check else 'updated'}")


if __name__ == "__main__":
    main()
