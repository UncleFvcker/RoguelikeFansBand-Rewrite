# Contract v206: Warrens P21 level-12 non-caster harvest

## Scope and authority

P21 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds 20 level-12 monsters
whose combat, ecology, allocation, movement, remains, and themed drops are
already expressible by existing actor fields.

This is a content-only harvest. It adds no ability, effect type, protocol field,
save field, state-hash input, compatibility path, or general framework.

## Selected records

- Yeti / 雪人, Grizzly bear / 灰熊, Black mamba / 黑曼巴蛇, White wolf / 白狼,
  Tiger / 老虎, and Swamp rabbit / 沼泽兔 reuse existing animal, wilderness,
  poison, cold, group, swimming, riding, remains, and attribute-damage facts.
- Nether worm mass / 虚空蠕虫团 and Abyss worm mass / 深渊蠕虫团 reuse existing
  experience-drain, multiplication, random movement, swimming, wall destruction,
  invisibility, and resistance facts.
- Golfimbul, the Hill Orc Chief / 山丘兽人首领高尔芬博, Swordsman / 剑客,
  Mauhur, the Orc Captain / 半兽人队长毛胡尔, and Meng You, the Brother of
  Meng Huo / 孟获之弟孟优 reuse Unique, escort, remains, Warrior-themed drops,
  and door interaction. Meng You retains the existing `DUNGEON_31` allocation
  filter and therefore does not enter the Warrens global pool.
- Ixitxachitl / 异西鳐, Mine-dog / 地雷狗, Hellcat / 地狱猫, Air spirit /
  气元素精灵, Skeleton human / 人类骷髅, Zombified human / 僵尸人类,
  Frumious bandersnatch / 狂暴的班德斯纳奇, and Spotted jelly / 斑点果冻
  reuse aquatic movement, poison, death explosion, groups, invisibility,
  nonliving/undead facts, resistances, fixed movement, and acid melee.

The source `S:BERSERK` records on Yeti and Grizzly bear and `S:MULTIPLY` records
on the worm masses are Possessor-only hints. They are intentionally not
projected as monster casting; the ecology-bearing `F:MULTIPLY` flags still map
to actor allocation.

Mauhur has no authoritative `D:` description. Its localized description value
therefore repeats the authoritative display name instead of inventing lore.

## Content and acceptance

- Strict monster selection grows from 176 to 196 records; the demo pack grows
  from 241 to 261 actors and remains at 116 abilities.
- Demo pack is 1.202.0 with 86 terrains, 261 actors, 204 items, 116 abilities,
  and 19 loot tables. Content hash is
  `8f68bd58310207e0a9e7d1370d1a09731213fb1323753f6f28e2182b8ef2f8dc`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v206 with 470 exact fixtures and zero waivers.
- Only `death.raise-dead-basic-pool` is refreshed because the two new undead
  records intentionally extend its summon pool; the other 469 fixtures remain
  byte-for-byte valid.
- Strict source sync, deterministic content compilation, the exact allocation
  roster, and a focused non-caster test guard the batch.
