#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Execute RFB master COST_REAL in a small C harness; optionally sync source metadata.

python scripts/generate-item-value-reference.py D:/codex/Frogcomposband/master --sync
Requires a C compiler (CC, default gcc). No RFB working-tree files are read.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


def function(source, name):
    match = re.search(r"(?m)^(?:static )?\w+[ *]+" + re.escape(name) + r"\([^;]*?\)\s*\{", source)
    if not match:
        raise ValueError(f"source function missing: {name}")
    depth = 1
    end = match.end()
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[match.start():end]


def records(source, *, artifact=False):
    result = {}
    index = 0
    for line in source.splitlines():
        if line.startswith("N:"):
            _, number, name = line.split(":", 2)
            index = index + 1 if number == "*" else int(number)
            # RFB zero-initializes optional I/P values; init1.c adds four IGNORE flags to artifacts.
            result[index] = {"name": name, "flags": ["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"] if artifact else [], "identity": [0,0,0], "parameters": ["0","0d0","0","0","0"]}
        elif line.startswith("I:"):
            result[index]["identity"] = list(map(int, line[2:].split(":")))
        elif line.startswith("P:"):
            result[index]["parameters"] = line[2:].split(":")
        elif line.startswith("W:"):
            result[index]["allocation"] = line[2:].split(":")
        elif line.startswith("F:"):
            result[index]["flags"].extend(flag for token in line[2:].split("|") if (flag := token.strip()))
        elif re.match(r"^E:[A-Z_]+:\d+(?::|$)", line):
            result[index]["effect"] = line[2:].split(":")
            if len(result[index]["effect"]) == 2:
                # init1.c permits omitted timeout, e.g. Poseidon's EARTHQUAKE:10.
                result[index]["effect"].append("0")
    return result


