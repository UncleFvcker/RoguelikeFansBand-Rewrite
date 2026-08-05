# Contract v174: Monster Door Interaction

Status: active slice in the cumulative contract-v176 baseline. Protocol
`1.137`, demo pack `1.170.0`, save v1, state hash Schema v63. Old development
saves are not supported.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Actor content expresses `OPEN_DOOR` and `BASH_DOOR` as independent
  capabilities. Small Kobold, Kobold, Large Kobold, and Mughash have both;
  Wild Cat and Warg only bash.
- Door content carries the original feature power independently from player
  open/bash skill difficulties. The compressed locked door represents original
  `LOCKED_DOOR_1`; successful monster unlocking changes it to the ordinary
  closed door.
- An ordinary door opens without RNG, consumes the action, and is entered only
  on a later action. Locked doors use `randint0(currentHp / 10) > power`.
  Original `randint0(0)` and `randint0(1)` return zero without consuming RNG.
- If opening or unlocking does not succeed, a bash-capable monster attempts
  the same HP/power contest in that action. A successful bash consumes one
  50-percent draw, produces a broken or open door, and moves the monster into
  the doorway immediately. Failure still consumes the action.
- Pathfinding treats a door as reachable only when that actor can open or bash
  it. Terrain mutation, changed cells, action energy, and observable door
  events use the normal monster action transaction.

## Verification

The `monster_doors` unit module independently covers ordinary opening without
movement or RNG, locked-to-closed unlocking, and successful bash-and-enter.
No contract fixture combines door behavior with another facility or workflow.

