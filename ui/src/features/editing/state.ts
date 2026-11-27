import type { ConnectionEdit } from '../../ipc/bindings';
import type { SignalDrafts } from './signalSettings';
import { message } from '../../i18n';
import type { Text } from '../../i18n';
import type { IntegrationInspection, PlanDiagnostic, SavePreview } from '../../ipc/types';
import type { ChangePreview, ChangeSet, ConfigurationChange } from '../../ipc/types';
import type { Rejection } from './batchEditing';

export type EditingState = {
  changes: ConfigurationChange[];
  changePreview: ChangePreview | null;
  preparedChangeSet: ChangeSet | null;
  /** Rejections attached to the grid cell that caused them, by field ID. */
  fieldErrors: Record<string, Text>;
  /** The last rejected change, split into diagnostics for the problems window. */
  rejection: Rejection | null;
  integrationInspection: IntegrationInspection | null;
  integrationIssues: PlanDiagnostic[];
  connectionDrafts: ConnectionEdit[];
  variantSelection: Record<string, string[]> | null;
  signalDrafts: SignalDrafts;
  integrationIds: Record<string, string>;
  integrationPeriod: string;
  integrationUnapplied: boolean;
  integrationPreview: SavePreview | null;
  integrationPreviewPath: string;
  integrationNotice: Text;
  integrationProcessing: boolean;
};

export function initialEditingState(): EditingState {
  return {
    changes: [],
    changePreview: null,
    preparedChangeSet: null,
    fieldErrors: {},
    rejection: null,
    integrationInspection: null,
    integrationIssues: [],
    connectionDrafts: [],
    variantSelection: null,
    signalDrafts: {},
    integrationIds: {},
    integrationPeriod: '',
    integrationUnapplied: false,
    integrationPreview: null,
    integrationPreviewPath: '',
    integrationNotice: message('controller.integration.notInspected'),
    integrationProcessing: false,
  };
}