def build_harness(read, directory):
    scoring = read("src/object3.c")
    defines = read("src/defines.h")
    constants = {m[1]: m[2] for m in re.finditer(r"(?m)^#define\s+(\w+)\s+(-?\d+)\b", defines)}
    constants.update({m[1]:m[2] for m in re.finditer(r"(?m)^\s*(\w+)\s*=\s*(\d+)\s*,",defines)})
    index = 0
    for match in re.finditer(r"(?m)^\s+(EGO_\w+)(?:\s*=\s*(\d+))?\s*,", defines):
        index = int(match[2]) if match[2] else index + 1
        constants[match[1]] = str(index)
    kind_source = read("src/obj_kind.c")
    two_hands = function(kind_source, "object_allow_two_hands_wielding")
    energy = function(read("src/xtra2.c"), "bow_energy")
    devices = read("src/devices.c")
    # Keep source value expressions and their local numeric declarations verbatim.
    # Cast/description code is deliberately not compiled into this read-only oracle.
    effect_cases = []
    pending = []
    for block in re.split(r"(?m)^[ \t]+case EFFECT_", devices[devices.index("cptr do_effect("):])[1:]:
        token = block.split(":", 1)[0]
        pending.append(token)
        value = re.search(r'if \(value\) return format\("%d", (.*?)\);', block)
        if not value:
            if "if (name)" in block or "break;" in block:
                pending.clear()
            continue
        prefix = re.sub(r"/\*.*?\*/|//[^\n]*", "", block.split("if (name)", 1)[0], flags=re.S)
        declarations = re.findall(r"\bint\s+\w+\s*=\s*[^;]+;", prefix)
        for name in pending:
            effect_cases.append(f'if (!strcmp(token, "{name}")) {{ {" ".join(declarations)} return {value[1]}; }}')
        pending.clear()
    effect_helpers = function(devices, "_extra") + "\n" + devices[devices.index("static int _avg_damroll("):devices.index("/************************************************************************\n * The Effects")]
    effects = "int source_effect_value(const char *token, effect_t *effect) {\n" + "\n".join(effect_cases) + '\nfprintf(stderr,"Unknown value effect: %s\\n", token); exit(2);\n}'
    object_info = read("src/obj_info.c")
    stats_table = object_info[object_info.index("static _flag_info_t _stats_flags[]"):object_info.index("static int _opposite_flag(")]
    pairs = [(a, b) for a, b in re.findall(r"\{\s*(OF_\w+),\s*(OF_\w+),", stats_table) if "OF_INVALID" not in (a, b)]
    pairs.append(("OF_LIFE", "OF_DEC_LIFE"))
    flavor = read("src/flavor.c")
    def resistance_table(name):
        start = flavor.index(name + "[]")
        table = flavor[start:flavor.index("};", start)]
        return dict(re.findall(r'\{\s*"([^"]+)"\s*,\s*(OF_\w+)', table))
    resist = resistance_table("flag_insc_resistance")
    vulnerable = resistance_table("flag_insc_vulnerability")
    pairs.extend((flag, vulnerable[name]) for name, flag in resist.items() if name in vulnerable)
    clean = "void remove_opposite_flags(u32b *flags) {\n" + "\n".join(
        f"if (have_flag(flags,{a}) && have_flag(flags,{b})) {{ remove_flag(flags,{a}); remove_flag(flags,{b}); }}"
        for a, b in pairs) + "\n}"
    used = scoring + two_hands + energy + clean
    flags = sorted(set(re.findall(r"\bOF_\w+", used)) - {"OF_ARRAY_SIZE"})
    tokens = sorted(set(re.findall(r"\b(?:ART_|EGO_|SV_|TV_|FEEL_|TOGGLE_)\w+", used)))
    needed = "\n".join(f"#define {token} {constants[token]}" for token in tokens)
    needed += "\n" + "\n".join(f"#define {flag} {i}" for i, flag in enumerate(flags))
    header = r'''
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <assert.h>
typedef int32_t s32b;
typedef uint32_t u32b;
typedef const char *cptr;
typedef void (*debug_hook)(const char *);
#define TRUE true
#define FALSE false
#define MAX(a,b) ((a)>(b)?(a):(b))
#define MIN(a,b) ((a)<(b)?(a):(b))
#define ABS(a) ((a)<0?-(a):(a))
#define OF_ARRAY_SIZE 256
#define COST_REAL 1
#define MAX_NLEN 1024
typedef struct {int x,y;} point_t, *point_ptr;
typedef struct {int type,power,extra,cost,oracle_value;} effect_t;
typedef struct {int tval,sval,pval,ac,to_h,to_d,to_a,dd,ds,mult,weight,xtra4;
 int name1,name2,art_name,feeling,k_idx; u32b curse_flags; u32b flags[OF_ARRAY_SIZE]; effect_t activation;} object_type, object_kind, obj_t;
object_kind k_info[1];
#define OFC_PERMA_CURSE 1
#define have_flag(f,i) ((f)[i] != 0)
#define remove_flag(f,i) ((f)[i] = 0)
void obj_flags(object_type *o,u32b *f) {memcpy(f,o->flags,sizeof(o->flags));}
void obj_flags_known(object_type *o,u32b *f) {obj_flags(o,f);}
bool obj_is_identified(object_type *o) {return false;}
bool object_is_known(object_type *o) {return false;}
bool object_is_(object_type *o,int t,int s) {return o->tval==t && o->sval==s;}
bool object_is_artifact(object_type *o) {return o->name1 || o->art_name;}
bool object_is_melee_weapon(object_type *o) {return o->tval>=20 && o->tval<=23;}
bool object_is_ammo(object_type *o) {return o->tval>=16 && o->tval<=18;}
bool object_is_armour(object_type *o) {return o->tval>=30 && o->tval<=38;}
bool object_is_shield(object_type *o) {return o->tval==34;}
bool object_is_jewelry(object_type *o) {return o->tval==40 || o->tval==45;}
bool object_is_device(object_type *o) {return false;}
bool obj_has_effect(object_type *o) {return o->activation.type != 0;}
effect_t obj_get_effect(object_type *o) {return o->activation;}
int effect_value(effect_t *e) {return e->oracle_value;}
int device_value(object_type *o,int options) {return 0;}
int weaponmaster_get_toggle(void) {return 0;}
void object_prep(object_type *o,int index) {*o=k_info[index];}
void object_desc_s(char *buf,size_t n,object_type *o,int mode) {*buf=0;}
'''
    header += needed + "\n" + function(read("src/rect.c"), "interpolate") + "\n" + two_hands + "\n" + energy + "\n" + clean
    header += "\n" + effect_helpers + "\n" + effects
    main = r'''
int main(void) {
 char mode[8], token[8192];
 while (scanf("%7s",mode)==1) {
   if (!strcmp(mode,"E")) {
     effect_t effect={0};
     if (scanf("%8191s %d %d %d",token,&effect.power,&effect.extra,&effect.cost)!=4) return 3;
     printf("%d\n",source_effect_value(token,&effect));
     continue;
   }
   object_type o={0}; memset(k_info,0,sizeof(k_info));
   int artifact=0, permanent=0;
   int *fields[]={&o.tval,&o.sval,&o.pval,&o.ac,&o.to_h,&o.to_d,&o.to_a,&k_info[0].to_h,
      &o.dd,&o.ds,&k_info[0].dd,&k_info[0].ds,&o.mult,&k_info[0].mult,&o.weight,&o.xtra4,
      &o.name2,&o.name1,&artifact,&permanent,&o.activation.oracle_value,&o.activation.cost};
   for (int i=0;i<22;i++) if (scanf("%d",fields[i])!=1) return 3;
   o.art_name=artifact; o.curse_flags=permanent?OFC_PERMA_CURSE:0;
   o.activation.type=(o.activation.oracle_value || o.activation.cost)?1:0;
   k_info[0].tval=o.tval; k_info[0].sval=o.sval;
   if (scanf("%8191s",token)!=1) return 3;
   for (char *flag=strtok(token,",");flag;flag=strtok(NULL,",")) {
     if (!strcmp(flag,"-")) continue;
     int found=0;
     for(int i=0;i<FLAG_COUNT;i++) if(!strcmp(flag,flag_names[i])) {o.flags[i]=1;found=1;break;}
     if(!found) {fprintf(stderr,"Unknown flag: %s\n",flag);return 3;}
   }
   if (o.activation.type) o.flags[OF_ACTIVATE]=1;
   printf("%d\n",new_object_cost(&o,COST_REAL));
 }
 return 0;
}
'''
    names = f"#define FLAG_COUNT {len(flags)}\nconst char *flag_names[]={{" + ",".join(json.dumps(f[3:]) for f in flags) + "};\n"
    path = directory / "oracle.c"
    path.write_text(header + "\n" + scoring.replace('#include "angband.h"', '') + "\n" + names + main, encoding="utf-8")
    exe = directory / ("oracle.exe" if os.name == "nt" else "oracle")
    subprocess.run([os.environ.get("CC", "gcc"), "-std=c99", "-O0", "-fwrapv", str(path), "-o", str(exe)], check=True, capture_output=True)
    return exe, [flag[3:] for flag in flags]


