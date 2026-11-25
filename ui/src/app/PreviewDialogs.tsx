import { useState } from 'react';
import { useLocale } from '../i18n';
import type { Workbench } from './useWorkbench';
import {
  renderDraftGuard,
  renderCatalogConfirmation,
  renderProjectPreview,
} from '../features/project/ProjectDialogs';
import { renderChangePreview } from '../features/editing/ChangePreviewDialog';
import {
  renderApplicationPreview,
  renderSavePreview,
  renderGenerationPreview,
  renderHandoffImport,
} from '../features/delivery/DeliveryDialogs';

export const previewDialogsFields = [
  'applicationPreview',
  'applyChanges',
  'buildTarget',
  'busy',
  'cancelChangePreview',
  'cancelOperation',
  'cancelSavePreview',
  'catalogConfirmation',
  'changePreview',
  'changes',
  'chooseDirectory',
  'confirmHandoffImport',
  'confirmProjectPreview',
  'confirmSave',
  'ecuImportDirectory',
  'generateEcu',
  'generationKind',
  'generationPreview',
  'generationPreviewPath',
  'guard',
  'handoffImportDestination',
  'handoffImportOpen',
  'initializeApplicationPreviewed',
  'integrationPreview',
  'integrationPreviewPath',
  'integrationUnapplied',
  'notice',
  'preparedChangeSet',
  'previewPath',
  'projectPreview',
  'projectPreviewKind',
  'projection',
  'resolveCatalogConfirmation',
  'resolveGuard',
  'saveIntegration',
  'savePreview',
  'savingForReplacement',
  'setApplicationPreview',
  'setEcuImportDirectory',
  'setGenerationPreview',
  'setGenerationPreviewPath',
  'setHandoffImportDestination',
  'setHandoffImportOpen',
  'setIntegrationPreviewPath',
  'setPreviewPath',
  'setProjectPreview',
  'workspace',
] as const satisfies readonly (keyof Workbench)[];

export function PreviewDialogs({
  controller: c,
}: {
  controller: Pick<Workbench, (typeof previewDialogsFields)[number]>;
}) {
  const locale = useLocale();
  const [projectFile, setProjectFile] = useState('');
  const [applicationFile, setApplicationFile] = useState('');
  return (
    renderDraftGuard(c, locale) ??
    renderCatalogConfirmation(c, locale) ??
    renderChangePreview(c, locale) ??
    renderApplicationPreview(c, locale, applicationFile, setApplicationFile) ??
    renderProjectPreview(c, locale, projectFile, setProjectFile) ??
    renderSavePreview(c, locale) ??
    renderGenerationPreview(c, locale) ??
    renderHandoffImport(c, locale)
  );
}
