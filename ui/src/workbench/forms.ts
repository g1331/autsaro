import type { Text } from '../i18n';
import { composedMessage, isMessage, ProductError } from '../i18n';
import type { IntegrationInspection } from '../types';

export type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';

export type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';

export type StageRecord = { state: StageState; detail: Text };

export type Notice = { tone: 'error' | 'info'; text: Text } | null;

/** Period shared by the application timing events; null when no event is planned. */
export function applicationPeriodMs(
  description: IntegrationInspection['description'] | undefined,
): number | null {
  return description?.schedule?.entities.find((entity) => entity.application)?.periodMs ?? null;
}

/** Draft text of the quick-edit period field. */
export function applicationPeriodDraft(
  description: IntegrationInspection['description'] | undefined,
): string {
  const period = applicationPeriodMs(description);
  return period === null ? '' : String(period);
}

export function labelFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

export function intInRange(value: string, label: Text, min: number, max: number): number {
  const parsed = Number(value);
  if (
    !/^\d+$/.test(value.trim()) ||
    !Number.isSafeInteger(parsed) ||
    parsed < min ||
    parsed > max
  ) {
    throw new ProductError(
      composedMessage('controller.error.integerRange', { label }, { min, max }),
    );
  }
  return parsed;
}

export function errorText(error: unknown): Text {
  if (error instanceof ProductError) return error.text;
  if (isMessage(error)) return error;
  if (Array.isArray(error)) return error.map(errorText);
  if (error && typeof error === 'object' && 'message' in error && !(error instanceof Error)) {
    const diagnostic = error as { message: unknown; code?: unknown; remedy?: unknown };
    return composedMessage(
      'controller.error.diagnostic',
      {
        detail: errorText(diagnostic.message),
        remedy: diagnostic.remedy == null ? '' : errorText(diagnostic.remedy),
      },
      { code: diagnostic.code == null ? '' : String(diagnostic.code) },
    );
  }
  return error instanceof Error ? error.message : String(error);
}

export interface PreviewDelta {
  oldStart: number;
  newStart: number;
  oldText: string;
  newText: string;
}

export function previewDelta(before: string, after: string, xml = true): PreviewDelta {
  // Generated ARXML often puts many XML elements on one physical line.
  // Break only adjacent tags for display; the complete source stays available below.
  const oldLines = (xml ? before.replaceAll('><', '>\n<') : before).split('\n');
  const newLines = (xml ? after.replaceAll('><', '>\n<') : after).split('\n');
  let first = 0;
  while (first < oldLines.length && first < newLines.length && oldLines[first] === newLines[first])
    first++;
  let tail = 0;
  while (
    tail < oldLines.length - first &&
    tail < newLines.length - first &&
    oldLines[oldLines.length - 1 - tail] === newLines[newLines.length - 1 - tail]
  )
    tail++;
  return {
    oldStart: first + 1,
    newStart: first + 1,
    oldText: oldLines.slice(first, oldLines.length - tail).join('\n'),
    newText: newLines.slice(first, newLines.length - tail).join('\n'),
  };
}
