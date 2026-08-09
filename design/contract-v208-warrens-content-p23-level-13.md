# Contract v208: Warrens P23 level-13 monsters

## Scope and authority

P23 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds all ten selected
level-13 monsters: Drider / 蛛化精灵, Mongbat / 猴蝠, Killer brown beetle /
杀手褐甲虫, Boldor, King of the Yeeks / 伊克之王博尔多, Ogre / 食人魔,
Creeping mithril coins / 爬行秘银币, Druid / 德鲁伊, Cloaker / 暗幕魔兽,
Black orc / 黑兽人, and Ochre jelly / 赭冻怪.

The batch reuses existing melee, poison, paralysis, fear, status immunity,
stationary movement, group, escort, light, habitat, remains, gold, themed-drop,
and monster-casting contracts. It adds no effect type, runtime branch, protocol
field, save field, state-hash input, compatibility path, or parameter override
layer.

## Ability reuse and parameter signatures

P23 generates five stable ability records:

- `heal-39`
- `haste-self`
- `kin-boldor-king-of-the-yeeks`
- `bolt-fire-9d8-4`
- `bolt-electricity-4d8-4`

The original plan counted four new records, but strict generation showed that
the Druid's `HASTE` had runtime support without a legacy monster-facing
ability record. `haste-self` therefore binds the existing 25-tick Haste status
effect without adding a mechanism.

Drider reuses confusion, `3d8` curse, darkness, `2d6+4` missile, and `3d6`
shooting. Boldor reuses blink, long teleport, blindness, and slow; its `S_KIN`
uses the existing fixed-kind summon contract. Black orc reuses `2d7` shooting.
`DETECT_MONSTERS` and `MAPPING` remain Possessor-only source hints and do not
become monster abilities.

## Content and acceptance

- Strict monster selection grows from 203 to 213 records; the demo pack grows
  from 268 to 278 actors and from 124 to 129 abilities.
- Demo pack is 1.204.0 with 86 terrains, 278 actors, 204 items, 129 abilities,
  and 19 loot tables. Content hash is
  `4b1c3378af39464ad9450bfc3148fc338b79f3ccd17bedf6fe2f776d226e23cb`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v208 with 470 exact fixtures and zero waivers.
- All 470 existing fixtures remain exact; no fixture refresh is required.
- Strict source sync, deterministic content compilation, the exact allocation
  roster, and one focused level-13 behavior test guard the batch.
