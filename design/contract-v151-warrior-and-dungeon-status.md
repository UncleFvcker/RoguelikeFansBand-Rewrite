# Contract v151: RFB Warrior entry and factual dungeon status

Status: Phase 17 post-Gate-3 product amendment.

Contract v151 removes the staged journey-objective presentation and makes Warrior the only career offered by New Game. Protocol remains `1.123`, the save container remains v1, and state hash Schema remains `55`. The demo pack advances to `1.142.0`; the active baseline remains 455 exact fixtures with zero waivers.

## Fixed source

The behavior and content reference remains RFB v1.3.0.7 at commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

- `src/warrior.c` supplies Warrior's six attribute modifiers, base skills and per-ten-level growth, 115% life, base HP 18, 100% experience, no-spell identity, advanced bonuses, Sword Dancing power, and birth-item roles.
- `lib/edit/b_info.txt`, body record 0, supplies the Standard humanoid equipment layout: two weapon/shield hands, shooting, quiver, two rings, neck, light, body, cloak, head, hands, and feet.
- `lib/edit/k_info.txt` supplies the selected birth-item values: Broad Sword weight 150 and `2d6`, Chain Mail weight 220 and AC 14, Short Bow weight 30 with `x2.50`, and Arrow weight 2 with `3d4`.

Old prose is not copied. English and Chinese descriptions are newly written summaries of the same gameplay roles. This bounded selection follows explicit product direction and does not authorize bulk promotion of legacy tables, proper names, help text, algorithms, or assets.

## Warrior slice

`demo.build.warrior` is Human + Warrior + Ordinary and is the only build rendered by New Game. Existing demo builds remain compiled for old saves and system tests but are not character-creation choices.

The existing Warrior class already carried the source-aligned early numeric profile:

- attributes `STR +4 / INT -2 / WIS -2 / DEX +2 / CON +2 / CHR +1`;
- life 115%, base HP 18, experience 100%;
- source-aligned disarming, device, saving throw, stealth, searching, perception, melee, and shooting bases/growth.

Its birth kit now uses Broad Sword, Chain Mail, Short Bow, and 22 Arrows. The fixed arrow count is the integer midpoint approximation of RFB's `15..30` birth roll because the current class content schema accepts fixed quantities only. Broad Sword, Chain Mail, and Arrow weight/damage/AC fields use the selected source values.

Known semantic gaps are explicit:

- the current launcher model has no `x2.50` multiplier, so Short Bow uses range 8 and carries the Arrow `3d4` shot dice;
- Chain Mail's source `-2` to-hit field is not mapped to generic `modifiers.attack`, because that field changes the player's base attack layer and incorrectly collapses Warrior melee skill;
- Arrow break chance and the Warrior player carry limit are current-engine compatibility values rather than fields from the selected source records;
- Sword Dancing at level 30, level-based weapon damage/blows, regeneration, fear resistance, strong pseudo-identification, and spellbook-destruction experience are not implemented in this early Warrens slice.

## Standard body

Warrior uses `demo.race.rfb-human`, whose explicit 13-slot body follows RFB Standard ordering. The current single-type slot model represents the two `WEAPON_SHIELD` hands as one weapon hand and one shield hand; the quiver slot is present but ammunition remains an inventory stack until quiver behavior exists. No Warrior-specific body is invented. A separate player actor only raises carry capacity to 600 tenths of a pound so the source-weight birth kit does not begin overburdened; equipment slots still come exclusively from the race.

## Dungeon status presentation

The player-facing panel no longer selects or displays `prepare`, `enter`, `descend`, `guardian`, `return`, `retire`, or `complete` objectives. For a Warrens session it shows only:

- dungeon name: Warrens;
- current depth and maximum depth, including `0 / 9` on the surface;
- `Kobold Lord · undefeated` while the campaign remains active.

The boss row disappears after authoritative victory or retirement. Contextual control onboarding remains separate. Legacy worlds display no active dungeon rather than being mislabeled as Warrens.

## Evidence

- content validation fixes the Warrior identity, birth kit values, and RFB Standard body slots;
- core coverage fixes effective Warrior attributes, HP, skills, carry state, equipped kit, body layout, and zero-RNG fixed birth quantity;
- all 16 Warrens connectivity seeds and the guardian/victory/return/retire proof now run with `demo.build.warrior`;
- native initialization, save, and replay coverage use Warrior in the production Warrens world;
- frontend coverage fixes the single-career selector and dungeon/depth/boss visibility rules;
- fixture 455 fixes Warrior victory-surface retirement and post-retirement rejection.
