# Contract v207: Warrens P22 level-12 casters

## Scope and authority

P22 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds seven level-12 casters:
Gazer / 注视者, Moon beast / 月兽, Master yeek / 大师伊克, Priest / 牧师,
Dark elven priest / 黑暗精灵祭司, Moaning spirit / 呻吟的幽灵, and
Devilfish / 魔鬼鱼.

All seven use existing monster casting, targeting, status, damage, healing,
teleport, summon, breath, melee, movement, and themed-drop contracts. P22 adds
no effect type, runtime branch, parameter override layer, protocol field, save
field, state-hash input, or compatibility path.

## Ability reuse and parameter signatures

Monster casting continues to bind stable ability IDs. Identical parameter
signatures share one content record; different source values receive one
generated record without adding a new ability mechanism.

P22 generates exactly eight records:

- `paralyze`
- `heal-36`
- `curse-8d8`
- `summon-legacy-import-l12-1d1`
- `bolt-physical-2d6-4`
- `breath-chaos-17-600-r2`
- `breath-disenchant-17-500-r2`
- `breath-time-33-150-r2`

Moon beast, Priest, and Dark elven priest share `heal-36`; Master yeek and
Priest share the level-12 one-monster summon. Existing blindness, confusion,
fear, darkness, slow, poison ball, blink, long teleport, light/dark/sound
breaths, melee attribute/mana drain, wall passage, Priest drops, and Evil
Priest drops are reused unchanged.

`DETECT_EVIL`, `DETECT_MONSTERS`, and `BLESS` remain Possessor-only source
hints and do not become monster abilities. Devilfish retains `NEVER_BLOW` and
therefore casts without a melee routine.

## Content and acceptance

- Strict monster selection grows from 196 to 203 records; the demo pack grows
  from 261 to 268 actors and from 116 to 124 abilities.
- Demo pack is 1.203.0 with 86 terrains, 268 actors, 204 items, 124 abilities,
  and 19 loot tables. Content hash is
  `463afcc8f813025b618ed68697d3cc67c99483ed56f5dc598d62a30a120c8502`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v207 with 470 exact fixtures and zero waivers.
- Only `death.raise-dead-basic-pool` is refreshed because Moaning spirit extends
  the undead summon pool and all seven actors extend the general imported pool;
  the other 469 fixtures remain byte-for-byte valid.
- Strict source sync, deterministic content compilation, the exact allocation
  roster, and a focused spell-set/share test guard the batch.
