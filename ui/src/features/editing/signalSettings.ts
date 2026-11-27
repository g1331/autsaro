import type { SignalSetting, IntegrationEdit } from '../../ipc/bindings';
import type { EditingState } from './state';
import { intInRange } from '../../shared/ui/forms';
import { message } from '../../i18n';

export type SignalDrafts = Record<string, Partial<Record<SignalSetting['field'], string>>>;

export function integrationPayload(state: EditingState): IntegrationEdit {
  if (state.variantSelection)
    return { variantSelection: state.variantSelection, applicationPeriodMs: null };
  const settings: Record<string, SignalSetting[]> = {};
  const ids: Record<string, number> = {};
  for (const [port, input] of Object.entries(state.integrationIds)) {
    const signal = state.integrationInspection?.description?.signals.find(
      (item) => item.port === port,
    );
    const draft = state.signalDrafts[port] ?? {};
    const extended = draft.extended === undefined ? signal?.extended : draft.extended === 'true';
    ids[port] = intInRange(input, `${port} · CAN ID`, 0, extended ? 0x1fffffff : 0x7ff);
    const changes: SignalSetting[] = [];
    for (const field of ['startBit', 'bitLength', 'pduLength', 'frameLength'] as const) {
      const raw = draft[field];
      if (raw !== undefined)
        changes.push({
          field,
          value: intInRange(raw, `${port} · ${field}`, 0, field === 'startBit' ? 65535 : 64),
        });
    }
    for (const field of ['extended', 'fd', 'dual', 'bigEndian'] as const) {
      if (draft[field] !== undefined) changes.push({ field, value: draft[field] === 'true' });
    }
    if (changes.length) settings[port] = changes;
  }
  return {
    canIds: ids,
    ...(state.connectionDrafts.length ? { connections: state.connectionDrafts } : {}),
    ...(Object.keys(settings).length ? { signalSettings: settings } : {}),
    applicationPeriodMs: intInRange(
      state.integrationPeriod,
      message('controller.field.applicationPeriod'),
      1,
      2147483647,
    ),
  };
}
