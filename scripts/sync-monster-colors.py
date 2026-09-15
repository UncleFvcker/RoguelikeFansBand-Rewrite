#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Sync built-in monster foregrounds from RFB Git objects; preserve image mappings."""
import argparse
import json
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]

    def git(*argv):
        return subprocess.check_output(["git", "-C", str(args.source), *argv]).decode("utf-8")

    commit = git("rev-parse", "master").strip()
    read = lambda path: git("show", f"{commit}:{path}")
    attrs = dict(re.findall(r"case '(.?)': return \((TERM_\w+)\);", read("src/init1.c")))
    palette = {}
    for r, g, b, name in re.findall(
        r"\{0x00, (0x\w+), (0x\w+), (0x\w+)\},\s*/\* (TERM_\w+) \*/",
        read("src/variable.c"),
    ):
        palette[name] = f"#{int(r, 16):02x}{int(g, 16):02x}{int(b, 16):02x}"
    colors = {}
    index = None
    for line in read("lib/edit/r_info.txt").splitlines():
        if line.startswith("N:"):
            index = int(line.split(":", 2)[1])
        elif line.startswith("G:"):
            colors[index] = palette[attrs[line[4]]]

    actors = [json.loads(path.read_text(encoding="utf-8"))
              for path in sorted((root / "packs/rfb-demo-original/actors").glob("*.json"))]
    selection = json.loads((root / "packs/rfb-demo-original/legacy-warrens-monster-selection.json").read_text(encoding="utf-8"))
    indexes = {"demo.actor." + a["id"]: a["sourceIndex"] for a in selection["monsters"]}
    # Original demo encounters have no RFB identity. These are project art choices.
    original = dict(zip(
        ("ash-drake", "cinder-adept", "dread-vampire", "gloom-weaver", "hex-chanter",
         "mind-lasher", "risen-thrall", "serpent-of-chaos", "slag-crawler", "veil-warden"),
        ("U", "R", "v", "s", "C", "B", "W", "G", "o", "I"), strict=True))
    monsters = {}
    for actor in actors:
        if actor["role"] != "monster":
            continue
        index = actor.get("allocation", {}).get("legacyIndex", indexes.get(actor["id"]))
        monsters[actor["id"]] = (colors[index] if index is not None
                                 else palette[attrs[original[actor["id"].removeprefix("demo.actor.")]]])
    for name in ("ascii-default", "image-demo", "rfb-pixel-28"):
        path = root / "web/public/tilesets" / name / "tileset.json"
        raw = path.read_bytes()
        text = raw.decode("utf-8").replace("\r\n", "\n")
        # Preserve existing layout and image fields; new entries use one line per ID.
        decoder = json.JSONDecoder()
        cursor = re.search(r'"mappings"\s*:\s*\{', text).end()
        replacements, found = [], set()
        while True:
            cursor += len(text[cursor:]) - len(text[cursor:].lstrip(" \n\t,"))
            if text[cursor] == "}":
                break
            actor, end = decoder.raw_decode(text, cursor)
            start = text.index("{", end)
            _, cursor = decoder.raw_decode(text, start)
            if actor in monsters:
                found.add(actor)
                value = re.sub(r'("foreground"\s*:\s*)"#[0-9a-fA-F]{6}"',
                               lambda m: m[1] + json.dumps(monsters[actor]), text[start:cursor])
                replacements.append((start, cursor, value))
        additions = [f'    "{actor}": {{ "foreground": "{color}" }}'
                     for actor, color in monsters.items() if actor not in found]
        if additions:
            insert = len(text[:cursor].rstrip())
            replacements.append((insert, cursor, ",\n" + ",\n".join(additions) + "\n  "))
        for start, end, value in reversed(replacements):
            text = text[:start] + value + text[end:]
        path.write_bytes(text.replace("\n", "\r\n" if b"\r\n" in raw else "\n").encode("utf-8"))
    print(f"Synced {len(monsters)} monster colors to three tilesets from master@{commit}")


if __name__ == "__main__":
    main()
