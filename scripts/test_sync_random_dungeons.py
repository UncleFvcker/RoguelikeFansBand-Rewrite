# SPDX-License-Identifier: MPL-2.0
"""Synthetic source/parser checks; run with the final RD validation batch."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "random_dungeons", Path(__file__).with_name("sync-random-dungeons.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class RandomDungeonSourceTests(unittest.TestCase):
    def test_unused_entrance_keeps_its_weight_without_creating_a_map(self):
        source = """N:Unplaced Gate
T:WILD:SNOW:NIGHT
W:25:*:3
L:.:ICE_FLOOR
L:>:ENTRANCE(GLOW | MARK, 29)
M:...
"""
        pool, _ = module.wilderness_encounters(source)
        self.assertEqual(pool[0]["rarity"], 3)
        self.assertEqual(pool[0]["time"], "night")
        self.assertNotIn("entranceMap", pool[0])
        pool, _ = module.wilderness_encounters(source.replace("M:...", "M:.>."))
        self.assertEqual(pool[0]["entranceMap"]["dungeonId"], "demo.dungeon.random-sea")

    def test_profile_replacement_keeps_other_json_and_is_idempotent(self):
        source = '{\n  "dungeons": [\n    { "id": "other", "value": 42 }\n  ],\n  "unchanged": true\n}\n'
        addition = {"id": "random", "random": True}
        first = module.merge_world_array(source, "dungeons", {"random"}, [addition])
        second = module.merge_world_array(first, "dungeons", {"random"}, [addition])
        self.assertIn('{ "id": "other", "value": 42 }', first)
        self.assertTrue(first.endswith('  "unchanged": true\n}\n'))
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
