# Contract v205: Warrens P20 special mechanics

## Scope and authority

P20 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds six level-11/12
records whose remaining blockers are experience drain, contact poison,
appearance projection, a named drop alias, or dungeon-restricted allocation.

Each blocker stays a narrow content/runtime contract. No compatibility layer,
general effect framework, protocol field, save field, or new state-hash input is
added.

## Five narrow contracts

- `DRAIN_EXP`: Grape jelly / 葡萄果冻 drains `10d6 + 2%` of current experience,
  capped at 25,000. Current level follows the remaining experience; historical
  maximum experience/level and already-earned attribute rewards are preserved.
- `A:POISON`: Plague rat / 瘟疫鼠 adds a `1d2` contact aura. The already selected
  Blubbering icky thing also regains its authoritative `1d1` aura. A successful
  player contact hit triggers the aura even when that hit kills the monster;
  poison resistance, immunity, and the existing delayed-poison status path are
  reused.
- `SHAPECHANGER`: Chaos shapechanger / 混沌变形者 projects another monster's
  appearance at spawn and before each action. Kind, combat statistics, loot,
  faction, and AI remain those of the shapechanger; the existing saved, hashed,
  and projected `appearanceKindId` carries only the disguise.
- `DROP_WARRIOR_SHOOT`: Knight archer / 骑士弓箭手 maps this source alias to the
  existing Archer theme table and its original 50% themed-drop branch.
- `DUNGEON_31` / `DUNGEON_35`: King Duosi, the Chief of Southerings /
  南蛮大王朵思大王 and Wallaby / 小袋鼠 retain their Hideout and Plains of Oz
  restrictions. Dungeon definitions expose an optional positive, unique legacy
  index; the Warrens is index 30, so neither restricted actor enters its global
  allocation pool.

`COMPOST` on Plague rat remains an explicit omission because the pack has no
sewer quest allocation consumer. It is not approximated as a drop or a general
region rule.

## Content selection

The strict selection grows from 170 to 176 records and the demo pack from 235
to 241 actors. The six new actors are Chaos shapechanger, Grape jelly, Plague
rat, Knight archer, King Duosi, and Wallaby. Their source spells generate only
two new parameterized abilities, `bolt-fire-9d8-3` and `drain-mana-7`, bringing
the pack to 116 abilities.

## Compatibility and acceptance

- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Demo pack is 1.201.0 with 86 terrains, 241 actors, 204 items, 116 abilities,
  and 19 loot tables. Content hash is
  `e2fd133bcd3f2e3c2fd4d3ab8e25da6c437bfa18bede03d039d55a3db35406ae`.
- Active baseline is contract-v205 with 470 exact fixtures and zero waivers.
  All fixtures verify without refresh because none reaches the new actors or
  changes an existing allocation result.
- Focused importer, content, experience drain, contact aura, shape projection,
  special drop, region filtering, and validation tests cover the five contracts.

