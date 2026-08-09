# Contract v202: Warrens P18 pseudo dragon

## Scope and authority

P18 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It selects source index 193,
Pseudo dragon / 伪龙, including its ordered melee routine, resistances,
movement, allocation, drops, remains, and spell frequency.

The source uses `LIGHT` for one melee effect while its broader elemental
vocabulary uses `LITE`. The importer now treats only those two source tokens as
the same existing `light` damage type. No runtime damage type, effect,
compatibility path, protocol field, save field, or state-hash input is added.

## Imported behavior

- Two claws deal `1d4` physical damage each.
- The bite deals `1d5` physical damage, then independently has a 20% chance to
  deal `1d3` light damage and a 20% chance to deal `1d3` dark damage.
- Light and dark breath reuse the existing health-scaled cone effect at 17% of
  current health, capped at 400 damage, with radius 2.
- Confusion, fear, flight, door bashing, maximum hit points, forced sleep,
  light/dark resistance, corpse drops, and the original 5% casting frequency
  all use existing content fields.

The strict selection grows from 163 to 164 records. The two breath definitions
are the only new abilities: `breath-light-17-400-r2` and
`breath-dark-17-400-r2`.

## Compatibility and acceptance

- Protocol remains 1.145, save remains v1, and state hash remains Schema v68.
- Demo pack is 1.198.0 with 229 actors, 146 items, 114 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `6177272314068a98182321ef35baf1214c726a2230a156a57fb71f2bf72112e8`.
- Active baseline is contract-v202 with 470 exact fixtures and zero waivers.
- Strict source sync, content lock, focused importer/content/localization tests,
  workspace checks, and all contract fixtures are verified.
