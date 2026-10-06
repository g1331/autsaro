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
      <h3>实例结构批次</h3>
      <p className="field-help">
        创建、必需字段和子实例在同一批次预览后一次发布。尚未创建对象只使用批次键，不冒充后台
        objectId；默认值不自动落盘。
      </p>
      {options.length ? (
        <label>
          创建允许的实例
          <select
            aria-label="创建实例定义"
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
            <option value="">选择后台提供的定义，加入草稿</option>
            {options.map((option) => (
              <option
                key={option.definition.definitionId}
                value={option.definition.definitionId}
                disabled={!option.definition.writable}
              >
                {option.definition.definitionId} · {option.definition.lowerMultiplicity}–
                {option.definition.upperMultiplicity ?? '不限'}
                {option.definition.reason ? ` · ${option.definition.reason}` : ''}
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
            <h3>待创建 · {creation.definitionId.split('/').pop()}</h3>
            <label>
              SHORT-NAME
              <input
                aria-label={`待创建名称 ${creation.changeId}`}
                value={creation.shortName}
                disabled={blocked}
                onChange={(event) => c.stageChange({ ...creation, shortName: event.target.value })}
              />
            </label>
            <p className="mono path-text">
              待核定路径{' '}
              {proposedPath({ kind: 'created', changeId: creation.changeId }) ??
                '请填写名称并完成父实例'}
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
                    label: `${rawPath} · ${permitted.dest} · 同批待创建`,
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
                      {descriptor.kind} · 必需 {descriptor.lowerMultiplicity} · {descriptor.unit}
                    </small>
                  </label>
                  {!descriptor.writable ? (
                    <p>{descriptor.reason}</p>
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
                        <option value="">明确选择引用</option>
                        {candidates.map((candidate) => (
                          <option key={candidate.key} value={candidate.key}>
                            {candidate.label}
                          </option>
                        ))}
                      </select>
                      {selectedReference ? (
                        <p className="mono path-text">
                          草稿引用 {selectedReference.rawPath} · {selectedReference.dest}
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
                          按当前核定候选更新草稿路径
                        </button>
                      ) : null}
                      <p className="field-help">
                        现有目标与同批目标的定义 / DEST
                        均来自此父实例范围内的后台核定；完整批次预览核对最终图与路径。
                      </p>
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
                      <option value="">未设置</option>
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
                      ? `范围 ${descriptor.minimum ?? '不限'}–${descriptor.maximum ?? '不限'} · `
                      : ''}
                    {descriptor.defaultValue
                      ? `默认 ${descriptor.defaultValue.lexeme} (${descriptor.defaultOrigin})，未落盘`
                      : '无默认值'}
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
                      明确采用默认值
                    </button>
                  ) : null}
                  {change ? (
                    <button
                      type="button"
                      disabled={blocked}
                      onClick={() => stageField(creation, descriptor, { state: 'absent' })}
                    >
                      明确取消字段值
                    </button>
                  ) : null}
                </div>
              );
            })}
            {option?.children.length ? (
              <label>
                同批子实例
                <select
                  value=""
                  aria-label={`同批子实例 ${creation.changeId}`}
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
                  <option value="">选择后台允许的子定义</option>
                  {option.children.map((child) => (
                    <option
                      key={child.definitionId}
                      value={child.definitionId}
                      disabled={!child.writable}
                    >
                      {child.definitionId} · 必需 {child.lowerMultiplicity}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
            {!option ? (
              <p className="error-text">
                此创建项不再具有当前父实例范围内的后台定义描述；请还原批次并重新选择。
              </p>
            ) : null}
          </section>
        );
      })}
    </section>
  );
}
