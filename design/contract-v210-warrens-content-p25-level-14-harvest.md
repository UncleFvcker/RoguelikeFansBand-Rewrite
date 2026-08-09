# Contract v210: Warrens P25 Level 14 Harvest

## Scope and authority

P25 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the 19 level-14 records
whose observable monster behavior is already represented by the current actor,
melee, allocation, resistance, drop, and monster-casting contracts:

- Death sword, Software bug, Lurker, Nixie, Vlasta;
- Giant white dragon fly, Snaga sapper, Blue icky thing, Gibbering mouther;
- Irish wolfhound of Flora, Flesh golem, Cheerful leprechaun, Giant flea;
- Ufthak of Cirith Ungol, Orcish Artillery, Hibagon, Giant cockroach, Lion, and
  Snow leopard.

All Chinese names come from the authoritative `master` localization table. The
Hibagon source record has no description line, so its locale description remains
the identity-only `Hibagon.` / `比婆怪兽。` rather than adding local lore.

## Reused contracts

The five casters reuse existing stable ability records: cold and light breath,
blind, confuse, scare, blink, and the `3d6` physical bolt. The other facts reuse
existing multiplication, random movement, group allocation, habitat, dungeon
index, wilderness-only, rideable, Unique, remains, themed drop, and exploding
melee fields. Possessor-only `S:MULTIPLY`, `CLAIRVOYANCE`, and
`DETECT_MONSTERS` hints do not become monster abilities.

This batch adds no ability, effect, protocol field, state-hash input, save field,
compatibility path, or generic framework.

## Deferred level-14 records

- Warg keeps the existing Pest Control actor until its hand-authored identity is
  merged; no duplicate actor is created.
- Plague monk and Skaven assassin remain blocked on a real sewer-task consumer
  for `COMPOST`.
- Lady Zhurong and Flaming crow remain blocked on fire contact aura behavior.
- The Variant Maintainer remains blocked on `POLYMORPH` and fixed Software bug
  summoning.

## Content and acceptance

- Strict monster selection grows from 214 to 233 records; the demo pack grows
  from 279 to 298 actors, while abilities remain at 130.
- Demo pack is 1.206.0 with content hash
  `e4d423b5dc4cb246897e44a006f1b7cf3d638d30a5c88ab604140bbcafbba7bf`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v210 with 470 exact fixtures and zero waivers.
- Full verification keeps all 470 fixture results unchanged; no fixture is
  refreshed.
- Focused tests cover the complete level-14 roster, shared casting identities,
  multiplication/random movement, death explosion, groups, dungeon filtering,
  wilderness-only allocation, riding, and Unique identity.
