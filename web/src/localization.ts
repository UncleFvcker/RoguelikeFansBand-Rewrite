// SPDX-License-Identifier: MPL-2.0

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";

export const SUPPORTED_LOCALES = ["en-US", "zh-CN"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];
export type MessageKey = string;
export type LocalizationArgs = Record<string, FluentVariable>;
export type LocaleSources = Record<SupportedLocale, readonly string[]>;

export class Localization {
  readonly #bundles: Record<SupportedLocale, FluentBundle>;
  #locale: SupportedLocale;

  constructor(locale: SupportedLocale, sources: LocaleSources) {
    this.#locale = locale;
    this.#bundles = {
      "en-US": createBundle("en-US", sources["en-US"]),
      "zh-CN": createBundle("zh-CN", sources["zh-CN"]),
    };
  }

  get locale(): SupportedLocale {
    return this.#locale;
  }

  setLocale(locale: SupportedLocale): void {
    this.#locale = locale;
  }

  hasMessage(locale: SupportedLocale, key: MessageKey): boolean {
    return this.#bundles[locale].hasMessage(key);
  }

  format(key: MessageKey, args?: LocalizationArgs): string {
    return this.#formatFrom(this.#locale, key, args) ?? this.#formatFrom("en-US", key, args) ?? `[${key}]`;
  }

  localizeDocument(root: ParentNode = document): void {
    for (const element of root.querySelectorAll<HTMLElement>("[data-l10n-id]")) {
      const key = element.dataset.l10nId;
      if (key) element.textContent = this.format(key);
    }
    for (const element of root.querySelectorAll<HTMLElement>("[data-l10n-aria-label]")) {
      const key = element.dataset.l10nAriaLabel;
      if (key) element.setAttribute("aria-label", this.format(key));
    }
    for (const element of root.querySelectorAll<HTMLElement>("[data-l10n-label]")) {
      const key = element.dataset.l10nLabel;
      if (key) element.setAttribute("label", this.format(key));
    }
    document.documentElement.lang = this.#locale;
  }

  #formatFrom(locale: SupportedLocale, key: MessageKey, args?: LocalizationArgs): string | undefined {
    const bundle = this.#bundles[locale];
    const message = bundle.getMessage(key);
    if (!message?.value) return undefined;
    const errors: Error[] = [];
    const value = bundle.formatPattern(message.value, args, errors);
    if (errors.length > 0) {
      console.warn(`Fluent formatting error for ${locale}/${key}`, errors);
    }
    return value;
  }
}

export function isSupportedLocale(value: string | null): value is SupportedLocale {
  return SUPPORTED_LOCALES.some((locale) => locale === value);
}

function createBundle(locale: SupportedLocale, sources: readonly string[]): FluentBundle {
  const bundle = new FluentBundle(locale, { useIsolating: false });
  for (const source of sources) {
    const errors = bundle.addResource(new FluentResource(source));
    if (errors.length > 0) {
      throw new Error(`Invalid Fluent resource for ${locale}: ${errors.join("; ")}`);
    }
  }
  return bundle;
}
