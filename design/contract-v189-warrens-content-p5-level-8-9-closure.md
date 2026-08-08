# Contract v189: Warrens Content P5 Level 8–9 and Shallow Closure

Status: active baseline. Protocol `1.139`, demo pack `1.184.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`9d6be77bac135d2dad8f6c6067f34750c57f02121f905e8606197c2d043d606d`.

## Authoritative source and selection

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The fixed-index selection in
`packs/rfb-demo-original/legacy-warrens-monster-selection.json` now contains 63
monsters. Chinese names follow `master:src/monster_name_zh.inc` exactly and
Chinese descriptions use the selected `master:lib/edit/r_info.txt` records.

The level-eight and level-nine source set contains 45 records. Seven were already
formal content: Bandit, Hunting hawk of Julian, Bloodshot icky thing, Black
harpy, Brodda the Easterling, Chiokovo, and Crow of Durthang. P5 adds every
remaining record whose active allocation, movement, melee, spell, resistance,
status, drop, and remains behavior can be expressed by the current runtime.

## P5 monster batch

Twelve monsters enter the global allocation pool:

- Skeleton orc retains doors, Orc/Undead categories, elemental resistances,
  status immunities, and its physical attack;
- Nurgling retains `FRIENDS(2d2, 25%)`, doors, demon categories, disease bite,
  poison immunity, and fire resistance;
- Brown yeek retains carried radius-one light, doors, acid resistance,
  `DROP_60`, and its physical attack;
- Carnivorous flying monkey retains flight, three ordered attacks, animal tags,
  and corpse drops;
- Lemure retains the source `FRIENDS` group marker, doors, demon categories,
  fire resistance, and its physical attack;
- Hill orc retains `FRIENDS(3d3)`, doors, light vulnerability, dark resistance,
  `DROP_60`, Warrior-theme drops, and corpse/skeleton remains;
- Giant grey rat retains 25% random movement, multiplication, poison resistance,
  and its poisonous bite;
- Skaven retains `FRIENDS`, doors, two ordered attacks, combined `DROP_60` and
  `DROP_90`, and corpse/skeleton remains;
- Rock mole retains door bashing, wall and ground-item destruction, two ordered
  bites, and corpse drops;
- Giant pink ant retains door bashing and its bite-plus-Strength-draining sting;
- War bear retains `FRIENDS(1d7)`, door bashing, three ordered attacks, and
  corpse/skeleton remains. `BERSERK` remains possessor-only and does not become
  monster casting;
- Killer bee retains speed 120, flight, `FRIENDS(1d4, 50%)`, poison, and its
  Strength-draining sting.

The batch adds no ability, item, or loot-table definition. Formal content is now
128 actors, 146 items, and 82 abilities.

## Deferred level-eight and level-nine records

Twenty-six records remain outside formal allocation. Their blocking behavior is
already explicit rather than hidden behind omissions:

- Wormtongue, Robin Hood, and Nami need theft, ground-item pickup, traps, sleep
  AI, or special friendly behavior;
- Lagduf and Balcmeg need the chance-gated Stun melee rider;
- Giant salamander, Wounded Bear, Orc shaman, five baby dragons, and Skaven
  shaman need sleep AI; the shamans also need their Priest drop theme;
- Space monster, Phantom warrior, and Jibaku ghost need wall passage, while
  Space monster and Green mold also need the Terrify melee rider;
- Gremlin needs food theft and ground-item pickup;
- Portuguese man-o-war needs aquatic-only movement and the Paralysis melee
  rider;
- Wounded Bear, Bloodfang, and Eagle need wilderness-only allocation;
- King cobra needs the Blind melee rider;
- Culverin needs a ranged-only `NEVER_BLOW` actor path;
- Unruly horse needs riding behavior;
- Lousy needs the `S_LOUSE` summon category.

## Shallow milestone closure

P1–P5 have now enumerated all 173 authoritative source records at levels 1–9.
The formal pack contains 95 actor definitions with a matching shallow legacy
index: 63 are maintained by the strict source selection/sync path and 32 are
earlier hand-authored or task actors. The remaining 78 records stay attached to
named runtime mechanisms in the batch documents and implementation backlog;
none are silently absent from the shallow census.

This closes the current shallow-content import milestone, not the deferred
mechanisms themselves. Future work should unlock those records by implementing
the named shared behavior first, then adding them through the same strict
selection path.

## Verification

The fixed-index source sync and content lock verification pass at 128 actors.
Importer, content, localization, core, and contract checks pass. Selective replay
keeps `dungeon`, `monsters`, `scrolls`, and `campaign` unchanged; only the
`magic-realms` category is refreshed because Raise Dead can now select Skeleton
orc. The standalone debug desktop build also succeeds. Protocol, save, and
state-hash inputs do not change.
