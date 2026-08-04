# Contract v170: Strength Encumbrance

Status: active baseline. Protocol `1.136`, demo pack `1.164.0`, save v1,
state hash Schema v61. The built-in content hash is
`59d9801214e8f62544b9ffa96a0d56cdfd790d248ec04c90a94246a8089eaf8f`.

## Rules

- Carrying capacity is derived from effective Strength using RFB's original
  38-entry table. Capacity ranges from 50.0 lb at Strength 3 to 195.0 lb at
  Strength 18/220 and remains capped there after victory.
- Carried weight includes inventory and equipped items. The shared pack still
  has 26 slots, with equipped containers adding 4, 8, or 12 slots.
- Weight never rejects pickup, shop purchase, or Home withdrawal. Those actions
  remain atomic and are limited by inventory slots and their existing resource
  checks.
- Encumbrance reduces derived player Speed by one at 120% capacity and by one
  more for each additional 20%. Weight from 100% through 119.9% has no Speed
  penalty, matching the original integer formula.
- `PlayerDto` projects authoritative current weight, dynamic capacity, and the
  current encumbrance Speed penalty. The inventory summary marks overweight
  state and displays the active penalty.

## Verification

Pure stat tests cover the original Strength table and penalty thresholds.
Focused core tests cover overburdened pickup and purchasing, inventory slot
rejection, Home transfers, and town transactions. The obsolete mixed movement
fixture for weight rejection was removed because that behavior no longer
exists.
