#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Compile original RFB negative-generation rules against the rewrite's RNG stream.

python scripts/generate-negative-equipment-reference.py D:/codex/Frogcomposband/master
Reads Git master objects only. Requires CC (default gcc). COST_REAL is independently
covered by generate-item-value-reference.py; this harness supplies explicit pre-curse
values to the unchanged source curse_object, and compares its random transitions.
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
    return subprocess.check_output(["git", "-C", repo, "show", f"{commit}:src/{path}"], encoding="utf-8")


artifact, spells, obj, defines = [source(path) for path in ["artifact.c", "mspells1.c", "object2.c", "defines.h"]]
names = ["_add_bad_flag", "one_high_vulnerability", "one_ele_vulnerability", "one_vulnerability", "one_stat_biff", "one_biff", "curse_object"]
functions = "\n".join(function(artifact, name) for name in names) + "\n" + function(spells, "get_curse")
flags = sorted(set(re.findall(r"\bOF_\w+", functions)) - {"OF_ARRAY_SIZE"})
# one_stat_biff uses the source's contiguous STR/DEC_STR enum ranges.
for prefix in ["OF_", "OF_DEC_"]:
    flags = [flag for flag in flags if flag not in [prefix + stat for stat in ["STR", "INT", "WIS", "DEX", "CON", "CHR"]]]
    flags.extend(prefix + stat for stat in ["STR", "INT", "WIS", "DEX", "CON", "CHR"])
flag_ids = {flag: index for index, flag in enumerate(flags)}
cf = {name: int(value, 16) for name, value in re.findall(r"#define (OFC_\w+)\s+(0x[0-9a-fA-F]+)", defines)}
mask = defines[defines.index("#define TRC_SPECIAL_MASK"):defines.index("#define TRC_P_FLAG_MASK")]
quality = function(obj, "apply_magic")
quality = quality[:quality.index("    if (obj_drop_theme")]+"    return power;\n}"
quality = quality.replace("bool apply_magic(", "int apply_magic(")
mode_bits = {name: int(value, 16) for name, value in re.findall(r"#define (AM_\w+)\s+(0x[0-9a-fA-F]+)", defines)}

cases = []
for level in [0, 1, 10, 11, 30, 80, 127]:
    for tval in [23, 36, 45, 40, 65, 39, 46, 17]:
        for seed in range(8):
            cases.append(dict(kind="power", seed=seed, level=level, tval=tval, good=75, great=20, mode=0, noEgos=False, luck=0, chance=0))
for mode in [mode_bits["AM_GOOD"], mode_bits["AM_GREAT"], mode_bits["AM_GOOD"] | mode_bits["AM_GREAT"], mode_bits["AM_GOOD"] | mode_bits["AM_GREAT"] | mode_bits["AM_SPECIAL"]]:
    for no_egos in [False, True]:
        for tval in [23, 45, 65]:
            for seed in range(6):
                cases.append(dict(kind="power", seed=seed, level=80, tval=tval, good=75, great=20, mode=mode, noEgos=no_egos, luck=0, chance=0))
for luck in [-1, 0, 1]:
    for chance in [-125, 0, 125]:
        for seed in range(4):
            cases.append(dict(kind="power", seed=seed, level=15, tval=23, good=0, great=0, mode=0, noEgos=False, luck=luck, chance=chance))
for tval in [23, 19, 17, 36, 45, 39]:
    for value in [0, 9999, 10000, 29999, 30000, 60000, 100000, 200000]:
        for seed in range(6):
            cases.append(dict(kind="curse", seed=seed, value=value, tval=tval, flags=[]))
protected = [flag[3:] for flag in flags if not flag.startswith("OF_DEC_") and not flag.startswith("OF_VULN_")]
for seed in range(6):
    cases.append(dict(kind="curse", seed=seed, value=10000, tval=45, flags=protected))
    cases.append(dict(kind="curse", seed=seed, value=10000, tval=45, flags=["DEC_STR", "DEC_SPEED", "VULN_FIRE"]))
for power in [0, 1, 2]:
    for tval in [23, 17, 36, 45]:
        for seed in range(24):
            cases.append(dict(kind="get-curse", seed=seed, power=power, tval=tval))

