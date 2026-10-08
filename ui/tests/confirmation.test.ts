import { afterEach, expect, test, vi } from 'vitest';
import { localize, message, translate } from '../src/i18n';
import { requestConfirmation } from '../src/confirmation';

const platform = vi.hoisted(() => ({ native: true, confirm: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => platform.native }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: platform.confirm }));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

test('native cancellation is awaited and never falls through to browser confirmation', async () => {
  platform.native = true;
  platform.confirm.mockResolvedValue(false);
  const browserConfirm = vi.fn(() => {
    throw new Error('browser confirmation was used');
  });
  vi.stubGlobal('window', { confirm: browserConfirm });
  const prompt = message('workflow.confirm.removeDtc');
  expect(await requestConfirmation(prompt)).toBe(false);
  expect(platform.confirm).toHaveBeenCalledWith(localize(prompt), {
    title: translate('workflow.action.confirm'),
    kind: 'warning',
  });
  expect(browserConfirm).not.toHaveBeenCalled();
});

test('browser confirmation preserves the user decision', async () => {
  platform.native = false;
  vi.stubGlobal('window', { confirm: (message: string) => message === 'Accept?' });
  expect(await requestConfirmation('Accept?')).toBe(true);
  expect(await requestConfirmation('Discard?')).toBe(false);
});

test('browser confirmation renders a product descriptor before asking', async () => {
  platform.native = false;
  const browserConfirm = vi.fn(() => false);
  vi.stubGlobal('window', { confirm: browserConfirm });
  const prompt = message('workflow.confirm.generateFiles', {
    count: 3,
    directory: 'D:/Project/output',
  });
  expect(await requestConfirmation(prompt)).toBe(false);
  expect(browserConfirm).toHaveBeenCalledWith(localize(prompt));
});
