import { useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { ArrowDown, ArrowUp, CircleX, Info, TriangleAlert } from 'lucide-react';
import { message, useLocale } from '../i18n';
import { editorCategoryKeys, editorSeverityKeys } from '../i18n/editors';
import { displayPath } from '../pathDisplay';
import type { Workbench } from './useWorkbench';
import { CopyText } from './Dialog';
import { distinct } from './forms';
import { fieldName } from './batchEditing';
import type { RejectedDiagnostic } from './batchEditing';
import { iconSize } from './icons';
import { moduleName } from './projectGroups';
import { tabId } from './Tabs';

type Severity = 'error' | 'warning' | 'info';
const severities: Severity[] = ['error', 'warning', 'info'];
const severityIcons = { error: CircleX, warning: TriangleAlert, info: Info };

/** Where a problem can be located: an object or parameter, else its source file. */
export type LocateTarget = {
  objectId: string | null;
  fieldId: string | null;
  sourceId: string | null;
};

/**
 * The problem locate relay (interaction spec): handle drafts, reveal the tree node, show the
 * object in the container view, then focus the parameter cell, else the container title, else
 * open the source file.
 */
export function useLocate(c: Workbench) {
  const targets = useMemo(
    () => ({
      objects: new Map(c.projection?.objects.map((object) => [object.objectId, object])),
      fields: new Map(c.projection?.fields.map((field) => [field.fieldId, field])),
      sources: new Set(c.projection?.sources.map((source) => source.sourceId)),
      objectsByPath: new Map(c.projection?.objects.map((object) => [object.path, object])),
      sourcesByPath: new Map(c.projection?.sources.map((source) => [source.path, source])),
    }),
    [c.projection],
  );
  /** Resolves what a problem names to a target, or null when nothing can be shown. */
  function resolve(issue: {
    objectId?: string | null;
    fieldId?: string | null;
    sourceId?: string | null;
    path?: string | null;
    file?: string | null;
  }): LocateTarget | null {
    const field = issue.fieldId ? targets.fields.get(issue.fieldId) : undefined;
    const objectId =
      (issue.objectId && targets.objects.has(issue.objectId) ? issue.objectId : null) ??
      field?.objectId ??
      (issue.path ? (targets.objectsByPath.get(issue.path)?.objectId ?? null) : null);
    const sourceId =
      (issue.sourceId && targets.sources.has(issue.sourceId) ? issue.sourceId : null) ??
      (issue.file ? (targets.sourcesByPath.get(issue.file)?.sourceId ?? null) : null);
    if (!objectId && !sourceId) return null;
    return { objectId, fieldId: field ? field.fieldId : null, sourceId };
  }
  async function locate(target: LocateTarget) {
    // Locating the current object keeps the drafts, e.g. the change just rejected on it.
    const current =
      target.objectId !== null &&
      c.activeObjectId === target.objectId &&
      c.objectSelection.length <= 1;
    const run = async () => {
      c.setTreeVisible(true);
      c.setInspectorVisible(true);
      c.setInspectorTab('properties');
      c.setTreeFilter('');
      c.setObjectFilter('');
      if (target.objectId) {
        if (current) c.openDocument({ kind: 'configuration' });
        else await c.selectObject(target.objectId);
        if (target.fieldId) c.setActiveFieldId(target.fieldId);
        requestAnimationFrame(() => {
          // The parameter cell exists only in the container view.
          const containerTab = document.getElementById(tabId('configuration-view', 'container'));
          if (containerTab?.getAttribute('aria-selected') === 'false') containerTab.click();
          requestAnimationFrame(() => {
            const field = target.fieldId
              ? Array.from(document.querySelectorAll<HTMLElement>('[data-field-id]')).find(
                  (element) => element.dataset.fieldId === target.fieldId,
                )
              : null;
            const title = Array.from(
              document.querySelectorAll<HTMLElement>('[data-object-title]'),
            ).find((element) => element.dataset.objectTitle === target.objectId);
            const focus = field ?? title;
            focus?.scrollIntoView({ block: 'nearest' });
            focus?.focus({ preventScroll: true });
          });
        });
      } else if (target.sourceId) await c.readSource(target.sourceId);
    };
    if (current) await run();
    else await c.guardContext(message('activity.locate'), run);
  }
  return { resolve, locate, targets };
}

type Row = {
  key: string;
  severity: Severity;
  code: string;
  module: string;
  target: string;
  message: string;
  remedy: string;
  file: string;
  locate: LocateTarget | null;
  detail: unknown;
};
type SortKey = 'severity' | 'module' | 'target' | 'message' | 'file';

/**
 * Automatic diagnostics, explicit validation findings and standard-input findings in one
 * table (ADR 0020 decision 5): sortable, filtered by severity, module and text.
 */
export function ProblemsTable({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const { resolve, locate, targets } = useLocate(c);
  const [shown, setShown] = useState<Set<Severity>>(() => new Set(severities));
  const [module, setModule] = useState('');
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<{ key: SortKey; ascending: boolean }>({
    key: 'severity',
    ascending: true,
  });
  const [current, setCurrent] = useState<string | null>(null);
  const moduleOf = (objectId: string | null) =>
    (objectId ? moduleName(targets.objects.get(objectId) ?? { definitionId: null }) : null) ?? '';
  const rows: Row[] = [];
  c.diagnostics.forEach((issue, index) => {
    const located = resolve(issue);
    const field = issue.fieldId ? targets.fields.get(issue.fieldId) : undefined;
    const path =
      issue.path ?? (located?.objectId ? targets.objects.get(located.objectId)?.path : '');
    rows.push({
      key: `diagnostic-${index}`,
      severity: issue.severity,
      code: issue.code,
      module: moduleOf(located?.objectId ?? null),
      target: field ? `${path ?? ''} · ${fieldName(field)}` : (path ?? ''),
      message: text(issue.message),
      remedy: text(issue.remedy),
      file: displayPath(issue.file ?? ''),
      locate: located,
      detail: issue,
    });
  });
  c.issues.forEach((issue, index) => {
    const located = resolve({ path: issue.path, file: issue.file });
    rows.push({
      key: `workspace-${index}`,
      severity: issue.severity,
      code: issue.code,
      module: moduleOf(located?.objectId ?? null),
      target: issue.path ?? '',
      message: text(issue.message),
      remedy: '',
      file: displayPath(issue.file ?? ''),
      locate: located,
      detail: issue,
    });
  });
  distinct(c.integrationIssues).forEach((issue, index) => {
    const located = resolve({ path: issue.object, file: issue.file });
    rows.push({
      key: `standard-${index}`,
      severity: 'error',
      code: `${issue.code} · ${t(editorCategoryKeys[issue.category])}`,
      module: moduleOf(located?.objectId ?? null),
      target: issue.object ?? '',
      message: text(issue.message),
      remedy: text(issue.remedy),
      file: displayPath(issue.file ?? ''),
      locate: located,
      detail: issue,
    });
  });
  const modules = [...new Set(rows.map((row) => row.module).filter(Boolean))].sort();
  const counts = Object.fromEntries(
    severities.map((severity) => [
      severity,
      rows.filter((row) => row.severity === severity).length,
    ]),
  ) as Record<Severity, number>;
  const needle = query.trim().toLocaleLowerCase();
  const order = (row: Row): string | number =>
    sort.key === 'severity' ? severities.indexOf(row.severity) : row[sort.key];
  const visible = rows
    .filter(
      (row) =>
        shown.has(row.severity) &&
        (!module || row.module === module) &&
        (!needle ||
          [row.code, row.module, row.target, row.message, row.file]
            .join(' ')
            .toLocaleLowerCase()
            .includes(needle)),
    )
    .sort((left, right) => {
      const a = order(left);
      const b = order(right);
      const primary =
        typeof a === 'number' && typeof b === 'number' ? a - b : String(a).localeCompare(String(b));
      // Ties keep severity, then module, so the default reads errors first per module.
      return (
        (sort.ascending ? primary : -primary) ||
        severities.indexOf(left.severity) - severities.indexOf(right.severity) ||
        left.module.localeCompare(right.module)
      );
    });
  const header = (key: SortKey, label: string) => {
    const active = sort.key === key;
    const Arrow = sort.ascending ? ArrowUp : ArrowDown;
    return (
      <th scope="col" aria-sort={active ? (sort.ascending ? 'ascending' : 'descending') : 'none'}>
        <button
          type="button"
          className="sort-button"
          onClick={() => setSort({ key, ascending: active ? !sort.ascending : true })}
        >
          {label}
          {active ? <Arrow size={iconSize.inline} aria-hidden="true" /> : null}
        </button>
      </th>
    );
  };
  return (
    <>
      <div className="problem-filters" role="group" aria-label={t('editor.problems.filters')}>
        {severities.map((severity) => {
          const Icon = severityIcons[severity];
          return (
            <button
              key={severity}
              type="button"
              className={`ghost-button severity-${severity}`}
              aria-pressed={shown.has(severity)}
              onClick={() =>
                setShown((previous) => {
                  const next = new Set(previous);
                  if (next.has(severity)) next.delete(severity);
                  else next.add(severity);
                  return next;
                })
              }
            >
              <Icon size={iconSize.inline} aria-hidden="true" />
              {t(editorSeverityKeys[severity])}
              <small>{counts[severity]}</small>
            </button>
          );
        })}
        <select
          aria-label={t('editor.problems.module')}
          value={module}
          onChange={(event) => setModule(event.target.value)}
        >
          <option value="">{t('editor.problems.allModules')}</option>
          {modules.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <input
          aria-label={t('editor.problems.search')}
          placeholder={t('editor.problems.search')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>
      {rows.length ? (
        <div className="table-wrap">
          <table className="data-table problems-table">
            <colgroup>
              <col className="col-severity" />
              <col className="col-module" />
              <col className="col-target" />
              <col />
              <col className="col-file" />
              <col className="col-actions" />
            </colgroup>
            <thead>
              <tr>
                {header('severity', t('editor.problems.severity'))}
                {header('module', t('editor.container.module'))}
                {header('target', t('editor.problems.target'))}
                {header('message', t('editor.problems.message'))}
                {header('file', t('editor.problems.file'))}
                <th scope="col">
                  <span className="visually-hidden">{t('editor.problems.actions')}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {visible.map((row) => {
                const Icon = severityIcons[row.severity];
                const go = () => {
                  setCurrent(row.key);
                  if (row.locate) void locate(row.locate);
                };
                return (
                  <tr
                    key={row.key}
                    data-problem-code={row.code}
                    className={row.severity}
                    tabIndex={0}
                    aria-current={current === row.key ? 'true' : undefined}
                    onClick={() => setCurrent(row.key)}
                    onDoubleClick={go}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' && event.target === event.currentTarget) {
                        event.preventDefault();
                        go();
                      }
                    }}
                  >
                    <td className={`severity-${row.severity}`}>
                      <Icon size={iconSize.inline} aria-hidden="true" />
                      {t(editorSeverityKeys[row.severity])}
                    </td>
                    <td>{row.module}</td>
                    <td className="mono" title={row.target}>
                      {row.target}
                    </td>
                    <td title={row.remedy ? `${row.message}\n${row.remedy}` : row.message}>
                      <span className="mono secondary">{row.code}</span>{' '}
                      <span className="problem-message">{row.message}</span>
                    </td>
                    <td className="mono" title={row.file}>
                      {row.file}
                    </td>
                    <td className="problem-actions">
                      {row.locate ? (
                        <button type="button" onClick={go}>
                          {row.locate.objectId ? t('action.locate') : t('editor.common.viewSource')}
                        </button>
                      ) : (
                        <small>{t('editor.tools.unqualifiedTarget')}</small>
                      )}
                      <CopyText
                        text={JSON.stringify(row.detail, null, 2)}
                        label={t('action.copyDetails')}
                      />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {!visible.length ? <p className="empty-state">{t('editor.problems.noMatch')}</p> : null}
        </div>
      ) : null}
    </>
  );
}

/** The last rejected change, one field per line, locatable when it names an existing object. */
export function RejectionDetails({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const { resolve, locate } = useLocate(c);
  if (!c.rejection) return null;
  const item = (diagnostic: RejectedDiagnostic, index: number) => {
    const target = resolve(diagnostic);
    const Icon = severityIcons[diagnostic.severity];
    const rows: [string, ReactNode][] = [
      [
        t('editor.problems.severity'),
        <span className={`severity-${diagnostic.severity}`}>
          <Icon size={iconSize.inline} aria-hidden="true" />
          {t(editorSeverityKeys[diagnostic.severity])}
        </span>,
      ],
      [t('editor.problems.code'), <span className="mono">{diagnostic.code || '—'}</span>],
      [
        t('editor.problems.message'),
        <>
          {text(diagnostic.message)}
          {text(diagnostic.remedy) ? <small>{text(diagnostic.remedy)}</small> : null}
        </>,
      ],
      [
        t('editor.problems.objectPath'),
        <span className="mono path-text">{diagnostic.path || '—'}</span>,
      ],
      [
        t('editor.problems.sourceFile'),
        <span className="mono path-text">
          {diagnostic.file ? displayPath(diagnostic.file) : '—'}
        </span>,
      ],
    ];
    return (
      <div key={index} className="rejection-item">
        <dl className="property-list">
          {rows.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        {target ? (
          <button type="button" onClick={() => void locate(target)}>
            {target.objectId ? t('action.locate') : t('editor.common.viewSource')}
          </button>
        ) : null}
      </div>
    );
  };
  return (
    <section
      id="rejection-details"
      className="rejection-details"
      tabIndex={-1}
      aria-labelledby="rejection-details-title"
    >
      <header>
        <h3 id="rejection-details-title">{t('editor.problems.rejected')}</h3>
        <CopyText
          text={JSON.stringify(c.rejection.detail, null, 2)}
          label={t('action.copyDetails')}
        />
      </header>
      {c.rejection.items.map(item)}
    </section>
  );
}
