# Contract v198: Warrens P14 Disenchanter eye

## Scope and authority

P14 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the level-5
Disenchanter eye at legacy index 104 and the one missing melee behavior that
blocked it. Existing resistance tiers, timed statuses, item enchantments,
monster casting, allocation, actor saves, and item saves remain the owners of
state and behavior.

## Narrow DISENCHANT melee

A dice-less `DISENCHANT` blow imports as the new melee effect
`{ "type": "disenchant" }`. A dice-bearing `DISENCHANT` remains ordinary
disenchant damage, so the special case cannot discard source damage dice or
widen another effect family.

After an ordinary melee hit, the effect follows the source's 4:1 split:

- four outcomes try to remove one currently active, modeled positive timed
  status; candidates are the existing haste, heroism/berserk/blessing,
  protection, temporary resistance, vengeance, stone skin, poetic
  inspiration, branding, necromantic, vampiric, and wraithform statuses;
- one outcome chooses an equipped weapon, armor, or ammunition item. If it has
  positive `toHit`, `toDamage`, or `toArmor`, each positive component loses
  one; a component still above five has the source's independent 20% chance
  to lose one more. Artifacts retain their 71% resistance.

Both outcomes first use the player's effective Disenchant resistance tier.
Negative statuses are never removed. Empty equipment candidates and selected
items without positive enchantments are honest no-ops. Actor-to-actor attacks
do not invent monster equipment or player-timeout semantics.

The current model has no item pval or selected-item `OF_RES_DISEN` knowledge,
so P14 does not add shadow fields for those source-only branches. Equipped
resistance already contributes through the existing effective player
resistance merge.

## Content selection

The strict selection grows from 133 to 134 records and the shallow formal
roster grows from 165 to 166. Disenchanter eye / 解除附魔之眼 keeps its source
level, HP dice, speed, armor, experience, allocation, stationary flight,
light vulnerability, Disenchant resistance, fear immunity, corpse chance,
gaze, and 1-in-9 casting frequency. Its `DRAIN_MANA` spell generates and uses
the existing drain-resource program at amount 3.

Silver jelly and Dark elf are the two remaining active surveyed shallow
records. P14 does not weaken their independent behavior gaps.

## Compatibility and acceptance

- Protocol remains 1.144, save remains v1, and state hash remains Schema v67.
  The effect mutates only existing status and item-enchantment state.
- Demo pack is 1.194.0 with 199 actors, 146 items, 105 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `47efafab50f3e2787d0a713aa2726b226fbddb8c93bc958a662a08411c2c369b`.
- Active baseline is contract-v198 with 470 exact fixtures and zero waivers.
- Focused importer and core tests cover the dice-less mapping, damage-dice
  preservation, positive-status removal, negative-status preservation,
  resistance, equipment degradation, and zero HP damage. Content validation,
  source synchronization, localization, and all contract scenarios verify.
