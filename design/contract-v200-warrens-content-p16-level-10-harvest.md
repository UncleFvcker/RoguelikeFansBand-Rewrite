# Contract v200: Warrens P16 level-10 harvest

## Scope and authority

P16 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It selects twenty active level-10
records whose combat, movement, allocation, group, Unique, drop, resistance,
light, and status behavior is already represented by the current runtime.

No protocol, save, state-hash, effect, importer mapping, compatibility path, or
general framework is added. Source `S:` tokens used only by the possessor system
remain outside monster casting; the matching `MULTIPLY` actor flags continue to
drive the existing reproduction behavior.

## Content selection

The strict selection grows from 136 to 156 records:

- Barracuda / 梭鱼, Giant piranha / 巨型食人鱼: existing aquatic movement,
  water resistance, and ordered bites.
- Giant spider / 巨型蜘蛛, Giant white tick / 巨型白蜱, Giant fruit fly /
  巨型果蝇: existing poison, flight, random movement, reproduction, and remains.
- The Borshin / 波尔申, Grishnakh, the Hill Orc / 山丘兽人葛力斯那克,
  Hobbes the Tiger / 老虎霍布斯: existing Unique lifecycle, full HP, ordered
  melee, status effects, escorts or drops as declared by the source.
- Kamikaze yeek / 神风特攻伊克: existing light and self-destruct explosion.
- Sand-dweller / 沙漠栖息者, Shadow Creature of Fiona / 菲奥娜的暗影生物:
  existing groups, doors, habitats, drops, resistances, and remains.
- Clear mushroom patch / 透明蘑菇丛, Hairy mold / 长毛霉菌, Undead mass /
  亡灵聚合体: existing stationary movement, reproduction, poison, attribute
  loss, status immunity, and ordered blows.
- Owlbear / 枭熊, Blue horror / 蓝色惧妖, Wolf / 狼, Panther / 黑豹:
  existing animal movement, groups, doors, fear, remains, and habitats.
- Creeping gold coins / 爬行金币: existing gold-only drops and poison immunity.
- Lynx / 猞猁: imported with its original snow/wood habitats and `WILD_ONLY`;
  it does not enter ordinary Warrens allocation.

The batch creates no active monster ability. `DETECT_MONSTERS` and `MULTIPLY`
spell tokens on these records are possessor-only commands and are not projected
as monster abilities by RFB or the strict synchronizer.

## Compatibility and acceptance

- Protocol remains 1.145, save remains v1, and state hash remains Schema v68.
- Demo pack is 1.196.0 with 221 actors, 146 items, 106 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `9c57c9fee1ffad6eebe37c8be662219f2723ced96554a3adea008e06a6d0f3a2`.
- Active baseline is contract-v200 with 470 exact fixtures and zero waivers.
- The expanded undead summon candidate pool changes one `magic-realms` fixture;
  that category alone is refreshed and all 470 fixtures verify afterward.
- The strict source sync, content lock, and focused content/importer tests pass.
