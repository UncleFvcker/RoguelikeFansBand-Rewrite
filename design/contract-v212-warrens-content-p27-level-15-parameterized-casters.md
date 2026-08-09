# Contract v212: Warrens P27 Level 15 Parameterized Casters

## Scope and authority

P27 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds eight level-15 records:

- Dweller on the threshold and Dark naga;
- Wererat and Mi-Go;
- Griffon and Floating orb;
- Undead devilfish and Radiant Kavu.

Their English identities follow the authoritative `N:` records. Chinese names
and descriptions exactly follow the authoritative localization table and `D:`
records.

## Parameterized ability content

The batch generates eleven stable ability records and their matching Ability
Programs:

- `bolt-acid-7d8-5`, `drain-mana-8`, and
  `summon-legacy-import-l15-1d1`;
- `bolt-cold-6d8-5` and `heal-45`;
- `kin-wererat` and `summon-demon-l15-1d3-1`;
- `bolt-physical-4d5` and `bolt-physical-2d6-5`;
- `breath-nether-14-550-r2` and `breath-nexus-33-250-r2`.

Dark naga and Wererat share the cold bolt, Dweller on the threshold and Mi-Go
share the level-15 single-monster summon, and Dark naga and Radiant Kavu share
`heal-45`. Existing fear, paralyze, confuse, darkness, blink, curse, poison
ball, disenchantment breath, time breath, and poison breath records are reused.
Possessor-only `BLESS` and `HEROISM` hints do not become monster abilities.

This batch adds no effect, protocol field, state-hash input, save field,
compatibility path, or generic framework.

## Content and acceptance

- Strict monster selection grows from 256 to 264 records; the demo pack grows
  from 321 to 329 actors and from 130 to 141 abilities.
- Demo pack is 1.208.0 with content hash
  `068d5296c10176d40507e531b3a9cb3605e5c5d1288304ad36ed534527dd1bcd`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v212 with 470 exact fixtures and zero waivers.
- Full verification leaves all 470 fixture results unchanged; no fixture is
  refreshed.
- Focused tests lock all eight casting rosters, shared parameterized identities,
  immobility, riding, aquatic movement, and intrinsic light.
