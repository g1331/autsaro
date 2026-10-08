import i18next from 'i18next';
import { initReactI18next, useTranslation } from 'react-i18next';
import { backendResources } from './backend';
import { shellResources } from './shell';
import { editorResources } from './editors';
import { workflowResources } from './workflow';
import { controllerResources } from './controller';
import { displayPath } from '../pathDisplay';

export type LanguagePreference = 'system' | 'zh-CN' | 'en';
export type Language = 'zh-CN' | 'en';
export type MessageParams = Record<string, string | number | boolean | null>;
export type Message = { key: string; params: MessageParams };
export type LocalizedText = Message | string | LocalizedText[];
/** Frontend-only composition retains nested messages until presentation. IPC stays LocalizedText. */
export type Text =
  LocalizedText | { key: string; params: MessageParams; values: Record<string, Text> } | Text[];
export function composedMessage(
  key: string,
  values: Record<string, Text>,
  params: MessageParams = {},
): Text {
  return { key, params, values };
}
export class ProductError extends Error {
  constructor(readonly text: Text) {
    super(typeof text === 'string' ? text : 'Product operation failed');
  }
}
export const message = (key: string, params: MessageParams = {}): Message => ({ key, params });
export function isMessage(value: unknown): value is Message {
  return (
    typeof value === 'object' &&
    value !== null &&
    'key' in value &&
    typeof value.key === 'string' &&
    'params' in value &&
    typeof value.params === 'object' &&
    value.params !== null &&
    !Array.isArray(value.params) &&
    Object.values(value.params).every(
      (param) =>
        param === null ||
        typeof param === 'string' ||
        typeof param === 'number' ||
        typeof param === 'boolean',
    )
  );
}
export function isLocalizedText(value: unknown): value is LocalizedText {
  return (
    typeof value === 'string' ||
    isMessage(value) ||
    (Array.isArray(value) && value.every(isLocalizedText))
  );
}
export function resolveLanguage(
  preference: LanguagePreference,
  systemLanguage = typeof navigator === 'undefined' ? 'en' : navigator.language,
): Language {
  return preference === 'system'
    ? systemLanguage.toLowerCase().startsWith('zh')
      ? 'zh-CN'
      : 'en'
    : preference;
}
const preferenceKey = 'autsaro.language';
export function readLanguagePreference(): LanguagePreference {
  const saved = typeof localStorage === 'undefined' ? null : localStorage.getItem(preferenceKey);
  return saved === 'zh-CN' || saved === 'en' ? saved : 'system';
}
export function persistLanguagePreference(preference: LanguagePreference): void {
  localStorage.setItem(preferenceKey, preference);
}
export const resources = Object.fromEntries(
  (['zh-CN', 'en'] as const).map((language) => [
    language,
    {
      translation: {
        ...backendResources[language],
        ...shellResources[language],
        ...editorResources[language],
        ...workflowResources[language],
        ...controllerResources[language],
      },
    },
  ]),
);
const chineseKeys = Object.keys(resources['zh-CN'].translation);
const englishKeys = Object.keys(resources.en.translation);
if (
  chineseKeys.length !== englishKeys.length ||
  chineseKeys.some((key) => !(key in resources.en.translation))
) {
  throw new Error('Incomplete bilingual localization resources');
}
export const i18n = i18next.createInstance();
void i18n.use(initReactI18next).init({
  resources,
  lng: resolveLanguage(readLanguagePreference()),
  supportedLngs: ['zh-CN', 'en'],
  fallbackLng: false,
  keySeparator: false,
  interpolation: { escapeValue: false },
  initAsync: false,
});
export function translate(key: string, params: MessageParams = {}): string {
  const displayParams = { ...params };
  for (const name of ['path', 'file', 'directory', 'backup']) {
    const value = displayParams[name];
    if (typeof value === 'string') displayParams[name] = displayPath(value);
  }
  if (!i18n.exists(key, { ...displayParams, lng: i18n.language }))
    throw new Error(`Missing localization key: ${i18n.language}:${key}`);
  return i18n.t(key, displayParams);
}
export function localize(value: Text | null | undefined): string {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(localize).join('\n');
  const params =
    'values' in value
      ? {
          ...value.params,
          ...Object.fromEntries(
            Object.entries(value.values).map(([key, item]) => [key, localize(item)]),
          ),
        }
      : value.params;
  return translate(value.key, params);
}
export function useLocale() {
  useTranslation(undefined, { i18n });
  return { t: translate, text: localize, language: i18n.language as Language };
}
export function previewLanguage(preference: LanguagePreference): void {
  void i18n.changeLanguage(resolveLanguage(preference));
  if (typeof document !== 'undefined') document.documentElement.lang = i18n.language;
}
i18n.on('languageChanged', (language) => {
  if (typeof document !== 'undefined') {
    document.documentElement.lang = language;
    document.title = translate('controller.app.title');
  }
});
if (typeof document !== 'undefined') {
  document.documentElement.lang = i18n.language;
  document.title = translate('controller.app.title');
}
