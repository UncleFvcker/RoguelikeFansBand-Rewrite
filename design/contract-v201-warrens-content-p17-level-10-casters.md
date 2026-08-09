# Contract v201: Warrens P17 level-10 casters

## Scope and authority

P17 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It selects seven active level-10
casters whose spell families, targeting, combat, allocation, groups, drops,
resistances, light, and movement are already implemented.

The batch adds no protocol field, save field, state-hash input, effect type,
compatibility path, or general casting framework. The strict synchronizer keeps
the existing `legacy-import` actor tag so the already-supported `S_MONSTER`
category has the same "any imported monster" candidate pool as the full legacy
import path.

## Content selection

The strict selection grows from 156 to 163 records:

- Dark elven mage / 黑暗精灵法师: blind, confuse, missile, darkness, poison
  ball, cold bolt, Mage drops, and original 20% casting frequency.
- Orfax, Son of Boldor / 博尔多之子欧法克斯: healing, blink, teleport-to,
  slow, confuse, one-monster summon, escort, light, and Unique lifecycle.
- Servant of Glaaki / 格拉基的仆从: wound curse, fear, disease, group chance,
  undead traits, and original 8% casting frequency.
- Dark elven warrior / 黑暗精灵战士: missile, Warrior drops, ordered melee,
  light vulnerability, and original 8% casting frequency.
- Quiver slot / 箭袋插槽: `2d5` shooting, confusion spore, stationary movement,
  reproduction, and original 20% casting frequency.
- Disenchanter mold / 解除附魔霉菌: six-point mana drain, disenchantment melee,
  stationary movement, swimming, and intrinsic radius-one light.
- Tengu / 天狗: blink, teleport-to, teleport-other, long teleport, flight, and
  original 33% casting frequency.

Six new parameterized ability definitions are generated while every runtime
effect remains reused: `bolt-cold-6d8-3`, `heal-30`, `drag`, `banish`,
`drain-mana-6`, and `summon-legacy-import-l10-1d1`. The source-explicit `1d1`
summon and `2d5` shot supersede the earlier planning estimates.

## Compatibility and acceptance

- Protocol remains 1.145, save remains v1, and state hash remains Schema v68.
- Demo pack is 1.197.0 with 228 actors, 146 items, 112 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `d645415a7e27e519eb27d6e88b096e46c4ac7cdda01dd981a6468e92218142dc`.
- Active baseline is contract-v201 with 470 exact fixtures and zero waivers.
- Servant of Glaaki expands the undead summon candidate pool, changing one
  `magic-realms` fixture; that category alone is refreshed.
- Strict source sync, content lock, focused importer/content/localization tests,
  workspace checks, and all 470 contract fixtures are verified.
