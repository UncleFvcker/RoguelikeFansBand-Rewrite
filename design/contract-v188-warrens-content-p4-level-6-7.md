# Contract v188: Warrens Content P4 Level 6–7 Monsters

Status: active baseline. Protocol `1.139`, demo pack `1.183.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`8ebbd92c027da328b2c65f32d169a98942dc8310161c959719b0749670815a7c`.

## Authoritative source and selection

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The fixed-index selection in
`packs/rfb-demo-original/legacy-warrens-monster-selection.json` now contains 51
monsters. Chinese names follow `master:src/monster_name_zh.inc` exactly and
Chinese descriptions use the selected `master:lib/edit/r_info.txt` records.

The level-six and level-seven source set contains 29 records. Eight were already
formal content: Nibelung, Rat-thing, Snaga, Crypt Creep, Cave orc, Night Lizard,
Mughash the Kobold Lord, and Novice mindcrafter. P4 adds every remaining record
whose allocation, movement, melee, spell, resistance, status, drop, and remains
behavior can be represented without deleting an active source rule.

## P4 monster batch

Ten monsters enter the global allocation pool:

- Purple mushroom patch retains stationary swimming movement and three ordered
  spore attacks, each combining physical damage with Constitution drain;
- Disembodied hand that strangled people retains speed 130, flight, doors,
  undead tags and immunities, and its crushing attack;
- Giant brown bat retains speed 130, flight, 50% random movement, and its bite;
- Rattlesnake retains 50% random movement, swimming, door bashing, poison
  resistance, and its combined bite and poison routine;
- Zombified kobold and Rotting corpse retain their full ordered attacks,
  undead resistances and status immunities; Rotting corpse also retains its
  `FRIENDS(2d3, 50%)` group rule;
- Wood spider retains speed 120, `FRIENDS(3d3)`, door bashing, poison
  resistance, and its bite-plus-sting routine. `DETECT_MONSTERS` remains a
  possessor-only token and does not become monster casting;
- Manes retains `FRIENDS`, door interaction, demon tags, fire resistance, and
  its physical attack;
- Pink jelly retains stationary swimming movement, light vulnerability, and
  its Strength-draining touch;
- Caustic icky thing retains 50% random movement, swimming, intrinsic radius-two
  light, acid resistance, and acid touch.

The batch adds no new ability, item, or loot-table definition. Formal content is
now 116 actors, 146 items, and 82 abilities.

## Deferred level-six and level-seven records

Eleven records remain outside the selection because at least one active rule is
not formal yet: Brown mold needs the dice-less Confuse melee rider; Novice
archer and Novice ranger need sleep AI and the Archer drop theme; Creeping
silver coins needs silver-material interaction; Giant slug needs `KILL_BODY`
and its dice-less Slow crawl; Giant pink frog is wilderness-only; Dark elf needs
sleep AI and Darkness; Bloodshot eye needs the dice-less Blind melee rider;
Pink naga needs ground-item pickup; Lost soul needs wall passing, ground-item
pickup, and melee mana drain; Novice paladin needs sleep AI and the Paladin drop
theme.

Declared omissions remain limited to sex, special mind/possessor metadata, and
wilderness habitat tags. They do not replace an active combat, allocation,
movement, drop, material, or lifecycle rule.

## Verification

The fixed-index source sync, content lock verification, importer/content/core
checks, behavior-affected contract categories, and standalone desktop build form
the acceptance boundary. Dungeon, monster, scroll, and campaign fixtures remain
exact; only the `magic-realms` Raise Dead fixture changes because the shallow
undead pool gains four eligible actors. Protocol, save, and state-hash inputs do
not change.
