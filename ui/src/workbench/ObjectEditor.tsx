import { editorValueKindKeys, editorChangeKeys } from '../i18n/editors';
import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { useMemo } from 'react';
import { Search } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import type { FieldDescriptor, ValueState } from './projectTypes';
import { StructureEditor } from './StructureEditor';
import { CopyText } from './Dialog';

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
  return (
    <div className="document-page">
      <div className="document-heading">
        <div>
          <h2>{t('editor.objects.heading')}</h2>
          <span>
            {c.projection?.profile} · {c.projection?.release}
          </span>
        </div>
        <span>{t('editor.objects.selectedCount', { count: c.objectSelection.length })}</span>
      </div>
      <label className="search-field">
        <Search size={15} aria-hidden="true" />
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
        <table>
          <caption>{t('editor.objects.caption')}</caption>
          <thead>
            <tr>
              <th scope="col">{t('editor.common.select')}</th>
              <th scope="col">{t('editor.common.name')}</th>
              <th scope="col">{t('editor.common.type')}</th>
              <th scope="col">{t('editor.objects.definitionPath')}</th>
              <th scope="col">{t('editor.common.access')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((object) => (
              <tr
                key={object.objectId}
                data-object-row={object.objectId}
                className={c.objectSelection.includes(object.objectId) ? 'selected-row' : ''}
                aria-selected={c.objectSelection.includes(object.objectId)}
              >
                <td>
                  <input
                    type="checkbox"
                    aria-label={t('editor.objects.batchSelect', { path: object.path })}
                    checked={c.objectSelection.includes(object.objectId)}
                    onChange={() => void c.selectObject(object.objectId, true)}
                  />
                </td>
                <td>
                  <button
                    type="button"
                    className="table-link"
                    onClick={() => void c.selectObject(object.objectId)}
                  >
                    {object.shortName}
                  </button>
                </td>
                <td>{object.kind}</td>
                <td className="mono">
                  <span>{object.definitionId ?? t('editor.objects.noDefinition')}</span>
                  <small className="path-text">{object.path}</small>
                </td>
                <td>
                  {object.writable ? t('editor.common.configurable') : t('editor.common.readonly')}
                  <small>{text(object.reason ?? '')}</small>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="batch-bar">
        <span>{t('editor.objects.pendingCount', { count: c.changes.length })}</span>
        <button
          type="button"
          onClick={() => void c.prepareChanges()}
          disabled={!c.changes.length || Boolean(c.busy)}
        >
          {t('editor.objects.previewBatch')}
        </button>
        <button
          type="button"
          onClick={() => c.discardChanges()}
          disabled={!c.changes.length || Boolean(c.busy)}
        >
          {t('editor.objects.restoreBatch')}
        </button>
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
    </div>
  );
}

export function DescriptorField({
  field,
  controller: c,
}: {
  field: FieldDescriptor;
  controller: Workbench;
}) {
  const { t, text } = useLocale();
  const staged = c.changes.find(
    (change) =>
      (change.op === 'set-value' || change.op === 'set-reference') &&
      change.field.kind === 'existing' &&
      change.field.fieldId === field.fieldId,
  );
  const value: ValueState = staged?.op === 'set-value' ? staged.value : field.current;
  const explicit = value.state === 'explicit';
  const lexeme = explicit ? value.value.lexeme : '';
  const blocked = Boolean(c.actionReason('prepare-change'));
  const errors =
    c.projection?.diagnostics.filter(
      (issue) => issue.fieldId === field.fieldId && issue.severity === 'error',
    ) ?? [];
  const description = `field-description-${field.fieldId}`;
  const stageValue = (next: ValueState) =>
    c.stageChange({
      op: 'set-value',
      changeId: field.fieldId,
      field: { kind: 'existing', fieldId: field.fieldId },
      expected: field.current,
      value: next,
    });
  const currentReference = staged?.op === 'set-reference' ? staged.value : field.reference;
  const candidates =
    c.projection?.referenceCandidates.filter((candidate) => candidate.fieldId === field.fieldId) ??
    [];
  return (
    <div className="descriptor-field">
      <label htmlFor={`field-${field.fieldId}`}>
        {field.definitionId.split('#').pop()?.split('/').pop()}{' '}
        <small>
          {field.kind ? t(editorValueKindKeys[field.kind]) : field.elementKind}
          {field.unit ? ` · ${field.unit}` : ''}
        </small>
      </label>
      {!field.writable ? (
        <div className="readonly-value" tabIndex={-1} data-field-id={field.fieldId}>
          <span className="mono">
            {currentReference?.state === 'explicit'
              ? `${currentReference.rawPath} · DEST=${currentReference.dest}`
              : field.current.state === 'explicit'
                ? field.current.value.lexeme
                : (field.defaultValue?.lexeme ?? t('editor.common.unset'))}
          </span>
          <p>{field.reason ? text(field.reason) : t('editor.objects.noCapability')}</p>
        </div>
      ) : field.kind === 'reference' ? (
        <>
          <p className="mono path-text">
            {currentReference?.state === 'explicit'
              ? `${currentReference.rawPath} · DEST=${currentReference.dest}`
              : t('editor.objects.unsetReference')}
          </p>
          <select
            id={`field-${field.fieldId}`}
            data-field-id={field.fieldId}
            value={
              currentReference?.state === 'explicit' && currentReference.target?.kind === 'existing'
                ? `${currentReference.target.objectId}:${currentReference.dest}`
                : ''
            }
            aria-invalid={errors.length > 0}
            aria-describedby={description}
            disabled={blocked}
            onChange={(event) => {
              const candidate = candidates.find(
                (item) => `${item.targetId}:${item.dest}` === event.target.value,
              );
              if (candidate)
                c.stageChange({
                  op: 'set-reference',
                  changeId: field.fieldId,
                  field: { kind: 'existing', fieldId: field.fieldId },
                  expected: field.reference ?? { state: 'absent' },
                  value: {
                    state: 'explicit',
                    rawPath: candidate.path,
                    dest: candidate.dest,
                    target: { kind: 'existing', objectId: candidate.targetId },
                  },
                });
            }}
          >
            <option value="">{t('editor.objects.chooseTarget')}</option>
            {candidates.map((candidate) => (
              <option
                key={`${candidate.targetId}:${candidate.dest}`}
                value={`${candidate.targetId}:${candidate.dest}`}
              >
                {candidate.shortName} · {candidate.path} · {candidate.dest}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={blocked}
            onClick={() =>
              c.stageChange({
                op: 'set-reference',
                changeId: field.fieldId,
                field: { kind: 'existing', fieldId: field.fieldId },
                expected: field.reference ?? { state: 'absent' },
                value: { state: 'absent' },
              })
            }
          >
            {t('editor.objects.removeReference')}
          </button>
          {!candidates.length ? (
            <p className="field-help">{t('editor.objects.noTargets')}</p>
          ) : null}
        </>
      ) : (
        <>
          {field.kind === 'boolean' || field.enumeration.length ? (
            <select
              id={`field-${field.fieldId}`}
              data-field-id={field.fieldId}
              value={lexeme}
              aria-invalid={errors.length > 0}
              aria-describedby={description}
              disabled={blocked}
              onChange={(event) =>
                stageValue({
                  state: 'explicit',
                  value: { kind: field.kind!, lexeme: event.target.value },
                })
              }
            >
              <option value="">
                {explicit ? t('editor.objects.emptyExplicit') : t('editor.common.unset')}
              </option>
              {(field.kind === 'boolean' ? ['true', 'false'] : field.enumeration).map((entry) => (
                <option key={entry} value={entry}>
                  {entry}
                </option>
              ))}
            </select>
          ) : (
            <input
              id={`field-${field.fieldId}`}
              data-field-id={field.fieldId}
              value={lexeme}
              aria-invalid={errors.length > 0}
              aria-describedby={description}
              disabled={blocked}
              onChange={(event) =>
                stageValue({
                  state: 'explicit',
                  value: { kind: field.kind!, lexeme: event.target.value },
                })
              }
            />
          )}
          <div className="field-actions">
            <button
              type="button"
              disabled={blocked}
              onClick={() => stageValue({ state: 'absent' })}
            >
              {t('editor.objects.removeValue')}
            </button>
            {field.defaultValue ? (
              <button
                type="button"
                disabled={blocked}
                onClick={() => stageValue({ state: 'explicit', value: field.defaultValue! })}
              >
                {t('editor.objects.useDefault')}
              </button>
            ) : null}
          </div>
        </>
      )}
      <div id={description} className="field-help">
        <span>
          {t('editor.objects.multiplicity', {
            state: explicit ? t('editor.objects.explicit') : t('editor.objects.notWritten'),
            lower: field.lowerMultiplicity,
            upper: field.upperMultiplicity ?? t('editor.common.unlimited'),
          })}
        </span>
        {field.minimum !== null || field.maximum !== null ? (
          <span>
            {t('editor.objects.range', {
              minimum: field.minimum ?? t('editor.common.unlimited'),
              maximum: field.maximum ?? t('editor.common.unlimited'),
            })}
          </span>
        ) : null}
        {field.defaultValue ? (
          <span>
            {t('editor.objects.default', {
              value: field.defaultValue.lexeme,
              origin: field.defaultOrigin,
            })}
          </span>
        ) : null}
        {errors.map((error, index) => (
          <p className="error-text" key={`${error.code}-${index}`}>
            {error.code} · {text(error.message)}
          </p>
        ))}
      </div>
    </div>
  );
}

export function ObjectInspector({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const object = c.projection?.objects.find((item) => item.objectId === c.activeObjectId);
  const fields =
    c.projection?.fields.filter((field) =>
      c.objectSelection.length > 1
        ? c.objectSelection.includes(field.objectId)
        : field.objectId === c.activeObjectId,
    ) ?? [];
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
          <span className="role-tag">
            {object.kind} ·{' '}
            {object.writable ? t('editor.common.configurable') : t('editor.common.readonly')}
          </span>
          <p className="mono path-text">{object.path}</p>
          <CopyText text={object.path} label={t('editor.objects.copyPath')} />
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
              {fields.map((field) => (
                <div key={field.fieldId}>
                  {c.objectSelection.length > 1 ? (
                    <h3 className="mono path-text">
                      {c.projection?.objects.find((item) => item.objectId === field.objectId)?.path}
                    </h3>
                  ) : null}
                  <DescriptorField field={field} controller={c} />
                </div>
              ))}
              {!fields.length ? (
                <p className="empty-state">{t('editor.objects.noFields')}</p>
              ) : null}
              <div className="inspector-actions">
                <button
                  type="button"
                  onClick={() => void c.prepareChanges()}
                  disabled={!c.changes.length || Boolean(c.busy)}
                >
                  {t('editor.objects.preview')}
                </button>
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
