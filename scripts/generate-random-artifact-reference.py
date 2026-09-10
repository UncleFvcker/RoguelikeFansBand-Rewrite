#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Sync artifact activation values from RFB master C code; read only Git objects."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def load_value_oracle():
    spec = importlib.util.spec_from_file_location("item_value_reference", Path(__file__).with_name("generate-item-value-reference.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sync_activation_values(read, directory, root):
    oracle = load_value_oracle()
    exe, _ = oracle.build_harness(read, directory)
    path = root / "packs/rfb-demo-original/randomArtifacts/source.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    profiles = data["deviceGeneration"]["activations"]
    lines = []
    for profile in profiles:
        token = profile["id"].removeprefix("rfb.device-activation.random-artifact.").upper().replace("-", "_")
        lines.append(f'E {token} {profile["deviceCheckDifficulty"]} 0 {profile["recovery"]["intervalTicks"] // 10}')
    result = subprocess.run([str(exe)], input="\n".join(lines) + "\n", text=True, encoding="utf-8", capture_output=True, check=True)
    values = list(map(int, result.stdout.splitlines()))
    assert len(values) == len(profiles)
    for profile, value in zip(profiles, values):
        profile["rfbValue"] = value
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return len(profiles)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source")
    args = parser.parse_args()
    commit = subprocess.check_output(["git", "-C", args.source, "rev-parse", "master"], text=True).strip()
    def read(path):
        return subprocess.check_output(["git", "-C", args.source, "show", f"{commit}:{path}"], text=True, encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="rfb-random-artifact-") as temporary:
        count = sync_activation_values(read, Path(temporary), Path(__file__).resolve().parent.parent)
    print(f"RFB {commit}: synchronized {count} random-artifact activation values")


if __name__ == "__main__":
    main()
