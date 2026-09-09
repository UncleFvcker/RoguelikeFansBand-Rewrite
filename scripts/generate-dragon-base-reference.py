#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Extract master dragon-base rules and execute them with the rewrite's RNG stream.

python scripts/generate-dragon-base-reference.py D:/codex/Frogcomposband/master
Requires gcc (or CC). Outputs reference data, never reads the source working tree.
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
function = runpy.run_path(str(Path(__file__).with_name("generate-item-value-reference.py")))["function"]
repo = sys.argv[1]
commit = subprocess.check_output(["git", "-C", repo, "rev-parse", "master"], text=True).strip()


def source(path):
    return subprocess.check_output(["git", "-C", repo, "show", f"{commit}:{path}"], encoding="utf-8")


artifact, obj, kind, defines = [source("src/" + path) for path in ["artifact.c", "object2.c", "obj_kind.c", "defines.h"]]
functions = "\n".join(function(artifact, name) for name in ["one_ele_resistance", "one_dragon_ele_resistance", "one_high_resistance", "one_ele_slay"])
functions += "\n" + function(kind, "object_is_dragon_armor") + "\n" + function(obj, "dragon_resist")
fang = obj[obj.index("            if (object_is_(o_ptr, TV_SWORD, SV_DRAGON_FANG)"):obj.index("            if (o_ptr->sval == SV_RUNESWORD)")]
armor = obj[obj.index("            if (object_is_dragon_armor(o_ptr) &&"):obj.index("            if (power) obj_create_armor(o_ptr, lev, power, mode);")]
functions += "\nstatic int apply_dragon(object_type *o_ptr, int power, int mode) {\n" + fang + armor + "return power;\n}"
flags = sorted(set(re.findall(r"\bOF_\w+", functions)))
constants = sorted(set(re.findall(r"\b(?:TV_|SV_|AM_)\w+", functions)))
header = r'''
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
typedef int bool;
#define TRUE 1
#define FALSE 0
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
#define one_in_(m) (randint0(m)==0)
#define add_flag(a,f) ((a)[f]=1)
#define object_is_(o,t,s) ((o)->tval==(t) && (o)->sval==(s))
#define cheat_peek 0
#define object_mention(o) ((void)0)
typedef struct { int tval, sval, flags[32]; } object_type;
'''
header += "\n".join(f"#define {flag} {index}" for index, flag in enumerate(flags)) + "\n"
for name in constants:
    header += re.search(rf"^#define {name}\s+\S+", defines, re.M)[0] + "\n"
cases = [dict(seed=seed, tval=tval, sval=sval, power=power, mode=mode)
         for tval, sval in [(23, 35), (32, 8), (35, 7), (34, 6), (31, 6), (30, 4), (38, 6), (23, 17)]
         for mode in [0, 8, 9] for power in [-2, -1, 0, 1, 3] for seed in range(16)]
cases.extend(dict(seed=seed, tval=tval, sval=sval, power=0, mode=0)
             for tval, sval in [(23, 35), (32, 8)] for seed in range(16, 80))
main = ["int main(void) { object_type o; int power;"]
for case in cases:
    main += [f'memset(&o,0,sizeof(o)); seed_rng({case["seed"]}); o.tval={case["tval"]}; o.sval={case["sval"]};',
             f'power=apply_dragon(&o,{case["power"]},{case["mode"]});',
             'printf("%d %" PRIu64, power,draws);',
             'for(int j=0;j<4;++j) printf(" %" PRIu64,state[j]);',
             'for(int j=0;j<32;++j) if(o.flags[j]) printf(" %d",j); puts("");']
main.append("return 0;}")
with tempfile.TemporaryDirectory(prefix="rfb-dragon-reference-") as temp:
    cfile, exe = Path(temp) / "reference.c", Path(temp) / "reference.exe"
    cfile.write_text(header + functions + "\n" + "\n".join(main), encoding="utf-8")
    subprocess.run([os.environ.get("CC", "gcc"), "-std=c99", "-O0", str(cfile), "-o", str(exe)], check=True)
    rows = subprocess.check_output([str(exe)], text=True).splitlines()
assert len(rows) == len(cases)
for case, row in zip(cases, rows):
    power, draws, *tail = map(int, row.split())
    case["expected"] = dict(power=power, draws=draws, state=tail[:4], flags=sorted(flags[index][3:] for index in tail[4:]))

names = [None if match[1] == "NULL" else json.loads(match[1])
         for match in re.finditer(r'^\s*("(?:[^"\\]|\\.)*"|NULL)\s*,?', source("src/kind_name_zh.inc"), re.M)]
bases, index, entry = [], 0, None
for line in source("lib/edit/k_info.txt").splitlines():
    if line.startswith("N:"):
        _, number, name = line.split(":", 2)
        index = index + 1 if number == "*" else int(number)
        entry = dict(sourceIndex=index, name=name, chineseName=names[index], lines=[])
    if entry is not None:
        entry["lines"].append(line)
        if re.match(r"I:(23:35|32:8|35:7|34:6|31:6|30:4):", line):
            bases.append(entry)
assert len(bases) == 6 and all(base["chineseName"] for base in bases)
out = root / "crates/rfb-core/src/game/ego/dragon/reference.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(dict(sourceCommit=commit, bases=bases, cases=cases), ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
print(f"Wrote {len(cases)} independent C cases and {len(bases)} source bases to {out}")
