# Contract v193: Warrens P9 pickup and theft

## Scope and authoritative behavior

P9 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the content surfaces
and runtime transactions for `TAKE_ITEM`, `EAT_GOLD`, `EAT_ITEM`, `EAT_FOOD`,
and `EAT_LITE` without adding alternate inventory or loot models.

After a successful monster move, `TAKE_ITEM` visits ground stacks in stable
instance-ID order and transfers eligible stacks to the monster's existing
`CarriedBy` inventory. Gold, corpses, skeletons, statues, artifacts, and items
whose slay or brand harms that monster are excluded. `TAKE_ITEM` takes
precedence over `KILL_ITEM` on the same actor. The existing actor-death
transaction returns carried stacks to the ground, so stolen and picked-up
items use the same persistence and recovery path.

The four melee effects retain their source ordering and optional independent
chance:

- `EAT_GOLD` skips confused thieves, applies the original 38-entry Dexterity
  safety table plus player level unless paralyzed, uses the original
  `gold / 10 + 1d25` formula and large-theft branch, and blinks after a
  successful attempt;
- `EAT_ITEM` uses the same protection gate, skips artifacts, splits exactly
  one item from the selected inventory stack into monster-carried state, and
  blinks after the attempt;
- `EAT_FOOD` removes one eligible non-artifact food item;
- `EAT_LITE` drains `251..500` fuel from the equipped non-artifact light and
  leaves at least one fuel.

The theft blink uses the original radius `MAX_SIGHT * 2 + 5`, currently 45.
Pickup, prevention, theft, consumption, and displacement all project
localized events. No protocol DTO was widened because these results use the
existing generic event envelope.

## Formal content

The strict selection grows from 89 to 97 monsters. P9 adds 小香雪兰
(Freesia), 斯密戈 (Smeagol), 哥布林 (Goblin), 绿娜迦 (Green naga),
粉红娜迦 (Pink naga), 小魔怪 (Gremlin), 吼牛者霍比特人
(Bullroarer the Hobbit), and 库塔熊 (Kutar). Their Chinese names and
descriptions exactly follow `master:src/monster_name_zh.inc` and
`master:lib/edit/r_info.txt`.

The shallow formal roster is now 129 actors, 97 maintained by the strict
selection path; 44 surveyed level 1–9 records remain deferred. The whole demo
pack contains 162 actors, 146 items, 96 abilities, and 19 loot tables.

巧言 (Wormtongue), 罗宾汉 (Robin Hood), and 奈美 (Nami) are not unlocked by
the theft work alone: their original records still require the unsupported
`TRAPS` monster spell. They remain deferred rather than receiving a partial
formal import.

## Compatibility and acceptance

- Protocol remains 1.141; no command, projection DTO, or persisted field
  changed.
- Save remains v1 and state hash remains Schema v64.
- Demo pack is 1.188.0 with content hash
  `4b1c823041b0f60b452d1161546ad4b3eb338b8571b99a5ee9f80f8c3f44296d`.
- Active baseline is contract-v193 with 470 exact fixtures and zero waivers.
- Focused core coverage locks stack splitting and monster-carried ownership,
  death-compatible carrying, gold calculation and prevention, food/light
  consumption, theft blinking, and pickup exclusions.
