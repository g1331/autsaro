import { isTauri } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import { localize, translate } from './i18n';
import type { Text } from './i18n';

/** Tauri's injected window.confirm returns a Promise, despite the DOM type. */
export async function requestConfirmation(prompt: Text): Promise<boolean> {
  const rendered = localize(prompt);
  if (isTauri())
    return confirm(rendered, { title: translate('workflow.action.confirm'), kind: 'warning' });
  return window.confirm(rendered);
}
