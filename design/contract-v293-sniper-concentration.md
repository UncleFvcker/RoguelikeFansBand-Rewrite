# Contract v293: Sniper concentration foundation

Commit 1 adds the shared RFB Sniper shooting rules without importing the formal Sniper class or
claiming any production content ID.

## Content model

`ClassDefinition.snipingProfile` declares the preferred ammunition, bolt hit/critical rules,
half-speed excess and the level-derived concentration cap. `ClassAbilityDefinition` gains the
optional `minimumConcentration` and `hitPointCost` fields. HP cost is independent of the existing
resource identity and cost. The new `concentrate` ability effect is valid only for a class with a
sniping profile.

The first formal Sniper slice should use the original values: Bolt, `10 + level / 5` to hit,
50% of shooting speed above 100, 150% bolt critical chance, maximum concentration
`2 + (level + 5) / 10`, and 10% per concentration for ammunition damage, critical chance and
target-armor reduction.

## Runtime and protocol

`Game.sniperConcentration` and `probedActorKindIds` are authoritative. The latter is intentionally
empty until the Sniper probing technique is imported in Commit 3, but is included in this single
save/hash migration. Save loading rejects unknown/duplicate probed kinds, non-Sniper state, and
concentration above the current level cap.

Concentrate raises the value by one and still spends a normal action at the cap. A valid ordinary
shot consumes ammunition and uses the prior concentration for hit AC, ammunition damage and
critical chance before clearing it. Other world-advancing actions clear concentration. Preflight
rejections such as an unknown ability, invalid target, missing resource/HP or unavailable
ammunition neither clear concentration nor add RNG draws.

`PlayerDto.sniperConcentration` is visible only for a class with a sniping profile. `AbilityDto`
projects the concentration gate and HP cost. The Web status panel displays concentration as a
meter and annotates affected techniques.

## Coordination

- Protocol `1.196`
- State Hash Schema `v98`
- active baseline `contract-v293`
- save container remains `v1`; old development saves are intentionally unsupported
- content pack remains `1.312.0`; no formal item, ability, material or affix ID is added

The protocol/hash migration refreshes and re-verifies every active exact fixture. Focused tests
cover level caps, bolt derivation, damage/AC/critical formulas, action clearing, invalid-command
RNG stability, HP costs and save validation.
