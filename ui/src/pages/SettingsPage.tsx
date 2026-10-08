import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { useState } from 'react';
import type { ExecutionTools } from '../types';
import type { Appearance } from '../workbench/projectTypes';
import type { Workbench } from '../workbench/useWorkbench';
import { CopyText, Dialog } from '../workbench/Dialog';

const toolLabels: { key: keyof ExecutionTools; label: string }[] = [
  { key: 'compiler', label: 'shell.settings.compiler' },
  { key: 'objdump', label: 'shell.settings.objdump' },
  { key: 'git', label: 'shell.settings.git' },
  { key: 'python', label: 'shell.settings.python' },
];

export function SettingsPage({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const [category, setCategory] = useState<'appearance' | 'definitions' | 'tools'>('appearance');
  if (
    !c.settingsOpen ||
    c.guard ||
    c.changePreview ||
    c.savePreview ||
    c.integrationPreview ||
    c.projectPreview ||
    c.generationPreview ||
    c.applicationPreview ||
    c.handoffImportOpen
  )
    return null;
  const locked = Boolean(c.busy) || c.languageSaving || !c.native;
  function close() {
    if (c.busy || c.languageSaving) return;
    c.setAppearanceDraft(c.capabilities?.appearance ?? 'system');
    c.setLanguageDraft(c.savedLanguage);
    c.setResourceDraft({
      xsdArchive: c.capabilities?.xsdArchive ?? '',
      modArchive: c.capabilities?.modArchive ?? '',
    });
    c.setToolDraft(
      c.capabilities?.configuredExecutionTools ?? {
        compiler: '',
        objdump: '',
        git: '',
        python: '',
      },
    );
    c.setSettingsNotice('');
    c.setSettingsOpen(false);
  }
  const identity = c.projection?.ruleSetIdentity ?? c.capabilities?.ruleSetIdentity;
  return (
    <Dialog
      title={t('shell.settings.title')}
      onClose={close}
      footer={
        <>
          <span role="status">{text(c.settingsNotice)}</span>
          <button type="button" onClick={close} disabled={Boolean(c.busy) || c.languageSaving}>
            {t('shell.settings.close')}
          </button>
          {category === 'appearance' ? (
            <>
              <button
                className="primary-button"
                type="button"
                disabled={locked}
                onClick={() => void c.configureAppearance()}
              >
                {t('shell.settings.saveAppearance')}
              </button>
              <button
                className="primary-button"
                type="button"
                disabled={Boolean(c.busy) || c.languageSaving}
                aria-busy={c.languageSaving}
                onClick={() => void c.configureLanguage()}
              >
                {t('shell.settings.saveLanguage')}
              </button>
            </>
          ) : category === 'tools' ? (
            <button
              className="primary-button"
              type="button"
              disabled={locked}
              onClick={() => void c.configureTools()}
            >
              {t('shell.settings.saveTools')}
            </button>
          ) : null}
        </>
      }
    >
      <div className="settings-layout">
        <nav aria-label={t('shell.settings.categories')}>
          {(
            [
              ['appearance', 'shell.settings.appearance'],
              ['definitions', 'shell.settings.definitions'],
              ['tools', 'shell.settings.tools'],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              aria-current={category === key ? 'page' : undefined}
              className={category === key ? 'selected' : ''}
              onClick={() => setCategory(key)}
            >
              {t(label)}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          <section hidden={category !== 'appearance'}>
            <h3>{t('shell.settings.appearance')}</h3>
            <p>{t('shell.settings.previewHelp')}</p>
            <label>
              {t('shell.settings.theme')}
              <select
                aria-label={t('shell.settings.theme')}
                value={c.appearanceDraft}
                onChange={(event) => c.setAppearanceDraft(event.target.value as Appearance)}
              >
                <option value="light">{t('shell.settings.light')}</option>
                <option value="dark">{t('shell.settings.dark')}</option>
                <option value="system">{t('shell.settings.system')}</option>
              </select>
            </label>
            <label>
              {t('shell.settings.language')}
              <select
                name="language"
                aria-label={t('shell.settings.language')}
                value={c.languageDraft}
                disabled={Boolean(c.busy) || c.languageSaving}
                onChange={(event) =>
                  c.setLanguageDraft(event.target.value as typeof c.languageDraft)
                }
              >
                <option value="system">{t('shell.settings.system')}</option>
                <option value="zh-CN">{t('shell.settings.chinese')}</option>
                <option value="en">{t('shell.settings.english')}</option>
              </select>
            </label>
          </section>
          <section hidden={category !== 'definitions'}>
            <h3>{t('shell.settings.rules')}</h3>
            {c.capabilities?.ruleError ? (
              <p className="notice error" role="alert">
                {text(c.capabilities.ruleError)}
                {t('shell.settings.ruleRemedy')}
              </p>
            ) : null}
            {identity ? (
              <dl className="property-list">
                <div>
                  <dt>{t('shell.settings.release')}</dt>
                  <dd>{identity.release}</dd>
                </div>
                <div>
                  <dt>{t('shell.settings.ruleVersion')}</dt>
                  <dd>{identity.rulesVersion}</dd>
                </div>
                <div>
                  <dt>{t('shell.settings.sha')}</dt>
                  <dd className="mono path-text">
                    {identity.sha256}
                    <CopyText text={identity.sha256} />
                  </dd>
                </div>
              </dl>
            ) : (
              <p>{t('shell.settings.noIdentity')}</p>
            )}
            <details>
              <summary>{t('shell.settings.coverage')}</summary>
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>{t('shell.settings.scope')}</th>
                      <th>{t('shell.settings.subject')}</th>
                      <th>{t('shell.settings.coverageColumn')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(c.capabilities?.ruleCoverage ?? []).map((coverage) => (
                      <tr key={coverage.ruleId}>
                        <td>
                          {coverage.scope}
                          <br />
                          {coverage.ruleId}
                        </td>
                        <td>{coverage.subjects.join(t('shell.listSeparator'))}</td>
                        <td>
                          {coverage.supported
                            ? t('shell.settings.supported')
                            : t('shell.settings.unsupported')}
                          <small>{text(coverage.reason)}</small>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </details>
            <h3>{t('shell.settings.extensions')}</h3>
            <p>{t('shell.settings.extensionHelp')}</p>
            <button
              type="button"
              disabled={locked || !c.workspace}
              onClick={() => void c.importDefinitionCatalog()}
            >
              {t('shell.settings.acceptCatalog')}
            </button>
            {(c.projection?.acceptedExtensionDefinitions ?? []).map((identity) => (
              <section className="catalog-row" key={identity.catalogId}>
                <strong>{identity.catalogId}</strong>
                <span>
                  {identity.release} · {identity.version}
                </span>
                <p className="mono path-text">{identity.sha256}</p>
                <CopyText
                  text={JSON.stringify(identity, null, 2)}
                  label={t('shell.settings.copyIdentity')}
                />
                <button
                  type="button"
                  disabled={locked}
                  onClick={() => void c.removeDefinitionCatalog(identity.catalogId)}
                >
                  {t('shell.settings.remove')}
                </button>
              </section>
            ))}
            {c.projection?.extensionDefinitions.map((extension) => (
              <details key={extension.identity.catalogId}>
                <summary>
                  {extension.identity.catalogId} ·{' '}
                  {extension.available
                    ? t('shell.settings.available')
                    : t('shell.settings.unavailable')}
                </summary>
                <p className="mono path-text">
                  {t('shell.settings.source', {
                    source: extension.source ?? t('shell.settings.noSource'),
                  })}
                </p>
                <p>{text(extension.reason)}</p>
                <h3>{t('shell.settings.consumers')}</h3>
                {extension.consumers.length ? (
                  <ul>
                    {extension.consumers.map((consumer) => (
                      <li key={consumer} className="mono path-text">
                        {consumer}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p>{t('shell.settings.noConsumers')}</p>
                )}
                <CopyText
                  text={JSON.stringify(extension, null, 2)}
                  label={t('shell.settings.copyConsumers')}
                />
              </details>
            ))}
            {c.projection?.dirty ? (
              <p className="warning-text">{t('shell.settings.unsaved')}</p>
            ) : null}
            <details className="legacy-settings">
              <summary>{t('shell.settings.legacy')}</summary>
              <p>{t('shell.settings.legacyHelp')}</p>
              {c.capabilities?.resourceError ? (
                <p className="error-text">{text(c.capabilities.resourceError)}</p>
              ) : null}
              <div className="form-fields">
                <label>
                  {t('shell.settings.xsd')}
                  <input
                    aria-label={t('shell.settings.xsdPath')}
                    value={displayPath(c.resourceDraft.xsdArchive)}
                    disabled={locked}
                    onChange={(event) =>
                      c.setResourceDraft({ ...c.resourceDraft, xsdArchive: event.target.value })
                    }
                  />
                </label>
                <label>
                  {t('shell.settings.mod')}
                  <input
                    aria-label={t('shell.settings.modPath')}
                    value={displayPath(c.resourceDraft.modArchive)}
                    disabled={locked}
                    onChange={(event) =>
                      c.setResourceDraft({ ...c.resourceDraft, modArchive: event.target.value })
                    }
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={locked || !c.resourceDraft.xsdArchive || !c.resourceDraft.modArchive}
                onClick={() => void c.configureResources()}
              >
                {t('shell.settings.saveResources')}
              </button>
            </details>
          </section>
          <section hidden={category !== 'tools'}>
            <h3>{t('shell.settings.savedTools')}</h3>
            <p>{t('shell.settings.toolsHelp')}</p>
            {c.capabilities?.toolError ? (
              <p className="error-text">{text(c.capabilities.toolError)}</p>
            ) : null}
            <div className="form-fields">
              {toolLabels.map(({ key, label }) => (
                <label key={key}>
                  {t(label)}
                  <div className="path-picker">
                    <input
                      aria-label={t('shell.settings.savedToolPath', { tool: t(label) })}
                      value={displayPath(c.toolDraft[key])}
                      disabled={locked}
                      onChange={(event) =>
                        c.setToolDraft({ ...c.toolDraft, [key]: event.target.value })
                      }
                    />
                    <button
                      type="button"
                      disabled={locked}
                      onClick={() =>
                        void c.chooseBinary((path) =>
                          c.setToolDraft((previous) => ({ ...previous, [key]: path })),
                        )
                      }
                    >
                      {t('shell.settings.browse')}
                    </button>
                  </div>
                </label>
              ))}
            </div>
            <h3>{t('shell.settings.effectiveTools')}</h3>
            <dl>
              {toolLabels.map(({ key, label }) => (
                <div key={key}>
                  <dt>{t(label)}</dt>
                  <dd className="mono path-text">
                    {displayPath(
                      c.capabilities?.executionTools?.[key] ?? t('shell.settings.noTool'),
                    )}
                  </dd>
                </div>
              ))}
            </dl>
            <h3>{t('shell.settings.environment')}</h3>
            {c.capabilities?.environmentOverrides.length ? (
              <ul>
                {c.capabilities.environmentOverrides.map((entry) => (
                  <li key={entry} className="mono path-text">
                    {entry}
                  </li>
                ))}
              </ul>
            ) : (
              <p>{t('shell.settings.noEnvironment')}</p>
            )}
          </section>
        </div>
      </div>
    </Dialog>
  );
}
