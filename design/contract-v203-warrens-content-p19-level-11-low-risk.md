# Contract v203: Warrens P19 low-risk level-11 monsters

## Scope and authority

P19 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It begins the level-11 roster
with six records already expressible by the current actor and ability model.

No runtime effect, ability definition, protocol field, save field, state-hash
input, compatibility path, or general framework is added.

## Content selection

The strict selection grows from 164 to 170 records:

- Baby multi-hued dragon / 多彩龙幼龙: five existing elemental breaths,
  flight, doors, gold drops, corpse, resistances, and 5% casting frequency.
- Vorpal bunny / 锋锐兔: two ordered bites, mountain/snow habitat, bashing,
  blink, corpse, and 12% casting frequency.
- Hippocampus / 马头鱼尾怪: two bites, aquatic movement, water resistance,
  rideability, and corpse.
- Zombified orc / 僵尸兽人: three hits, doors, undead/orc traits, elemental
  resistances, and confusion/fear/sleep immunity.
- Shallow puddle / 浅水洼: two acid touches, stationary swimming, nonliving
  identity, elemental resistances, and status immunities.
- Lug, the Grotesque / 怪诞者卢格: ordered four-hit routine with a 10% stun,
  escort allocation, Warrior drops, remains, resistances, and Unique lifecycle.

The remaining level-11 records that require shapechanging, restricted dungeon
habitats, or a poison aura remain outside this batch.

## Compatibility and acceptance

- Protocol remains 1.145, save remains v1, and state hash remains Schema v68.
- Demo pack is 1.199.0 with 235 actors, 146 items, 114 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `f3a9f16c4d40fa7b6b4472f856fbf22af7e33727324afc5f2962ad84cf11912b`.
- Active baseline is contract-v203 with 470 exact fixtures and zero waivers.
- Adding Zombified orc to the undead candidate pool changes one Raise Dead
  fixture; only the `magic-realms` category is refreshed.
- Strict source sync, content lock, focused content/localization tests,
  workspace checks, and all 470 contract fixtures are verified.
