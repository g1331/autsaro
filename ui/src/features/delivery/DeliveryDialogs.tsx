import type { useLocale } from '../../i18n';
import type { Workbench } from '../../app/useWorkbench';
import { displayPath } from '../../pathDisplay';
import { CopyText, Dialog, TextSnapshot } from '../../shared/ui/Dialog';
import { fileStatusLabel, ownerLabel, targetLabel } from '../../shared/ui/labels';

export function renderApplicationPreview(
  c: Pick<
    Workbench,
    | 'applicationPreview'
    | 'busy'
    | 'cancelOperation'
    | 'initializeApplicationPreviewed'
    | 'notice'
    | 'setApplicationPreview'
  >,
  locale: ReturnType<typeof useLocale>,
  applicationFile: string,
  setApplicationFile: (path: string) => void,
) {
  const { t } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  if (c.applicationPreview) {
    const preview = c.applicationPreview;
    const selected =
      preview.files.find((file) => file.path === applicationFile) ?? preview.files[0];
    const close = () => c.setApplicationPreview(null);
    return (
      <Dialog
        size="preview"
        {...busy}
        notice={c.notice}
        title={t('workflow.application.title')}
        onClose={close}
        footer={
          <>
            <span>{t('workflow.application.seedCount', { count: preview.files.length })}</span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.initializeApplicationPreviewed()}
            >
              {t('action.confirmInitialize')}
            </button>
          </>
        }
      >
        <p>{t('workflow.application.explanation')}</p>
        {preview.slots.map((slot) => (
          <dl key={slot.producerSlot}>
            <dt>{t('workflow.application.slot')}</dt>
            <dd className="mono path-text">{slot.producerSlot}</dd>
            <dt>{t('workflow.application.component')}</dt>
            <dd className="mono path-text">{slot.componentPath}</dd>
            <dt>{t('workflow.application.sources')}</dt>
            <dd>
              {slot.sourcePaths.map((path) => (
                <p key={path} className="mono path-text">
                  {displayPath(path)}
                </p>
              ))}
            </dd>
            <dt>{t('workflow.application.headers')}</dt>
            <dd>
              {slot.generatedHeaders.map((path) => (
                <p key={path} className="mono path-text">
                  {path}
                </p>
              ))}
            </dd>
            <dt>{t('workflow.application.entries')}</dt>
            <dd className="mono path-text">{slot.entrySymbols.join('\n')}</dd>
          </dl>
        ))}
        <CopyText text={JSON.stringify(preview.slots, null, 2)} label={t('action.copy')} />
        <div className="preview-layout">
          <nav aria-label={t('workflow.application.seedFiles')}>
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className="ghost-button"
                aria-current={selected?.path === file.path ? 'true' : undefined}
                onClick={() => setApplicationFile(file.path)}
              >
                {file.path}
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <h3 className="mono path-text">{selected.path}</h3>
                <TextSnapshot
                  key={selected.path}
                  text={selected.contents}
                  label={t('workflow.application.seedSnapshot')}
                />
              </>
            ) : null}
          </section>
        </div>
        <details>
          <summary>{t('workflow.application.manifestChanges')}</summary>
          <div className="preview-compare">
            <section>
              <h3>{t('workflow.application.before')}</h3>
              <TextSnapshot
                text={preview.manifestBefore}
                label={t('workflow.application.manifestBefore')}
              />
            </section>
            <section>
              <h3>{t('workflow.application.manifestAfter')}</h3>
              <TextSnapshot
                text={preview.manifestAfter}
                label={t('workflow.application.manifestAfter')}
              />
            </section>
          </div>
        </details>
      </Dialog>
    );
  }
  return null;
}

