import { useEffect } from 'react';
import { previewLanguage } from '../../i18n';
import type { LanguagePreference } from '../../i18n';

export function useLanguagePreview(languageDraft: LanguagePreference) {
  useEffect(() => {
    previewLanguage(languageDraft);
  }, [languageDraft]);
}
