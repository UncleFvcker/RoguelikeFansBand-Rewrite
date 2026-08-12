// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use rfb_content::ContentCatalog;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

mod rfb_matching_names;

const EN_US: [&str; 3] = [
    include_str!("../../../locales/en-US/ui.ftl"),
    include_str!("../../../locales/en-US/game.ftl"),
    include_str!("../../../locales/en-US/content.ftl"),
];
const ZH_CN: [&str; 3] = [
    include_str!("../../../locales/zh-CN/ui.ftl"),
    include_str!("../../../locales/zh-CN/game.ftl"),
    include_str!("../../../locales/zh-CN/content.ftl"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    ZhCn,
}

impl Locale {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }
}

pub struct Localizer {
    locale: Locale,
    english: FluentBundle<FluentResource>,
    chinese: FluentBundle<FluentResource>,
}

impl Localizer {
    pub fn new(locale: Locale) -> Result<Self, LocalizationError> {
        Ok(Self {
            locale,
            english: create_bundle(Locale::EnUs, &EN_US)?,
            chinese: create_bundle(Locale::ZhCn, &ZH_CN)?,
        })
    }

    #[must_use]
    pub const fn locale(&self) -> Locale {
        self.locale
    }

    pub const fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    #[must_use]
    pub fn has_message(&self, locale: Locale, key: &str) -> bool {
        self.bundle(locale).has_message(key)
    }

