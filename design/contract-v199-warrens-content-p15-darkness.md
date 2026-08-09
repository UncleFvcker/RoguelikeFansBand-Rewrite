# Contract v199: Warrens P15 darkness

## Scope and authority

P15 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds Silver jelly at legacy
index 73, Dark elf at index 122, and only the room-light state required by
their existing source behaviors. Existing monster casting, light sources,
floor storage, save, and state-hash paths remain the owners of runtime state.

## Permanent room glow

Every generated room cell starts with a persistent `glow` bit. Inline maps do
not infer glow. The active floor and every stored floor carry the same
fixed-size bit vector, and floor transitions move it without regeneration.

`darken-room` clears the eight-connected glowing component containing the
cast target. This is the current room boundary available from generated map
state; it does not add room IDs, lighting layers, duration state, or new RNG.
The ability targets a position or entity at range 8 and does not require line
of effect, matching the source spell's target-room operation.

Actor light retains one radius shape and adds a `darkness` polarity. A dark
source suppresses permanent room glow inside its radius. It does not suppress
the player's carried light, positive actor light, or outdoor daylight. Silver
jelly's carried radius-1 darkness remains inactive while the actor sleeps,
using the existing non-intrinsic light-source rule.

## Content selection

The strict selection grows from 134 to 136 records and the shallow formal
roster grows from 166 to 168, leaving no active surveyed shallow records.

- Silver jelly / 银色果冻 retains its source level, HP dice, speed, armor,
  experience, allocation, stationary swimming, immunities, poison resistance,
  two light-eating touches, and 1-in-16 spell frequency. Its source level-3
  mana drain reuses the existing amount-2 drain-resource ability.
- Dark elf / 黑暗精灵 retains its source level, HP dice, speed, armor,
  experience, allocation, sleep, door interaction, remains, drops, light
  vulnerability, dark resistance, two hits, and 1-in-10 spell frequency. Its
  spell list reuses confusion and physical missile and adds 制造黑暗.

## Compatibility and acceptance

- Protocol is 1.145 and save remains v1. Required `TerrainSaveDto.glow` covers
  active and stored floors; old development saves are intentionally rejected.
- Permanent glow enters state hash Schema v68. Dark actor polarity is content
  data and the resulting visible cells remain deterministic projections.
- Demo pack is 1.195.0 with 201 actors, 146 items, 106 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `b67309b1973ab483e71c90fce594d20af1d66bbb7b4ada6665fbcdbd4f513e18`.
- Active baseline is contract-v199 with 470 exact fixtures and zero waivers.
- Focused importer and core tests cover source flag mapping, room clearing,
  dark-source precedence, floor storage, save round-trip, and state hash.
  Content validation, generated schemas and bindings, localization, and all
  contract scenarios verify.
