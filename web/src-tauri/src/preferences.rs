// SPDX-License-Identifier: MPL-2.0

use crate::native_storage::{DesktopCommandError, DesktopResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub visuals: VisualPreferences,
    pub display: DisplayPreferences,
    pub format_version: u32,
    pub locale: String,
    pub input_preset: String,
    pub tileset_preset: String,
    pub camera_mode: String,
    pub zoom: f64,
    pub key_bindings: Vec<KeyBinding>,
    pub travel: rfb_protocol::TravelOptionsDto,
    pub operations: rfb_protocol::OperationOptionsDto,
    pub mogaminator: rfb_protocol::MogaminatorPreferencesDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyBinding {
    pub preset: String,
    pub trigger: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferenceSnapshot {
    pub revision: u32,
    pub preferences: Preferences,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplayPreferences {
    pub highlight_player: bool,
    pub target_path: bool,
    pub unsafe_grids: bool,
    pub alert_trap_detect: bool,
    pub effective_speed: bool,
    pub decimal_stats: bool,
    pub experience_needed: bool,
    pub show_origins: bool,
    pub describe_slots: bool,
    pub show_weights: bool,
    pub show_discounts: bool,
    pub show_item_icons: bool,
    pub monster_distance: bool,
    pub list_stairs: bool,
    pub alert_poison: bool,
    pub hp_warning_percent: u8,
    pub mana_warning_percent: u8,
}
impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            highlight_player: false,
            target_path: false,
            unsafe_grids: false,
            alert_trap_detect: false,
            effective_speed: false,
            decimal_stats: true,
            experience_needed: false,
            show_origins: true,
            describe_slots: true,
            show_weights: true,
            show_discounts: true,
            show_item_icons: true,
            monster_distance: false,
            list_stairs: false,
            alert_poison: true,
            hp_warning_percent: 50,
            mana_warning_percent: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualPreferences {
    pub overrides: BTreeMap<String, VisualOverride>,
    pub palette: [String; 16],
    pub theme: MapTheme,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapTheme {
    pub background: String,
    pub grid: String,
    pub memory_color: String,
    pub hidden_color: String,
    pub player: String,
    pub path: String,
    pub uncovered: String,
    pub pet: String,
    pub memory_opacity: f64,
    pub light_tint_opacity: f64,
    pub darkness_opacity: f64,
}
impl Default for VisualPreferences {
    fn default() -> Self {
        Self {
            overrides: BTreeMap::new(),
            palette: [
                "#000000", "#ffffff", "#808080", "#ff8000", "#c00000", "#008040", "#0000ff",
                "#804000", "#404040", "#c0c0c0", "#ff00ff", "#ffff00", "#ff0000", "#00ff00",
                "#00ffff", "#c08040",
            ]
            .map(String::from),
            theme: MapTheme {
                background: "#090d12".into(),
                grid: "#18212d".into(),
                memory_color: "#12213a".into(),
                hidden_color: "#000000".into(),
                player: "#fff2a8".into(),
                path: "#88dcff".into(),
                uncovered: "#c99c64".into(),
                pet: "#245c46".into(),
                memory_opacity: 0.58,
                light_tint_opacity: 0.18,
                darkness_opacity: 0.62,
            },
        }
    }
}
fn visual_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}
fn visual_glyph(glyph: &str) -> bool {
    let mut chars = glyph.chars();
    chars.next().is_some_and(|c| !c.is_control() && !c.is_whitespace() && !matches!(c as u32,
        0x00ad | 0x061c | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x206f | 0xfeff | 0xfe00..=0xfe0f | 0xe0000..=0xe0fff))
        && chars.next().is_none()
}
impl VisualPreferences {
    fn valid(&self) -> bool {
        self.palette.iter().all(|color| visual_color(color))
            && self.overrides.len() <= 20000
            && self.overrides.iter().all(|(id, item)| {
                id.len() <= 192
                    && id.split('.').count() >= 3
                    && id.split('.').all(|part| {
                        !part.is_empty()
                            && part.bytes().all(|c| {
                                c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c)
                            })
                    })
                    && (item.glyph.is_some()
                        || item.foreground.is_some()
                        || item.background.is_some())
                    && item.glyph.as_deref().is_none_or(visual_glyph)
                    && item.foreground.as_deref().is_none_or(visual_color)
                    && item.background.as_deref().is_none_or(visual_color)
            })
            && [
                &self.theme.background,
                &self.theme.grid,
                &self.theme.memory_color,
                &self.theme.hidden_color,
                &self.theme.player,
                &self.theme.path,
                &self.theme.uncovered,
                &self.theme.pet,
            ]
            .into_iter()
            .all(|c| visual_color(c))
            && [
                self.theme.memory_opacity,
                self.theme.light_tint_opacity,
                self.theme.darkness_opacity,
            ]
            .into_iter()
            .all(|n| n.is_finite() && (0.0..=1.0).contains(&n))
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            display: DisplayPreferences::default(),
            visuals: VisualPreferences::default(),
            format_version: 5,
            locale: "zh-CN".into(),
            input_preset: "original".into(),
            tileset_preset: "ascii".into(),
            camera_mode: "player-centered".into(),
            zoom: 1.0,
            key_bindings: Vec::new(),
            travel: rfb_protocol::TravelOptionsDto::default(),
            operations: rfb_protocol::OperationOptionsDto::default(),
            mogaminator: rfb_core::Game::default_behavior_preferences().mogaminator,
        }
    }
}

fn error(code: &str, detail: impl ToString) -> DesktopCommandError {
    DesktopCommandError::new(code, detail.to_string())
}

fn preset(value: &str) -> bool {
    matches!(value, "original" | "roguelike")
}

impl Preferences {
    pub(super) fn behavior(&self) -> rfb_protocol::BehaviorPreferencesDto {
        rfb_protocol::BehaviorPreferencesDto {
            locale: if self.locale == "zh-CN" {
                rfb_protocol::LocaleDto::ZhCn
            } else {
                rfb_protocol::LocaleDto::EnUs
            },
            travel: self.travel,
            operations: self.operations,
            mogaminator: self.mogaminator.clone(),
        }
    }
    fn validate(&self) -> DesktopResult<()> {
        if self.format_version != 5
            || !self.visuals.valid()
            || [
                self.display.hp_warning_percent,
                self.display.mana_warning_percent,
            ]
            .iter()
            .any(|p| *p > 90 || *p % 10 != 0)
            || !matches!(self.locale.as_str(), "zh-CN" | "en-US")
            || !preset(&self.input_preset)
            || !matches!(self.tileset_preset.as_str(), "ascii" | "image")
            || !matches!(self.camera_mode.as_str(), "player-centered" | "full-map")
            || ![0.75, 1.0, 1.25, 1.5, 2.0].contains(&self.zoom)
            || self.key_bindings.len() > 256
        {
            return Err(error(
                "preferences-invalid",
                "Invalid preference values or format version",
            ));
        }
        let mut seen = BTreeSet::new();
        for binding in &self.key_bindings {
            let register = binding.action.strip_prefix("Register:").is_some_and(|key| {
                key.len() == 1 && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
            });
            // Canonical actions from command-shortcuts.ts; these are input actions, not GameCommands.
            let action = matches!(
                binding.action.as_str(),
                "!" | "$"
                    | "%"
                    | "&"
                    | "@"
                    | ":"
                    | ")"
                    | "\""
                    | "'"
                    | "Enter"
                    | "?"
                    | "~"
                    | "Q"
                    | "M"
                    | "L"
                    | "["
                    | "Y"
                    | "/"
                    | "n"
                    | "W"
                    | "p"
                    | "j"
                    | "+"
                    | "w"
                    | "t"
                    | "d"
                    | "k"
                    | "e"
                    | "i"
                    | "I"
                    | "G"
                    | "b"
                    | "U"
                    | "m"
                    | "A"
                    | "E"
                    | "q"
                    | "r"
                    | "a"
                    | "u"
                    | "z"
                    | "F"
                    | "v"
                    | "{"
                    | "}"
                    | "C"
                    | "="
                    | "l"
                    | "s"
                    | "S"
                    | "o"
                    | "D"
                    | "c"
                    | "B"
                    | "T"
                    | "f"
                    | "R"
                    | "g"
                    | "J"
                    | "H"
                    | "1"
                    | "2"
                    | "3"
                    | "4"
                    | "5"
                    | "6"
                    | "7"
                    | "8"
                    | "9"
                    | ";"
                    | "-"
                    | "."
                    | "Z"
                    | "0"
                    | "*"
                    | "`"
                    | "<"
                    | "]"
                    | "_"
                    | "Ctrl+v"
                    | "Ctrl+f"
                    | "Ctrl+s"
                    | "Ctrl+x"
                    | "Ctrl+q"
                    | "Ctrl+p"
                    | "Ctrl+g"
            );
            let last = binding.trigger.rsplit('+').next().unwrap_or("");
            if !preset(&binding.preset)
                || binding.trigger.is_empty()
                || binding.trigger.encode_utf16().count() > 64
                || binding.trigger.chars().any(char::is_control)
                || matches!(last, "Escape" | "\\" | "Meta" | "Control" | "Shift" | "Alt")
                || !(register || action)
                || !seen.insert((&binding.preset, &binding.trigger))
            {
                return Err(error(
                    "preferences-invalid",
                    "Invalid or duplicate key binding",
                ));
            }
        }
        rfb_core::Game::validate_behavior_preferences(&self.behavior())
            .map_err(|e| error("preferences-invalid", e))
    }
}

// Lock a separate file: replacing preferences.json must not release the cross-process lock.
fn lock(root: &Path) -> DesktopResult<File> {
    fs::create_dir_all(root).map_err(|e| error("preferences-write", e))?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("preferences.lock"))
        .map_err(|e| error("preferences-write", e))?;
    file.try_lock().map_err(|e| error("preferences-busy", e))?;
    Ok(file)
}