VALUE_FIELDS = "tval sval pval ac toH toD toA baseToH dd ds baseDd baseDs mult baseMult weight capacity ego fixedArtifact artifact permanentCurse activationValue activationTimeout".split()


def oracle_values(exe, objects, scoring_flags=None):
    lines = []
    for obj in objects:
        flags = obj.get("flags", [])
        if scoring_flags is not None:
            flags = [flag for flag in flags if flag in scoring_flags]
        lines.append("V " + " ".join(str(int(obj.get(key, 0))) for key in VALUE_FIELDS) + " " + ",".join(flags or ["-"]))
    result = subprocess.run([str(exe)], input="\n".join(lines) + "\n", text=True, encoding="utf-8", capture_output=True, check=True)
    return list(map(int, result.stdout.splitlines()))


def source_instances(read, exe, root, scoring_flags):
    """Base and fixed-artifact numbers come directly from k_info/a_info, not Rust."""
    kinds, artifacts = (records(read(f"lib/edit/{name}_info.txt"), artifact=name == "a") for name in ["k", "a"])
    items = {data["id"]: data for path in sorted((root/"items").glob("*.json"))
             if (data := json.loads(path.read_text(encoding="utf-8")))}
    cases = []
    def dice_or_mult(parameters):
        text = parameters[1]
        if text.startswith("x"):
            whole, fraction = text[1:].split(".")
            return 0, 0, int(whole)*100+int(fraction.ljust(2,"0"))
        dd, ds = map(int, text.split("d"))
        return dd, ds, 0
    for kind_id, definition in items.items():
        if "rfbValue" not in definition:
            continue
        artifact = definition.get("artifactGeneration")
        base_definition = items[artifact["baseItemKindId"]] if artifact else definition
        base = kinds[base_definition["rfbBaseKind"]["sourceIndex"]]
        entry = artifacts[artifact["sourceIndex"]] if artifact else base
        tval, sval, pval = entry["identity"]
        # object2.c prices whistles from k_info; new_object_cost's zero is
        # not an equipment score or the actual price of these objects.
        if tval == 4:
            continue
        dd, ds, mult = dice_or_mult(entry["parameters"])
        base_dd, base_ds, base_mult = dice_or_mult(base["parameters"])
        ac, _, to_h, to_d, to_a = entry["parameters"]
        obj = dict(tval=tval, sval=sval, pval=pval, ac=int(ac), toH=int(to_h), toD=int(to_d), toA=int(to_a),
                   baseToH=int(base["parameters"][2]), dd=dd, ds=ds, baseDd=base_dd, baseDs=base_ds,
                   # a_info W is level:rarity:weight:cost (trailing fields ignored);
                   # k_info W is level:extra:max_level:weight:cost.
                   mult=mult, baseMult=base_mult, weight=int(entry["allocation"][2 if artifact else 3]),
                   flags=sorted(set(base["flags"] + entry["flags"])), capacity=(pval+1)*4 if (tval,sval)==(46,1) else 60 if tval==46 else 0,
                   artifact=bool(artifact), fixedArtifact=artifact["sourceIndex"] if artifact else 0,
                   permanentCurse="PERMA_CURSE" in entry["flags"] or (tval,sval)==(23,34))
        # devices.c::obj_get_effect falls through to the base kind if the
        # fixed artifact has no activation of its own (e.g. spectral scales226).
        effect = entry.get("effect", base.get("effect"))
        if effect:
            extra = int(effect[3]) if len(effect)>3 else 0
            obj["activationValue"] = int(subprocess.check_output([str(exe)], input=f'E {effect[0]} {effect[1]} {extra} {effect[2]}\n', text=True))
            obj["activationTimeout"] = int(effect[2])
        cases.append(dict(kindId=kind_id, object=obj))
    for case, expected in zip(cases, oracle_values(exe, [case["object"] for case in cases], scoring_flags)):
        case["expected"] = expected
    return cases


