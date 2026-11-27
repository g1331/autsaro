import { VariantEditor } from './VariantEditor';
import type { Workbench } from '../../app/useWorkbench';
import { useLocale } from '../../i18n';
import { displayPath } from '../../pathDisplay';
import { applicationPeriodMs } from './applicationPeriod';

export const integrationFields = [
  'applyIntegration',
  'busy',
  'inspectIntegration',
  'integrationIds',
  'signalDrafts',
  'variantSelection',
  'previewVariantSelection',
  'connectionDrafts',
  'setConnectionDrafts',
  'setVariantSelection',
  'setSignalDrafts',
  'integrationInspection',
  'integrationIssues',
  'integrationNotice',
  'integrationPeriod',
  'integrationUnapplied',
  'integrationSavePending',
  'native',
  'previewIntegrationSave',
  'reopenIntegration',
  'restoreIntegrationDraft',
  'setIntegrationIds',
  'setIntegrationPeriod',
  'setIntegrationPreview',
  'setIntegrationUnapplied',
  'workspace',
] as const satisfies readonly (keyof Workbench)[];
type IntegrationView = Pick<Workbench, (typeof integrationFields)[number]>;

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

export function IntegrationPanel({ controller }: { controller: IntegrationView }) {
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
  function updateDirty({
    numericIds = ids,
    periodText = period,
    settings = controller.signalDrafts,
    connections = controller.connectionDrafts,
  }: {
    numericIds?: typeof ids;
    periodText?: string;
    settings?: typeof controller.signalDrafts;
    connections?: typeof controller.connectionDrafts;
  }) {
    onDraftChange(
      connections.length > 0 ||
        Object.keys(settings).length > 0 ||
        periodText !== String(planned) ||
        Boolean(plan?.signals.some((item) => numericIds[item.port] !== String(item.canId))),
    );
    setPreview(null);
  }
  const connections =
    inspection?.connections?.filter((binding) => binding.options.length > 1) ?? [];
  const choices = {
    extended: [t('shell.integration.standardId'), t('shell.integration.extendedId')],
    fd: ['Classical CAN', 'CAN FD'],
    dual: [t('shell.integration.singleFormat'), t('shell.integration.bothFormats')],
    bigEndian: ['Intel', 'Motorola'],
  };
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
      {inspection?.layers && (
        <>
          <VariantEditor
            key={JSON.stringify([workspace.name, inspection.layers.base])}
            preview={controller.previewVariantSelection}
            layers={inspection.layers}
            draft={controller.variantSelection}
            disabled={locked}
            update={(next) => {
              controller.setVariantSelection(next);
              onDraftChange(next !== null);
              setPreview(null);
            }}
          />
          <div className="section-actions">
            <button onClick={apply} disabled={locked || !changed}>
              {t('action.apply')}
            </button>
            <button onClick={restoreIntegrationDraft} disabled={locked || !changed}>
              {t('action.revert')}
            </button>
            <button
              onClick={showPreview}
              disabled={
                locked || changed || (!workspace.dirty && !controller.integrationSavePending)
              }
            >
              {t('action.previewSave')}
            </button>
          </div>
        </>
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
          {planned !== null && !inspection?.layers && (
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
                        updateDirty({ numericIds: next });
                      }}
                      disabled={busy || locked}
                      inputMode="numeric"
                    />
                  </label>
                ))}
                {plan.signals.map((signal) => {
                  const values = {
                    extended: signal.extended,
                    fd: signal.fd,
                    ...(signal.receive ? { dual: signal.acceptsFd } : {}),
                    bitLength: signal.bitLength,
                    startBit: signal.startBit,
                    bigEndian: signal.bigEndian,
                    frameLength: signal.dlc,
                    pduLength: signal.pduLength,
                  };
                  return (Object.keys(values) as (keyof typeof values)[]).map((field) => {
                    const value =
                      controller.signalDrafts[signal.port]?.[field] ?? String(values[field]);
                    const update = (next: string) => {
                      const drafts = { ...controller.signalDrafts };
                      const row = { ...drafts[signal.port] };
                      if (next === String(values[field])) delete row[field];
                      else row[field] = next;
                      if (Object.keys(row).length) drafts[signal.port] = row;
                      else delete drafts[signal.port];
                      controller.setSignalDrafts(drafts);
                      updateDirty({ settings: drafts });
                    };
                    return (
                      <label className="field" key={signal.port + field}>
                        <span>
                          {t(`shell.integration.${field}`)} · {signal.port}
                        </span>
                        {field === 'extended' ||
                        field === 'fd' ||
                        field === 'dual' ||
                        field === 'bigEndian' ? (
                          <select
                            disabled={locked}
                            value={value}
                            onChange={(event) => update(event.target.value)}
                          >
                            <option value="false">{choices[field][0]}</option>
                            <option value="true">{choices[field][1]}</option>
                          </select>
                        ) : (
                          <input
                            disabled={locked}
                            inputMode="numeric"
                            value={value}
                            onChange={(event) => update(event.target.value)}
                          />
                        )}
                      </label>
                    );
                  });
                })}
                <label className="field">
                  <span>{t('shell.integration.period')}</span>
                  <input
                    value={period}
                    onChange={(event) => {
                      setPeriod(event.target.value);
                      updateDirty({ periodText: event.target.value });
                    }}
                    disabled={busy || locked}
                    inputMode="numeric"
                  />
                </label>
                {connections.length > 0 && (
                  <details className="integration-connections">
                    <summary>{t('shell.integration.connections')}</summary>
                    <div className="integration-fields">
                      {connections.map((binding) => {
                        const same = (item: { object: string; definition: string }) =>
                          item.object === binding.object && item.definition === binding.definition;
                        const value =
                          controller.connectionDrafts.find(same)?.target ?? binding.current;
                        return (
                          <label
                            className="field"
                            key={JSON.stringify([binding.object, binding.definition])}
                          >
                            <span>
                              {binding.definition.startsWith('/')
                                ? binding.definition.split('/').at(-1)
                                : t(`shell.integration.${binding.definition}`)}{' '}
                              · {binding.object}
                            </span>
                            <select
                              disabled={locked}
                              value={value}
                              onChange={(event) => {
                                const drafts = controller.connectionDrafts.filter(
                                  (item) => !same(item),
                                );
                                if (event.target.value !== binding.current)
                                  drafts.push({
                                    object: binding.object,
                                    definition: binding.definition,
                                    target: event.target.value,
                                  });
                                controller.setConnectionDrafts(drafts);
                                updateDirty({ connections: drafts });
                              }}
                            >
                              {binding.options.map((option) => (
                                <option key={option}>{option}</option>
                              ))}
                            </select>
                          </label>
                        );
                      })}
                    </div>
                  </details>
                )}
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
                  disabled={
                    busy ||
                    locked ||
                    changed ||
                    (!workspace.dirty && !controller.integrationSavePending)
                  }
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
