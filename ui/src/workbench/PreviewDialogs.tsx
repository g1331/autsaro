import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { useState } from 'react';
import type { Workbench } from './useWorkbench';
import { CopyText, Dialog, TextSnapshot } from './Dialog';

export function PreviewDialogs({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const [projectFile, setProjectFile] = useState('');
  const [applicationFile, setApplicationFile] = useState('');
  if (c.guard) {
    const project = c.guard.kind === 'project';
    return (
      <Dialog
        title={text(c.guard.title)}
        onClose={() => void c.resolveGuard('cancel')}
        footer={
          <>
            <button type="button" onClick={() => void c.resolveGuard('cancel')}>
              {project ? t('workflow.guard.cancelReplacement') : t('workflow.guard.stay')}
            </button>
            <button type="button" onClick={() => void c.resolveGuard('discard')}>
              {project ? t('workflow.guard.discardContinue') : t('workflow.guard.discardDrafts')}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => void c.resolveGuard('apply')}
            >
              {project ? t('workflow.guard.saveFirst') : t('workflow.guard.applyContinue')}
            </button>
          </>
        }
      >
        <p>
          {project
            ? t('workflow.guard.projectExplanation')
            : t('workflow.guard.contextExplanation')}
        </p>
        <p>
          {t('workflow.guard.state', {
            unsaved: t(
              c.workspace?.dirty || c.projection?.dirty
                ? 'workflow.common.yes'
                : 'workflow.common.no',
            ),
            unapplied: t(c.unapplied || c.creating ? 'workflow.common.yes' : 'workflow.common.no'),
          })}
        </p>
      </Dialog>
    );
  }
  if (c.changePreview)
    return (
      <Dialog
        title={t('workflow.batch.title')}
        onClose={() => {
          if (!c.busy) c.cancelChangePreview();
        }}
        footer={
          <>
            <span>{t('workflow.batch.count', { count: c.changePreview.impacts.length })}</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.cancelChangePreview()}
            >
              {t('workflow.batch.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy) || !c.preparedChangeSet}
              onClick={() => void c.applyChanges()}
            >
              {t('workflow.batch.apply')}
            </button>
          </>
        }
      >
        <p className="mono path-text">
          {t('workflow.batch.inputIdentity', { fingerprint: c.changePreview.inputFingerprint })}
          <br />
          {t('workflow.batch.definitionIdentity', {
            fingerprint: c.changePreview.definitionFingerprint,
          })}
          <br />
          {t('workflow.batch.confirmationIdentity', { revision: c.changePreview.changeRevision })}
        </p>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('workflow.batch.objectChangeId')}</th>
                <th>{t('workflow.batch.before')}</th>
                <th>{t('workflow.batch.after')}</th>
                <th>{t('workflow.batch.impact')}</th>
              </tr>
            </thead>
            <tbody>
              {c.changePreview.impacts.map((impact, index) => (
                <tr key={`${impact.changeId}-${index}`}>
                  <td className="mono path-text">
                    {impact.path}
                    <small>{impact.changeId}</small>
                  </td>
                  <td className="mono">{impact.before}</td>
                  <td className="mono">{impact.after}</td>
                  <td>
                    {impact.incoming ? t('workflow.batch.incoming') : t('workflow.batch.explicit')}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {c.changePreview.diagnostics.map((issue, index) => (
          <p
            key={`${issue.code}-${index}`}
            className={issue.severity === 'error' ? 'error-text' : 'field-help'}
          >
            {issue.scope} · {issue.code} · {text(issue.message)} · {text(issue.remedy)}
          </p>
        ))}
      </Dialog>
    );
  if (c.applicationPreview) {
    const preview = c.applicationPreview;
    const selected =
      preview.files.find((file) => file.path === applicationFile) ?? preview.files[0];
    const close = () => {
      if (!c.busy) c.setApplicationPreview(null);
    };
    return (
      <Dialog
        title={t('workflow.application.title')}
        onClose={close}
        footer={
          <>
            <span>{t('workflow.application.seedCount', { count: preview.files.length })}</span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('workflow.application.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.initializeApplicationPreviewed()}
            >
              {t('workflow.application.confirm')}
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
        <CopyText
          text={JSON.stringify(preview.slots, null, 2)}
          label={t('workflow.application.copySlot')}
        />
        <div className="preview-layout">
          <nav aria-label={t('workflow.application.seedFiles')}>
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className={selected?.path === file.path ? 'selected' : ''}
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
                <CopyText text={selected.contents} label={t('workflow.application.copySeed')} />
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
              <CopyText
                text={preview.manifestBefore}
                label={t('workflow.application.copyManifestBefore')}
              />
              <TextSnapshot
                text={preview.manifestBefore}
                label={t('workflow.application.manifestBefore')}
              />
            </section>
            <section>
              <h3>{t('workflow.application.manifestAfter')}</h3>
              <CopyText
                text={preview.manifestAfter}
                label={t('workflow.application.copyManifestAfter')}
              />
              <TextSnapshot
                text={preview.manifestAfter}
                label={t('workflow.application.manifestAfter')}
              />
            </section>
          </div>
        </details>
        {c.notice?.tone === 'error' ? (
          <>
            <p role="alert" className="error-text">
              {text(c.notice.text).slice(0, 2000)}
            </p>
            <CopyText text={text(c.notice.text)} label={t('workflow.application.copyError')} />
          </>
        ) : null}
      </Dialog>
    );
  }
  if (c.projectPreview) {
    const preview = c.projectPreview;
    const selected = preview.files.find((file) => file.path === projectFile) ?? preview.files[0];
    return (
      <Dialog
        title={
          c.projectPreviewKind === 'create'
            ? t('workflow.project.createTitle')
            : t('workflow.project.saveAsTitle')
        }
        onClose={() => {
          if (!c.busy) c.setProjectPreview(null);
        }}
        footer={
          <>
            <span>{t('workflow.project.fileCount', { count: preview.files.length })}</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.setProjectPreview(null)}
            >
              {t('workflow.common.cancel')}
            </button>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              className="primary-button"
              onClick={() => void c.confirmProjectPreview()}
            >
              {c.projectPreviewKind === 'create'
                ? t('workflow.project.confirmCreate')
                : t('workflow.project.confirmSaveAs')}
            </button>
          </>
        }
      >
        <p className="mono path-text">
          {displayPath(preview.directory)} · {preview.name} · {preview.templateId}
        </p>
        <p>{t('workflow.project.previewExplanation')}</p>
        <div className="preview-layout">
          <nav aria-label={t('workflow.project.files')}>
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className={selected?.path === file.path ? 'selected' : ''}
                onClick={() => setProjectFile(file.path)}
              >
                {file.path}
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <h3>{selected.path}</h3>
                <CopyText text={selected.contents} label={t('workflow.project.copyFile')} />
                <TextSnapshot
                  key={selected.path}
                  text={selected.contents}
                  label={t('workflow.project.snapshot')}
                />
              </>
            ) : null}
          </section>
        </div>
        <details>
          <summary>
            {t('workflow.project.extensionCount', {
              count: preview.acceptedExtensionDefinitions.length,
            })}
          </summary>
          <pre>{JSON.stringify(preview.acceptedExtensionDefinitions, null, 2)}</pre>
        </details>
      </Dialog>
    );
  }
  const save = c.savePreview ?? c.integrationPreview;
  if (save) {
    const standard = Boolean(c.integrationPreview);
    const selectedPath = standard ? c.integrationPreviewPath : c.previewPath;
    const selected = save.files.find((file) => file.path === selectedPath) ?? save.files[0];
    const close = () => c.cancelSavePreview();
    return (
      <Dialog
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
              {t('workflow.common.cancel')}
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
              {t('workflow.save.confirm')}
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
                className={selected?.path === file.path ? 'selected' : ''}
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
                        <CopyText text={selected.before} label={t('workflow.save.copyOriginal')} />
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
                        <CopyText text={selected.after} label={t('workflow.save.copyProposed')} />
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
  if (c.generationPreview) {
    const preview = c.generationPreview;
    const file =
      preview.files.find((item) => item.path === c.generationPreviewPath) ?? preview.files[0];
    const close = () => {
      if (!c.busy) c.setGenerationPreview(null);
    };
    return (
      <Dialog
        title={
          c.generationKind === 'handoff'
            ? t('workflow.generation.handoffTitle')
            : t('workflow.generation.title')
        }
        onClose={close}
        footer={
          <>
            <span>
              {t('workflow.generation.fileCount', {
                count: preview.files.length,
                target: c.legacyTarget,
              })}
            </span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('workflow.common.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() =>
                c.workspace?.integrationCandidate ? void c.generateEcu() : c.confirmGenerate()
              }
            >
              {c.generationKind === 'handoff'
                ? t('workflow.generation.confirmExport')
                : t('workflow.generation.confirm')}
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
                className={file?.path === item.path ? 'selected' : ''}
                onClick={() => c.setGenerationPreviewPath(item.path)}
              >
                {item.path}
                <small>
                  {item.status} · {item.owner ?? t('workflow.generation.ownerUnavailable')}
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
                  <dd>{file.owner ?? t('workflow.generation.backendUnavailable')}</dd>
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
                        <CopyText text={file.before} label={t('workflow.generation.copyBefore')} />
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
                        <CopyText text={file.after} label={t('workflow.generation.copyAfter')} />
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
  if (c.handoffImportOpen) {
    const close = () => {
      if (!c.busy) c.setHandoffImportOpen(false);
    };
    const v2 = c.handoffImportMode === 'v2';
    return (
      <Dialog
        title={t('workflow.handoff.title')}
        onClose={close}
        footer={
          <>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              {t('workflow.handoff.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={
                Boolean(c.busy) || !c.ecuImportDirectory || (v2 && !c.handoffImportDestination)
              }
              onClick={() => void c.confirmHandoffImport()}
            >
              {t('workflow.handoff.confirm')}
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
                {t('workflow.handoff.choosePackage')}
              </button>
            </div>
          </label>
          <label>
            {t('workflow.handoff.sourceMode')}
            <select
              aria-label={t('workflow.handoff.sourceModeLabel')}
              value={c.handoffImportMode}
              disabled={Boolean(c.busy)}
              onChange={(event) => c.setHandoffImportMode(event.target.value as 'v2' | 'legacy')}
            >
              <option value="v2">{t('workflow.handoff.v2')}</option>
              <option value="legacy">{t('workflow.handoff.legacy')}</option>
            </select>
          </label>
          {v2 ? (
            <label>
              {t('workflow.handoff.destination')}
              <div className="path-picker">
                <input
                  aria-label={t('workflow.handoff.destination')}
                  value={c.handoffImportDestination}
                  readOnly
                />
                <button
                  type="button"
                  disabled={Boolean(c.busy)}
                  onClick={() => void c.chooseDirectory(c.setHandoffImportDestination)}
                >
                  {t('workflow.handoff.chooseDestination')}
                </button>
              </div>
            </label>
          ) : null}
        </div>
        <p className="field-help">{t('workflow.handoff.sourceHelp')}</p>
        {c.notice?.tone === 'error' ? (
          <>
            <p role="alert" className="error-text">
              {text(c.notice.text).slice(0, 2000)}
            </p>
            <CopyText text={text(c.notice.text)} label={t('workflow.handoff.copyError')} />
          </>
        ) : null}
      </Dialog>
    );
  }
  return null;
}
