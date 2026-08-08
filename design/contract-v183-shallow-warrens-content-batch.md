# Contract v183: Shallow Warrens Content Batch

Status: active baseline. Protocol `1.139`, demo pack `1.178.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`9dcd0be8ca01927b4b25cf466c654149e8a8de627360967cdf418ee601e687b6`.

## Authoritative source

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. Monster and item indexes, names,
levels, rarity, HP dice, speed, defenses, blows, flags, allocation depth, item
weight, value, slot, and equipment modifiers come from that ref. Chinese display
names exactly follow its runtime Chinese name tables.

## First formal batch

The pack adds ten supported level-one monsters: Large White Snake, Giant White
Centipede, White Icky Thing, Large Brown Snake, Green Worm Mass, Grid Bug,
Jackal, Soldier Ant, Insect Swarm, and Bomb Mosquito. Blue Yeek at level 2 and
Black Naga at level 3 complete the first shallow drop-producing slice.

Their formal definitions exercise the already completed Warrens contracts:

- global allocation retains original index, rarity, maximum depth, glyph
  preference, friends dice, multiplication, and random movement;
- instance HP dice, flying/swimming movement, door bashing, multi-blow attacks,
  poison, electricity, death explosion, and intrinsic light use the existing
  W7-W12 runtime paths;
- Blue Yeek and Black Naga use their original `DROP_60` through
  `demo.loot-table.warrens`; corpse and skeleton flags remain independent W13
  remains definitions.

Grey Mold and Blinking Dot remain outside this batch. Their original records
require a formal immobile movement rule and a formal BLINK ability binding,
respectively; neither behavior is silently discarded.

Several selected actors also carry original perception/AI metadata such as
`STUPID`, `WEIRD_MIND`, `EMPTY_MIND`, and `POS_GAIN_AC`, plus wilderness-only
habitat tags. Those flags are not presented as implemented behavior: their
per-actor omissions are recorded in the Warrens mechanism backlog until the
monster-knowledge, special-mind, and wilderness systems exist.

## Shallow items and drops

The fixed-source selection grows from 35 to 40 items with Broken Dagger (45),
Broken Sword (46), Pointy Hat (225), Filthy Rag (246), and Paper Armour (248).
All five pass `sync-demo-items` without active behavior or unmapped functional
flags. Their Warrens minimum depths are 0, 0, 3, 0, and 5, matching original
object levels. The `DROP_60` monsters and ordinary floor loot share this table,
so the new equipment is reachable through both established generation paths.

## Verification

The fixed-source sync, deterministic source compilation, content lock check,
focused content tests, relevant contract categories, and standalone desktop
build form the acceptance boundary. No protocol DTO or state-hash input changes
are introduced by this batch.
