#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Sync item and public appearance colors from RFB master Git objects."""
import argparse
import json
from pathlib import Path
import re
import runpy
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    git = lambda *argv: subprocess.check_output(["git", "-C", str(args.source), *argv]).decode("utf-8")
    commit = git("rev-parse", "master").strip()
    read = lambda path: git("show", f"{commit}:{path}")
    attrs = dict(re.findall(r"case '(.?)': return \((TERM_\w+)\);", read("src/init1.c")))
    palette = {name: f"#{int(r, 16):02x}{int(g, 16):02x}{int(b, 16):02x}"
               for r, g, b, name in re.findall(
                   r"\{0x00, (0x\w+), (0x\w+), (0x\w+)\},\s*/\* (TERM_\w+) \*/", read("src/variable.c"))}
    sources, index = {}, -1
    for line in read("lib/edit/k_info.txt").splitlines():
        if line.startswith("N:"):
            token = line.split(":")[1]
            index = index + 1 if token == "*" else int(token)
            sources[index] = {}
        elif line.startswith("G:"):
            sources[index]["color"] = palette[attrs[line[4]]]
        elif line.startswith("I:"):
            sources[index]["kind"] = tuple(map(int, line.split(":")[1:3]))

    items = {item["id"]: item for path in sorted((root / "packs/rfb-demo-original/items").glob("*.json"))
             for item in [json.loads(path.read_text(encoding="utf-8"))]}
    # Explicit source identities for adapted remains and devices without rfbBaseKind.
    adapted = {"corpse-remains": 679, "skeleton-remains": 678, "human-skeleton": 677}
    # Project-authored equipment has no source kind; these are project art choices.
    original = {"burdened-mail": "s", "calm-pendant": "B", "dragon-bane": "G",
                "ember-edge": "R", "relic-blade": "v", "sealed-amulet": "v",
                "swift-treads": "B", "warding-band": "y"}

    def color(item):
        if "artifactGeneration" in item:
            return color(items[item["artifactGeneration"]["baseItemKindId"]])
        if "rfbBaseKind" in item:
            kind = item["rfbBaseKind"]
            source = sources[kind["sourceIndex"]]
            assert source["kind"] == (kind["tval"], kind["sval"]), item["id"]
            return source["color"]
        short = item["id"].removeprefix("demo.item.")
        if short in adapted:
            return sources[adapted[short]]["color"]
        for tag, index in (("wand", 312), ("staff", 313), ("rod", 314)):
            if tag in item.get("tags", []):
                return sources[index]["color"]
        return palette[attrs[original[short]]]

    colors = {id: color(item) for id, item in items.items()}
    for item in items.values():
        if "appearanceNameKey" in item:
            id = "core.appearance." + item["appearanceNameKey"]
            value = colors[item["id"]]
            if item["appearanceNameKey"] == "item-demo-unfamiliar-food-name":
                value = palette[attrs["u"]]  # Shared generic food appearance, independent of hidden kind.
            assert id not in colors or colors[id] == value, f"Conflicting public appearance: {id}"
            colors[id] = value
        if "artifact" in item.get("tags", []) and "artifactGeneration" not in item:
            # Unidentified hand-authored artifacts reveal only their symbol, not their identity.
            colors[f"core.appearance.symbol-{ord(item['glyph']):x}"] = "#c0c0c0"
    runpy.run_path(str(root / "scripts/sync-monster-colors.py"))["write_colors"](root, colors)
    print(f"Synced {len(items)} items and {len(colors) - len(items)} public appearances to three tilesets from master@{commit}")


if __name__ == "__main__":
    main()
