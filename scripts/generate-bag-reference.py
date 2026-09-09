#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run master obj_create_quiver, ego selection and COST_REAL with an injected RNG.

python scripts/generate-bag-reference.py D:/codex/Frogcomposband/master
Reads Git objects only; requires CC (default gcc). Does not update source content.
"""
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parent.parent
support = runpy.run_path(str(Path(__file__).with_name("generate-item-value-reference.py")))
repo = sys.argv[1]
commit = subprocess.check_output(["git", "-C", repo, "rev-parse", "master"], text=True).strip()


def source(path):
    return subprocess.check_output(["git", "-C", repo, "show", f"{commit}:{path}"], encoding="utf-8")


ego_source, defines = source("src/ego.c"), source("src/defines.h")
functions = "\n".join(support["function"](ego_source, name) for name in
                      ["_ego_rarity", "_ego_weight", "_choose_type", "ego_choose_type", "obj_create_quiver"])
kinds = support["records"](source("lib/edit/k_info.txt"))
egos = support["records"](source("lib/edit/e_info.txt"))
names = [None if match[1] == "NULL" else json.loads(match[1])
         for match in re.finditer(r'^\s*("(?:[^"\\]|\\.)*"|NULL)\s*,?', source("src/kind_name_zh.inc"), re.M)]
quiver = next(index for index, kind in kinds.items() if kind["identity"][:2] == [46, 0])
bases = [dict(sourceIndex=index, chineseName=names[index], **kinds[index]) for index in [722, 723, 724, quiver]]
assert [base["identity"] for base in bases[:3]] == [[46, 1, 0], [46, 1, 1], [46, 1, 2]]
cases = [dict(sourceIndex=base["sourceIndex"], power=power, level=level, seed=seed, forcedEgo=0)
         for base in bases for power in [-1, 0, 1, 2, 3] for level in [20, 50, 80] for seed in range(16)]
cases += [dict(sourceIndex=base["sourceIndex"], power=2, level=50, seed=0, forcedEgo=index)
          for base in bases[:3] for index in range(265, 269)]
header = r'''
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
typedef int bool;
typedef bool (*_ego_p)(int);
typedef struct { int rarity, level, max_level, type; } ego_type;
typedef struct { int sval, pval, xtra4, name2, weight; } object_type;
static ego_type e_info[269];
static int max_e_idx=269, apply_magic_ego, obj_drop_theme=0;
#define MAX(a,b) ((a)>(b)?(a):(b))
/* QUIVER has no theme predicate in ego_choose_type. These other types are not called. */
#define _ego_p_ring NULL
#define _ego_p_amulet NULL
#define _ego_p_body_armor NULL
#define _ego_p_shield NULL
#define _ego_p_helmet NULL
#define _ego_p_gloves NULL
#define _ego_p_boots NULL
static uint64_t state[4], draws;
static uint64_t rotl(uint64_t x, int k) { return (x << k) | (x >> (64-k)); }
static uint64_t next(void) {
    uint64_t r=rotl(state[1]*5,7)*9, t=state[1]<<17;
    state[2]^=state[0]; state[3]^=state[1]; state[1]^=state[2]; state[0]^=state[3];
    state[2]^=t; state[3]=rotl(state[3],45); ++draws; return r;
}
static void seed_rng(uint64_t seed) {
    for (int i=0; i<4; ++i) {
        uint64_t z=(seed+=UINT64_C(0x9e3779b97f4a7c15));
        z=(z^(z>>30))*UINT64_C(0xbf58476d1ce4e5b9);
        z=(z^(z>>27))*UINT64_C(0x94d049bb133111eb); state[i]=z^(z>>31);
    } draws=0;
}
static uint32_t randint0(uint32_t m) {
    if (m<=1) return 0;
    uint64_t bound=m, threshold=(-bound)%bound, r;
    do { r=next(); } while(r<threshold);
    return r%bound;
}
#define randint1(m) (randint0(m)+1)
#define one_in_(m) (randint0(m)==0)
'''
for name in sorted(set(re.findall(r"\b(?:SV_|EGO_TYPE_)\w+", functions))):
    match = re.search(rf"^#define {name}\s+(\S+)|^\s*{name}\s*=\s*(\w+)", defines, re.M)
    header += f"#define {name} {match[1] or match[2]}\n"
index = 0
for match in re.finditer(r"(?m)^\s+(EGO_\w+)(?:\s*=\s*(\d+))?\s*,", defines):
    index = int(match[2]) if match[2] else index + 1
    if 265 <= index <= 268:
        header += f"#define {match[1]} {index}\n"
main = ["int main(void) { object_type o;"]
for index in range(265, 269):
    level, maximum, rarity = egos[index]["allocation"]
    main.append(f"e_info[{index}]=(ego_type){{{rarity},{level},{0 if maximum == '*' else maximum},EGO_TYPE_QUIVER}};")
for case in cases:
    kind = kinds[case["sourceIndex"]]
    main += [f'memset(&o,0,sizeof(o)); seed_rng({case["seed"]}); apply_magic_ego={case["forcedEgo"]};',
             f'o.sval={kind["identity"][1]}; o.pval={kind["identity"][2]}; o.weight={kind["allocation"][-2]};',
             f'obj_create_quiver(&o,{case["level"]},{case["power"]},0);',
             'printf("%d %d %d %" PRIu64,o.xtra4,o.weight,o.name2,draws);',
             'for(int j=0;j<4;++j) printf(" %" PRIu64,state[j]); puts("");']
main.append("return 0;}")
with tempfile.TemporaryDirectory(prefix="rfb-bag-reference-") as temporary:
    directory = Path(temporary)
    cfile, exe = directory / "bags.c", directory / "bags.exe"
    cfile.write_text(header + functions + "\n" + "\n".join(main), encoding="utf-8")
    subprocess.run([os.environ.get("CC", "gcc"), "-std=c99", "-O0", str(cfile), "-o", str(exe)], check=True)
    rows = subprocess.check_output([str(exe)], text=True).splitlines()
    assert len(rows) == len(cases)
    objects = []
    value_exe, scoring_flags = support["build_harness"](source, directory)
    for case, row in zip(cases, rows):
        capacity, weight, ego, draws, *state = map(int, row.split())
        case["expected"] = dict(capacity=capacity, weight=weight, ego=ego, draws=draws, state=state)
        kind = kinds[case["sourceIndex"]]
        obj = dict(tval=46, sval=kind["identity"][1], pval=kind["identity"][2], capacity=capacity, weight=weight,
                   ego=ego, flags=kind["flags"] + (egos[ego]["flags"] if ego else []))
        if ego and "effect" in egos[ego]:
            token, power, timeout = egos[ego]["effect"]
            obj["activationValue"] = int(subprocess.check_output([str(value_exe)], input=f'E {token} {power} 0 {timeout}\n', text=True))
            obj["activationTimeout"] = int(timeout)
        objects.append(obj)
    for case, value in zip(cases, support["oracle_values"](value_exe, objects, scoring_flags)):
        case["expected"]["value"] = value
out = root / "crates/rfb-core/src/game/ego/noncraft/bag-reference.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(dict(sourceCommit=commit, bases=bases, cases=cases), ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
print(f"Wrote {len(cases)} original C capacity/ego/RNG/value cases to {out}")
