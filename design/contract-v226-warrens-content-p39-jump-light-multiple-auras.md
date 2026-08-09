# Contract v226: Warrens P39 Jump Light and Multiple Auras

## Authority and scope

P39 reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds Blinking light
(source index 1279) and The Icky Queen (source index 909).

The Chinese actor names and descriptions use the authoritative
`master:src/monster_name_zh.inc`, `master:lib/help/PossessorStats.csv` and
`master:lib/edit/r_info.txt` strings for those source indexes.

## Implemented boundary

- `JMP_LIGHT(5d5)` compiles to one monster-only `jump-damage` effect. It rolls
  `5d5`, multiplies the result by `5/4`, projects light damage from the caster
  through radius five with ordinary RFB area falloff, then consumes one
  bounded destination draw to blink the caster within radius ten.
- Damage and resulting deaths resolve before the blink event. Planning keeps
  the damage footprint caster-centred and rejects casts with no hostile target
  in that footprint or no legal landing cell.
- Actor contact auras are now an ordered `contactAuras` list. Existing
  one-aura actors migrate directly; no legacy singular field is retained.
- The Icky Queen declares poison `2d3` followed by acid `2d3`. Every successful
  player melee strike resolves both in that order, reusing existing resistance,
  poison-status, immediate-damage and fatal-stop behavior.
- `DRAIN_MANA` and `S_KIN` generate the ordinary parameter records
  `drain-mana-11` and `kin-the-icky-queen`; blind, confusion, fear, drops,
  resistances, escort and door/item behavior reuse existing contracts.

No save field, state-hash input, protocol DTO, compatibility path or generic
effect-sequencing framework is added.

## Acceptance

- Strict monster selection increases from 365 to 367 records.
- Demo content increases from 430 to 432 actors and from 171 to 174 abilities.
- Demo pack is 1.222.0 with content hash
  `604d16879ffd80f5e678cc6363a900f3a9491fd8768ffd84f8ee4c3f940630d2`.
- Protocol remains 1.152 and State Hash Schema remains v72.
- Active baseline is contract-v226 with 470 exact fixtures and zero waivers;
  P39 adds focused Rust contracts without changing state-hash input.
