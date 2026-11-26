import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useEffectEvent } from 'react';
import { composedMessage, translate, useLocale } from '../i18n';
import { native } from '../ipc/native';
import { errorText } from '../shared/ui/forms';
import type { WorkbenchSession } from './session';

export function useWindowTitle(patchState: WorkbenchSession['patchState']) {
  const { language } = useLocale();
  const updateNativeTitle = useEffectEvent(() => {
    void getCurrentWindow()
      .setTitle(translate('controller.app.title'))
      .catch((error: unknown) => {
        patchState({
          notice: {
            tone: 'error',
            text: composedMessage('controller.error.windowTitle', { error: errorText(error) }),
          },
        });
      });
  });
  useEffect(() => {
    if (native) updateNativeTitle();
  }, [language]);
}