fn read(root: &Path) -> DesktopResult<Option<PreferenceSnapshot>> {
    let bytes = match fs::read(root.join("preferences.json")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error("preferences-read", e)),
    };
    let saved: PreferenceSnapshot =
        serde_json::from_slice(&bytes).map_err(|e| error("preferences-corrupt", e))?;
    saved
        .preferences
        .validate()
        .map_err(|e| error("preferences-corrupt", e.detail))?;
    if saved.revision == 0 {
        return Err(error("preferences-corrupt", "Invalid revision"));
    }
    Ok(Some(saved))
}

fn save(
    root: &Path,
    expected_revision: Option<u32>,
    preferences: Preferences,
) -> DesktopResult<PreferenceSnapshot> {
    preferences.validate()?;
    let _lock = lock(root)?;
    let current = read(root)?;
    if current.as_ref().map(|value| value.revision) != expected_revision {
        return Err(error(
            "preferences-stale",
            "Preferences changed; reload before editing again",
        ));
    }
    let saved = PreferenceSnapshot {
        revision: expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| error("preferences-write", "Revision exhausted"))?,
        preferences,
    };
    let bytes = serde_json::to_vec_pretty(&saved).map_err(|e| error("preferences-write", e))?;
    let temporary = root.join("preferences.pending");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|e| error("preferences-write", e))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| error("preferences-write", e))?;
    drop(file);
    fs::rename(&temporary, root.join("preferences.json"))
        .map_err(|e| error("preferences-write", e))?;
    Ok(saved)
}

