> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# Contract v216: Mutation Definitions and Authoritative State

Status: implemented P3.7-M1 baseline; protocol 1.152, content 1.212.0, save v1,
State Hash Schema v73, 470 exact fixtures, zero waivers.

## Contract

- The content catalog contains exactly 152 `rfb.mutation.*` definitions pinned
  to the P3.7-M0 ledger. Each definition carries the authoritative Chinese name
  and description plus the original rating.
- `CharacterProgress` owns active and locked mutation IDs as ordered sets. New
  characters start with both sets empty, and locked must remain a subset of
  active.
- Saves persist both sets without a development-save fallback. Load rejects
  duplicate IDs, unknown content references, and locked IDs that are not
  active.
- Player projection lists active mutations in stable ID order with ID, name,
  description, rating, and lock state.
- Both mutation sets participate in the authoritative state hash. No mutation
  acquisition, removal, random selection, activation, periodic effect, passive
  bonus, or UI behavior is part of M1.

## Acceptance

- Source-pack validation and the frozen-ledger parity test cover all 152
  definitions.
- Focused core coverage proves projection, save round trip, invalid-save
  rejection, and state-hash sensitivity.
- Protocol bindings, JSON Schemas, content lock, and all exact contract
  fixtures are generated from the new baseline.
