# Contract v295: Sniper advanced shots and monster probe

Contract v295 completes the shared runtime surface required by the remaining RFB Sniper
techniques. This batch introduces no formal item, ability, class, build, material, or affix
IDs; the formal Sniper content slice remains a later single-class import.

`sniper-shot` adds Evil, Holy, Exploding, Double, Thunder, Needle, and Final modes. Every
mode continues through the ordinary projectile transaction for launcher and ammunition
selection, energy, heavy-launcher handling, weapon and riding proficiency, critical hits,
damage, death, recovery, breakage, and Easy Tiring II.

- Evil and Holy use the original alignment, light vulnerability, slay/kill, illumination,
  darkness, and 40-percent impact breakage rules.
- Thunder applies its electricity immunity and ammunition-brand modifier.
- Exploding applies physical area damage with radius `(concentration + 1) / 2 + 1` and
  destroys ammunition on impact.
- Double consumes two distinct ammunition instances in one action after changing the
  effective concentration to `(concentration + 1) / 2`. A one-item stack becomes an
  ordinary single shot; killing the target aborts the second shot.
- Needle retains the nested random test. Unique and Unique2 targets consume the same random
  draws but cannot receive the vital hit; a failed test deals exactly one point.
- Final uses a 5.0x special multiplier, destroys ammunition on impact, then applies
  `7 + randint0(7)` slow and `1d25` stun to the player.

Special shot multipliers and ammunition brand/slay multipliers use the stronger value rather
than multiplying together. Concentration's general ammunition-damage bonus applies afterward.

`probe-monsters` is a self-target effect. It collects each currently visible, non-fuzzy actor
with a projectable line from the player, without grouping duplicate kinds. The typed result
includes entity and kind IDs, glyph, position, current and maximum HP, speed, armor class,
alignment, faction, resistances, status immunities, melee routine, and castable ability IDs.
Each stable kind is recorded in the already-persisted `probedActorKindIds` lore set. The Web
client opens a localized browsable panel from this typed event outcome.

Coordination point: Protocol 1.198, State Hash Schema v98, save container v1, pack 1.312.0,
active baseline `contract-v295`. Existing exact fixtures do not enter an unbound Sniper
ability path, so they are verified without refreshing their assertions.
