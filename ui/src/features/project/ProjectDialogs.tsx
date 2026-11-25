import type { useLocale } from '../../i18n';
import type { Workbench } from '../../app/useWorkbench';
import { displayPath } from '../../pathDisplay';
import { Dialog, TextSnapshot } from '../../shared/ui/Dialog';

export function renderDraftGuard(
  c: Pick<
    Workbench,
    | 'busy'
    | 'cancelOperation'
    | 'changes'
    | 'guard'
    | 'integrationUnapplied'
    | 'notice'
    | 'projection'
    | 'resolveGuard'
    | 'workspace'
  >,
  locale: ReturnType<typeof useLocale>,
) {
  const { t, text } = locale;
  if (c.guard) {
    const project = c.guard.kind === 'project';
    const unappliedCount = c.changes.length + (c.integrationUnapplied ? 1 : 0);
    return (
      <Dialog
        alert
        size="confirm"
        notice={c.notice}
        busy={c.busy}
        onCancelBusy={c.cancelOperation}
        title={text(c.guard.title)}
        onClose={() => void c.resolveGuard('cancel')}
        footer={
          <>
            <button
              type="button"
              data-initial-focus
              disabled={Boolean(c.busy)}
              onClick={() => void c.resolveGuard('cancel')}
            >
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="danger-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.resolveGuard('discard')}
            >
              {project ? t('action.discardContinue') : t('action.discardContinue')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.resolveGuard('apply')}
            >
              {project ? t('action.saveContinue') : t('action.applyContinue')}
            </button>
          </>
        }
      >
        <p>
          {project
            ? t('workflow.guard.projectExplanation')
            : t('workflow.guard.contextExplanation')}
        </p>
        {unappliedCount ? <p>{t('workflow.guard.unapplied', { count: unappliedCount })}</p> : null}
        {c.workspace?.dirty || c.projection?.dirty ? <p>{t('workflow.guard.unsaved')}</p> : null}
      </Dialog>
    );
  }
  return null;
}

export function renderCatalogConfirmation(
  c: Pick<Workbench, 'catalogConfirmation' | 'resolveCatalogConfirmation'>,
  locale: ReturnType<typeof useLocale>,
) {
  const { t } = locale;
  if (c.catalogConfirmation) {
    const { kind, subject } = c.catalogConfirmation;
    const remove = kind === 'remove';
    return (
      <Dialog
        alert
        size="confirm"
        title={t(remove ? 'activity.removeDefinitions' : 'activity.acceptDefinitions')}
        onClose={() => c.resolveCatalogConfirmation(false)}
        footer={
          <>
            <button
              type="button"
              data-initial-focus
              onClick={() => c.resolveCatalogConfirmation(false)}
            >
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className={remove ? 'danger-button' : 'primary-button'}
              onClick={() => c.resolveCatalogConfirmation(true)}
            >
              {t(remove ? 'action.removeDefinitions' : 'action.confirmAccept')}
            </button>
          </>
        }
      >
        {remove ? (
          <p>{t('workflow.confirm.removeCatalog', { catalogId: subject })}</p>
        ) : (
          <>
            <p>{t('workflow.confirm.acceptCatalog')}</p>
            <p className="mono path-text">{displayPath(subject)}</p>
          </>
        )}
      </Dialog>
    );
  }
  return null;
}

export function renderProjectPreview(
  c: Pick<
    Workbench,
    | 'busy'
    | 'cancelOperation'
    | 'confirmProjectPreview'
    | 'notice'
    | 'projectPreview'
    | 'projectPreviewKind'
    | 'setProjectPreview'
  >,
  locale: ReturnType<typeof useLocale>,
  projectFile: string,
  setProjectFile: (path: string) => void,
) {
  const { t } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  if (c.projectPreview) {
    const preview = c.projectPreview;
    const selected = preview.files.find((file) => file.path === projectFile) ?? preview.files[0];
    return (
      <Dialog
        size="preview"
        {...busy}
        notice={c.notice}
        title={
          c.projectPreviewKind === 'create'
            ? t('workflow.project.createTitle')
            : t('workflow.project.saveAsTitle')
        }
        onClose={() => c.setProjectPreview(null)}
        footer={
          <>
            <span>{t('workflow.project.fileCount', { count: preview.files.length })}</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.setProjectPreview(null)}
            >
              {t('action.cancel')}
            </button>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              className="primary-button"
              onClick={() => void c.confirmProjectPreview()}
            >
              {c.projectPreviewKind === 'create'
                ? t('action.confirmCreate')
                : t('action.confirmSave')}
            </button>
          </>
        }
      >
        <p className="mono path-text">
          {preview.name} · {displayPath(preview.directory)}
        </p>
        <p>{t('workflow.project.previewExplanation')}</p>
        <div className="preview-layout">
          <nav aria-label={t('workflow.project.files')}>
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className="ghost-button"
                aria-current={selected?.path === file.path ? 'true' : undefined}
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
  return null;
}
