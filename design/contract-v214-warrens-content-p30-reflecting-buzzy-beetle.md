# Contract v214: Warrens P30 Reflecting Buzzy Beetle

## Scope and authority

P30 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds source record 951,
Buzzy beetle (`铁甲虫`), at level 15. The actor retains its four `2d5` hits,
five elemental resistances, confusion/sleep immunity, nonliving identity,
door bashing, gold drop, allocation, and source speed.

The English identity follows the authoritative `N:` record. The Chinese name
exactly follows the authoritative runtime localization table. The source has
no `D:` description, so the pack descriptions are independent factual text.

## Narrow bolt reflection

Actor content gains the default-false monster-only `reflectsBolts` fact.
When a player single-target ability or device bolt reaches such an actor:

- one `1-in-4` failure draw leaves the original hit unchanged; the other 75%
  reflect the bolt without damaging the reflector;
- reflection tries at most ten random destinations within two cells of the
  original caster, using the original Y-then-X draw order, then falls back to
  the caster position if none is projectable;
- an independent 50% draw controls whether the reflected path can strike the
  player; another actor on the reflected line remains a valid first target;
- the already rolled raw damage and existing armor, resistance, fatality,
  wake, death-explosion, and no-reward monster-death paths are reused.

Pure beams, area damage, cones, breaths, thrown items, and launched ammunition
do not enter this branch. P30 adds no generic projectile-redirection layer,
compatibility path, or persisted state.

## Content and acceptance

- Strict monster selection grows from 266 to 267 records; the demo pack grows
  from 331 to 332 actors, while abilities remain at 143.
- Demo pack is 1.210.0 with content hash
  `b6f4741928ed2c1ae56f65d5614b06a25a200cdcb2eb9abe44f96fe1da424e00`.
- Protocol remains 1.148, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v214 with 470 exact fixtures and zero waivers.
- Full verification leaves all 470 fixture results unchanged; no fixture is
  refreshed.
- Focused tests lock content import, the 75%/25% branches, direction changes,
  reflected player hits, and the exclusion of beam damage.
