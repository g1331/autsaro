import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { FolderOpen } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import { NoticeBanner } from './Dialog';
import { iconSize } from './icons';

export function SourceEntry({ controller: c, logo }: { controller: Workbench; logo: string }) {
  const { t, text } = useLocale();
  const openReason = c.actionReason('open');
  const createReason = c.actionReason('create');
  const saveAs = c.source === 'save-as';
  // The start page is in front: its own operations report here, next to the button.
  const notice = c.notice ? (
    <NoticeBanner notice={c.notice} onDismiss={() => c.setNotice(null)} />
  ) : null;
  return (
    <div className="start-page">
      <header>
        <img src={logo} width="76" height="76" alt="" />
        <h1>{c.workspace ? t('editor.source.heading') : t('controller.app.title')}</h1>
        <p>{c.workspace ? t('editor.source.retained') : t('editor.source.intro')}</p>
      </header>
      <div className="start-layout">
        <nav aria-label={t('editor.source.heading')}>
          <button
            type="button"
            className="ghost-button"
            aria-current={c.source === 'empty' ? 'true' : undefined}
            onClick={() => c.openProjectEntry('empty')}
          >
            {t('action.newProject')}
          </button>
          <button
            type="button"
            className="ghost-button"
            aria-current={c.source === 'import' ? 'true' : undefined}
            onClick={() => c.openProjectEntry('import')}
          >
            {t('action.importArxml')}
          </button>
          {c.workspace ? (
            <button
              type="button"
              className="ghost-button"
              aria-current={saveAs ? 'true' : undefined}
              onClick={() => c.openProjectEntry('save-as')}
            >
              {t('action.saveAs')}
            </button>
          ) : null}
          <button
            type="button"
            className="ghost-button"
            onClick={() => void c.openMemberProject()}
            disabled={Boolean(openReason)}
          >
            {t('action.openProject')}
          </button>
          <button
            type="button"
            className="ghost-button"
            onClick={c.importHandoff}
            disabled={Boolean(openReason)}
          >
            {t('action.importHandoff')}
          </button>
        </nav>
        <section>
          {c.source !== 'import' ? (
            <div className="form-fields">
              {saveAs ? (
                <p>{t('editor.source.saveAsHelp', { name: c.workspace?.name ?? '' })}</p>
              ) : (
                <label>
                  {t('editor.source.name')}
                  <input
                    value={c.projectName}
                    onChange={(event) => c.setProjectName(event.target.value)}
                    autoComplete="off"
                  />
                </label>
              )}
              <label>
                {t('editor.source.directory')}
                <div className="path-picker">
                  <input
                    value={displayPath(c.projectDirectory)}
                    readOnly
                    aria-label={t('editor.source.directory')}
                  />
                  <button
                    type="button"
                    onClick={() => void c.chooseDirectory(c.setProjectDirectory)}
                    disabled={!c.native || Boolean(c.busy)}
                  >
                    <FolderOpen size={iconSize.control} aria-hidden="true" />
                    {t('action.browse')}
                  </button>
                </div>
              </label>
              <button
                className="primary-button"
                type="button"
                onClick={() => (saveAs ? void c.previewSaveAs() : c.createProject())}
                disabled={Boolean(saveAs ? c.actionReason('save') : createReason)}
              >
                {saveAs ? t('editor.source.previewSaveAs') : t('editor.source.previewFiles')}
              </button>
              {saveAs ? (
                c.actionReason('save') ? (
                  <p className="field-help">{text(c.actionReason('save'))}</p>
                ) : null
              ) : createReason ? (
                <p className="field-help">{text(createReason)}</p>
              ) : null}
              {notice}
            </div>
          ) : (
            <div className="form-fields">
              <button
                type="button"
                onClick={() => void c.chooseFiles()}
                disabled={Boolean(openReason)}
              >
                {t('action.chooseArxml')}
              </button>
              <label>
                {t('editor.source.pathLines')}
                <textarea
                  value={c.importPathText
                    .split(/(\r?\n)/)
                    .map(displayPath)
                    .join('')}
                  rows={7}
                  onChange={(event) => {
                    c.setImportPathText(event.target.value);
                    c.setImportPaths(
                      event.target.value
                        .split(/\r?\n/)
                        .map((path) => path.trim())
                        .filter(Boolean),
                    );
                  }}
                />
              </label>
              <p>{t('editor.source.inputCount', { count: c.importPaths.length })}</p>
              <button
                type="button"
                className="primary-button"
                disabled={Boolean(openReason) || !c.importPaths.length}
                onClick={c.importProject}
              >
                {t('action.import')}
              </button>
              {openReason ? <p className="field-help">{text(openReason)}</p> : null}
              {notice}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
