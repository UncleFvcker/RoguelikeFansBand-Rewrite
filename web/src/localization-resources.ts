// SPDX-License-Identifier: MPL-2.0

import enContent from "../../locales/en-US/content.ftl?raw";
import enGame from "../../locales/en-US/game.ftl?raw";
import enUi from "../../locales/en-US/ui.ftl?raw";
import zhContent from "../../locales/zh-CN/content.ftl?raw";
import zhGame from "../../locales/zh-CN/game.ftl?raw";
import zhUi from "../../locales/zh-CN/ui.ftl?raw";

import type { LocaleSources } from "./localization";

export const LOCALIZATION_SOURCES: LocaleSources = {
  "en-US": [enUi, enGame, enContent],
  "zh-CN": [zhUi, zhGame, zhContent],
};