header = r'''
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef int bool;
typedef uint32_t u32b;
#define TRUE 1
#define FALSE 0
#define MAX_CURSE 28
#define OF_ARRAY_SIZE 128
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
#define magik(p) ((p)<=0 || randint0(100)<(p))
#define have_flag(a,f) ((a)[f])
#define add_flag(a,f) ((a)[f]=1)
typedef struct { int tval, level; u32b flags[OF_ARRAY_SIZE], curse_flags; } object_type;
static u32b base_flags[OF_ARRAY_SIZE];
static int before_value, has_pval;
static void obj_flags(object_type *o, u32b *f) { for (int i=0;i<OF_ARRAY_SIZE;++i) f[i]=base_flags[i] | o->flags[i]; }
static int obj_value_real(object_type *o) { return before_value; }
static bool object_is_weapon(object_type *o) { return o->tval>=19 && o->tval<=23; }
static bool object_is_armour(object_type *o) { return o->tval>=30 && o->tval<=38; }
static bool object_is_jewelry(object_type *o) { return o->tval==40 || o->tval==45; }
static bool object_is_device(object_type *o) { return o->tval==55 || o->tval==65 || o->tval==66; }
static u32b get_curse(int power, object_type *o);
#define TV_RING 45
#define TV_AMULET 40
#define TV_STAFF 55
#define MAX_DEPTH 128
#define MUT_BAD_LUCK 1
#define VIRTUE_CHANCE 0
static int dungeon_type, no_egos, coffee_break, bad_luck, chance;
static struct {int obj_good, obj_great;} d_info[1];
static struct {int good_luck;} player, *p_ptr=&player;
static int mut_present(int id) {return bad_luck;}
static int virtue_current(int id) {return chance;}
'''
header += "\n".join(f"#define {name} {index}" for name, index in flag_ids.items()) + "\n"
header += "\n".join(f"#define {name} UINT32_C({value})" for name, value in cf.items()) + "\n" + mask + "\n"
header += "\n".join(f"#define {name} {value}" for name, value in mode_bits.items()) + "\n"
main = ['int main(void) { object_type o; int power=0; u32b picked=0;']
for case in cases:
    main.append(f'memset(&o,0,sizeof(o)); memset(base_flags,0,sizeof(base_flags)); seed_rng({case["seed"]}); o.tval={case["tval"]};')
    if case["kind"] == "power":
        main.append(f'd_info[0].obj_good={case["good"]}; d_info[0].obj_great={case["great"]}; no_egos={int(case["noEgos"])}; bad_luck={int(case["luck"]<0)}; player.good_luck={int(case["luck"]>0)}; chance={case["chance"]};')
        main.append(f'power=apply_magic(&o,{case["level"]},{case["mode"]});')
    elif case["kind"] == "get-curse":
        main.append(f'picked=get_curse({case["power"]},&o);')
    else:
        main.extend(f'base_flags[{flag_ids["OF_"+flag]}]=1;' for flag in case["flags"])
        main.append(f'before_value={case["value"]}; curse_object(&o);')
    main.append('printf("%d %u %u %" PRIu64, power,picked,o.curse_flags,draws);')
    main.append('for(int j=0;j<4;++j) printf(" %" PRIu64,state[j]);')
    main.append('for(int j=0;j<OF_ARRAY_SIZE;++j) if(o.flags[j]) printf(" %d",j); puts("");')
main.append('return 0;}')
with tempfile.TemporaryDirectory(prefix="rfb-negative-reference-") as temp:
    cfile, exe = Path(temp)/"reference.c", Path(temp)/"reference.exe"
    cfile.write_text(header+functions+"\n"+quality+"\n"+"\n".join(main), encoding="utf-8")
    subprocess.run([os.environ.get("CC", "gcc"), "-std=c99", "-O0", "-fwrapv", str(cfile), "-o", str(exe)], check=True)
    output = subprocess.check_output([str(exe)], text=True).splitlines()
assert len(output) == len(cases)
for case, row in zip(cases, output):
    power, picked, curse, draws, *tail = map(int, row.split())
    case["expected"] = dict(power=power, picked=picked, curse=curse, draws=draws, state=tail[:4], flags=sorted(flags[index][3:] for index in tail[4:]))
out = root/"crates/rfb-core/src/game/ego/curses/reference.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(dict(sourceCommit=commit, cases=cases), ensure_ascii=False, separators=(",", ":"))+"\n", encoding="utf-8")
print(f"Wrote {len(cases)} independent C cases to {out}")
