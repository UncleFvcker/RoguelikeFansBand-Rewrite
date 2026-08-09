# Wilderness W0: authoritative world-map data

## Scope and authority

W0 reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The importer reads only
`lib/edit/w_info.txt` and the `N`/`P` location fields in
`lib/edit/d_info.txt`; it never reads the legacy checkout or working tree.

The production world now carries the normal RFB wilderness as optional
`WorldDefinition.wilderness` data:

- a `99 x 66` fixed map stored as a legend plus fixed-width string rows;
- all 15 legacy wilderness terrain kinds, with the source danger level and
  road bit preserved by each used legend symbol;
- the source start at `(28, 52)`;
- active locations only for `demo.town.outpost` and
  `demo.dungeon.warrens`, both at `(28, 52)`.

The other source town symbols retain their underlying terrain, danger, and
road data, but they do not become town definitions or enterable locations.
Unimplemented source dungeons are likewise absent from `locations`. W0 does
not create placeholder content.

## Synchronization and validation

`legacy-wilderness-selection.json` binds source town 1 (`前哨站`) and source
dungeon 30 (`Warrens`) to the two content definitions that already exist.
The focused synchronization command is:

```powershell
$env:RFB_LEGACY_SOURCE='D:\codex\Frogcomposband\master'
cargo run -p rfb-legacy-import -- sync-demo-wilderness packs/rfb-demo-original/legacy-wilderness-selection.json packs/rfb-demo-original/worlds/warrens-journey.json
```

Synchronization rejects source name/index drift, duplicate or unsupported
targets, malformed legend fields, unknown row symbols, inconsistent row
widths, non-edge boundaries, duplicate towns, invalid start coordinates, and
missing dungeon positions. Content compilation independently validates the
dimensions, legend symbols, every row, the edge boundary, the start cell,
unique locations, bounds, and town/dungeon references.

## Deferred runtime boundary

W0 is static content only. It does not project the wilderness through the
protocol, create travel state, replace the existing Outpost tactical floor,
render a world map, allocate wilderness encounters, or enter a location.
Those behaviors begin with W1 and later vertical slices.

The demo pack advances to `1.192.0`, with content hash
`02577f7c9262ee49d7f73ec13e3271a674cedc4e1af297e9359032cfb5532962`.
Protocol `1.144`, save v1, state-hash Schema v67, and the contract-v196
470-fixture baseline are unchanged.
