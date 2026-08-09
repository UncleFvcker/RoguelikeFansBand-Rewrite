# Contract v209: Warrens P24 Slime Mold

## Scope and authority

P24 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds Slime Mold / 黏菌
(source index 962) after implementing the two runtime behaviors that previously
blocked strict import: `MOVE_BODY` and `REGENERATE`.

The implementation adds two default-false actor facts, `movesWeakerBodies` and
`regenerates`. It does not add a generic trigger system, per-actor regeneration
parameters, a compatibility path, or a second movement framework.

## Runtime contracts

Every living, wounded monster on the active local floor receives a regeneration
pass each 100 `worldTick` pulses. The recovered amount is `maxHp / 100`. When
that division is zero, exactly one `bounded(2)` draw decides whether the base
amount becomes one. `regenerates` doubles the resulting amount, and the final
amount is capped at 400 before current HP is clamped to maximum HP. Full-health
and dead monsters consume no regeneration RNG.

`movesWeakerBodies` makes an occupied step traversable only when all of these
conditions hold:

- source and blocker are on the same side;
- source experience value is strictly greater than blocker experience value;
- the source can enter the blocker's terrain;
- the blocker can enter the source's terrain;
- the blocker is alive and is not the player's current mount.

The actors then exchange positions and the displaced actor wakes. Existing
`KILL_BODY` interaction keeps precedence, and a hostile blocker remains a
combat target rather than a movement shortcut.

## Content and acceptance

Slime Mold retains experience value 10, `2d8` hit points, speed 105, two `2d5`
poison touches, swimming, multiplication, swamp habitat, and the existing
Shriek ability. Its Mind Blast produces one stable `mind-blast-7d7` ability
record using the existing ordered damage/status program.

- Strict monster selection grows from 213 to 214 records; the demo pack grows
  from 278 to 279 actors and from 129 to 130 abilities.
- Demo pack is 1.205.0 with content hash
  `3d94f3bff136355b23ad4a864f8308197606e79d2c92a3f36ad07f6b69a2c886`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v209 with 470 exact fixtures and zero waivers.
- The public low-HP regeneration draw intentionally refreshes only
  `dungeon.maze-streamer-layout-seed-151` and
  `task.shared-id-completion-closes-all-entrances`; the other 468 fixtures
  retain their exact results.
- Focused tests cover strict import, two-way terrain rejection, successful
  swap and wake, the 100-tick interval, shared base recovery, doubling, the
  400-point cap, and the one-draw low-HP branch.
