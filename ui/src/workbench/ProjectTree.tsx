import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import {
  Box,
  Braces,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  FileCode2,
  FolderTree,
  Search,
  Settings2,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { KeyboardEvent } from 'react';
import type { Workbench } from './useWorkbench';
import { labelFromPath } from './forms';
import { PanelResizeHandle } from './PanelResizeHandle';

const objectIcons = new Map<string, LucideIcon>([
  ['AR-PACKAGE', FolderTree],
  ['ECUC-MODULE-CONFIGURATION-VALUES', Settings2],
  ['ECUC-CONTAINER-VALUE', Box],
  ['SW-BASE-TYPE', Braces],
  ['IMPLEMENTATION-DATA-TYPE', Braces],
  ['APPLICATION-PRIMITIVE-DATA-TYPE', Braces],
]);

export function ProjectTree({ controller: c }: { controller: Workbench }) {
  const actions = useRef(c);
  useLayoutEffect(() => {
    actions.current = c;
  }, [c]);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const index = useMemo(() => {
    const projection = c.projection;
    const objects = new Map((projection?.objects ?? []).map((object) => [object.objectId, object]));
    const children = new Map<string | null, NonNullable<typeof projection>['objects']>();
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
    for (const object of projection?.objects ?? []) {
      const parent = object.parentId && objects.has(object.parentId) ? object.parentId : null;
      const list = children.get(parent) ?? [];
      list.push(object);
      children.set(parent, list);
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
    return {
      objects,
      children,
      searchText,
      hasChildren: new Set([...children.keys()].filter((key): key is string => key !== null)),
    };
  }, [c.projection]);
  const model = useMemo(() => {
    const query = c.treeFilter.trim().toLocaleLowerCase();
    const visible = new Set<string>();
    const revealed = new Set<string>();
    let ancestor = c.treeRevealId ? index.objects.get(c.treeRevealId)?.parentId : null;
    while (ancestor && !revealed.has(ancestor)) {
      revealed.add(ancestor);
      ancestor = index.objects.get(ancestor)?.parentId;
    }
    for (const object of index.objects.values()) {
      if (!query || index.searchText.get(object.objectId)?.includes(query)) {
        visible.add(object.objectId);
        let parent = object.parentId;
        while (parent && !visible.has(parent)) {
          visible.add(parent);
          parent = index.objects.get(parent)?.parentId ?? null;
        }
      }
    }
    const rows: {
      object: NonNullable<Workbench['projection']>['objects'][number];
      depth: number;
    }[] = [];
    const visit = (parent: string | null, depth: number) => {
      for (const object of index.children.get(parent) ?? []) {
        if (!visible.has(object.objectId)) continue;
        rows.push({ object, depth });
        if (query || !collapsed.has(object.objectId) || revealed.has(object.objectId))
          visit(object.objectId, depth + 1);
      }
    };
    visit(null, 1);
    return { rows, revealed, hasChildren: index.hasChildren };
  }, [index, c.treeFilter, c.treeRevealId, collapsed]);
  const activeInRows = model.rows.some((row) => row.object.objectId === c.activeObjectId);
  const objectRows = useMemo(
    () =>
      c.treeMode === 'objects'
        ? model.rows.map(({ object, depth }) => {
            const expanded =
              Boolean(c.treeFilter) ||
              !collapsed.has(object.objectId) ||
              model.revealed.has(object.objectId);
            const selected = c.objectSelection.includes(object.objectId);
            const Icon = objectIcons.get(object.kind) ?? FileCode2;
            return (
              <div
                key={object.objectId}
                role="treeitem"
                tabIndex={
                  c.activeObjectId === object.objectId ||
                  (!activeInRows && model.rows[0]?.object.objectId === object.objectId)
                    ? 0
                    : -1
                }
                aria-level={depth}
                aria-selected={selected}
                aria-label={object.shortName}
                aria-description={object.kind}
                title={`${object.shortName}\n${object.kind}\n${object.path}`}
                aria-expanded={model.hasChildren.has(object.objectId) ? expanded : undefined}
                data-object-id={object.objectId}
                className={`tree-row${selected ? ' selected' : ''}`}
                style={{ paddingLeft: `${depth * 12}px` }}
                onClick={(event) =>
                  void actions.current.selectObject(object.objectId, event.ctrlKey || event.metaKey)
                }
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    void actions.current.selectObject(
                      object.objectId,
                      event.ctrlKey || event.metaKey,
                    );
                  }
                }}
              >
                {model.hasChildren.has(object.objectId) ? (
                  <button
                    type="button"
                    tabIndex={-1}
                    aria-label={`${expanded ? '收起' : '展开'} ${object.shortName}`}
                    onClick={(event) => {
                      event.stopPropagation();
                      actions.current.setTreeRevealId(null);
                      setCollapsed((previous) => {
                        const next = new Set(previous);
                        if (expanded) next.add(object.objectId);
                        else next.delete(object.objectId);
                        return next;
                      });
                    }}
                  >
                    {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                  </button>
                ) : (
                  <span className="tree-spacer" />
                )}
                <Icon size={14} aria-hidden="true" />
                <span className="tree-name">{object.shortName}</span>
              </div>
            );
          })
        : null,
    [model, collapsed, c.treeMode, c.treeFilter, c.objectSelection, c.activeObjectId, activeInRows],
  );
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
      const id = active.dataset.objectId;
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
      if (event.key === 'ArrowLeft' && active.getAttribute('aria-expanded') !== 'true') {
        const parent = c.projection?.objects.find((object) => object.objectId === id)?.parentId;
        rows.find((row) => row.dataset.objectId === parent)?.focus();
      }
    }
  }
  return (
    <aside className="project-tree" aria-label="工程树" hidden={!c.treeVisible}>
      <PanelResizeHandle kind="tree" />
      <header>
        <strong>{c.workspace?.name ?? '工程'}</strong>
        <button
          type="button"
          className="panel-icon-button"
          aria-label="折叠工程树"
          title="折叠工程树"
          onClick={() => c.setTreeVisible(false)}
        >
          <ChevronLeft size={16} aria-hidden="true" />
        </button>
      </header>
      <div className="panel-tabs" role="tablist" aria-label="工程树视图">
        {(['objects', 'files'] as const).map((mode) => (
          <button
            key={mode}
            type="button"
            role="tab"
            aria-selected={c.treeMode === mode}
            onClick={() => c.setTreeMode(mode)}
          >
            {mode === 'objects' ? '对象' : '文件'}
          </button>
        ))}
      </div>
      <label className="search-field">
        <Search size={14} aria-hidden="true" />
        <input
          aria-label="搜索工程树名称、路径、定义、值"
          value={c.treeFilter}
          onChange={(event) => c.setTreeFilter(event.target.value)}
          placeholder="名称 / 路径 / 定义 / 值"
        />
      </label>
      <div
        className="tree-content"
        role="tree"
        aria-label={c.treeMode === 'objects' ? '配置对象' : '源文件'}
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
                className={`tree-row${c.activeSourceId === source.sourceId ? ' selected' : ''}`}
                onClick={() => void c.readSource(source.sourceId)}
              >
                <FileCode2 size={14} aria-hidden="true" />
                <span className="tree-name" title={source.path}>
                  {labelFromPath(source.path)}
                </span>
                <small>{source.readonly ? '只读' : '源'}</small>
              </button>
            ))}
        {!c.projection ? <p className="empty-state">正在读取真实工程投影。</p> : null}
      </div>
      {c.activeObjectId && c.treeMode === 'objects' && !activeInRows ? (
        <p className="filter-retained">
          当前对象在树中不可见；选择与草稿保留。
          <button
            type="button"
            onClick={() => {
              c.setTreeFilter('');
              c.setTreeRevealId(c.activeObjectId);
            }}
          >
            揭示当前对象
          </button>
        </p>
      ) : null}
    </aside>
  );
}
