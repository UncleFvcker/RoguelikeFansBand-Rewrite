# Contract v217: New Life and Attribute Potentials

Status: implemented P3.7-M3 baseline; protocol 1.153, content 1.214.0, save v1,
State Hash Schema v74, 471 exact fixtures, zero waivers.

## Contract

- Every character owns six saved personal attribute potentials generated from
  six `1d7` rolls totaling 24 and encoded as `78 + die * 10`.
- Birth potential and HP generation use dedicated deterministic seed streams;
  neither consumes the authoritative gameplay RNG used by maps and shops.
- Permanent natural growth stops at the smaller of personal potential and the
  current global cap. New Life clamps both current and historical maximum
  values when a rerolled potential is lower.
- `hp_progression` remains the sole HP-growth authority. Candidate sequences
  must reach 87 percent at levels 5, 10, and 25, and finish within 87–117
  percent on the rewrite's existing 1–10 growth scale.
- New Life uses the gameplay RNG and commits one transaction: reroll HP, restore
  life force to 1000, reroll and clamp potentials, remove all unlocked
  mutations in source order, then refresh effective HP/resources once. Locked
  mutations survive.
- `PlayerProgressSaveDto.attributePotentials` is required. Player projection
  exposes each attribute's potential. Invalid encodings, totals, caps, and save
  references are rejected.

## Acceptance

- Focused core tests cover deterministic birth rolls, HP acceptance bands,
  permanent-growth caps, save/projection/hash participation, invalid saves,
  transaction RNG, clamp behavior, mutation order and lock protection.
- The formal `新生药水` uses the authoritative identity and flavor, has an
  explicit Black Market acquisition path, and is active in the item ledger.
- Content validation, source-backed item audit, generated protocol/content
  schemas, save round trip, and all 471 exact fixtures pass with zero waivers.
