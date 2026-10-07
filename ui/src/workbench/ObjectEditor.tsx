import { useMemo } from 'react';
import { Search } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import type { FieldDescriptor, ValueState } from './projectTypes';
import { StructureEditor } from './StructureEditor';
import { CopyText } from './Dialog';

export function ObjectEditor({ controller: c }: { controller: Workbench }) {
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
          <h2>配置对象</h2>
          <span>
            {c.projection?.profile} · {c.projection?.release}
          </span>
        </div>
        <span>{c.objectSelection.length} 个已选对象</span>
      </div>
      <label className="search-field">
        <Search size={15} aria-hidden="true" />
        <input
          aria-label="搜索对象表名称、路径、定义、值"
          value={c.objectFilter}
          onChange={(event) => c.setObjectFilter(event.target.value)}
          placeholder="搜索名称 / 路径 / 定义 / 值"
        />
      </label>
      {c.activeObjectId && !rows.some((object) => object.objectId === c.activeObjectId) ? (
        <p className="filter-retained">
          当前对象不在筛选结果中，选择与草稿保留。
          <button onClick={() => c.setObjectFilter('')}>清空筛选</button>
        </p>
      ) : null}
      <div className="table-wrap">
        <table>
          <caption>真实源对象投影</caption>
          <thead>
            <tr>
              <th scope="col">选择</th>
              <th scope="col">名称</th>
              <th scope="col">类型</th>
              <th scope="col">定义 / 路径</th>
              <th scope="col">访问</th>
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
                    aria-label={`批次选择 ${object.path}`}
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
                  <span>{object.definitionId ?? '无可核定定义'}</span>
                  <small className="path-text">{object.path}</small>
                </td>
                <td>
                  {object.writable ? '可配置' : '只读'}
                  <small>{object.reason}</small>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="batch-bar">
        <span>{c.changes.length} 项未应用变化 · 预览不写入</span>
        <button
          type="button"
          onClick={() => void c.prepareChanges()}
          disabled={!c.changes.length || Boolean(c.busy)}
        >
          预览整批影响
        </button>
        <button
          type="button"
          onClick={() => c.discardChanges()}
          disabled={!c.changes.length || Boolean(c.busy)}
        >
          还原批次草稿
        </button>
      </div>
      {c.changes.length ? (
        <details>
          <summary>检查待应用操作</summary>
          {c.changes.map((change) => (
            <div className="pending-change" key={change.changeId}>
              <span>
                {change.op} · {change.changeId}
              </span>
              <button type="button" onClick={() => c.discardChanges(change.changeId)}>
                移除此操作
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
        {field.definitionId.split('/').pop()}{' '}
        <small>
          {field.kind ?? field.elementKind}
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
                : (field.defaultValue?.lexeme ?? '未设置')}
          </span>
          <p>{field.reason ?? '此定义未提供编辑能力'}</p>
        </div>
      ) : field.kind === 'reference' ? (
        <>
          <p className="mono path-text">
            {currentReference?.state === 'explicit'
              ? `${currentReference.rawPath} · DEST=${currentReference.dest}`
              : '未设置引用'}
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
            <option value="">选择后台核定目标</option>
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
            明确移除引用条目
          </button>
          {!candidates.length ? (
            <p className="field-help">当前后台没有核定合法引用候选；不自由填写未核定对象。</p>
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
              <option value="">{explicit ? '显式空值' : '未设置'}</option>
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
              移除显式值
            </button>
            {field.defaultValue ? (
              <button
                type="button"
                disabled={blocked}
                onClick={() => stageValue({ state: 'explicit', value: field.defaultValue! })}
              >
                明确采用默认值
              </button>
            ) : null}
          </div>
        </>
      )}
      <div id={description} className="field-help">
        <span>
          {explicit ? '显式值' : '未落盘'} · 基数 {field.lowerMultiplicity}–
          {field.upperMultiplicity ?? '不限'}
        </span>
        {field.minimum !== null || field.maximum !== null ? (
          <span>
            范围 {field.minimum ?? '不限'}–{field.maximum ?? '不限'}
          </span>
        ) : null}
        {field.defaultValue ? (
          <span>
            默认 {field.defaultValue.lexeme} · {field.defaultOrigin}
          </span>
        ) : null}
        {errors.map((error, index) => (
          <p className="error-text" key={`${error.code}-${index}`}>
            {error.code} · {error.message}
          </p>
        ))}
      </div>
    </div>
  );
}

export function ObjectInspector({ controller: c }: { controller: Workbench }) {
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
            {object.kind} · {object.writable ? '可配置' : '只读'}
          </span>
          <p className="mono path-text">{object.path}</p>
          <CopyText text={object.path} label="复制对象路径" />
          <p className="mono path-text">
            {c.projection?.sources.find((source) => source.sourceId === object.sourceId)?.path}
          </p>
          {object.reason ? <p className="field-help">{object.reason}</p> : null}
          {c.inspectorTab === 'properties' ? (
            <>
              {object.writable && !c.actionReason('prepare-change') ? (
                <label className="descriptor-field">
                  实例名称
                  <input
                    aria-label="实例名称"
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
                <p className="empty-state">当前对象没有可核定参数条目；未知内容保持原文。</p>
              ) : null}
              <div className="inspector-actions">
                <button
                  type="button"
                  onClick={() => void c.prepareChanges()}
                  disabled={!c.changes.length || Boolean(c.busy)}
                >
                  预览更改
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
                    加入移除批次
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
                      {incoming ? '入站' : '出站'} · {edge.dest}
                    </strong>
                    <p className="mono path-text">{edge.rawPath}</p>
                    {target ? (
                      <button type="button" onClick={() => void c.selectObject(target.objectId)}>
                        {target.shortName} · 定位
                      </button>
                    ) : (
                      <p>{edge.reason ?? '未解析目标'}</p>
                    )}
                  </section>
                );
              })}
              {!references.length ? <p>后台索引没有此对象的引用。</p> : null}
            </div>
          )}
        </>
      ) : (
        <p className="empty-state">选择真实配置对象以检查属性、定义和引用。</p>
      )}
    </div>
  );
}
