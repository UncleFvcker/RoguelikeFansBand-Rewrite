# Contract v171: Warrens Ecology

Status: active baseline. Protocol `1.136`, demo pack `1.165.0`, save v1,
state hash Schema v61. This is a content-only milestone; it adds no old-save
compatibility path.

The built-in content hash is
`ab54279248422c2d39dc6e91b8827f6be1f15c4d9ab4c79ee60707e766abbb52`.

## Legacy reference

The read-only reference is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`. Its Warrens definition covers
depths 1–9, prefers glyphs `kKyYrRfFcCbB`, applies `MONSTER_DIV_16`, and names
Mughash as the final guardian. Original names, flags, levels, rarity, HP dice,
attack routines, and allocation code were inspected; no source code, map,
description text, or assets were copied.

## Content boundary

- The normal table now contains twelve supported legacy species from Newt at
  depth 1 through Chiokovo and the Hunting Hawk of Julian at depth 8.
- Entry weights use the original allocation basis `100 / rarity`; minimum depth
  follows monster level and maximum depth remains Warrens depth 9.
- Small Kobold, Kobold, and Large Kobold use their original poison resistance,
  Warrior-themed drop chance, attack routine, and average HP. Rat-thing uses its
  two bites and an isolated fear ability at the closest integer representation
  of `1_IN_9`.
- Mughash's current guardian definition uses forced maximum HP and four blows.
  Its escort and global unique lifecycle remain explicit future mechanisms.
- Warg is removed from the normal Warrens table. It remains defined for the
  future Pest Control task. Giant White Mouse is held out until reproduction
  and random movement exist.

## Deliberate differences

The current encounter table is a supported static slice, not a claim that the
legacy global allocator has been reproduced. Non-preferred monsters at quarter
weight, two-stage out-of-depth boosts, leader-first group expansion, dice-based
friends, unique persistence, movement domains, monster door use, and random HP
instances remain pending. Their ordered acceptance boundaries are recorded in
[Warrens monster mechanism backlog](warrens-monster-mechanism-backlog.md).

## Verification

Content tests fix the exact normal roster, weights, and depth windows and assert
that Warg is absent. Pack compilation and source verification cover all new
actor, ability, program, loot, and localization references. Contract refresh is
restricted to fixture categories whose projected output actually changes; no
desktop E2E suite is required for this content milestone.
