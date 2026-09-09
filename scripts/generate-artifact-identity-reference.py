#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""C COST_REAL inputs for E8.5a hand-prepared instances, not artifact generation.

python scripts/generate-artifact-identity-reference.py D:/codex/Frogcomposband/master
Reads master through Git objects. Uses the existing source-extracted value harness.
"""
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parent.parent
support = runpy.run_path(str(Path(__file__).with_name("generate-item-value-reference.py")))
repo = sys.argv[1]
commit = subprocess.check_output(["git", "-C", repo, "rev-parse", "master"], text=True).strip()


def read(path):
    return subprocess.check_output(["git", "-C", repo, "show", f"{commit}:{path}"], encoding="utf-8")


with tempfile.TemporaryDirectory() as directory:
    exe, flags = support["build_harness"](read, Path(directory))
    bases = support["source_instances"](read, exe, root / "packs/rfb-demo-original", flags)
    cases = []
    for base in bases:
        if base["kindId"] not in ["demo.item.dagger", "demo.item.feanorian-lamp"]:
            continue
        obj = dict(base["object"], artifact=True, pval=2, weight=77, toH=7, toD=9,
                   activationValue=300, activationTimeout=10)
        obj["flags"] = sorted(set(obj["flags"]) | {"STR", "RES_FIRE", "FREE_ACT", "BRAND_COLD", "BLESSED"})
        if obj["tval"] == 23:
            obj.update(dd=3, ds=7)
        cases.append(dict(kindId=base["kindId"], object=obj))
    for case, price in zip(cases, support["oracle_values"](exe, [case["object"] for case in cases], flags)):
        case["expected"] = price
    assert len(cases) == 2
    # Reuse one literal actually present in the authoritative Chinese source.
    # This test name is not a generated name table or an enabled content item.
    name = "(永恒蘑菇)"
    assert f'quark_add("{name}")' in read("src/spells3.c")
    result = dict(sourceCommit=commit, name=name, nameSource="src/spells3.c:Eternal Mushroom", cases=cases)
    output = root / "crates/rfb-core/src/game/tests/artifact-identity-reference.json"
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    print(output)
    print([(case["kindId"], case["expected"]) for case in cases])
