# Contract v217: Warrens P32 King Mulu and Spider Summoning

## Scope and authority

P32 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds source record 1077,
King Mulu, the Chief of Southerings (`南蛮大王木鹿大王`), at level 16. The actor
retains its Unique identity, four melee blows, intrinsic radius-two light,
human/evil identity, fixed maximum HP, item drops, door interaction,
`DUNGEON_31` allocation, and source casting frequency.

The English identity follows the authoritative `N:` record and the Chinese
name exactly follows the authoritative runtime localization table. Pack
descriptions are independently written factual summaries.

## Narrow category mapping

The existing summon-token table gains one entry: `S_SPIDER → spider`. King
Mulu therefore binds two generated abilities and their flat Ability Programs:

- `summon-ant-l16-1d3-1`;
- `summon-spider-l16-1d3-1`.

Both reuse the current `summon-category` effect with maximum level 16, count
`1d3+1`, radius two, and the existing imported summon lifetime. Spider
candidates already carry the explicit `spider` tag in the strict selection.
P32 adds no effect, protocol field, state-hash input, save field, compatibility
path, or generic summon framework.

## Content and acceptance

- Strict monster selection grows from 279 to 280 records; the demo pack grows
  from 344 to 345 actors and from 143 to 145 abilities.
- Demo pack is 1.213.0 with content hash
  `a5ff0d74568c28b7ac966b6a0f6dcbef64a6cb3cc9fc004b3ba6abbc8829d1eb`.
- Protocol remains 1.151, save remains v1, and state hash remains Schema v72.
- Active baseline is contract-v217 with 470 exact fixtures and zero waivers.
- Full verification leaves all 470 fixture results unchanged; no fixture is
  refreshed.
- Focused tests cover the importer mapping, generated categories and counts,
  King Mulu's casting set, Unique identity, and dungeon-index restriction.
