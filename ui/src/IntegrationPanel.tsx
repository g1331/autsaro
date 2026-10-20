import type { Workbench } from './workbench/useWorkbench';
import { useLocale } from './i18n';
import { displayPath } from './pathDisplay';
import { applicationPeriodMs } from './workbench/forms';

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
    integrationNotice: notice,
    integrationIssues: issues,
    integrationInspection: inspection,
    inspectIntegration: inspect,
    reopenIntegration: reopen,
    applyIntegration: apply,
    previewIntegrationSave: showPreview,
    restoreIntegrationDraft,
    setIntegrationIds: setIds,
    setIntegrationPeriod: setPeriod,
    setIntegrationPreview: setPreview,
    setIntegrationUnapplied: onDraftChange,
  } = controller;
  if (!workspace) return null;
  const busy = Boolean(controller.busy);
  const locked = busy;
  const plan = inspection?.description;
  // The historical chain and the mainline both plan one shared application period.
  const planned = applicationPeriodMs(plan);
  return (
    <div className="workflow-page integration-view" aria-label={t('shell.integration.workspace')}>
      <div className="section-header">
        <div>
          <p className="eyebrow">{t('shell.integration.eyebrow')}</p>
          <h2>{t('shell.document.integration')}</h2>
          <p role="status">
            {busy ? t('shell.busy', { operation: text(controller.busy) }) : text(notice)}
          </p>
        </div>
        <div className="section-actions">
          <button onClick={inspect} disabled={busy || locked || !native || changed}>
            {t('action.validate')}
          </button>
          <button onClick={reopen} disabled={busy || locked || !native}>
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
                {displayPath(issue.file ?? '')} {issue.object}
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
          {plan.composition && (
            <div className="inspector-block">
              <p>{t('shell.integration.compositionReadOnly')}</p>
              {plan.composition.components.map((item) => (
                <p className="mono path-text" key={item.instance}>
                  {item.component} · {item.instance}
                </p>
              ))}
            </div>
          )}
          {planned !== null && (
            <div className="inspector-block">
              <h3>{t('shell.integration.parameters')}</h3>
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
                      value={ids[signal.port] ?? ''}
                      onChange={(event) => {
                        const next = { ...ids, [signal.port]: event.target.value };
                        setIds(next);
                        onDraftChange(
                          period !== String(planned) ||
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
                    value={period}
                    onChange={(event) => {
                      setPeriod(event.target.value);
                      onDraftChange(
                        event.target.value !== String(planned) ||
                          plan.signals.some((item) => ids[item.port] !== String(item.canId)),
                      );
                      setPreview(null);
                    }}
                    disabled={busy || locked}
                    inputMode="numeric"
                  />
                </label>
              </div>
              {plan.diagnostic && (
                <p>
                  {t('shell.integration.diagnostic', {
                    request: plan.diagnostic.requestCanId,
                    response: plan.diagnostic.responseCanId,
                    did: plan.diagnostic.did.toString(16).toUpperCase(),
                  })}
                </p>
              )}
              <div className="section-actions">
                <button
                  className="primary-button"
                  onClick={apply}
                  disabled={busy || locked || !changed}
                >
                  {t('action.apply')}
                </button>
                <button
                  onClick={() => restoreIntegrationDraft()}
                  disabled={busy || locked || !changed}
                >
                  {t('action.revert')}
                </button>
                <button
                  onClick={showPreview}
                  disabled={busy || locked || changed || !workspace.dirty}
                >
                  {t('action.previewSave')}
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