def reference_cases(flags):
    cases = []
    bases = {
        "sword": {"tval":23,"sval":17,"dd":2,"ds":5,"baseDd":2,"baseDs":5,"weight":130},
        "ammo": {"tval":17,"sval":1,"dd":2,"ds":4,"baseDd":2,"baseDs":4,"weight":2},
        "armor": {"tval":37,"sval":6,"ac":16,"toH":-2,"baseToH":-2,"weight":270},
        "ring": {"tval":45,"weight":2}, "amulet": {"tval":40,"weight":3},
        "light": {"tval":39,"sval":1,"weight":50},
        "quiver": {"tval":46,"capacity":60,"weight":10},
        "bow": {"tval":19,"sval":13,"mult":300,"baseMult":300,"weight":40},
    }
    def add(name, base, **updates):
        cases.append({"name":name,"object":base | updates})
    swimsuit = {"tval":36,"sval":50,"weight":2,"flags":["IGNORE_ACID","IGNORE_ELEC","IGNORE_FIRE","IGNORE_COLD","AGGRAVATE"]}
    add("swimsuit-minimum-value",swimsuit)
    add("swimsuit-aggravation-discount",swimsuit,toD=1)
    add("swimsuit-negative-enchantment-minimum",swimsuit,toA=-1)
    for name, base in bases.items():
        add(name+"-plain",base)
        add(name+"-enchanted",base,pval=3,toH=8,toD=11,toA=7,ego=1,flags=["STR","SPEED","RES_FIRE","FREE_ACT"])
        add(name+"-cursed",base,pval=3,toH=-8,toD=-11,toA=-7,ego=1,flags=["DEC_STR","DEC_STEALTH","TY_CURSE","AGGRAVATE"])
        add(name+"-negative-pval",base,pval=-3,flags=["STR","SPEED","LIFE","STEALTH"])
        for pval in [-2,-1]:
            add(name+"-unsigned-pval-"+str(pval),base,pval=pval,flags=["STR","SPEED","LIFE","STEALTH"])
        add(name+"-activation",base,activationValue=1501,activationTimeout=80)
        add(name+"-artifact",base,artifact=True,toA=10,flags=["IGNORE_ACID","RES_ACID"])
    for flag in flags:
        if flag != "ACTIVATE":
            add("ring-flag-"+flag,bases["ring"],flags=[flag],pval=3)
    for pval in [-10,-3,-2,-1,0,1,2,10,11]:
        add("pval-"+str(pval),bases["ring"],pval=pval,flags=["STR","INT","WIS","DEX","CON","CHR","LIFE","SPEED"])
    for to_a in [-100,-31,-30,-25,-20,-15,-10,-5,-1,0,1,4,5,9,10,15,20,25,30,31,100]:
        add("ac-"+str(to_a),bases["armor"],toA=to_a,flags=["IGNORE_ACID"])
    for sval, mult in [(2,200),(12,250),(13,300),(23,350),(24,400),(63,200),(70,0),(50,0)]:
        add("launcher-"+str(sval),bases["bow"],sval=sval,baseMult=mult,mult=mult+75,pval=3,toD=20,toH=9,flags=["XTRA_SHOTS","BRAND_FIRE"])
    for ego in [81,126,183,184,185,186,207,208,224,237,266,268]:
        base = bases["ammo"] if 180 <= ego < 190 else bases["armor"] if ego < 180 else bases["light"] if ego==237 else bases["quiver"] if ego>260 else bases["ring"]
        add("ego-"+str(ego),base,ego=ego,toD=12)
    for artifact in [136,226,275,282,291,294,297,328,378,381]:
        add("artifact-"+str(artifact),bases["sword"],artifact=True,fixedArtifact=artifact)
    add("artifact-light-jewelry",bases["light"],artifact=True,pval=3,flags=["STR"],toA=10)
    for flags in [["RES_ACID","VULN_ACID"],["STR","DEC_STR"],["LIFE","DEC_LIFE"],
                  ["SPELL_POWER","DEC_SPELL_POWER"],["BLOWS","DEC_BLOWS"],
                  ["RES_ACID","RES_ELEC","RES_FIRE","RES_COLD","RES_POIS","RES_LITE","RES_DARK","RES_CONF","RES_NETHER","RES_NEXUS","RES_FEAR","IM_FIRE","VULN_SHARDS","SPEED","TELEPATHY"],
                  ["KILL_EVIL","KILL_DEMON","SLAY_EVIL","BRAND_FIRE","BRAND_COLD","BRAND_MANA","VORPAL2","BLOWS","STUN"]]:
        for name in ["ring","sword"]:
            add(name+"-combined-"+str(len(cases)),bases[name],pval=3,flags=flags,toD=10)
    return cases


