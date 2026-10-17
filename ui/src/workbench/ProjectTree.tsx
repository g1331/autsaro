import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import {
  Box,
  Braces,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleX,
  FileCode2,
  FolderTree,
  Search,
  Settings2,
  TriangleAlert,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { KeyboardEvent } from 'react';
import type { Workbench } from './useWorkbench';
import { labelFromPath } from './forms';
import { PanelResizeHandle } from './PanelResizeHandle';
import { workspaceTitle } from './labels';
import { Tabs, tabPanelProps } from './Tabs';
import { iconSize } from './icons';
import { objectRoleClass } from './objectRole';
import { groupedObjectTree, moduleLabel, objectMarks } from './projectGroups';
import type { ObjectGroup, TreeNode } from './projectGroups';
import { Tooltip } from './Tooltip';

const objectIcons = new Map<string, LucideIcon>([
  ['AR-PACKAGE', FolderTree],
  ['ECUC-MODULE-CONFIGURATION-VALUES', Settings2],
  ['ECUC-CONTAINER-VALUE', Box],
  ['SW-BASE-TYPE', Braces],
  ['IMPLEMENTATION-DATA-TYPE', Braces],
  ['APPLICATION-PRIMITIVE-DATA-TYPE', Braces],
]);

const groupLabelKeys: Record<ObjectGroup, string> = {
  modules: 'editor.tree.group.modules',
  application: 'editor.tree.group.application',
  communication: 'editor.tree.group.communication',
  types: 'editor.tree.group.types',
  bsw: 'editor.tree.group.bsw',
  other: 'editor.tree.group.other',
};

export function ProjectTree({ controller: c }: { controller: Workbench }) {
  const { t, language } = useLocale();
  const actions = useRef(c);
  useLayoutEffect(() => {
    actions.current = c;
  }, [c]);
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set(['group:bsw']));
  const index = useMemo(() => {
    const projection = c.projection;
    const objects = projection?.objects ?? [];
    const tree = groupedObjectTree(objects);
    const fieldText = new Map<string, string[]>();
    for (const field of projection?.fields ?? []) {
      const values = fieldText.get(field.objectId) ?? [];
      values.push(
        field.definitionId,
        field.current.state === 'explicit'
          ? field.current.value.lexeme
          : (field.defaultValue?.lexeme ?? ''),
        field.reference?.state === 'explicit' ? field.reference.rawPath : '',
      );
      fieldText.set(field.objectId, values);
    }
    const searchText = new Map<string, string>();
    const paths = new Map<string, string>();
    for (const object of objects) {
      paths.set(object.objectId, object.path);
      searchText.set(
        object.objectId,
        [
          object.shortName,
          object.path,
          object.definitionId,
          ...(fieldText.get(object.objectId) ?? []),
        ]
          .join(' ')
          .toLocaleLowerCase(),
      );
    }
    const parents = new Map<string, string>();
    const link = (node: TreeNode) =>
      node.children.forEach((child) => {
        parents.set(child.key, node.key);
        link(child);
      });
    tree.forEach(link);
    return { tree, searchText, paths, parents };
  }, [c.projection]);
  const marks = useMemo(
    () =>
      objectMarks(index.tree, {
        objects: c.projection?.objects ?? [],
        fields: c.projection?.fields ?? [],
        diagnostics: c.projection?.diagnostics ?? [],
        integrationIssues: c.integrationIssues,
        saved: c.savedValues,
        changes: c.changes,
      }),
    [index, c.projection, c.integrationIssues, c.savedValues, c.changes],
  );
  const model = useMemo(() => {
    const query = c.treeFilter.trim().toLocaleLowerCase();
    const revealed = new Set<string>();
    let ancestor = c.treeRevealId ? index.parents.get(c.treeRevealId) : undefined;
    while (ancestor && !revealed.has(ancestor)) {
      revealed.add(ancestor);
      ancestor = index.parents.get(ancestor);
    }
    const rows: { node: TreeNode; depth: number; parent: string | null; expanded: boolean }[] = [];
    // A node is visible when it or a descendant matches the filter.
    const matches = (node: TreeNode): boolean =>
      node.children.some(matches) ||
      (node.type === 'object' &&
        (!query || Boolean(index.searchText.get(node.key)?.includes(query))));
    const visit = (nodes: TreeNode[], depth: number, parent: string | null) => {
      for (const node of nodes) {
        if (!matches(node)) continue;
        const expanded = Boolean(query) || !collapsed.has(node.key) || revealed.has(node.key);
        rows.push({ node, depth, parent, expanded });
        if (expanded) visit(node.children, depth + 1, node.key);
      }
    };
    visit(index.tree, 1, null);
    return { rows };
  }, [index, c.treeFilter, c.treeRevealId, collapsed]);
  const activeInRows = model.rows.some((row) => row.node.key === c.activeObjectId);
  const objectRows = useMemo(() => {
    if (c.treeMode !== 'objects') return null;
    const toggle = (key: string, expanded: boolean) => {
      actions.current.setTreeRevealId(null);
      setCollapsed((previous) => {
        const next = new Set(previous);
        if (expanded) next.add(key);
        else next.delete(key);
        return next;
      });
    };
    return model.rows.map(({ node, depth, parent, expanded }, position) => {
      const name =
        node.type === 'group'
          ? t(groupLabelKeys[node.group], { lng: language })
          : moduleLabel(node.object);
      const mark = marks.get(node.key);
      const markText = [
        mark?.errors ? t('editor.tree.errors', { count: mark.errors, lng: language }) : '',
        mark?.warnings ? t('editor.tree.warnings', { count: mark.warnings, lng: language }) : '',
        mark?.modified ? t('editor.tree.modified', { lng: language }) : '',
        mark?.unapplied ? t('editor.tree.unapplied', { lng: language }) : '',
      ].filter(Boolean);
      const object = node.type === 'object' ? node.object : null;
      const path = object ? index.paths.get(object.objectId) : undefined;
      const Icon = object ? (objectIcons.get(object.kind) ?? FileCode2) : null;
      const select = (multiple: boolean) =>
        object
          ? void actions.current.selectObject(object.objectId, multiple)
          : toggle(node.key, expanded);
      return (
        <div
          key={node.key}
          role="treeitem"
          tabIndex={c.activeObjectId === node.key || (!activeInRows && position === 0) ? 0 : -1}
          aria-level={depth}
          aria-selected={object ? c.objectSelection.includes(object.objectId) : undefined}
          aria-label={name}
          aria-description={[object?.kind, ...markText].filter(Boolean).join(' · ') || undefined}
          title={
            object
              ? [name !== object.shortName ? object.shortName : '', object.kind, path, ...markText]
                  .filter(Boolean)
                  .join('\n')
              : undefined
          }
          aria-expanded={node.children.length ? expanded : undefined}
          data-tree-key={node.key}
          data-parent-key={parent ?? undefined}
          data-object-id={object?.objectId}
          className={node.type === 'group' ? 'tree-row tree-group' : 'tree-row'}
          style={{ paddingLeft: `${depth * 12}px` }}
          onClick={(event) => select(event.ctrlKey || event.metaKey)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              select(event.ctrlKey || event.metaKey);
            }
          }}
        >
          {node.children.length ? (
            <button
              type="button"
              className="ghost-button"
              tabIndex={-1}
              aria-label={t(expanded ? 'editor.tree.collapseObject' : 'editor.tree.expandObject', {
                name,
                lng: language,
              })}
              onClick={(event) => {
                event.stopPropagation();
                toggle(node.key, expanded);
              }}
            >
              {expanded ? (
                <ChevronDown size={iconSize.inline} aria-hidden="true" />
              ) : (
                <ChevronRight size={iconSize.inline} aria-hidden="true" />
              )}
            </button>
          ) : (
            <span className="tree-spacer" />
          )}
          {Icon && object ? (
            <Icon size={iconSize.inline} aria-hidden="true" className={objectRoleClass(object)} />
          ) : null}
          <span className="tree-name">{name}</span>
          {node.type === 'group' ? <small>{node.children.length}</small> : null}
          {markText.length ? (
            <small className="tree-marks" aria-hidden="true">
              {mark?.errors ? (
                <span className="tree-mark-error">
                  <CircleX size={iconSize.inline} />
                  {mark.errors}
                </span>
              ) : null}
              {mark?.warnings ? (
                <span className="tree-mark-warning">
                  <TriangleAlert size={iconSize.inline} />
                  {mark.warnings}
                </span>
              ) : null}
              {mark?.unapplied ? <span className="tree-mark-unapplied">◆</span> : null}
              {mark?.modified ? <span className="tree-mark-modified">●</span> : null}
            </small>
          ) : null}
        </div>
      );
    });
  }, [
    model,
    marks,
    index,
    c.treeMode,
    c.objectSelection,
    c.activeObjectId,
    activeInRows,
    language,
    t,
  ]);
  const sources =
    c.projection?.sources.filter(
      (source) =>
        !c.treeFilter || source.path.toLocaleLowerCase().includes(c.treeFilter.toLocaleLowerCase()),
    ) ?? [];
  const activeSourceInRows = sources.some((source) => source.sourceId === c.activeSourceId);
  function keyboard(event: KeyboardEvent<HTMLElement>) {
    const tree = event.currentTarget;
    const rows = Array.from(tree.querySelectorAll<HTMLElement>('[role="treeitem"]'));
    const active =
      document.activeElement instanceof HTMLElement
        ? document.activeElement.closest<HTMLElement>('[role="treeitem"]')
        : null;
    const index = active ? rows.indexOf(active) : -1;
    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      const next =
        event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? rows.length - 1
            : Math.max(0, Math.min(rows.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1)));
      rows[next]?.focus();
    } else if (active && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
      event.preventDefault();
      const id = active.dataset.treeKey;
      if (!id) return;
      c.setTreeRevealId(null);
      setCollapsed((previous) => {
        const next = new Set(previous);
        if (event.key === 'ArrowLeft') next.add(id);
        else next.delete(id);
        return next;
      });
      if (event.key === 'ArrowRight' && active.getAttribute('aria-expanded') === 'true')
        rows[index + 1]?.focus();
      if (event.key === 'ArrowLeft' && active.getAttribute('aria-expanded') !== 'true')
        rows.find((row) => row.dataset.treeKey === active.dataset.parentKey)?.focus();
    }
  }
  return (
    <aside className="project-tree" aria-label={t('editor.tree.heading')} hidden={!c.treeVisible}>
      <PanelResizeHandle kind="tree" />
      <header>
        <strong className="workspace-name" title={c.workspace ? workspaceTitle(c) : undefined}>
          {c.workspace ? workspaceTitle(c) : t('editor.tree.project')}
        </strong>
        <Tooltip label={t('editor.tree.collapse')}>
          <button
            type="button"
            className="icon-button"
            aria-label={t('editor.tree.collapse')}
            onClick={() => c.setTreeVisible(false)}
          >
            <ChevronLeft size={iconSize.control} aria-hidden="true" />
          </button>
        </Tooltip>
      </header>
      <div className="panel-tabs">
        <Tabs
          id="tree"
          label={t('editor.tree.views')}
          selected={c.treeMode}
          onSelect={c.setTreeMode}
          items={[
            { key: 'objects', label: t('editor.tree.objects') },
            { key: 'files', label: t('editor.tree.files') },
          ]}
        />
      </div>
      <div className="tree-panel" {...tabPanelProps('tree', c.treeMode)}>
        <label className="search-field">
          <Search size={iconSize.inline} aria-hidden="true" />
          <input
            aria-label={t('editor.tree.search')}
            value={c.treeFilter}
            onChange={(event) => c.setTreeFilter(event.target.value)}
            placeholder={t('editor.tree.placeholder')}
          />
        </label>
        <div
          className="tree-content"
          role="tree"
          aria-label={
            c.treeMode === 'objects' ? t('editor.objects.heading') : t('editor.tree.sourceFiles')
          }
          aria-multiselectable
          onKeyDown={keyboard}
        >
          {c.treeMode === 'objects'
            ? objectRows
            : sources.map((source, index) => (
                <button
                  key={source.sourceId}
                  type="button"
                  role="treeitem"
                  tabIndex={
                    c.activeSourceId === source.sourceId || (!activeSourceInRows && index === 0)
                      ? 0
                      : -1
                  }
                  aria-level={1}
                  aria-selected={c.activeSourceId === source.sourceId}
                  className="tree-row ghost-button"
                  onClick={() => void c.readSource(source.sourceId)}
                >
                  <FileCode2 size={iconSize.inline} aria-hidden="true" />
                  <span className="tree-name" title={displayPath(source.path)}>
                    {labelFromPath(source.path)}
                  </span>
                  <small>
                    {source.readonly ? t('editor.common.readonly') : t('editor.tree.source')}
                  </small>
                </button>
              ))}
          {!c.projection ? <p className="empty-state">{t('editor.tree.loading')}</p> : null}
        </div>
      </div>
      {c.activeObjectId && c.treeMode === 'objects' && !activeInRows ? (
        <p className="filter-retained">
          {t('editor.tree.retained')}
          <button
            type="button"
            onClick={() => {
              c.setTreeFilter('');
              c.setTreeRevealId(c.activeObjectId);
            }}
          >
            {t('editor.tree.reveal')}
          </button>
        </p>
      ) : null}
    </aside>
  );
}
