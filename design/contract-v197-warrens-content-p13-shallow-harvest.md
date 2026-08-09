# Contract v197: Warrens P13 shallow harvest

## Scope and authority

P13 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It selects seven of the ten
active shallow records left by contract-v196. Existing melee effects,
monster casting, theft, traps, factions, habitats, appearance projection,
allocation, Unique state, and actor saves remain the owners of behavior.

## Exact zero HURT

Greater Hell-Beast declares three dice-less `HURT` blows. RFB resolves those
as `damroll(0, 0)`, so the importer emits the existing physical damage effect
with `damageDice: 0`, `damageSides: 0`, and `armorMitigated: true`. Validation
allows that one exact shape only. Dice-less elemental damage, poison, and
`DISENCHANT` remain unsupported rather than being widened into approximate
damage.

The three blows still make ordinary hit checks and emit hit events, but deal
zero damage. No new melee effect, damage path, protocol field, or save field
is introduced.

## Louse summon

`S_LOUSE` reuses the existing `summon-category` ability program with the RFB
default `1d3+1` count, caster-level maximum 9, and the existing long legacy
summon duration. Giant White Louse receives the `louse` tag and is the only
matching formal actor. Lousy itself uses `louse-king`, so it cannot recursively
summon another Lousy. This adds one importer mapping, not a new summon
framework or compatibility layer.

## Content selection

The strict selection grows from 126 to 133 records and the shallow formal
roster grows from 158 to 165:

- Greater Hell-Beast / 高阶地狱兽: zero-damage gaze/crush, resistances,
  wall destruction, swimming, teleport, and blink.
- Yellow jelly / 黄色果冻: stationary mana drain and existing immunities.
- Zog / 佐格虫: two damaging hits, doors, and ordinary drops; dice-less
  `DROOL` remains the existing non-effect method omission.
- Wormtongue, Agent of Saruman / 萨鲁曼的爪牙，巧言: theft, item pickup,
  traps, healing, slow, cold bolt, and poison ball.
- Robin Hood, the Outlaw / 绿林好汉，罗宾汉: theft, item pickup, traps,
  healing, shooting, and woodland habitat.
- Lousy, the King of Louses / 虱子王劳西: random movement, attribute drain,
  flight, and the bounded louse summon.
- Duck / 鸭子: aquatic-only shore/swamp habitat, friends, bite, and shriek.

The generated abilities are cold bolt `6d8+2`, poison ball `12d2`, physical
shot `3d6`, and summon louse `1d3+1`. Existing `HEAL`, `SLOW`, `TRAPS`,
`SHRIEK`, `BLINK`, and teleport abilities are reused.

Silver jelly, Disenchanter eye, and Dark elf are the three remaining active
surveyed shallow records. P13 does not weaken their independent content gaps.

## Compatibility and acceptance

- Protocol remains 1.144, save remains v1, and state hash remains Schema v67.
- Demo pack is 1.193.0 with 198 actors, 146 items, 104 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `de810d68f142e4f1574f5d17ed58323c0d10f877c29373dc752a7b0493394698`.
- Active baseline is contract-v197 with 470 exact fixtures and zero waivers.
- Focused importer, content validation, and core combat tests cover zero HURT
  and louse summon generation. The global allocation expansion changes 16
  deterministic fixture snapshots; all 470 contract scenarios verify.