export function renderSavePreview(
  c: Pick<
    Workbench,
    | 'busy'
    | 'cancelOperation'
    | 'cancelSavePreview'
    | 'confirmSave'
    | 'integrationPreview'
    | 'integrationPreviewPath'
    | 'notice'
    | 'previewPath'
    | 'saveIntegration'
    | 'savePreview'
    | 'savingForReplacement'
    | 'setIntegrationPreviewPath'
    | 'setPreviewPath'
  >,
  locale: ReturnType<typeof useLocale>,
) {
  const { t } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  const save = c.savePreview ?? c.integrationPreview;
  if (save) {
    const standard = Boolean(c.integrationPreview);
    const selectedPath = standard ? c.integrationPreviewPath : c.previewPath;
    const selected = save.files.find((file) => file.path === selectedPath) ?? save.files[0];
    const close = () => c.cancelSavePreview();
    return (
      <Dialog
        size="preview"
        {...busy}
        notice={c.notice}
        title={t('workflow.save.title')}
        onClose={close}
        footer={
          <>
            <span>
              {t('workflow.save.changedCount', {
                count: save.files.filter((file) => file.changed).length,
              })}
            </span>
            <button type="button" onClick={close} disabled={Boolean(c.busy)}>
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={standard ? c.saveIntegration : c.confirmSave}
              disabled={
                Boolean(c.busy) ||
                (!save.files.some((file) => file.changed) && !c.savingForReplacement)
              }
            >
              {t('action.confirmSave')}
            </button>
          </>
        }
      >
        <p>{t('workflow.save.explanation')}</p>
        <div className="preview-layout">
          <nav aria-label={t('workflow.save.files')}>
            {save.files.map((file) => (
              <button
                type="button"
                key={file.path}
                className="ghost-button"
                aria-current={selected?.path === file.path ? 'true' : undefined}
                onClick={() =>
                  standard ? c.setIntegrationPreviewPath(file.path) : c.setPreviewPath(file.path)
                }
              >
                {file.path}
                <small>
                  {file.changed ? t('workflow.save.changed') : t('workflow.save.unchanged')}
                </small>
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <p className="mono path-text">{selected.path}</p>
                <div className="preview-compare">
                  <div>
                    <h3>{t('workflow.save.original')}</h3>
                    {selected.before !== null ? (
                      <>
                        <TextSnapshot
                          key={selected.path + ':before'}
                          text={selected.before}
                          label={t('workflow.save.snapshotBefore')}
                        />
                      </>
                    ) : (
                      <p>{t('workflow.save.noOriginal')}</p>
                    )}
                  </div>
                  <div>
                    <h3>{t('workflow.save.proposed')}</h3>
                    {selected.after !== null ? (
                      <>
                        <TextSnapshot
                          key={selected.path + ':after'}
                          text={selected.after}
                          label={t('workflow.save.snapshotAfter')}
                        />
                      </>
                    ) : (
                      <p>{t('workflow.save.noText')}</p>
                    )}
                  </div>
                </div>
              </>
            ) : null}
          </section>
        </div>
      </Dialog>
    );
  }
  return null;
}

export function renderGenerationPreview(
  c: Pick<
    Workbench,
    | 'buildTarget'
    | 'busy'
    | 'cancelOperation'
    | 'generateEcu'
    | 'generationKind'
    | 'generationPreview'
    | 'generationPreviewPath'
    | 'notice'
    | 'setGenerationPreview'
    | 'setGenerationPreviewPath'
  >,
  locale: ReturnType<typeof useLocale>,
) {
  const { t } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  if (c.generationPreview) {
    const preview = c.generationPreview;
    const file =
      preview.files.find((item) => item.path === c.generationPreviewPath) ?? preview.files[0];
    const close = () => c.setGenerationPreview(null);
    return (
      <Dialog
        size="preview"
        {...busy}
        notice={c.notice}
        title={
          c.generationKind === 'handoff'
            ? t('workflow.generation.handoffTitle')
            : t('workflow.generation.title')
        }
        onClose={close}
        footer={
          <>
            <span>
              {t('workflow.generation.writeSummary', {
                count: preview.files.length,
                directory: displayPath(preview.outputDirectory),
                target: targetLabel(c.buildTarget),
              })}
            </span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.generateEcu()}
            >
              {c.generationKind === 'handoff'
                ? t('action.confirmExport')
                : t('action.confirmGenerate')}
            </button>
          </>
        }
      >
        <p className="mono path-text">{displayPath(preview.outputDirectory)}</p>
        <p>{t('workflow.generation.explanation')}</p>
        <div className="preview-layout">
          <nav aria-label={t('workflow.generation.files')}>
            {preview.files.map((item) => (
              <button
                type="button"
                key={item.path}
                className="ghost-button"
                aria-current={file?.path === item.path ? 'true' : undefined}
                onClick={() => c.setGenerationPreviewPath(item.path)}
              >
                {item.path}
                <small>
                  {fileStatusLabel(item.status)} ·{' '}
                  {item.owner ? ownerLabel(item.owner) : t('workflow.generation.ownerUnavailable')}
                </small>
              </button>
            ))}
          </nav>
          <section>
            {file ? (
              <>
                <h3 className="mono path-text">{file.path}</h3>
                <dl>
                  <dt>{t('workflow.generation.owner')}</dt>
                  <dd>
                    {file.owner
                      ? ownerLabel(file.owner)
                      : t('workflow.generation.backendUnavailable')}
                  </dd>
                  <dt>{t('workflow.generation.producer')}</dt>
                  <dd className="mono path-text">
                    {file.producerId ?? t('workflow.generation.backendUnavailable')}
                  </dd>
                  {file.snapshotOf ? (
                    <>
                      <dt>{t('workflow.generation.snapshotSource')}</dt>
                      <dd className="mono path-text">{file.snapshotOf}</dd>
                    </>
                  ) : null}
                </dl>
                <div className="preview-compare">
                  <div>
                    <h3>{t('workflow.generation.before')}</h3>
                    {file.before !== null ? (
                      <>
                        <TextSnapshot
                          key={file.path + ':before'}
                          text={file.before}
                          label={t('workflow.generation.snapshotBefore')}
                        />
                      </>
                    ) : (
                      <p>{t('workflow.generation.newOrBinary')}</p>
                    )}
                  </div>
                  <div>
                    <h3>{t('workflow.generation.after')}</h3>
                    {file.after !== null ? (
                      <>
                        <TextSnapshot
                          key={file.path + ':after'}
                          text={file.after}
                          label={t('workflow.generation.snapshotAfter')}
                        />
                      </>
                    ) : (
                      <p>
                        {file.status === 'removed'
                          ? t('workflow.generation.removed')
                          : t('workflow.generation.binary')}
                      </p>
                    )}
                  </div>
                </div>
              </>
            ) : null}
          </section>
        </div>
      </Dialog>
    );
  }
  return null;
}

