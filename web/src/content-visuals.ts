// SPDX-License-Identifier: MPL-2.0

import emberMote from "../../packs/rfb-demo-original/actors/ember-mote.json";
import explorer from "../../packs/rfb-demo-original/actors/explorer.json";
import luminousShard from "../../packs/rfb-demo-original/items/luminous-shard.json";
import floor from "../../packs/rfb-demo-original/terrain/floor.json";
import wall from "../../packs/rfb-demo-original/terrain/wall.json";

const definitions = [floor, wall, explorer, emberMote, luminousShard];

export const CONTENT_GLYPHS: Readonly<Record<string, string>> = Object.freeze(
  Object.fromEntries(definitions.map((definition) => [definition.id, definition.glyph])),
);
