# Contract v169: Shared inventory, containers, and tools

Status: active baseline. Protocol `1.135`, demo pack `1.163.0`, save v1,
state-hash Schema `61`, 463 exact fixtures, and zero waivers. The demo content
hash is `d9e227cc7757ff82a66c7afadf8da2846a1751920f53fa3f1f0a74c640b8a0ac`.

## Inventory contract

- Every player starts with 26 shared inventory slots. One compatible stack is
  one slot; equipped items do not consume inventory slots.
- Fabric Bag, Leather Pouch, and Dwarven Backpack add 4, 8, and 12 slots while
  equipped in `container`. They do not own nested item lists.
- Pickup, shop purchase, Home withdrawal, and container removal preflight both
  weight and slots. Rejection is atomic and does not advance RNG.
- Arrow stacks have a maximum quantity of 99.

## Equipment contract

- The standard and RFB Human body templates expose `container` and `tool`.
- Original armor hit penalties and glove hit/damage modifiers enter the melee
  equipment pipeline without changing ranged skill.
- Shovel and Pick retain their original melee profiles and digging bonus.
  Equipping either in `tool` applies only its final digging bonus; modifiers,
  resistances, immunities, slays, brands, passives, affixes, and melee profile
  do not contribute from that slot.
- Equipping the same item in `weapon` applies its complete weapon and equipment
  properties. The saved equipment slot remains valid across round trips.
- `Equip` accepts optional `slotId`. Missing `slotId` preserves deterministic
  automatic selection. Explicit targets must name a real body-slot instance;
  tools accept only `tool` or `weapon`, and other equipment accepts only its
  declared slot type.

## Verification boundary

Focused core tests cover full inventory rejection, compatible stack merging,
container capacity and removal, atomic shop rejection, tool/weapon selection,
invalid target rejection, and weapon-slot save round trips. Content tests pin
the selected original values and importer mappings. The 463 active fixtures
retain their original seeds, preconditions, and commands; only assertions were
refreshed for the formal content and shared state-hash changes.
