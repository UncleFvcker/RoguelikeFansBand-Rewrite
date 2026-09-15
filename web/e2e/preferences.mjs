// SPDX-License-Identifier: MPL-2.0
// Drive the shared production editor; localStorage is no longer a settings API.
export async function setPreferences(driver, values) {
  await driver.waitFor('return !document.querySelector("#preferences-save").disabled', "global preferences loaded", 30000);
  await driver.execute(`if (!document.querySelector("#player-ui-settings-dialog").open) {
    document.querySelector(document.documentElement.dataset.appMode === "playing" ? "#player-ui-settings-open" : "#session-settings").click();
  } return true;`);
  await driver.waitFor('return document.querySelector("#player-ui-settings-dialog").open && !document.querySelector("#preferences-save").disabled', "settings editor ready");
  await driver.execute(`const ids = { locale: "language-select", inputPreset: "input-preset", tilesetPreset: "tileset-preset", cameraMode: "camera-mode", zoom: "zoom-level" };
    for (const [key, value] of Object.entries(arguments[0])) {
      if (key === "travel") {
        const travel = { alwaysPickup: "travel-always-pickup", autoDetectTraps: "travel-auto-detect", autoMapArea: "travel-auto-map", disturbTrapDetect: "travel-disturb-detect" };
        for (const [field, checked] of Object.entries(value)) {
          const input = document.getElementById(travel[field]); input.checked = checked;
          input.dispatchEvent(new Event("change", { bubbles: true }));
        }
        continue;
      }
      const input = document.getElementById(ids[key]); input.value = String(value);
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }
    document.querySelector("#preferences-save").click(); return true;`, [values]);
  await driver.waitFor(`const dialog = document.querySelector("#player-ui-settings-dialog");
    if (dialog.getAttribute("aria-busy") === "true") return false;
    if (!document.querySelector("#preferences-status").dataset.savedRevision) throw new Error(document.querySelector("#preferences-status").textContent);
    return true;`, "preferences saved and applied", 30000);
  await driver.execute('document.querySelector("#player-ui-settings-close").click(); document.activeElement?.blur(); return true;');
}
