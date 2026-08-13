# Contract v294: Sniper projectile modes

Contract v294 adds the shared projectile transaction needed by the first eight Sniper
techniques. This batch introduces no formal item, ability, class, build, material, or
affix IDs; formal Sniper content remains a later import.

`AbilityEffectSpecDto` now projects a `sniper-shot` effect with eight modes:
Shining, Retreat, Disarm, Burning, Shatter, Freezing, Knockback, and Piercing. They
reuse the ordinary projectile transaction for launcher and ammunition selection,
energy, heavy-launcher handling, weapon and riding proficiency, slays, brands,
critical hits, damage, death, ammunition breakage, and Easy Tiring II fatigue.

- Shining illuminates the full trajectory and applies the RFB light-vulnerability
  multiplier.
- Retreat teleports the player `10 + 2 * concentration` squares after the shot.
- Disarm removes traps crossed by the projectile.
- Burning and Freezing use RFB immunity, vulnerability, brand, and concentration
  multipliers.
- Shatter can remove the first destructible empty wall in the trajectory. Its
  `Projectile` terrain-change source never grants mining proficiency, materials,
  vein gold, or mining loot.
- Knockback pushes a surviving target `3 + 1d5` squares, stopping at terrain or an
  entity.
- Piercing spends one concentration level only to continue past a collision, so an
  initial value of N permits at most N+1 resolved collisions. Each collision trains
  projectile proficiency independently.

A real normal or special shot clears concentration after using its starting value.
Preflight rejection and target cancellation do not clear it or consume projectile
RNG. Ordinary shooting retains its existing external behavior.

Coordination point: Protocol 1.197, State Hash Schema v98, save container v1,
pack 1.312.0, active baseline `contract-v294`.
