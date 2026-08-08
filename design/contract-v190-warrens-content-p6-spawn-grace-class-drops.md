# Contract v190: Warrens Content P6 Spawn Grace and Class Drops

Status: active baseline. Protocol `1.140`, demo pack `1.185.0`, save v1, and
state hash Schema v64. Old development saves are not supported.

The built-in content hash is
`e7a7697de6aab4160c2398cba429559fa7fd62c46b65f3bb929490d859395f3e`.

## Authoritative source

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, read
through Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. Chinese monster names and
descriptions follow that ref exactly.

`RF1_FORCE_SLEEP` does not apply the ordinary sleep timer. Placement gives the
monster `MFLAG_NICE`: during that one-player-action window it cannot cast, and
melee effect rolls above 50 against the player are limited to `25 + roll / 2`.
The flag is cleared after the player's next world-advancing command. Monsters
created during that command retain their own grace window for the following
player action.

## Runtime and persistence

- Actor content adds `forceSleep`; spawned runtime actors stamp it into the
  authoritative `nice` state.
- Nice actors may still detect, move, open or bash doors, and make melee
  attacks. Their monster spell path returns before cooldown or RNG work.
- The high-damage limiter is applied to the raw player-directed melee effect
  roll before armor or resistance. Monster-versus-monster combat is unchanged.
- Zero-time commands do not consume the grace window. Saves and stored floors
  persist the required `nice` boolean, and state hashes include it under Schema
  v64.

## Class drop tables

The strict monster synchronizer now accepts the five class themes required by
this batch: Mage, Archer, Priest, Evil Priest, and Paladin. Each maps to a
formal shallow loot table and keeps the original 50-percent themed branch.
Tables contain only already-formal items that satisfy the corresponding RFB
predicate and are suitable for Warrens depth 0–9. The existing Warrior table
is unchanged.

The synchronizer also writes generated monster abilities and programs beside
the actor output. If an authored ability already owns the natural file name,
the generated legacy file receives a `legacy-` prefix instead of overwriting
the authored definition.

## P6 monster batch

Thirteen previously deferred monsters enter the global allocation pool:

- Novice mage, Novice priest, Novice archer, Novice ranger, and Novice paladin
  retain their source light, groups, doors, remains, spells, spawn grace, and
  matching class drops;
- Giant salamander retains random movement, intrinsic light, swimming, fire
  resistance, fire bite, fire breath, corpse, and spawn grace;
- Orc shaman and Skaven shaman retain doors, remains, Evil Priest drops,
  source spells, and spawn grace;
- Baby blue, white, green, black, and red dragons retain maximum HP, flight,
  elemental bites, matching resistances and breaths, gold-only drops, corpses,
  and spawn grace.

The strict selection grows from 63 to 76 monsters. The formal pack grows from
128 to 141 actors, from 82 to 93 abilities, and from 14 to 19 loot tables;
items remain at 146.

## Verification

Focused tests cover zero-RNG casting suppression, cooldown preservation,
save/hash round-trip, zero-time retention, world-advance clearing, and the
original high-damage formula. Source synchronization, content lock, schema,
localization, core and importer checks pass. Because the save-backed state-hash
input changed, all 470 exact fixtures are refreshed to Schema v64; waivers
remain zero. The standalone desktop build is the playable acceptance artifact.
