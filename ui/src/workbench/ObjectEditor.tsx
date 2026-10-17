import { editorChangeKeys, editorSeverityKeys, editorValueKindKeys } from '../i18n/editors';
import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { useId, useMemo } from 'react';
import type { ReactNode } from 'react';
import { Search } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import type { FieldDescriptor } from './projectTypes';
import { fieldName } from './batchEditing';
import { multiplicity } from './projectGroups';
import { StructureEditor } from './StructureEditor';
import { CopyText } from './Dialog';
import { iconSize } from './icons';
import { CommandActions, CommandButton, CommandReasons } from './CommandActions';
import { commandById } from './commands';
import { objectRoleClass } from './objectRole';

export function ObjectEditor({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const rows = useMemo(() => {
    const fields = new Map<string, string[]>();
    for (const field of c.projection?.fields ?? []) {
      const list = fields.get(field.objectId) ?? [];
      list.push(
        field.current.state === 'explicit'
          ? field.current.value.lexeme
          : (field.defaultValue?.lexeme ?? ''),
        field.definitionId,
      );
      fields.set(field.objectId, list);
    }
    const query = c.objectFilter.toLocaleLowerCase();
    return (c.projection?.objects ?? []).filter(
      (object) =>
        !query ||
        [object.shortName, object.path, object.definitionId, ...(fields.get(object.objectId) ?? [])]
          .join(' ')
          .toLocaleLowerCase()
          .includes(query),
    );
  }, [c.projection, c.objectFilter]);
  // A checkbox shows batch selection only; the current object alone is not a batch (D8).
  const batch = c.objectSelection.length > 1;
  return (
    <>
      <p className="field-help">
        {t('editor.objects.selectedCount', { count: c.objectSelection.length })}
      </p>
      <label className="search-field">
        <Search size={iconSize.inline} aria-hidden="true" />
        <input
          aria-label={t('editor.objects.searchLabel')}
          value={c.objectFilter}
          onChange={(event) => c.setObjectFilter(event.target.value)}
          placeholder={t('editor.objects.searchPlaceholder')}
        />
      </label>
      {c.activeObjectId && !rows.some((object) => object.objectId === c.activeObjectId) ? (
        <p className="filter-retained">
          {t('editor.objects.filterRetained')}
          <button onClick={() => c.setObjectFilter('')}>{t('editor.objects.clearFilter')}</button>
        </p>
      ) : null}
      <div className="table-wrap">
        {/* Fixed columns keep rows to one line; full paths and reasons stay in titles and the inspector. */}
        <table className="object-table">
          <caption>{t('editor.objects.caption')}</caption>
          <colgroup>
            <col className="col-select" />
            <col className="col-name" />
            <col className="col-type" />
            <col />
            <col className="col-access" />
          </colgroup>
          <thead>
            <tr>
              <th scope="col">{t('editor.common.select')}</th>
              <th scope="col">{t('editor.common.name')}</th>
              <th scope="col">{t('editor.common.type')}</th>
              <th scope="col">{t('editor.objects.definition')}</th>
              <th scope="col">{t('editor.common.access')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((object) => {
              // Non-ECUC elements report their kind as the definition; the type column shows it.
              const definition =
                object.definitionId && object.definitionId !== object.kind
                  ? object.definitionId
                  : null;
              const access = object.writable
                ? t('editor.common.configurable')
                : t('editor.common.readonly');
              return (
                <tr
                  key={object.objectId}
                  data-object-row={object.objectId}
                  aria-current={object.objectId === c.activeObjectId ? 'true' : undefined}
                >
                  <td>
                    <input
                      type="checkbox"
                      aria-label={t('editor.objects.batchSelect', { path: object.path })}
                      checked={batch && c.objectSelection.includes(object.objectId)}
                      onChange={() => void c.selectObject(object.objectId, true)}
                    />
                  </td>
                  <td title={object.path}>
                    <button
                      type="button"
                      className="table-link"
                      onClick={() => void c.selectObject(object.objectId)}
                    >
                      {object.shortName}
                    </button>
                  </td>
                  <td title={object.kind}>{object.kind}</td>
                  <td
                    className="mono"
                    title={`${definition ?? t('editor.objects.noDefinition')}\n${object.path}`}
                  >
                    {definition ?? '—'}
                  </td>
                  <td title={object.reason ? `${access} · ${text(object.reason)}` : access}>
                    {access}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </>
  );
}

/** Pending drafts of the configuration document, with the batch preview for multi-item and structural changes. */
export function BatchBar({ controller: c }: { controller: Workbench }) {
  const { t } = useLocale();
  return (
    <>
      <div className="batch-bar">
        <span>{t('editor.objects.pendingCount', { count: c.changes.length })}</span>
        <CommandActions
          className="batch-actions"
          actions={[
            { command: commandById(c, 'preview-changes') },
            { command: commandById(c, 'discard-changes') },
          ]}
        />
      </div>
      {c.changes.length ? (
        <details>
          <summary>{t('editor.objects.inspectPending')}</summary>
          {c.changes.map((change) => (
            <div className="pending-change" key={change.changeId}>
              <span>
                {t(editorChangeKeys[change.op])} · {change.changeId}
              </span>
              <button type="button" onClick={() => c.discardChanges(change.changeId)}>
                {t('editor.objects.removeOperation')}
              </button>
              <pre>{JSON.stringify(change, null, 2)}</pre>
            </div>
          ))}
        </details>
      ) : null}
    </>
  );
}

export function ObjectInspector({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const reasonsId = useId();
  const previewChanges = commandById(c, 'preview-changes');
  const object = c.projection?.objects.find((item) => item.objectId === c.activeObjectId);
  // The focused grid row, while it belongs to the shown object or batch selection.
  const field = c.projection?.fields.find(
    (item) =>
      item.fieldId === c.activeFieldId &&
      (c.objectSelection.length > 1
        ? c.objectSelection.includes(item.objectId)
        : item.objectId === c.activeObjectId),
  );
  const references =
    c.projection?.references.filter(
      (edge) => edge.objectId === c.activeObjectId || edge.targetId === c.activeObjectId,
    ) ?? [];
  const rename = c.changes.find(
    (change) =>
      change.op === 'rename-instance' &&
      change.object.kind === 'existing' &&
      change.object.objectId === object?.objectId,
  );
  return (
    <div className="object-inspector">
      {object ? (
        <>
          <h2 tabIndex={-1} data-object-title={object.objectId}>
            {object.shortName}
          </h2>
          <span className={['role-tag', objectRoleClass(object)].filter(Boolean).join(' ')}>
            {object.kind} ·{' '}
            {object.writable ? t('editor.common.configurable') : t('editor.common.readonly')}
          </span>
          <p className="mono path-text">{object.path}</p>
          <CopyText text={object.path} label={t('action.copyPath')} />
          <p className="mono path-text">
            {displayPath(
              c.projection?.sources.find((source) => source.sourceId === object.sourceId)?.path ??
                '',
            )}
          </p>
          {object.reason ? <p className="field-help">{text(object.reason ?? '')}</p> : null}
          {c.inspectorTab === 'properties' ? (
            <>
              {object.writable && !c.actionReason('prepare-change') ? (
                <label className="descriptor-field">
                  {t('editor.objects.instanceName')}
                  <input
                    data-rename-input
                    aria-label={t('editor.objects.instanceName')}
                    value={rename?.op === 'rename-instance' ? rename.shortName : object.shortName}
                    disabled={Boolean(c.busy)}
                    onChange={(event) =>
                      c.stageChange({
                        op: 'rename-instance',
                        changeId: object.objectId,
                        object: { kind: 'existing', objectId: object.objectId },
                        expectedShortName: object.shortName,
                        shortName: event.target.value,
                      })
                    }
                  />
                </label>
              ) : null}
              {field ? <FieldDetails controller={c} field={field} /> : null}
              <div className="inspector-actions">
                <CommandButton command={previewChanges} reasonId={`${reasonsId}-0`} />
                {object.writable && !c.actionReason('prepare-change') ? (
                  <button
                    type="button"
                    onClick={() =>
                      c.stageChange({
                        op: 'remove-instance',
                        changeId: object.objectId,
                        object: { kind: 'existing', objectId: object.objectId },
                        expectedShortName: object.shortName,
                      })
                    }
                  >
                    {t('editor.objects.removeInstance')}
                  </button>
                ) : null}
              </div>
              <CommandReasons commands={[previewChanges]} id={reasonsId} />
              <StructureEditor controller={c} object={object} />
            </>
          ) : (
            <div className="reference-list">
              {references.map((edge, index) => {
                const incoming = edge.targetId === object.objectId;
                const targetId = incoming ? edge.objectId : edge.targetId;
                const target = c.projection?.objects.find((item) => item.objectId === targetId);
                return (
                  <section key={`${edge.fieldId}-${index}`}>
                    <strong>
                      {incoming ? t('editor.objects.incoming') : t('editor.objects.outgoing')} ·{' '}
                      {edge.dest}
                    </strong>
                    <p className="mono path-text">{edge.rawPath}</p>
                    {target ? (
                      <button type="button" onClick={() => void c.selectObject(target.objectId)}>
                        {t('editor.objects.locateTarget', { name: target.shortName })}
                      </button>
                    ) : (
                      <p>{edge.reason ? text(edge.reason) : t('editor.objects.unresolved')}</p>
                    )}
                  </section>
                );
              })}
              {!references.length ? <p>{t('editor.objects.noReferences')}</p> : null}
            </div>
          )}
        </>
      ) : (
        <p className="empty-state">{t('editor.objects.empty')}</p>
      )}
    </div>
  );
}

/** Read-only facts of the current parameter; its value is edited in the grid (ADR 0020). */
function FieldDetails({ controller: c, field }: { controller: Workbench; field: FieldDescriptor }) {
  const { t, text } = useLocale();
  const diagnostics =
    c.projection?.diagnostics.filter(
      (issue) => issue.fieldId === field.fieldId && issue.severity !== 'info',
    ) ?? [];
  const rejection = c.fieldErrors[field.fieldId];
  const candidates =
    c.projection?.referenceCandidates.filter((candidate) => candidate.fieldId === field.fieldId) ??
    [];
  const reference = field.reference;
  const rows: [string, ReactNode][] = [
    [t('editor.objects.definition'), <span className="mono path-text">{field.definitionId}</span>],
    [t('editor.common.type'), field.kind ? t(editorValueKindKeys[field.kind]) : field.elementKind],
    [
      t('editor.container.multiplicity'),
      <span className="mono">
        {multiplicity(field.lowerMultiplicity, field.upperMultiplicity)}
      </span>,
    ],
  ];
  if (field.minimum !== null || field.maximum !== null)
    rows.push([
      t('editor.container.range'),
      <span className="mono">{`${field.minimum ?? '−∞'}..${field.maximum ?? '+∞'}`}</span>,
    ]);
  if (field.defaultValue)
    rows.push([
      t('editor.container.default'),
      <span className="mono">
        {field.defaultValue.lexeme}
        {field.defaultOrigin ? ` · ${field.defaultOrigin}` : ''}
      </span>,
    ]);
  if (field.unit) rows.push([t('editor.container.unit'), field.unit]);
  if (field.enumeration.length)
    rows.push([
      t('editor.container.choices'),
      <span className="mono path-text">{field.enumeration.join(', ')}</span>,
    ]);
  if (field.kind === 'reference')
    rows.push([
      t('editor.container.target'),
      <span className="mono path-text">
        {reference?.state === 'explicit'
          ? `${reference.rawPath} · DEST=${reference.dest}`
          : t('editor.objects.unsetReference')}
      </span>,
    ]);
  if (!field.writable)
    rows.push([
      t('editor.common.access'),
      field.reason ? text(field.reason) : t('editor.objects.noCapability'),
    ]);
  return (
    <section className="field-details" aria-label={fieldName(field)}>
      <h3>{fieldName(field)}</h3>
      <dl className="property-list">
        {rows.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
      {field.kind === 'reference' && candidates.length ? (
        <details>
          <summary>{t('editor.container.candidates', { count: candidates.length })}</summary>
          <ul className="candidate-list">
            {candidates.map((candidate) => (
              <li key={`${candidate.targetId}:${candidate.dest}`} className="mono path-text">
                {candidate.path} · {candidate.dest}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {rejection || diagnostics.length ? (
        <div className="field-error">
          {rejection ? <p>{text(rejection)}</p> : null}
          {diagnostics.map((issue, index) => (
            <div key={`${issue.code}-${index}`}>
              <p>
                {t(editorSeverityKeys[issue.severity])} · {issue.code} · {text(issue.message)}
              </p>
              {text(issue.remedy) ? <p className="field-help">{text(issue.remedy)}</p> : null}
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
