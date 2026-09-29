import { isTauri } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';

/** Tauri's injected window.confirm returns a Promise, despite the DOM type. */
export async function requestConfirmation(message: string): Promise<boolean> {
  if (isTauri()) return confirm(message, { title: '确认操作', kind: 'warning' });
  return window.confirm(message);
}
