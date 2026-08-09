# Contract v225: Warrens P38B Healing and Summoning

## Authority and scope

P38B reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds Daemonette of
Slaanesh, Meng Huo the King of Southerings, Ixitxachitl priest, Quylthulg,
Paladin and Ranger.

Chinese display names use the authoritative `master:src/monster_name_zh.inc`
and `master:lib/help/PossessorStats.csv` entries for the same source indexes.

## Implemented boundary

- Eight generated ability records preserve the source healing amounts,
  summon categories and levels, kin identity, count dice or damage dice.
- Daemonette of Slaanesh reuses P38A's fire and cold bolts alongside the new
  level-18 demon summon.
- Quylthulg and Ranger share the same level-20 single-monster summon.
- Meng Huo's kin summon targets his own actor kind; Ixitxachitl priest and
  Paladin receive the source-derived 57 and 60 point healing records.
- Existing fear, curses, confusion, blind, slow, blink and target-drag
  abilities remain shared rather than duplicated.
- Detection, mapping and blessing remain possessor-only tokens and do not
  enter monster casting profiles.

No effect type, runtime branch, protocol field, save field, compatibility path
or generalized parameter framework is added.

## Acceptance

- Strict monster selection increases from 359 to 365 records.
- Demo content increases from 424 to 430 actors and from 163 to 171 abilities.
- Demo pack is 1.221.0 with content hash
  `acc9186760331c90d5c3218755950ac186460f234760b8e9e995645ec41caba7`.
- Protocol remains 1.152 and State Hash Schema remains v72.
- Active baseline is contract-v225 with 470 exact fixtures and zero waivers.
