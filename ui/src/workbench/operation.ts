export interface WorkbenchOperation {
  kind: 'action' | 'handoffImport' | 'integrationEdit' | 'savePreview';
  label: string;
}
