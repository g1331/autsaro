import type { IntegrationInspection } from '../../ipc/types';

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