    pub fn format(
        &self,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Result<String, LocalizationError> {
        if let Some(value) = format_from(self.bundle(self.locale), key, args)? {
            return Ok(value);
        }
        if self.locale != Locale::EnUs
            && let Some(value) = format_from(&self.english, key, args)?
        {
            return Ok(value);
        }
        Err(LocalizationError::MissingMessage(key.to_owned()))
    }

    /// Formats one locale without falling back to English.
    ///
    /// Logic driven by localized text, such as Mogaminator rules,
    /// must fail on a missing translation instead of silently changing the
    /// language being matched.
    pub fn format_exact(
        &self,
        locale: Locale,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Result<String, LocalizationError> {
        format_from(self.bundle(locale), key, args)?
            .ok_or_else(|| LocalizationError::MissingMessage(key.to_owned()))
    }

    fn bundle(&self, locale: Locale) -> &FluentBundle<FluentResource> {
        match locale {
            Locale::EnUs => &self.english,
            Locale::ZhCn => &self.chinese,
        }
    }
}

/// Resolves the authoritative item matching name used by Mogaminator.
///
/// Display names remain in Fluent. RFB-derived English matching names and the
/// few rewrite-only aliases that need different search nouns stay private to
/// this resolver; callers decide whether the resolved true name may be shown.
pub struct MogaminatorNames {
    localizer: Localizer,
}

impl MogaminatorNames {
    pub fn new(locale: Locale) -> Result<Self, LocalizationError> {
        Ok(Self {
            localizer: Localizer::new(locale)?,
        })
    }

    pub fn item_name(
        &self,
        content: &ContentCatalog,
        kind_id: &str,
        affix_ids: &[String],
        activation_profile_id: Option<&str>,
    ) -> Result<String, MogaminatorNameError> {
        let item = content
            .item(kind_id)
            .ok_or_else(|| MogaminatorNameError::UnknownItem(kind_id.to_owned()))?;
        let base_name = match self.localizer.locale() {
            Locale::EnUs => {
                if let Some(name) = rfb_matching_names::english_item_name(kind_id) {
                    name.to_owned()
                } else {
                    self.localizer
                        .format_exact(self.localizer.locale(), &item.name_key, None)?
                }
            }
            Locale::ZhCn if kind_id == "demo.item.resonance-rod" => "魔棒".to_owned(),
            Locale::ZhCn => {
                self.localizer
                    .format_exact(self.localizer.locale(), &item.name_key, None)?
            }
        };
        let base_name = if let Some(profile_id) = activation_profile_id {
            let profile = item
                .device_generation
                .as_ref()
                .and_then(|generation| {
                    generation
                        .activations
                        .iter()
                        .find(|profile| profile.id == profile_id)
                })
                .ok_or_else(|| MogaminatorNameError::UnknownActivation(profile_id.to_owned()))?;
            let activation_name =
                self.localizer
                    .format_exact(self.localizer.locale(), &profile.name_key, None)?;
            match self.localizer.locale() {
                Locale::EnUs => format!("{base_name} of {activation_name}"),
                Locale::ZhCn => format!("{activation_name}{base_name}"),
            }
        } else {
            base_name
        };
        let mut prefixes = String::new();
        let mut suffixes = String::new();
        for affix_id in affix_ids.iter().collect::<BTreeSet<_>>() {
            let affix = content
                .affix(affix_id)
                .ok_or_else(|| MogaminatorNameError::UnknownAffix(affix_id.clone()))?;
            let affix_name =
                self.localizer
                    .format_exact(self.localizer.locale(), &affix.name_key, None)?;
            if self.localizer.locale() == Locale::ZhCn
                && let Some(prefix) = chinese_prefix(&affix_name)
            {
                prefixes.push_str(prefix);
            } else {
                suffixes.push(' ');
                suffixes.push_str(affix_name.trim());
            }
        }
        Ok(format!("{prefixes}{}{suffixes}", base_name.trim()))
    }
}

fn chinese_prefix(name: &str) -> Option<&str> {
    let name = name.trim();
    let unwrapped = name
        .strip_prefix('(')
        .and_then(|name| name.strip_suffix(')'))
        .unwrap_or(name);
    (unwrapped.ends_with('之') || unwrapped.ends_with('的')).then_some(unwrapped)
}

#[derive(Debug, Error)]
pub enum MogaminatorNameError {
    #[error("unknown item definition {0}")]
    UnknownItem(String),
    #[error("unknown affix definition {0}")]
    UnknownAffix(String),
    #[error("unknown device activation {0}")]
    UnknownActivation(String),
    #[error(transparent)]
    Localization(#[from] LocalizationError),
}

fn create_bundle(
    locale: Locale,
    sources: &[&str],
) -> Result<FluentBundle<FluentResource>, LocalizationError> {
    let language: LanguageIdentifier = locale
        .id()
        .parse::<LanguageIdentifier>()
        .map_err(|error| LocalizationError::InvalidLocale(error.to_string()))?;
    let mut bundle = FluentBundle::new(vec![language]);
    bundle.set_use_isolating(false);
    for source in sources {
        let resource = FluentResource::try_new((*source).to_owned()).map_err(|(_, errors)| {
            LocalizationError::InvalidResource(
                errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        bundle.add_resource(resource).map_err(|errors| {
            LocalizationError::InvalidResource(
                errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
    }
    Ok(bundle)
}

fn format_from(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs<'_>>,
) -> Result<Option<String>, LocalizationError> {
    let Some(message) = bundle.get_message(key) else {
        return Ok(None);
    };
    let Some(pattern) = message.value() else {
        return Ok(None);
    };
    let mut errors = Vec::new();
    let value = bundle
        .format_pattern(pattern, args, &mut errors)
        .into_owned();
    if errors.is_empty() {
        Ok(Some(value))
    } else {
        Err(LocalizationError::Format(
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }
}

#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("invalid locale: {0}")]
    InvalidLocale(String),
    #[error("invalid Fluent resource: {0}")]
    InvalidResource(String),
    #[error("missing Fluent message {0}")]
    MissingMessage(String),
    #[error("Fluent formatting failed: {0}")]
    Format(String),
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fluent_bundle::FluentArgs;
    use rfb_content::{ContentCatalog, compile_pack_dir};

    use super::*;

    #[test]
    fn bundled_resources_format_in_both_languages() {
        let mut localizer = Localizer::new(Locale::EnUs).expect("resources should load");
        let mut args = FluentArgs::new();
        args.set("target", "ration of food");
        args.set("quantity", 3);
        assert_eq!(
            localizer
                .format("message-item-pickup-success", Some(&args))
                .expect("English should format"),
            "You pick up ration of food ×3."
        );

        localizer.set_locale(Locale::ZhCn);
        args.set("target", "食物口粮");
        assert_eq!(
            localizer
                .format("message-item-pickup-success", Some(&args))
                .expect("Chinese should format"),
            "你将 3 个食物口粮收入了背包。"
        );
    }

    #[test]
    fn both_locales_expose_representative_ui_and_content_keys() {
        let localizer = Localizer::new(Locale::EnUs).expect("resources should load");
        for locale in [Locale::EnUs, Locale::ZhCn] {
            for key in [
                "app-title",
                "controls-numpad",
                "message-combat-hit",
                "item-demo-ration-of-food-name",
            ] {
                assert!(localizer.has_message(locale, key), "{locale:?}/{key}");
            }
        }
    }

    #[test]
    fn trouble_at_home_uses_authoritative_chinese_names() {
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for (key, expected) in [
            ("task-demo-trouble-at-home-name", "家里的麻烦 (前哨镇)"),
            ("actor-demo-mean-looking-mercenary-name", "面相凶狠的雇佣兵"),
            ("actor-demo-singing-happy-drunk-name", "快乐唱歌的醉汉"),
            ("item-demo-piece-of-elvish-waybread-name", "精灵干粮"),
            ("item-demo-booze-potion-name", "烈酒"),
        ] {
            assert_eq!(
                localizer
                    .format_exact(Locale::ZhCn, key, None)
                    .expect("Trouble at Home name should format"),
                expected
            );
        }
    }

    #[test]
    fn crows_nest_uses_authoritative_chinese_names() {
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for (key, expected) in [
            ("task-demo-crows-nest-name", "乌鸦巢 (前哨镇)"),
            ("item-demo-human-skeleton-name", "一具人类骨架"),
            ("item-demo-enlightenment-staff-name", "启明法杖"),
            ("device-activation-demo-enlightenment-name", "启示"),
        ] {
            assert_eq!(
                localizer
                    .format_exact(Locale::ZhCn, key, None)
                    .expect("Crow's Nest name should format"),
                expected
            );
        }
    }

    #[test]
    fn old_man_willow_uses_authoritative_chinese_names() {
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for (key, expected) in [
            ("task-demo-old-man-willow-name", "柳树老头任务 (前哨镇)"),
            ("actor-demo-old-man-willow-name", "柳树老头"),
            ("affix-legacy-elemental-jewelry-name", "(元素的)"),
        ] {
            assert_eq!(
                localizer
                    .format_exact(Locale::ZhCn, key, None)
                    .expect("Old Man Willow name should format"),
                expected
            );
        }
    }

    #[test]
    fn vapor_quest_uses_authoritative_chinese_names() {
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for (key, expected) in [
            ("task-demo-vapor-quest-name", "蒸汽任务 (前哨镇)"),
            ("actor-demo-gas-spore-name", "瓦斯孢子"),
            ("actor-demo-air-elemental-name", "气元素"),
            ("actor-demo-shimmering-vortex-name", "闪光漩涡"),
            ("actor-demo-weird-fume-name", "怪异烟雾"),
            ("item-demo-amulet-name", "护身符"),
            ("item-demo-detection-rod-name", "探测魔棒"),
            ("device-activation-demo-detection-name", "探测"),
        ] {
            assert_eq!(
                localizer
                    .format_exact(Locale::ZhCn, key, None)
                    .expect("Vapor Quest name should format"),
                expected
            );
        }
    }

    #[test]
    fn old_castle_uses_authoritative_chinese_names() {
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for (key, expected) in [
            ("task-demo-old-castle-name", "旧城堡 (前哨镇)"),
            ("actor-demo-anti-paladin-name", "反圣武士"),
            ("actor-demo-ancient-red-dragon-name", "上古红龙"),
            ("actor-demo-dracolich-name", "龙巫妖"),
            ("item-demo-crisdurian-name", "刽子手之剑『克里斯杜瑞安』"),
            ("item-demo-slayer-name", "刽子手之剑『杀戮者』"),
            ("item-demo-pain-name", "痛苦之大刀"),
        ] {
            assert_eq!(
                localizer
                    .format_exact(Locale::ZhCn, key, None)
                    .expect("Old Castle name should format"),
                expected
            );
        }
    }

    #[test]
    fn all_item_affix_and_artifact_names_have_exact_matching_messages() {
        let pack = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original");
        let artifact = compile_pack_dir(&pack).expect("original pack should compile");
        let localizer = Localizer::new(Locale::ZhCn).expect("resources should load");
        for locale in [Locale::EnUs, Locale::ZhCn] {
            for item in &artifact.content.items {
                localizer
                    .format_exact(locale, &item.name_key, None)
                    .unwrap_or_else(|error| {
                        panic!("{locale:?}/{}/{}: {error}", item.id, item.name_key)
                    });
            }
            for affix in &artifact.content.affixes {
                localizer
                    .format_exact(locale, &affix.name_key, None)
                    .unwrap_or_else(|error| {
                        panic!("{locale:?}/{}/{}: {error}", affix.id, affix.name_key)
                    });
            }
        }
    }

    #[test]
    fn mogaminator_name_uses_true_chinese_name_and_stable_affixes() {
        let pack = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original");
        let content = ContentCatalog::from_artifact(
            compile_pack_dir(&pack).expect("original pack should compile"),
        );
        let names = MogaminatorNames::new(Locale::ZhCn).expect("resources should load");

        assert_eq!(
            names
                .item_name(
                    &content,
                    "demo.item.short-sword",
                    &[
                        "demo.affix.vampiric".to_owned(),
                        "demo.affix.vampiric".to_owned(),
                    ],
                    None,
                )
                .expect("name should resolve"),
            "短剑 吸血"
        );
        assert_eq!(
            names
                .item_name(&content, "demo.item.relic-blade", &[], None)
                .expect("artifact name should resolve"),
            "遗珍之刃"
        );
        assert_eq!(
            names
                .item_name(
                    &content,
                    "demo.item.resonance-rod",
                    &[],
                    Some("demo.device-activation.trap-sense"),
                )
                .expect("device activation should resolve"),
            "陷阱感知魔棒"
        );
    }

    #[test]
    fn mogaminator_name_uses_english_when_selected() {
        let pack = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original");
        let content = ContentCatalog::from_artifact(
            compile_pack_dir(&pack).expect("original pack should compile"),
        );
        let names = MogaminatorNames::new(Locale::EnUs).expect("resources should load");

        assert_eq!(
            names
                .item_name(
                    &content,
                    "demo.item.short-sword",
                    &["demo.affix.vampiric".to_owned()],
                    None,
                )
                .expect("name should resolve"),
            "Short Sword vampiric"
        );
        assert_eq!(
            names
                .item_name(&content, "demo.item.relic-blade", &[], None)
                .expect("artifact name should resolve"),
            "Relic Blade"
        );
        assert_eq!(
            names
                .item_name(&content, "demo.item.sixfold-provision", &[], None)
                .expect("RFB matching name should resolve"),
            "Mushroom of Restoring"
        );
        assert_eq!(
            names
                .item_name(
                    &content,
                    "demo.item.resonance-rod",
                    &[],
                    Some("demo.device-activation.trap-sense"),
                )
                .expect("device activation should resolve"),
            "Rod of trap sense"
        );
    }

    #[test]
    fn chinese_ego_prefixes_follow_original_composition() {
        assert_eq!(chinese_prefix("杀戮之"), Some("杀戮之"));
        assert_eq!(chinese_prefix("(受祝福的)"), Some("受祝福的"));
        assert_eq!(chinese_prefix("吸血"), None);
    }
}
