import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { FolderOpen } from 'lucide-react';
import type { Workbench } from './useWorkbench';

export function SourceEntry({ controller: c, logo }: { controller: Workbench; logo: string }) {
  const { t, text } = useLocale();
  const openReason = c.actionReason('open');
  const createReason = c.actionReason('create');
  const saveAs = c.source === 'save-as';
  return (
    <div className="start-page">
      <header>
        <img src={logo} width="76" height="76" alt="" />
        <h1>{c.workspace ? t('editor.source.heading') : t('editor.source.workbench')}</h1>
        <p>{c.workspace ? t('editor.source.retained') : t('editor.source.intro')}</p>
      </header>
      <div className="start-layout">
        <nav aria-label={t('editor.source.heading')}>
          <button
            type="button"
            className={c.source === 'empty' ? 'selected' : ''}
            onClick={() => c.openProjectEntry('empty')}
          >
            {t('editor.source.new')}
          </button>
          <button
            type="button"
            className={c.source === 'import' ? 'selected' : ''}
            onClick={() => c.openProjectEntry('import')}
          >
            {t('editor.source.import')}
          </button>
          {c.workspace ? (
            <button
              type="button"
              className={saveAs ? 'selected' : ''}
              onClick={() => c.openProjectEntry('save-as')}
            >
              {t('editor.source.saveAs')}
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => void c.openMemberProject()}
            disabled={Boolean(openReason)}
          >
            {t('editor.source.open')}
          </button>
          <button type="button" onClick={c.importHandoff} disabled={Boolean(openReason)}>
            {t('editor.source.importHost')}
          </button>
          <button
            type="button"
            onClick={() => void c.chooseDirectory(c.importHandoffDirectory)}
            disabled={Boolean(openReason)}
          >
            {t('editor.source.importEcu')}
          </button>
        </nav>
        <section>
          {c.source !== 'import' ? (
            <div className="form-fields">
              <label>
                {t('editor.source.name')}
                <input
                  aria-label={t('editor.source.name')}
                  value={c.projectName}
                  onChange={(event) => c.setProjectName(event.target.value)}
                  autoComplete="off"
                />
              </label>
              {!saveAs ? (
                <label>
                  {t('editor.source.template')}
                  <select
                    aria-label={t('editor.source.template')}
                    value={c.templateId}
                    onChange={(event) => c.setTemplateId(event.target.value as typeof c.templateId)}
                  >
                    <option value="can-empty-v1">{t('editor.source.emptyCan')}</option>
                    <option value="can-signals-v1">{t('editor.source.signalsCan')}</option>
                    <option value="standard-ecu-v1">{t('editor.source.standardEcu')}</option>
                  </select>
                </label>
              ) : (
                <p>{t('editor.source.saveAsHelp')}</p>
              )}
              <label>
                {t('editor.source.emptyDirectory')}
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
                    <FolderOpen size={15} />
                    {t('editor.common.select')}
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
            </div>
          ) : (
            <div className="form-fields">
              <button
                type="button"
                onClick={() => void c.chooseFiles()}
                disabled={Boolean(openReason)}
              >
                {t('editor.source.chooseFiles')}
              </button>
              <label>
                {t('editor.source.pathLines')}
                <textarea
                  aria-label={t('editor.source.paths')}
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
                {t('editor.source.enter')}
              </button>
              {openReason ? <p className="field-help">{text(openReason)}</p> : null}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
