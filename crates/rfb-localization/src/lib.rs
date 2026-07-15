// SPDX-License-Identifier: MPL-2.0

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use thiserror::Error;
use unic_langid::LanguageIdentifier;

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

    fn bundle(&self, locale: Locale) -> &FluentBundle<FluentResource> {
        match locale {
            Locale::EnUs => &self.english,
            Locale::ZhCn => &self.chinese,
        }
    }
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
    use fluent_bundle::FluentArgs;

    use super::*;

    #[test]
    fn bundled_resources_format_in_both_languages() {
        let mut localizer = Localizer::new(Locale::EnUs).expect("resources should load");
        let mut args = FluentArgs::new();
        args.set("target", "luminous shard");
        args.set("quantity", 3);
        assert_eq!(
            localizer
                .format("message-item-pickup-success", Some(&args))
                .expect("English should format"),
            "You pick up luminous shard ×3."
        );

        localizer.set_locale(Locale::ZhCn);
        args.set("target", "发光碎片");
        assert_eq!(
            localizer
                .format("message-item-pickup-success", Some(&args))
                .expect("Chinese should format"),
            "你将 3 个发光碎片收入了背包。"
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
                "item-demo-luminous-shard-name",
            ] {
                assert!(localizer.has_message(locale, key), "{locale:?}/{key}");
            }
        }
    }
}