def sync_metadata(read, exe, root):
    kinds, egos, artifacts = (records(read(f"lib/edit/{name}_info.txt"), artifact=name == "a") for name in ["k","e","a"])
    items = [(path,json.loads(path.read_text(encoding="utf-8"))) for path in sorted((root/"items").glob("*.json"))]
    affixes = [(path,json.loads(path.read_text(encoding="utf-8"))) for path in sorted((root/"affixes").glob("*.json"))]
    token_pattern = re.compile(r"^rfb\.device-activation\.ego-\d+-(?:biased-)?(.+)$")
    updates = []
    for path, data in items + affixes:
        before = json.dumps(data, ensure_ascii=False)
        entry = None
        if "rfbEgo" in data:
            entry = egos[data["rfbEgo"]["sourceIndex"]]
            data["rfbEgo"]["flags"] = sorted(set(entry["flags"]))
        elif "artifactGeneration" in data:
            entry = artifacts[data["artifactGeneration"]["sourceIndex"]]
        elif "rfbBaseKind" in data and data["rfbBaseKind"]["tval"] in list(range(16,24))+list(range(30,41))+[45,46]:
            entry = kinds[data["rfbBaseKind"]["sourceIndex"]]
        if entry and "rfbEgo" not in data:
            data["rfbValue"] = {"flags":sorted(set(entry["flags"])),"pval":entry["identity"][2],"toArmor":int(entry["parameters"][4])}
        for profile in data.get("deviceGeneration",{}).get("activations",[]):
            token = None
            extra = 0
            match = token_pattern.match(profile["id"])
            if match:
                token = match[1].upper().replace("-","_")
            elif entry and "effect" in entry:
                token = entry["effect"][0]
                extra = int(entry["effect"][3]) if len(entry["effect"])>3 else 0
            elif profile["id"] == "rfb.device-activation.endless-quiver":
                token = "ENDLESS_QUIVER"
            elif profile["id"].startswith("rfb.device-activation.ego-breath-"):
                entry_effect = kinds[int(profile["id"].rsplit("-", 1)[1])]["effect"]
                token = entry_effect[0]
                extra = int(entry_effect[3]) if len(entry_effect) > 3 else 0
            elif data.get("rfbBaseKind",{}).get("tval")==38:
                token = profile["id"].split("dragon-breath-")[-1].upper().replace("-","_")
                if not token.startswith("BREATHE_"):
                    token = None
            if token:
                recovery = profile.get("recovery",data.get("deviceGeneration",{}).get("recovery",{}))
                timeout = recovery.get("intervalTicks",0)//10
                inp = f'E {token} {profile["deviceCheckDifficulty"]} {extra} {timeout}\n'
                result = subprocess.run([str(exe)],input=inp,text=True,encoding="utf-8",capture_output=True,check=True)
                profile["rfbValue"] = int(result.stdout.strip())
        if json.dumps(data,ensure_ascii=False)!=before:
            updates.append((path,data))
    for path,data in updates:
        path.write_text(json.dumps(data,ensure_ascii=False,indent=2,sort_keys=True)+"\n",encoding="utf-8")
    return len(updates)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source")
    parser.add_argument("--sync",action="store_true")
    args=parser.parse_args()
    source=Path(args.source).resolve()
    root=Path(__file__).resolve().parents[1]
    commit=subprocess.check_output(["git","-C",str(source),"rev-parse","master"],text=True).strip()
    def read(path):
        return subprocess.check_output(["git","-C",str(source),"show",f"{commit}:{path}"],text=True,encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="rfb-value-") as temporary:
        exe,flags=build_harness(read,Path(temporary))
        cases=reference_cases(flags)
        values=oracle_values(exe,[case["object"] for case in cases])
        assert len(values)==len(cases)
        for case,value in zip(cases,values): case["expected"]=value
        output=root/"crates/rfb-core/src/game/item_value/reference.json"
        output.parent.mkdir(exist_ok=True)
        output.write_text('{\n  "sourceCommit": '+json.dumps(commit)+',\n  "cases": [\n'+",\n".join("    "+json.dumps(case,ensure_ascii=False,separators=(",",":")) for case in cases)+'\n  ]\n}\n',encoding="utf-8")
        changed=sync_metadata(read,exe,root/"packs/rfb-demo-original") if args.sync else 0
        instances = source_instances(read,exe,root/"packs/rfb-demo-original", flags)
        (output.parent/"source-instances.json").write_text('{\n  "sourceCommit": '+json.dumps(commit)+',\n  "instances": [\n'+",\n".join("    "+json.dumps(case,ensure_ascii=False,separators=(",",":")) for case in instances)+'\n  ]\n}\n',encoding="utf-8")
        print(f"RFB {commit}: {len(cases)} C reference values; metadata files updated: {changed}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        print(error.stderr.decode("utf-8",errors="replace") if isinstance(error.stderr,bytes) else error.stderr)
        raise
