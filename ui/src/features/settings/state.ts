import { readLanguagePreference } from '../../i18n';
import type { LanguagePreference, Text } from '../../i18n';
import type { ExecutionTools } from '../../ipc/types';
import type { Notice } from '../../shared/ui/forms';
import type { Appearance } from '../../ipc/types';

export type SettingsState = {
  settingsOpen: boolean;
  settingsCategory: 'appearance' | 'language' | 'definitions' | 'tools';
  toolDraft: ExecutionTools;
  toolErrors: Partial<Record<keyof ExecutionTools, Text>>;
  settingsNotice: Notice;
  appearanceDraft: Appearance;
  languageDraft: LanguagePreference;
  savedLanguage: LanguagePreference;
  languageSaving: boolean;
};

export function initialSettingsState(): SettingsState {
  return {
    settingsOpen: false,
    settingsCategory: 'appearance',
    toolDraft: { compiler: '', objdump: '', git: '', python: '' },
    toolErrors: {},
    settingsNotice: null,
    appearanceDraft: 'system',
    languageDraft: readLanguagePreference(),
    savedLanguage: readLanguagePreference(),
    languageSaving: false,
  };
}
