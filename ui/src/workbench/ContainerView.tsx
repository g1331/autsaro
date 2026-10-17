import { useMemo, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { CircleX, TriangleAlert } from 'lucide-react';
import { ContextMenu } from 'radix-ui';
import { useLocale } from '../i18n';
import { editorValueKindKeys } from '../i18n/editors';
import type { Workbench } from './useWorkbench';
import type {
  ConfigurationChange,
  FieldDescriptor,
  ObjectProjection,
  ReferenceState,
  ValueState,
} from './projectTypes';
import { fieldName } from './batchEditing';
import { iconSize } from './icons';
import { objectRoleClass } from './objectRole';
import { fieldState, groupedObjectTree, moduleLabel, multiplicity } from './projectGroups';
import type { ObjectMarks, TreeNode } from './projectGroups';
import { MarkIcons, markLabels, useObjectMarks } from './ObjectMarks';
import {
  ObjectContextMenu,
  addChild,
  addableDefinitions,
  childCount,
  definitionName,
} from './ObjectMenu';
import { BatchBar, ObjectEditor } from './ObjectEditor';
import { Tabs, tabPanelProps } from './Tabs';
import { profileLabel } from './labels';

/** Decimal or `0x` hexadecimal integer lexeme, as the backend accepts it; null otherwise. */
export function parseInteger(lexeme: string): bigint | null {
  const match = /^([+-]?)(0x[0-9a-f]+|\d+)$/i.exec(lexeme.trim());
  if (!match) return null;
  const magnitude = BigInt(match[2]);
  return match[1] === '-' ? -magnitude : magnitude;
}

/** The other base of an integer lexeme: hexadecimal for decimal input and the reverse. */
export function alternateBase(lexeme: string): string | null {
  const value = parseInteger(lexeme);
  if (value === null) return null;
  if (/0x/i.test(lexeme)) return value.toString();
  const magnitude = value < 0n ? -value : value;
  return `${value < 0n ? '-' : ''}0x${magnitude.toString(16).toUpperCase()}`;
}

/** Immediate hint only; the backend still validates on apply. Lexemes can exceed Number precision. */
export function outsideRange(
  lexeme: string,
  field: Pick<FieldDescriptor, 'minimum' | 'maximum'>,
): boolean {
  const value = parseInteger(lexeme);
  if (value === null) return false;
  try {
    return (
      (field.minimum !== null && value < BigInt(field.minimum)) ||
      (field.maximum !== null && value > BigInt(field.maximum))
    );
  } catch {
    return false;
  }
}

/** The staged value or reference change of an existing field. */
export function stagedField(changes: readonly ConfigurationChange[], fieldId: string) {
  return changes.find(
    (change): change is Extract<ConfigurationChange, { op: 'set-value' | 'set-reference' }> =>
      (change.op === 'set-value' || change.op === 'set-reference') &&
      change.field.kind === 'existing' &&
      change.field.fieldId === fieldId,
  );
}

type View = 'container' | 'objects';

/** The configuration document: the container view, or all objects for search and batch selection. */
export function ConfigurationDocument({ controller: c }: { controller: Workbench }) {
  const { t } = useLocale();
  const [view, setView] = useState<View>('container');
  return (
    <div className="document-page configuration-document">
      <div className="document-heading">
        <Tabs<View>
          id="configuration-view"
          className="view-tabs"
          label={t('editor.container.views')}
          selected={view}
          onSelect={setView}
          items={[
            { key: 'container', label: t('editor.container.view') },
            { key: 'objects', label: t('editor.container.allObjects') },
          ]}
        />
        <span>
          {c.projection ? profileLabel(c.projection.profile) : null} · {c.projection?.release}
        </span>
      </div>
      <div {...tabPanelProps('configuration-view', view)}>
        {view === 'container' ? <ContainerView controller={c} /> : <ObjectEditor controller={c} />}
      </div>
      <BatchBar controller={c} />
    </div>
  );
}

export function ContainerView({ controller: c }: { controller: Workbench }) {
  const { t } = useLocale();
  const tree = useMemo(() => groupedObjectTree(c.projection?.objects ?? []), [c.projection]);
  const marks = useObjectMarks(c, tree);
  const object = c.projection?.objects.find((item) => item.objectId === c.activeObjectId);
  if (!c.projection) return <p className="empty-state">{t('editor.tree.loading')}</p>;
  if (!object) return <ModuleOverview controller={c} tree={tree} marks={marks} />;
  const selected =
    c.objectSelection.length > 1
      ? c.projection.objects.filter((item) => c.objectSelection.includes(item.objectId))
      : [object];
  const definition =
    object.definitionId && object.definitionId !== object.kind ? object.definitionId : null;
  return (
    <>
      <header className="container-heading">
        <h2>{moduleLabel(object)}</h2>
        <span className={['role-tag', objectRoleClass(object)].filter(Boolean).join(' ')}>
          {object.kind} ·{' '}
          {object.writable ? t('editor.common.configurable') : t('editor.common.readonly')}
        </span>
        {definition ? <p className="mono path-text">{definition}</p> : null}
        <p className="mono path-text">{object.path}</p>
      </header>
      {selected.length === 1 ? (
        <ChildContainers controller={c} object={object} marks={marks} />
      ) : null}
      <ParameterGrid controller={c} objects={selected} />
    </>
  );
}

function ModuleOverview({
  controller: c,
  tree,
  marks,
}: {
  controller: Workbench;
  tree: TreeNode[];
  marks: Map<string, ObjectMarks>;
}) {
  const { t } = useLocale();
  const modules = tree.find((node) => node.key === 'group:modules')?.children ?? [];
  const descendants = (node: TreeNode): number =>
    node.children.reduce((sum, child) => sum + 1 + descendants(child), 0);
  if (!modules.length) return <p className="empty-state">{t('editor.objects.empty')}</p>;
  return (
    <section className="container-section">
      <h2>{t('editor.tree.group.modules')}</h2>
      <p className="field-help">{t('editor.container.overviewHint')}</p>
      <div className="table-wrap">
        <table className="data-table">
          <thead>
            <tr>
              <th scope="col">{t('editor.container.module')}</th>
              <th scope="col">{t('editor.common.name')}</th>
              <th scope="col">{t('editor.container.containerCount')}</th>
              <th scope="col">{t('editor.container.status')}</th>
            </tr>
          </thead>
          <tbody>
            {modules.map((node) =>
              node.type === 'object' ? (
                <tr key={node.key} data-object-row={node.key}>
                  <td>
                    <button
                      type="button"
                      className="table-link"
                      onClick={() => void c.selectObject(node.key)}
                    >
                      {moduleLabel(node.object)}
                    </button>
                  </td>
                  <td>{node.object.shortName}</td>
                  <td className="numeric">{descendants(node)}</td>
                  <td>
                    <MarkCell mark={marks.get(node.key)} />
                  </td>
                </tr>
              ) : null,
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function MarkCell({ mark }: { mark: ObjectMarks | undefined }) {
  const labels = markLabels(mark);
  return (
    <span title={labels.join('\n') || undefined}>
      <MarkIcons mark={mark} />
      <span className="visually-hidden">{labels.join(' · ')}</span>
    </span>
  );
}

function ChildContainers({
  controller: c,
  object,
  marks,
}: {
  controller: Workbench;
  object: ObjectProjection;
  marks: Map<string, ObjectMarks>;
}) {
  const { t } = useLocale();
  const groups = new Map<
    string,
    {
      definitionId: string;
      lower: number | null;
      upper: number | null;
      members: ObjectProjection[];
    }
  >();
  for (const option of c.projection?.instanceDefinitions ?? [])
    if (option.parentId === object.objectId)
      groups.set(option.definition.definitionId, {
        definitionId: option.definition.definitionId,
        lower: option.definition.lowerMultiplicity,
        upper: option.definition.upperMultiplicity,
        members: [],
      });
  for (const child of c.projection?.objects ?? []) {
    if (child.parentId !== object.objectId) continue;
    const key = child.definitionId ?? child.kind;
    const group = groups.get(key) ?? { definitionId: key, lower: null, upper: null, members: [] };
    group.members.push(child);
    groups.set(key, group);
  }
  if (!groups.size) return null;
  const addable = new Set(
    addableDefinitions(c, object).map((option) => option.definition.definitionId),
  );
  const blocked = Boolean(c.actionReason('prepare-change'));
  return (
    <section className="container-section">
      <h3>{t('editor.container.children')}</h3>
      <div className="table-wrap">
        <table className="data-table child-table">
          <colgroup>
            <col className="col-name" />
            <col />
            <col className="col-status" />
          </colgroup>
          <thead>
            <tr>
              <th scope="col">{t('editor.common.name')}</th>
              <th scope="col">{t('editor.objects.definition')}</th>
              <th scope="col">{t('editor.container.status')}</th>
            </tr>
          </thead>
          {[...groups.values()].map((group) => {
            const name = definitionName(group.definitionId);
            const count =
              group.lower === null
                ? group.members.length
                : childCount(c, object.objectId, group.definitionId);
            const missing = group.lower === null ? 0 : group.lower - count;
            return (
              <tbody key={group.definitionId}>
                <tr className="row-group">
                  <th scope="rowgroup" colSpan={3}>
                    <span>{name}</span>
                    <small>
                      {group.lower === null
                        ? t('editor.container.count', { count })
                        : t('editor.container.countOf', {
                            count,
                            multiplicity: multiplicity(group.lower, group.upper),
                          })}
                    </small>
                    {addable.has(group.definitionId) && !blocked ? (
                      <button
                        type="button"
                        className="ghost-button"
                        onClick={() => addChild(c, object, group.definitionId)}
                      >
                        {t('editor.container.add', { name })}
                      </button>
                    ) : null}
                  </th>
                </tr>
                {group.members.map((child) => (
                  <ObjectContextMenu key={child.objectId} controller={c} object={child}>
                    <tr data-object-row={child.objectId}>
                      <td title={child.path}>
                        <button
                          type="button"
                          className="table-link"
                          onClick={() => void c.selectObject(child.objectId)}
                        >
                          {child.shortName}
                        </button>
                      </td>
                      <td className="mono secondary" title={child.definitionId ?? child.kind}>
                        {child.definitionId ?? child.kind}
                      </td>
                      <td>
                        <MarkCell mark={marks.get(child.objectId)} />
                      </td>
                    </tr>
                  </ObjectContextMenu>
                ))}
                {missing > 0 ? (
                  <tr className="row-missing">
                    <td colSpan={3}>
                      <TriangleAlert size={iconSize.inline} aria-hidden="true" />
                      {t('editor.container.missing', { count: missing, name })}
                    </td>
                  </tr>
                ) : null}
              </tbody>
            );
          })}
        </table>
      </div>
    </section>
  );
}

function ParameterGrid({
  controller: c,
  objects,
}: {
  controller: Workbench;
  objects: ObjectProjection[];
}) {
  const { t } = useLocale();
  const sections = objects.map((object) => ({
    object,
    fields: c.projection?.fields.filter((field) => field.objectId === object.objectId) ?? [],
  }));
  return (
    <section className="container-section">
      <h3>{t('editor.container.parameters')}</h3>
      {sections.some((section) => section.fields.length) ? (
        <div className="table-wrap">
          <table className="data-table parameter-grid">
            <colgroup>
              <col className="col-name" />
              <col className="col-value" />
              <col className="col-unit" />
              <col className="col-default" />
              <col className="col-range" />
              <col className="col-multiplicity" />
              <col className="col-status" />
            </colgroup>
            <thead>
              <tr>
                <th scope="col">{t('editor.container.parameter')}</th>
                <th scope="col">{t('editor.container.value')}</th>
                <th scope="col">{t('editor.container.unit')}</th>
                <th scope="col">{t('editor.container.default')}</th>
                <th scope="col">{t('editor.container.range')}</th>
                <th scope="col">{t('editor.container.multiplicity')}</th>
                <th scope="col">{t('editor.container.status')}</th>
              </tr>
            </thead>
            {sections.map(({ object, fields }) =>
              fields.length ? (
                <tbody key={object.objectId}>
                  {sections.length > 1 ? (
                    <tr className="row-group">
                      <th scope="rowgroup" colSpan={7} className="mono">
                        {object.path}
                      </th>
                    </tr>
                  ) : null}
                  {fields.map((field) => (
                    <ParameterRow key={field.fieldId} controller={c} field={field} />
                  ))}
                </tbody>
              ) : null,
            )}
          </table>
        </div>
      ) : (
        <p className="empty-state">{t('editor.objects.noFields')}</p>
      )}
    </section>
  );
}

function ParameterRow({ controller: c, field }: { controller: Workbench; field: FieldDescriptor }) {
  const { t, text } = useLocale();
  const staged = stagedField(c.changes, field.fieldId);
  const value: ValueState = staged?.op === 'set-value' ? staged.value : field.current;
  const explicit = value.state === 'explicit';
  const lexeme = explicit ? value.value.lexeme : '';
  const reference = staged?.op === 'set-reference' ? staged.value : field.reference;
  const diagnostics =
    c.projection?.diagnostics.filter((issue) => issue.fieldId === field.fieldId) ?? [];
  const errors = diagnostics.filter((issue) => issue.severity === 'error');
  const warnings = diagnostics.filter((issue) => issue.severity === 'warning');
  const rejection = c.fieldErrors[field.fieldId];
  const outOfRange = field.kind === 'integer' && explicit && outsideRange(lexeme, field);
  const invalid = errors.length > 0 || Boolean(rejection) || outOfRange;
  // Busy only pauses input (read-only keeps focus); other reasons make the whole grid read-only.
  const blocked = Boolean(c.actionReason('prepare-change')) && !c.busy;
  const editable = field.writable && !blocked;
  const modified = Boolean(c.savedValues) && c.savedValues?.[field.fieldId] !== fieldState(field);
  const nonDefault =
    explicit && field.defaultValue !== null && lexeme !== field.defaultValue.lexeme;
  const name = fieldName(field);
  const errorId = `field-error-${field.fieldId}`;
  const candidates =
    c.projection?.referenceCandidates.filter((candidate) => candidate.fieldId === field.fieldId) ??
    [];
  const stage = (change: ConfigurationChange) => {
    if (!c.busy) c.stageChange(change);
  };
  const stageValue = (next: ValueState) =>
    stage({
      op: 'set-value',
      changeId: field.fieldId,
      field: { kind: 'existing', fieldId: field.fieldId },
      expected: field.current,
      value: next,
    });
  const stageReference = (next: ReferenceState) =>
    stage({
      op: 'set-reference',
      changeId: field.fieldId,
      field: { kind: 'existing', fieldId: field.fieldId },
      expected: field.reference ?? { state: 'absent' },
      value: next,
    });
  const apply = () => void c.applyField(field.fieldId);
  const keyDown = (event: KeyboardEvent<HTMLElement>) => {
    // Enter during IME composition commits the candidate, not the value.
    if (!staged || event.nativeEvent.isComposing) return;
    if (event.key === 'Enter') {
      event.preventDefault();
      apply();
    } else if (event.key === 'Escape') {
      // Escape reverts this cell only; it must not also close a tool window.
      event.preventDefault();
      event.stopPropagation();
      c.discardChanges(field.fieldId);
    }
  };
  const control = {
    id: `field-${field.fieldId}`,
    'data-field-id': field.fieldId,
    'aria-label': name,
    'aria-invalid': invalid,
    'aria-describedby': invalid ? errorId : undefined,
    onKeyDown: keyDown,
    onFocus: () => c.setActiveFieldId(field.fieldId),
  };
  const shownReference =
    reference?.state === 'explicit' ? `${reference.rawPath} · DEST=${reference.dest}` : null;
  let cell;
  if (!editable)
    cell = (
      <span
        className="readonly-cell mono"
        tabIndex={-1}
        data-field-id={field.fieldId}
        onFocus={control.onFocus}
      >
        {shownReference ?? (explicit ? lexeme : t('editor.common.unset'))}
        {field.writable ? null : (
          <small>{field.reason ? text(field.reason) : t('editor.objects.noCapability')}</small>
        )}
      </span>
    );
  else if (field.kind === 'reference')
    cell = (
      <select
        {...control}
        value={
          reference?.state === 'explicit' && reference.target?.kind === 'existing'
            ? `${reference.target.objectId}:${reference.dest}`
            : ''
        }
        title={shownReference ?? undefined}
        onChange={(event) => {
          const candidate = candidates.find(
            (item) => `${item.targetId}:${item.dest}` === event.target.value,
          );
          if (candidate)
            stageReference({
              state: 'explicit',
              rawPath: candidate.path,
              dest: candidate.dest,
              target: { kind: 'existing', objectId: candidate.targetId },
            });
        }}
      >
        <option value="">
          {reference?.state === 'explicit' && reference.target?.kind !== 'existing'
            ? reference.rawPath
            : t('editor.objects.chooseTarget')}
        </option>
        {candidates.map((candidate) => (
          <option
            key={`${candidate.targetId}:${candidate.dest}`}
            value={`${candidate.targetId}:${candidate.dest}`}
          >
            {candidate.shortName} · {candidate.path} · {candidate.dest}
          </option>
        ))}
      </select>
    );
  else if (field.kind === 'boolean' || field.enumeration.length)
    cell = (
      <select
        {...control}
        value={lexeme}
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
    );
  else
    cell = (
      <input
        {...control}
        value={lexeme}
        placeholder={
          explicit ? undefined : (field.defaultValue?.lexeme ?? t('editor.common.unset'))
        }
        readOnly={Boolean(c.busy)}
        onChange={(event) =>
          stageValue({
            state: 'explicit',
            value: { kind: field.kind!, lexeme: event.target.value },
          })
        }
      />
    );
  const alternate = field.kind === 'integer' && explicit ? alternateBase(lexeme) : null;
  const problemText = [
    ...(rejection ? [text(rejection)] : []),
    ...errors.map((error) => `${error.code} · ${text(error.message)}`),
  ];
  const row = (
    <tr
      data-parameter-row={field.fieldId}
      className={
        [staged ? 'unapplied' : '', c.activeFieldId === field.fieldId ? 'current' : '']
          .filter(Boolean)
          .join(' ') || undefined
      }
    >
      <th scope="row" title={field.definitionId}>
        <span>{name}</span>
        <small>{field.kind ? t(editorValueKindKeys[field.kind]) : field.elementKind}</small>
      </th>
      <td>
        <div className="value-cell">
          {cell}
          {alternate ? (
            <small className="mono" title={alternate}>
              {alternate}
            </small>
          ) : null}
        </div>
        {invalid ? (
          <div id={errorId} className="field-error">
            {rejection ? <p>{text(rejection)}</p> : null}
            {!rejection && outOfRange ? (
              <p>
                {t('editor.objects.outOfRange', {
                  minimum: field.minimum ?? '−∞',
                  maximum: field.maximum ?? '+∞',
                })}
              </p>
            ) : null}
            {errors.map((error, index) => (
              <p key={`${error.code}-${index}`}>
                {error.code} · {text(error.message)}
              </p>
            ))}
          </div>
        ) : null}
      </td>
      <td className="mono secondary">{field.unit ?? ''}</td>
      <td className="mono secondary">{field.defaultValue?.lexeme ?? ''}</td>
      <td className="mono secondary">
        {field.minimum !== null || field.maximum !== null
          ? `${field.minimum ?? '−∞'}..${field.maximum ?? '+∞'}`
          : ''}
      </td>
      <td className="mono secondary">
        {multiplicity(field.lowerMultiplicity, field.upperMultiplicity)}
      </td>
      <td className="parameter-status">
        {staged ? (
          <>
            <span className="status-unapplied">{t('editor.container.unapplied')}</span>
            <button type="button" disabled={Boolean(c.busy)} onClick={apply}>
              {t('action.applyParameter')}
            </button>
          </>
        ) : null}
        {problemText.length ? (
          <span className="status-error" title={problemText.join('\n')}>
            <CircleX size={iconSize.inline} aria-hidden="true" />
            {t('editor.severity.error')}
          </span>
        ) : null}
        {warnings.length ? (
          <span
            className="status-warning"
            title={warnings.map((issue) => `${issue.code} · ${text(issue.message)}`).join('\n')}
          >
            <TriangleAlert size={iconSize.inline} aria-hidden="true" />
            {t('editor.severity.warning')}
          </span>
        ) : null}
        {modified ? <span>{t('editor.container.modified')}</span> : null}
        {nonDefault ? <span>{t('editor.container.nonDefault')}</span> : null}
      </td>
    </tr>
  );
  if (!editable) return row;
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild>{row}</ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content className="menu-popup" loop>
          {staged ? (
            <>
              <ContextMenu.Item className="menu-item" onSelect={apply}>
                <span>{t('action.applyParameter')}</span>
              </ContextMenu.Item>
              <ContextMenu.Item
                className="menu-item"
                onSelect={() => c.discardChanges(field.fieldId)}
              >
                <span>{t('editor.container.revert')}</span>
              </ContextMenu.Item>
              <ContextMenu.Separator />
            </>
          ) : null}
          {field.kind === 'reference' ? (
            <ContextMenu.Item
              className="menu-item"
              onSelect={() => stageReference({ state: 'absent' })}
            >
              <span>{t('editor.objects.removeReference')}</span>
            </ContextMenu.Item>
          ) : (
            <>
              {field.defaultValue ? (
                <ContextMenu.Item
                  className="menu-item"
                  onSelect={() => stageValue({ state: 'explicit', value: field.defaultValue! })}
                >
                  <span>{t('editor.objects.useDefault')}</span>
                </ContextMenu.Item>
              ) : null}
              <ContextMenu.Item
                className="menu-item"
                onSelect={() => stageValue({ state: 'absent' })}
              >
                <span>{t('editor.objects.removeValue')}</span>
              </ContextMenu.Item>
            </>
          )}
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}
