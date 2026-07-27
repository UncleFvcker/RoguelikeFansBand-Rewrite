# Contract v103: dynamic affix instances and equipment passives

## Scope

Contract v103 turns an affix from a purely static content reference into a
seeded item-instance payload. Static affix properties still work unchanged;
an affix may additionally declare weighted `rollGroups`, and every generated
item stores the selected property bundle.

The protocol is `1.103`, the demo pack is `1.94.0`, the state-hash schema is
`42`, and the active baseline contains 328 exact fixtures with zero waivers.
The built-in content hash is
`271fcf3f85ca347791150dbc8eec0040b9dd70e8315bdb3874bc2fc628d637bd`.

## Content contract

`ItemDefinition` and `AffixDefinition` share two new property surfaces:

- `equipmentBonuses`: extra melee attacks and bonuses to melee, ranged,
  throwing, device, saving throw, stealth, search, perception, disarming,
  digging, infravision, and light radius.
- `passives`: see invisible, telepathy, levitation, regeneration, hold life,
  six attribute sustains, blessed, easy spell, and device power.

An affix `rollGroup` declares a positive roll count and weighted candidates.
Each candidate has an inclusive depth interval and one non-empty property
bundle. Content validation rejects empty groups, zero weights, inverted depth
ranges, invalid resistance/immunity entries, empty candidates, and equipment
properties on non-equipment items.

Generation filters candidates by the current floor depth before drawing.
An empty eligible set contributes no property and consumes no RNG. Eligible
candidates use the existing stable weighted-index routine. Repeated results
merge deterministically: numeric values saturating-add, resistance and slay
tiers retain the stronger value, while immunities, brands, and passives form
sorted sets.

## Authoritative instance and migration

Every item instance stores sorted `rolledAffixes`; each entry contains the
affix ID and the fully materialized property bundle. Save loading validates
that IDs are unique, sorted, present in the item's `affixIds`, and that every
property collection is normalized and non-empty.

Old saves without `rolledAffixes` load an empty payload and consume zero RNG.
They are never re-rolled from the current content pack. This keeps old saves
stable when candidate weights or depth bands change later.

Rolled payloads enter state hash Schema 42. They survive ground, inventory,
equipment, carried-monster, stored-floor, and save round-trip transitions.
Knowledge remains separate: unidentified items do not expose hidden rolled
properties to the normal item DTO.

## Runtime consumers

Static and rolled modifiers, defenses, slays, brands, equipment bonuses, and
passives use one aggregate path. Equipment skill bonuses and extra melee
attacks enter `DerivedStatsPipeline` as equipment sources.

`regeneration` is the first active passive consumer. While any equipped item
provides it, the player heals 1 HP every 10 world ticks, capped at effective
maximum HP, and emits `equipment.regenerated`. The other passive IDs are now
authoritative content/save/DTO vocabulary but retain no independent gameplay
consumer yet; they are foundations for visibility, experience drain, sustain,
spell, and device iterations.

The Web inventory and equipment panels display every known equipment bonus and
passive in English and Chinese. Contract final-state assertions now retain the
full inventory/equipment DTOs, so knowledge gating and materialized rolls are
visible in fixtures instead of being represented only by state hashes.

## Demo and fixtures

`demo.affix.adaptive-echo` always grants regeneration and has separate shallow
and depth-10 candidate pools. `demo.actor.adaptive-echo` drops a fine
`demo.item.adaptive-glaive` through a dedicated isolated loot table.

- Fixture 327 uses seed 0 and records `+12 melee skill`, `+3 search`, healing,
  identification, equipment DTO, and an identical save round-trip hash.
- Fixture 328 uses seed 4 and records `+1 melee attack`, `+4 digging`, healing,
  and the same lifecycle boundaries.

Core tests additionally fix depth filtering, no-redraw legacy migration,
rolled-payload validation, derived-stat aggregation, and regeneration.

## Legacy import result

The e_info importer maps the pval skill/attack flags, infravision/light,
ability passives, and fear/blindness immunity. It also emits simplified
original-style roll recipes for Weapon Slaying, Weapon Craft, Crown Telepathy,
Light Scrying, Boots Speed, and Ring Speed.

The current real-source run imports 128/160 egos (up from 107/160 at P50) and
all 392 artifacts. `RES_FEAR` and `RES_BLIND` no longer appear in the unmapped
ego flag report. The generated compatibility source compiles to 128 affixes
with content hash
`0f106018471df0e60a16ab69bf3c496f13ceb0ef44d549476b662bcb06ba4378`.

The remaining 32 egos mainly depend on reflection, damage auras, curses,
extra shots/might, advanced brands, random resistance/sustain recipes, and
device or spell-power consumers.