export function renderHandoffImport(
  c: Pick<
    Workbench,
    | 'busy'
    | 'cancelOperation'
    | 'chooseDirectory'
    | 'confirmHandoffImport'
    | 'ecuImportDirectory'
    | 'handoffImportDestination'
    | 'handoffImportOpen'
    | 'notice'
    | 'setEcuImportDirectory'
    | 'setHandoffImportDestination'
    | 'setHandoffImportOpen'
  >,
  locale: ReturnType<typeof useLocale>,
) {
  const { t } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  if (c.handoffImportOpen) {
    const close = () => c.setHandoffImportOpen(false);
    return (
      <Dialog
        {...busy}
        notice={c.notice}
        title={t('workflow.handoff.title')}
        onClose={close}
        footer={
          <>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy) || !c.ecuImportDirectory || !c.handoffImportDestination}
              onClick={() => void c.confirmHandoffImport()}
            >
              {t('action.import')}
            </button>
          </>
        }
      >
        <p>{t('workflow.handoff.explanation')}</p>
        <div className="form-fields">
          <label>
            {t('workflow.handoff.directory')}
            <div className="path-picker">
              <input
                aria-label={t('workflow.handoff.directory')}
                value={c.ecuImportDirectory}
                readOnly
              />
              <button
                type="button"
                disabled={Boolean(c.busy)}
                onClick={() => void c.chooseDirectory(c.setEcuImportDirectory)}
              >
                {t('action.browse')}
              </button>
            </div>
          </label>
          <label>
            {t('editor.source.directory')}
            <div className="path-picker">
              <input
                aria-label={t('editor.source.directory')}
                value={c.handoffImportDestination}
                readOnly
              />
              <button
                type="button"
                disabled={Boolean(c.busy)}
                onClick={() => void c.chooseDirectory(c.setHandoffImportDestination)}
              >
                {t('action.browse')}
              </button>
            </div>
          </label>
        </div>
        <p className="field-help">{t('workflow.handoff.sourceHelp')}</p>
      </Dialog>
    );
  }
  return null;
}
