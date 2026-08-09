# Contract v221: Warrens P36 Elemental Contact Auras

## Scope and authority

P36 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the three remaining
level-17 elemental spheres: Freezing sphere, Jumping fireball, and Ball
lightning. English identities follow the authoritative `N:` records, while
Chinese names exactly match the authoritative runtime localization table.

## Narrow contact-aura contract

RFB derives elemental touch damage as `1 + level / 26` dice of
`1 + level / 17` sides, so all three level-17 records use `1d2`. The existing
`contactAura` content shape is expanded only from poison to fire, cold, and
electricity. Poison keeps its existing status-duration behavior; the three
elemental types apply immediate resistance-aware HP damage through the
existing damage transaction and `MonsterMeleeHit`/`PlayerDied` events.

The importer maps only `AURA_FIRE`, `AURA_COLD`, and `AURA_ELEC`, and rejects
records that would require more than one contact aura. It introduces no new
effect, status, protocol event, compatibility layer, or general aura system.

## Existing death behavior

Each sphere keeps its authoritative `EXPLODE` blow as an `8d8` self-
destructing melee routine of the same element. Existing flight, intrinsic
light, resistances, vulnerabilities, nonliving classification, allocation,
and death-explosion runtime are reused unchanged.

## Content and acceptance

- Strict monster selection grows from 297 to 300 records; the demo pack grows
  from 362 to 365 actors and remains at 152 abilities.
- Demo pack is 1.217.0 with content hash
  `d70909839615bb837a0b3ee4d348d29a887989f145d42c22aa90461dff67fcca`.
- Protocol remains 1.152, save remains v1, and state hash remains Schema v72.
- Active baseline is contract-v221 with 470 exact fixtures and zero waivers.
- Focused tests lock the three flag mappings, `1d2` contact auras, resistance-
  aware immediate damage, unchanged poison behavior, and matching `8d8`
  self-destruct damage.
