# Contract v187: Warrens Content P3 Casters and Simple Uniques

Status: active baseline. Protocol `1.139`, demo pack `1.182.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`ed657aa4243293b0a15da53281ff553b18bdce9b4f4e2bbb736cd6beec2162ae`.

## Authoritative source and strict spell binding

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The fixed-index selection in
`packs/rfb-demo-original/legacy-warrens-monster-selection.json` now contains 41
monsters. Chinese names follow `master:src/monster_name_zh.inc` exactly and
Chinese descriptions use the selected `master:lib/edit/r_info.txt` records.

`sync-demo-monsters` now routes active source spells through the same strict
mapping used by the full legacy importer. `1_IN_N` and `FREQ_N` remain the
authoritative casting frequency, possessor-only spells remain not applicable,
and any other unmapped token rejects the selected monster. Ability order follows
the source `S:` declaration and duplicate generated IDs are folded once.

## Supported casters

Ten shallow casters enter the global allocation pool:

- Green jelly and Radiation eye use level-scaled `DRAIN_MANA`;
- Ewok, Snaga, and Cave orc retain their explicit `SHOOT` dice;
- Crypt Creep uses `CAUSE_1` and level-seven `S_UNDEAD`;
- Bloodshot icky thing uses level-scaled `DRAIN_MANA`;
- Black harpy uses the original 17%-current-HP, 450-cap, radius-two sound breath;
- Novice mindcrafter uses Blind, Slow, Confuse, and Scare in source order;
- Crow of Durthang uses `CAUSE_1`.

The formal pack adds twelve monster-only abilities and matching flat Ability
Programs for the exact mapped parameter profiles. These definitions add no
player binding or resource policy.

## Simple Uniques

Grip, Wolf, and Fang retain level two, fixed maximum HP, random movement, door
bashing, ordered bite, immunities, corpse drop, allocation rarity, and Unique
lifecycle. Brodda the Easterling retains level nine, fixed maximum HP, four
ordered blows, doors, radius-two carried light, good item-only `1d2` drops,
remains, and the same lifecycle. Defeat state and single-live-instance behavior
reuse the existing Unique runtime without a new save field.

## Deferred records and omissions

The deprecated level-four Novice mindcrafter is not reintroduced. Lousy waits
for the unmapped `S_LOUSE` summon category. Theft, wilderness-only allocation,
special unsupported melee effects, source artifacts, friendly town actors, and
other special lifecycles remain deferred instead of being removed from source
behavior.

Declared omissions are limited to sex/speech metadata, special mind or
possessor hints, and wilderness habitat tags. They do not replace active combat,
allocation, drop, or Unique rules.

## Verification

The selective source sync, content lock, importer/content/core checks, and
standalone desktop build form the acceptance boundary. Only the `dungeon` and
`magic-realms` contract categories change: Warrens allocation gains the new
actors, and undead summons may now select Crypt Creep. Protocol, save, and
state-hash inputs do not change.