#[tauri::command]
pub fn default_preferences() -> Preferences {
    Preferences::default()
}

pub(super) fn current_behavior(
    app: &tauri::AppHandle,
) -> DesktopResult<rfb_protocol::BehaviorPreferencesDto> {
    load_preferences(app.clone())?
        .map(|s| s.preferences.behavior())
        .ok_or_else(|| {
            error(
                "preferences-unavailable",
                "Load global preferences before starting a character",
            )
        })
}

#[tauri::command]
pub fn load_preferences(app: tauri::AppHandle) -> DesktopResult<Option<PreferenceSnapshot>> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| error("preferences-read", e))?;
    let _lock = lock(&root)?;
    read(&root)
}

#[tauri::command]
pub fn save_preferences(
    app: tauri::AppHandle,
    expected_revision: Option<u32>,
    preferences: Preferences,
) -> DesktopResult<PreferenceSnapshot> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| error("preferences-write", e))?;
    save(&root, expected_revision, preferences)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_preferences_validate_unicode_and_colors_without_changing_behavior() {
        let mut p = Preferences::default();
        let before = p.behavior();
        p.visuals.overrides.insert(
            "demo.terrain.floor".into(),
            VisualOverride {
                glyph: Some("地".into()),
                foreground: Some("#ff0000".into()),
                background: None,
            },
        );
        p.validate().unwrap();
        assert_eq!(p.behavior(), before);
        assert_eq!(
            serde_json::from_value::<Preferences>(serde_json::to_value(&p).unwrap()).unwrap(),
            p
        );
        for glyph in ["\n", "\u{85}", "\u{202e}", " ", "ab"] {
            p.visuals
                .overrides
                .get_mut("demo.terrain.floor")
                .unwrap()
                .glyph = Some(glyph.into());
            assert!(p.validate().is_err());
        }
        p.visuals.overrides.clear();
        p.visuals.palette[0] = "red".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn display_is_global_and_validates_warning_thresholds() {
        let mut p = Preferences::default();
        let behavior = p.behavior();
        p.display.target_path = true;
        p.display.hp_warning_percent = 30;
        p.validate().unwrap();
        assert_eq!(p.behavior(), behavior);
        assert_eq!(
            serde_json::from_value::<Preferences>(serde_json::to_value(&p).unwrap()).unwrap(),
            p
        );
        p.display.hp_warning_percent = 35;
        assert_eq!(p.validate().unwrap_err().code, "preferences-invalid");
        let mut json = serde_json::to_value(Preferences::default()).unwrap();
        json["display"]["resourceBars"] = false.into();
        assert!(serde_json::from_value::<Preferences>(json).is_err());
    }

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rfb-preferences-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn stale_edit_and_invalid_values_preserve_committed_preferences() {
        let root = root();
        let first = save(&root, None, Preferences::default()).unwrap();
        let mut changed = first.preferences.clone();
        changed.locale = "en-US".into();
        let second = save(&root, Some(first.revision), changed).unwrap();
        assert_eq!(
            save(&root, Some(first.revision), first.preferences.clone())
                .unwrap_err()
                .code,
            "preferences-stale"
        );
        let mut invalid = first.preferences;
        invalid.zoom = 0.0;
        assert_eq!(
            save(&root, Some(second.revision), invalid)
                .unwrap_err()
                .code,
            "preferences-invalid"
        );
        fs::create_dir(root.join("preferences.pending")).unwrap();
        assert_eq!(
            save(&root, Some(second.revision), second.preferences.clone())
                .unwrap_err()
                .code,
            "preferences-write"
        );
        assert_eq!(read(&root).unwrap(), Some(second));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corruption_and_lock_conflict_do_not_reset_the_file() {
        let root = root();
        let held = lock(&root).unwrap();
        fs::write(root.join("preferences.json"), b"broken").unwrap();
        assert_eq!(
            save(&root, None, Preferences::default()).unwrap_err().code,
            "preferences-busy"
        );
        drop(held);
        assert_eq!(
            save(&root, None, Preferences::default()).unwrap_err().code,
            "preferences-corrupt"
        );
        assert_eq!(fs::read(root.join("preferences.json")).unwrap(), b"broken");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_load_injects_current_preferences_without_replacing_character_facts() {
        let root = root();
        let state = crate::AppState::new(root.join("profile"));
        let initial = state
            .initialize(
                "42",
                "demo.build.warrior",
                "demo.race.rfb-human",
                "Preferences",
                String::new(),
                Preferences::default().behavior(),
            )
            .unwrap();
        let bytes = state.save(String::new()).unwrap();
        let (_, before) = rfb_save::decode(&bytes).unwrap();
        let mut global = Preferences::default();
        global.locale = "en-US".into();
        global.travel.always_pickup = true;
        let persisted = save(&root, None, global.clone()).unwrap();
        let loaded = state
            .load_with_recovery(&bytes, persisted.preferences.behavior())
            .unwrap()
            .0;
        assert_eq!(loaded.travel_options, global.travel);
        assert_eq!(loaded.mogaminator.locale, rfb_protocol::LocaleDto::EnUs);
        assert_eq!(loaded.turn, initial.turn);
        let (_, after) = rfb_save::decode(&state.save(String::new()).unwrap()).unwrap();
        assert_eq!(before, after);
        assert_eq!(read(&root).unwrap(), Some(persisted));
        fs::remove_dir_all(root).unwrap();
    }
}
