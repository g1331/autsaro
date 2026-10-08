import type { Workbench } from './workbench/useWorkbench';
import { useLocale } from './i18n';

const roleNames: Record<string, string> = {
  ecu_extract: 'shell.role.ecu_extract',
  application: 'shell.role.application',
  service_client: 'shell.role.service_client',
  types: 'shell.role.types',
  bsw_description: 'shell.role.bsw_description',
  bsw_implementation: 'shell.role.bsw_implementation',
  ecuc_values: 'shell.role.ecuc_values',
  retained: 'shell.role.retained',
};

export function IntegrationPanel({ controller }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const {
    workspace,
    native,
    integrationUnapplied: changed,
    integrationIds: ids,
    integrationPeriod: period,
    integrationPreview: preview,
    integrationPreviewPath: selectedFile,
    integrationNotice: notice,
    integrationIssues: issues,
    integrationInspection: inspection,
    inspectIntegration: inspect,
    reopenIntegration: reopen,
    applyIntegration: apply,
    previewIntegrationSave: showPreview,
    saveIntegration: save,
    restoreIntegrationDraft,
    setIntegrationIds: setIds,
    setIntegrationPeriod: setPeriod,
    setIntegrationPreview: setPreview,
    setIntegrationPreviewPath: setSelectedFile,
    setIntegrationUnapplied: onDraftChange,
  } = controller;
  if (!workspace) return null;
  const busy = Boolean(controller.busy);
  const locked = busy;
  const plan = inspection?.description;
  const selected = preview?.files.find((file) => file.path === selectedFile);
  return (
    <div className="workflow-page integration-view" aria-label={t('shell.integration.workspace')}>
      <div className="section-header">
        <div>
          <p className="eyebrow">{t('shell.integration.eyebrow')}</p>
          <h2>{t('shell.document.integration')}</h2>
          <p role="status">{busy ? t('shell.integration.busy') : text(notice)}</p>
        </div>
        <div className="section-actions">
          <button
            className="outline-button small"
            onClick={inspect}
            disabled={busy || locked || !native || changed}
          >
            {t('shell.integration.inspect')}
          </button>
          <button
            className="outline-button small"
            onClick={reopen}
            disabled={busy || locked || !native}
          >
            {t('shell.integration.reopen')}
          </button>
        </div>
      </div>
      <p>{t('shell.integration.required')}</p>
      {issues.length > 0 && (
        <div className="inspector-block" role="alert">
          <h3>{t('shell.integration.blockers')}</h3>
          {issues.map((issue, index) => (
            <div key={`${issue.code}-${index}`}>
              <strong>
                {issue.code} · {t(`shell.integration.category.${issue.category}`)}
              </strong>
              <p>{text(issue.message)}</p>
              <p className="mono path-text">
                {issue.file} {issue.object}
              </p>
              <p>{text(issue.remedy)}</p>
            </div>
          ))}
        </div>
      )}
      {plan && (
        <>
          <div className="inspector-block">
            <h3>{t('shell.integration.roles')}</h3>
            <table>
              <thead>
                <tr>
                  <th>{t('shell.integration.source')}</th>
                  <th>{t('shell.integration.actualRoles')}</th>
                  <th>{t('shell.integration.sha')}</th>
                </tr>
              </thead>
              <tbody>
                {plan.sources.map((source) => (
                  <tr key={source.logicalPath}>
                    <td>{source.logicalPath}</td>
                    <td>
                      {source.roles
                        .map((role) => (roleNames[role] ? t(roleNames[role]) : role))
                        .join(t('shell.listSeparator'))}
                    </td>
                    <td className="mono path-text" title={source.rawSha256}>
                      {source.rawSha256.slice(0, 16)}…
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="inspector-block">
            <h3>{t('shell.integration.parameters')}</h3>
            <p className="mono path-text">{plan.component.instance}</p>
            <div className="integration-fields">
              {plan.signals.map((signal) => (
                <label className="field" key={signal.port}>
                  <span>
                    {signal.receive
                      ? t('shell.integration.receiveId')
                      : t('shell.integration.transmitId')}{' '}
                    · {signal.port}
                  </span>
                  <input
                    aria-label={
                      signal.receive
                        ? t('shell.integration.receiveId')
                        : t('shell.integration.transmitId')
                    }
                    value={ids[signal.port] ?? ''}
                    onChange={(event) => {
                      const next = { ...ids, [signal.port]: event.target.value };
                      setIds(next);
                      onDraftChange(
                        period !== String(plan.component.periodMs) ||
                          plan.signals.some((item) => next[item.port] !== String(item.canId)),
                      );
                      setPreview(null);
                    }}
                    disabled={busy || locked}
                    inputMode="numeric"
                  />
                </label>
              ))}
              <label className="field">
                <span>{t('shell.integration.period')}</span>
                <input
                  aria-label={t('shell.integration.periodA11y')}
                  value={period}
                  onChange={(event) => {
                    setPeriod(event.target.value);
                    onDraftChange(
                      event.target.value !== String(plan.component.periodMs) ||
                        plan.signals.some((item) => ids[item.port] !== String(item.canId)),
                    );
                    setPreview(null);
                  }}
                  disabled={busy || locked}
                  inputMode="numeric"
                />
              </label>
            </div>
            <p>
              {t('shell.integration.diagnostic', {
                request: plan.diagnostic.requestCanId,
                response: plan.diagnostic.responseCanId,
                did: plan.diagnostic.did.toString(16).toUpperCase(),
              })}
            </p>
            <div className="section-actions">
              <button
                className="primary-button compact"
                onClick={apply}
                disabled={busy || locked || !changed}
              >
                {t('shell.integration.apply')}
              </button>
              <button
                className="outline-button small"
                onClick={() => restoreIntegrationDraft()}
                disabled={busy || locked || !changed}
              >
                {t('shell.integration.restore')}
              </button>
              <button
                className="outline-button small"
                onClick={showPreview}
                disabled={busy || locked || changed || !workspace.dirty}
              >
                {t('shell.command.save')}
              </button>
            </div>
          </div>
        </>
      )}
      {preview && (
        <div className="inspector-block" aria-label={t('shell.integration.savePreviewA11y')}>
          <h3>{t('shell.integration.savePreview')}</h3>
          <p>
            {t('shell.integration.changedFiles', {
              count: preview.files.filter((file) => file.changed).length,
            })}
          </p>
          <div className="section-actions">
            {preview.files.map((file) => (
              <button
                className="outline-button small"
                key={file.path}
                onClick={() => setSelectedFile(file.path)}
              >
                {file.path.split(/[\\/]/).pop()} ·{' '}
                {file.changed ? t('shell.integration.changed') : t('shell.integration.unchanged')}
              </button>
            ))}
          </div>
          {selected && (
            <>
              <p className="mono path-text">{selected.path}</p>
              {selected.changed ? (
                <div className="integration-diff">
                  <details>
                    <summary>{t('shell.integration.before')}</summary>
                    <pre>{selected.before}</pre>
                  </details>
                  <details>
                    <summary>{t('shell.integration.after')}</summary>
                    <pre>{selected.after}</pre>
                  </details>
                </div>
              ) : (
                <p>{t('shell.integration.unchangedBytes')}</p>
              )}
            </>
          )}
          <button
            className="primary-button compact"
            onClick={save}
            disabled={busy || locked || changed}
          >
            {t('shell.integration.confirmSave')}
          </button>
        </div>
      )}
    </div>
  );
}
