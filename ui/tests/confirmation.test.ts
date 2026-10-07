import { afterEach, expect, test, vi } from 'vitest';

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
  const { requestConfirmation } = await import('../src/confirmation');
  expect(await requestConfirmation('Discard edits?')).toBe(false);
  expect(browserConfirm).not.toHaveBeenCalled();
});

test('browser confirmation preserves the user decision', async () => {
  platform.native = false;
  vi.stubGlobal('window', { confirm: (message: string) => message === 'Accept?' });
  const { requestConfirmation } = await import('../src/confirmation');
  expect(await requestConfirmation('Accept?')).toBe(true);
  expect(await requestConfirmation('Discard?')).toBe(false);
});
