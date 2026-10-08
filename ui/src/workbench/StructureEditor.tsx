import { editorValueKindKeys } from '../i18n/editors';
import { useLocale } from '../i18n';
import type { Workbench } from './useWorkbench';
import type {
  ConfigurationChange,
  DefinitionDescriptor,
  InstanceDefinitionOption,
  ObjectProjection,
  ObjectRef,
  ReferenceState,
  ValueState,
} from './projectTypes';

type Creation = Extract<ConfigurationChange, { op: 'create-instance' }>;

export function StructureEditor({
  controller: c,
  object,
}: {
  controller: Workbench;
  object: ObjectProjection;
}) {
  const { t, text } = useLocale();
  const options =
    c.projection?.instanceDefinitions.filter((option) => option.parentId === object.objectId) ?? [];
  const creations = c.changes.filter(
    (change): change is Creation => change.op === 'create-instance',
  );
  const blocked = Boolean(c.actionReason('prepare-change'));
  function rootOption(
    creation: Creation,
    seen = new Set<string>(),
  ): InstanceDefinitionOption | undefined {
    if (seen.has(creation.changeId)) return undefined;
    seen.add(creation.changeId);
    const parent = creation.parent;
    if (parent.kind === 'existing')
      return c.projection?.instanceDefinitions.find(
        (item) =>
          item.parentId === parent.objectId &&
          item.definition.definitionId === creation.definitionId,
      );
    const owner = creations.find((item) => item.changeId === parent.changeId);
    return owner ? rootOption(owner, seen) : undefined;
  }
  function proposedPath(target: ObjectRef, seen = new Set<string>()): string | null {
    if (target.kind === 'existing')
      return c.projection?.objects.find((item) => item.objectId === target.objectId)?.path ?? null;
    if (seen.has(target.changeId)) return null;
    seen.add(target.changeId);
    const creation = creations.find((item) => item.changeId === target.changeId);
    if (!creation?.shortName) return null;
    const parent = proposedPath(creation.parent, seen);
    return parent ? `${parent}/${creation.shortName}` : null;
  }
  function addCreation(definitionId: string, parent: ObjectRef, sourceId: string) {
    c.stageChange({
      op: 'create-instance',
      changeId: crypto.randomUUID(),
      parent,
      sourceId,
      definitionId,
      shortName: '',
    });
  }
  function stageField(
    creation: Creation,
    descriptor: DefinitionDescriptor,
    value: ValueState | ReferenceState,
  ) {
    const previous = c.changes.find(
      (change) =>
        (change.op === 'set-value' || change.op === 'set-reference') &&
        change.field.kind === 'new' &&
        change.field.object.kind === 'created' &&
        change.field.object.changeId === creation.changeId &&
        change.field.definitionId === descriptor.definitionId,
    );
    const entryKey =
      previous &&
      (previous.op === 'set-value' || previous.op === 'set-reference') &&
      previous.field.kind === 'new'
        ? previous.field.entryKey
        : crypto.randomUUID();
    const field = {
      kind: 'new' as const,
      object: { kind: 'created' as const, changeId: creation.changeId },
      definitionId: descriptor.definitionId,
      entryKey,
    };
    if (descriptor.kind === 'reference')
      c.stageChange({
        op: 'set-reference',
        changeId: previous?.changeId ?? crypto.randomUUID(),
        field,
        expected: { state: 'absent' },
        value: value as ReferenceState,
      });
    else
      c.stageChange({
        op: 'set-value',
        changeId: previous?.changeId ?? crypto.randomUUID(),
        field,
        expected: { state: 'absent' },
        value: value as ValueState,
      });
  }
  if (!options.length && !creations.length) return null;
  return (
    <section className="structure-editor">
      <h3>{t('editor.structure.heading')}</h3>
      <p className="field-help">{t('editor.structure.intro')}</p>
      {options.length ? (
        <label>
          {t('editor.structure.create')}
          <select
            aria-label={t('editor.structure.definitionLabel')}
            value=""
            disabled={blocked || !object.writable}
            onChange={(event) => {
              if (event.target.value)
                addCreation(
                  event.target.value,
                  { kind: 'existing', objectId: object.objectId },
                  object.sourceId,
                );
            }}
          >
            <option value="">{t('editor.structure.chooseDefinition')}</option>
            {options.map((option) => (
              <option
                key={option.definition.definitionId}
                value={option.definition.definitionId}
                disabled={!option.definition.writable}
              >
                {option.definition.definitionId} · {option.definition.lowerMultiplicity}–
                {option.definition.upperMultiplicity ?? t('editor.common.unlimited')}
                {option.definition.reason ? ` · ${text(option.definition.reason)}` : ''}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      {creations.map((creation) => {
        const root = rootOption(creation);
        const option =
          root?.definition.definitionId === creation.definitionId
            ? root
            : root?.recursiveDefinitions.find(
                (item) => item.definition.definitionId === creation.definitionId,
              );
        return (
          <section key={creation.changeId} className="pending-creation">
            <h3>
              {t('editor.structure.pendingCreation', {
                name: creation.definitionId.split('/').pop() ?? creation.definitionId,
              })}
            </h3>
            <label>
              SHORT-NAME
              <input
                aria-label={t('editor.structure.pendingName', { id: creation.changeId })}
                value={creation.shortName}
                disabled={blocked}
                onChange={(event) => c.stageChange({ ...creation, shortName: event.target.value })}
              />
            </label>
            <p className="mono path-text">
              {t('editor.structure.pendingPath')}{' '}
              {proposedPath({ kind: 'created', changeId: creation.changeId }) ??
                t('editor.structure.needName')}
            </p>
            {option?.fields.map((descriptor) => {
              const change = c.changes.find(
                (item) =>
                  (item.op === 'set-value' || item.op === 'set-reference') &&
                  item.field.kind === 'new' &&
                  item.field.object.kind === 'created' &&
                  item.field.object.changeId === creation.changeId &&
                  item.field.definitionId === descriptor.definitionId,
              );
              const state =
                change?.op === 'set-value' ? change.value : { state: 'absent' as const };
              const lexeme = state.state === 'explicit' ? state.value.lexeme : '';
              const references = option.referenceOptions.find(
                (item) => item.fieldDefinitionId === descriptor.definitionId,
              );
              const candidates: {
                key: string;
                value: Extract<ReferenceState, { state: 'explicit' }>;
                label: string;
              }[] = [];
              for (const target of references?.existingTargets ?? [])
                candidates.push({
                  key: `existing:${target.targetId}:${target.dest}`,
                  value: {
                    state: 'explicit',
                    rawPath: target.path,
                    dest: target.dest,
                    target: { kind: 'existing', objectId: target.targetId },
                  },
                  label: `${target.path} · ${target.dest}`,
                });
              for (const target of creations) {
                const rawPath = proposedPath({ kind: 'created', changeId: target.changeId });
                if (!rawPath) continue;
                for (const permitted of references?.createdTargets ?? []) {
                  if (permitted.definitionId !== target.definitionId) continue;
                  candidates.push({
                    key: `created:${target.changeId}:${permitted.dest}`,
                    value: {
                      state: 'explicit',
                      rawPath,
                      dest: permitted.dest,
                      target: { kind: 'created', changeId: target.changeId },
                    },
                    label: t('editor.structure.createdTarget', {
                      path: rawPath,
                      dest: permitted.dest,
                    }),
                  });
                }
              }
              const selectedReference =
                change?.op === 'set-reference' && change.value.state === 'explicit'
                  ? change.value
                  : null;
              let selectedKey = '';
              if (selectedReference?.target?.kind === 'existing')
                selectedKey = `existing:${selectedReference.target.objectId}:${selectedReference.dest}`;
              else if (selectedReference?.target?.kind === 'created')
                selectedKey = `created:${selectedReference.target.changeId}:${selectedReference.dest}`;
              const selectedCandidate = candidates.find(
                (candidate) => candidate.key === selectedKey,
              );
              return (
                <div className="descriptor-field" key={descriptor.definitionId}>
                  <label htmlFor={`new-field-${creation.changeId}-${descriptor.definitionId}`}>
                    {descriptor.definitionId.split('/').pop()}{' '}
                    <small>
                      {t('editor.structure.requiredField', {
                        kind: descriptor.kind
                          ? t(editorValueKindKeys[descriptor.kind])
                          : descriptor.elementKind,
                        count: descriptor.lowerMultiplicity,
                        unit: descriptor.unit,
                      })}
                    </small>
                  </label>
                  {!descriptor.writable ? (
                    <p>{text(descriptor.reason ?? '')}</p>
                  ) : descriptor.kind === 'reference' ? (
                    <>
                      <select
                        id={`new-field-${creation.changeId}-${descriptor.definitionId}`}
                        value={selectedKey}
                        disabled={blocked || !references}
                        onChange={(event) => {
                          const candidate = candidates.find(
                            (item) => item.key === event.target.value,
                          );
                          if (candidate) stageField(creation, descriptor, candidate.value);
                        }}
                      >
                        <option value="">{t('editor.structure.chooseReference')}</option>
                        {candidates.map((candidate) => (
                          <option key={candidate.key} value={candidate.key}>
                            {candidate.label}
                          </option>
                        ))}
                      </select>
                      {selectedReference ? (
                        <p className="mono path-text">
                          {t('editor.structure.draftReference', {
                            path: selectedReference.rawPath,
                            dest: selectedReference.dest,
                          })}
                        </p>
                      ) : null}
                      {selectedCandidate &&
                      selectedReference &&
                      selectedCandidate.value.rawPath !== selectedReference.rawPath ? (
                        <button
                          type="button"
                          disabled={blocked}
                          onClick={() => stageField(creation, descriptor, selectedCandidate.value)}
                        >
                          {t('editor.structure.updatePath')}
                        </button>
                      ) : null}
                      <p className="field-help">{t('editor.structure.referenceHelp')}</p>
                    </>
                  ) : descriptor.kind === 'boolean' || descriptor.enumeration.length ? (
                    <select
                      id={`new-field-${creation.changeId}-${descriptor.definitionId}`}
                      value={lexeme}
                      disabled={blocked}
                      onChange={(event) =>
                        stageField(
                          creation,
                          descriptor,
                          event.target.value
                            ? {
                                state: 'explicit',
                                value: { kind: descriptor.kind!, lexeme: event.target.value },
                              }
                            : { state: 'absent' },
                        )
                      }
                    >
                      <option value="">{t('editor.common.unset')}</option>
                      {(descriptor.kind === 'boolean'
                        ? ['true', 'false']
                        : descriptor.enumeration
                      ).map((entry) => (
                        <option key={entry} value={entry}>
                          {entry}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      id={`new-field-${creation.changeId}-${descriptor.definitionId}`}
                      value={lexeme}
                      disabled={blocked}
                      onChange={(event) =>
                        stageField(creation, descriptor, {
                          state: 'explicit',
                          value: { kind: descriptor.kind!, lexeme: event.target.value },
                        })
                      }
                    />
                  )}
                  <p className="field-help">
                    {descriptor.minimum !== null || descriptor.maximum !== null
                      ? t('editor.structure.range', {
                          minimum: descriptor.minimum ?? t('editor.common.unlimited'),
                          maximum: descriptor.maximum ?? t('editor.common.unlimited'),
                        })
                      : ''}
                    {descriptor.defaultValue
                      ? t('editor.structure.default', {
                          value: descriptor.defaultValue.lexeme,
                          origin: descriptor.defaultOrigin,
                        })
                      : t('editor.structure.noDefault')}
                  </p>
                  {descriptor.defaultValue && descriptor.writable ? (
                    <button
                      type="button"
                      disabled={blocked}
                      onClick={() =>
                        stageField(creation, descriptor, {
                          state: 'explicit',
                          value: descriptor.defaultValue!,
                        })
                      }
                    >
                      {t('editor.objects.useDefault')}
                    </button>
                  ) : null}
                  {change ? (
                    <button
                      type="button"
                      disabled={blocked}
                      onClick={() => stageField(creation, descriptor, { state: 'absent' })}
                    >
                      {t('editor.structure.cancelValue')}
                    </button>
                  ) : null}
                </div>
              );
            })}
            {option?.children.length ? (
              <label>
                {t('editor.structure.children')}
                <select
                  value=""
                  aria-label={t('editor.structure.childLabel', { id: creation.changeId })}
                  disabled={blocked}
                  onChange={(event) => {
                    if (event.target.value)
                      addCreation(
                        event.target.value,
                        { kind: 'created', changeId: creation.changeId },
                        creation.sourceId,
                      );
                  }}
                >
                  <option value="">{t('editor.structure.chooseChild')}</option>
                  {option.children.map((child) => (
                    <option
                      key={child.definitionId}
                      value={child.definitionId}
                      disabled={!child.writable}
                    >
                      {t('editor.structure.childRequired', {
                        definition: child.definitionId,
                        count: child.lowerMultiplicity,
                      })}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
            {!option ? <p className="error-text">{t('editor.structure.missingOption')}</p> : null}
          </section>
        );
      })}
    </section>
  );
}
