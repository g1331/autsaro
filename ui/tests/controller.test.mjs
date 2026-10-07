// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

test('the composed controller updates form state without exposing workspace replacement setters', async () => {
  const { useWorkbench } = await import('../src/workbench/useWorkbench');
  const { result, unmount } = renderHook(() => useWorkbench());
  expect(result.current.setWorkspace).toBeUndefined();
  expect(result.current.setCapabilities).toBeUndefined();
  act(() => result.current.setProjectName('Example'));
  expect(result.current.projectName).toBe('Example');
  expect(typeof result.current.applyChanges).toBe('function');
  unmount();
});
