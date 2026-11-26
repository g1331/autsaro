import type { Text } from '../i18n';

export interface WorkbenchOperation {
  kind: 'action' | 'handoffImport' | 'integrationEdit' | 'savePreview';
  label: Text;
}
