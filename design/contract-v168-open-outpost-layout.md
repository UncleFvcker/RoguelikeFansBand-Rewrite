# Contract v168: Open Outpost Layout

## Scope

The regular RFB Outpost is embedded in a much larger wilderness map. This revision uses that relationship as a scale reference without copying the original map, text, or terrain arrangement.

- The persistent Outpost surface is `96x32`, with wilderness space on every side of the town.
- The walled town sits inside that surface and keeps the Warrens entrance outside its east gate.
- The temple has continuous walkable passages on both sides and along its south edge.
- Procedural floors own their dimensions independently from the surface. The nine Warrens floors remain `66x22`, matching the established RFB-derived dungeon boundary.

## Development Policy

The project is still in active development and testing starts from a new save. Version `1.157.0` development saves are not migrated or accepted under the `1.158.0` content hash.

Routine acceptance uses source-pack verification and focused content/core tests. Large desktop E2E suites are reserved for related failures, explicit requests, or milestone acceptance.

Contract fixtures for location-dependent town actions use a validated direct player-position precondition. Only fixtures whose subject is movement may contain movement commands; shop, Home, and other facility fixtures each cover one operation without cross-facility routes. Generic purchase fixtures resolve the first currently projected stock entry instead of coupling their source to a generated item instance ID.

The active baseline contains 463 exact fixtures with zero waivers. Four facility-specific purchase fixtures were removed because they repeated the General Store's ordinary first-stock transaction; the Black Market fixture remains because its special price rule is distinct.
