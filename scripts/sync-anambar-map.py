#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Generate Anambar's surface and doors from RFB master Git objects; --check is read-only."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "packs/rfb-demo-original"
TOWN = "demo.town.anambar"
FLOOR = "demo.floor.anambar"


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
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    commit = subprocess.check_output(["git", "-C", str(args.source), "rev-parse", "master"], text=True).strip()

    def source(path):
        return subprocess.check_output(["git", "-C", str(args.source), "show", f"{commit}:{path}"]).decode("utf-8")

    town_source = source("lib/edit/t_ana.txt")
    rows = [line[2:] for line in town_source.splitlines() if line.startswith("M:")]
    assert len(rows) == 66 and all(len(row) == 198 for row in rows)
    positions = defaultdict(list)
    for y, row in enumerate(rows):
        for x, symbol in enumerate(row):
            positions[symbol].append({"x": x, "y": y})

    # Unconditional legend, before the task expressions. Preserve the source's blank cells.
    tags = {}
    for text in (source("lib/edit/t_pref.txt"), town_source.split("################## Cop Quests")[0]):
        for line in text.splitlines():
            if line.startswith("L:"):
                tags[line[2]] = line[4:]
    materials = {
        "FLOOR": "floor", "TREE": "surface-tree", "SHALLOW_WATER": "surface-water-shallow",
        "DEEP_WATER": "surface-water-deep", "MOUNTAIN": "surface-mountain", "DIRT": "dirt",
        "GRASS": "surface-grass", "BRAKE": "surface-brake", "FLOWER": "surface-flower",
        "RUBBLE": "rubble", "PERMANENT": "permanent-wall", "CLOSED_DOOR": "door-closed",
        "HOME": "home-entrance", "MUSEUM": "museum-entrance",
        "BUILDING_3": "casino-entrance", "BUILDING_5": "beastmaster-entrance",
        "BUILDING_8": "reporters-guild-entrance", "BUILDING_10": "thieves-guild-entrance",
        "BUILDING_12": "cornucopia-bank-entrance",
    }
    shop_symbols = dict(zip("012345679e", ["shroomery", "general-store", "armoury", "weaponsmith", "temple", "alchemist", "magic-shop", "black-market", "bookstore", "inn"]))
    facility_symbols = {"a": "library", "b": "mayor-office", "g": "weapon-master", "h": "warrior-guild", "j": "mammon-temple", "l": "archer-guild", "n": "police-station", "o": "trump-tower", "8": "home", "M": "museum"}
    outputs = {}
    for directory, mapping in (("shops", shop_symbols), ("townFacilities", facility_symbols)):
        for symbol, name in mapping.items():
            path = PACK / directory / f"anambar-{name}.json"
            value = read_json(path)
            materials[tags[symbol]] = value["entranceTerrainId"].removeprefix("demo.terrain.")
            text = replace_property(path.read_text(encoding="utf-8"), "entrancePosition", positions[symbol][0])
            outputs[path] = replace_property(text, "additionalEntrancePositions", positions[symbol][1:])

    terrain = defaultdict(list)
    for symbol, cells in positions.items():
        if symbol != " ":
            terrain["demo.terrain." + materials[tags[symbol]]].extend(cells)
    assert sum(map(len, terrain.values())) == 7671
    task_symbols = {"v": "orc-camp", "u": "clear-tunnels", "U": "scary-rock-treasure", "z": "dinosaur-quest", "w": "apina-island", "W": "lord-bovin-treachery", "8": "cop-quest", "y": "smugglers-den", "x": "cellar-killer"}
    rules = []
    for symbol, name in task_symbols.items():
        task_id = f"demo.task.anambar-{name}"
        cases = [{"taskId": task_id, "statuses": ["taken", "active"], "terrainId": f"demo.terrain.anambar-{name}-entry"}]
        default = "demo.terrain." + materials[tags[symbol]]
        if symbol == "8":
            default = "demo.terrain.permanent-wall"
            cases.append({"taskId": task_id, "statuses": ["completed"], "terrainId": "demo.terrain.home-entrance"})
        rules.append({"positions": positions[symbol], "defaultTerrainId": default, "cases": cases})

    world_path = PACK / "worlds/middle-earth.json"
    text = world_path.read_text(encoding="utf-8")

    def update_floor(floor):
        if floor["id"] == FLOOR:
            floor.update(width=198, height=66)
            floor["inlineMap"] = {"inheritWildernessTerrain": True, "playerPosition": {"x": 99, "y": 33}, "terrainOverrides": [{"terrainId": name, "positions": sorted(cells, key=lambda p: (p["y"], p["x"]))} for name, cells in sorted(terrain.items())], "taskTerrainOverrides": rules}
        elif floor.get("taskId", "").startswith("demo.task.anambar-"):
            floor["returnFloorId"] = FLOOR

    def update_task(task):
        if task["id"] == "demo.task.anambar-dinosaur-quest":
            task["failureReturnSpawn"] = {"actorKindId": "demo.actor.triceratops", "position": positions["t"][0], "chancePercent": 33}

    def update_location(location):
        if location.get("townId") == TOWN:
            location["mapOrigin"] = {"x": 0, "y": 0}

    text = replace_members(text, "proceduralFloors", update_floor)
    text = replace_members(text, "tasks", update_task)
    outputs[world_path] = replace_members(text, "locations", update_location)
    changed = [path for path, value in outputs.items() if path.read_text(encoding="utf-8") != value]
    if args.check and changed:
        raise SystemExit("Anambar source drift: " + ", ".join(str(p.relative_to(ROOT)) for p in changed))
    for path in changed:
        path.write_text(outputs[path], encoding="utf-8", newline="\n")
    print(f"Anambar {commit}: 198x66, 7671 explicit cells, 5397 inherited cells; {len(changed)} files {'differ' if args.check else 'updated'}")


if __name__ == "__main__":
    main()
