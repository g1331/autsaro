import { useLocale } from '../i18n';
import { useEffect, useRef, useState } from 'react';

const panels = {
  tree: { variable: '--panel-tree-width', label: 'editor.resize.tree', minimum: 180, direction: 1 },
  inspector: {
    variable: '--panel-inspector-width',
    label: 'editor.resize.inspector',
    minimum: 220,
    direction: -1,
  },
  tools: {
    variable: '--panel-tools-height',
    label: 'editor.resize.tools',
    minimum: 100,
    direction: -1,
  },
} as const;
type Panel = keyof typeof panels;

function geometry(handle: HTMLElement, kind: Panel) {
  const panel =
    kind === 'tools'
      ? handle.parentElement?.querySelector<HTMLElement>('.tool-content')
      : handle.parentElement;
  const container = panel?.closest<HTMLElement>(
    kind === 'tools' ? '.central-workspace' : '.workspace-frame',
  );
  if (!panel || !container || !panel.getClientRects().length) return null;
  if (!container.clientWidth || !container.clientHeight) return null;
  const root = panel.closest<HTMLElement>('.app-shell') ?? container;
  const spec = panels[kind];
  const vertical = kind === 'tools';
  let available = container.clientHeight * 0.6;
  if (!vertical) {
    const overlay = getComputedStyle(panel).position === 'absolute';
    const opposite = container.querySelector<HTMLElement>(
      kind === 'tree' ? '.inspector-pane' : '.project-tree',
    );
    const occupied =
      opposite && getComputedStyle(opposite).position !== 'absolute'
        ? opposite.getBoundingClientRect().width
        : 0;
    const rail = container.querySelector('.tool-rail')?.getBoundingClientRect().width ?? 42;
    available = container.clientWidth - rail - (overlay ? 32 : occupied + 320);
  }
  const maximum = Math.max(100, Math.floor(Math.min(600, available)));
  return {
    root,
    panel,
    container,
    minimum: Math.min(spec.minimum, maximum),
    maximum,
    size: Math.round(panel.getBoundingClientRect()[vertical ? 'height' : 'width']),
  };
}

export function PanelResizeHandle({ kind }: { kind: Panel }) {
  const { t } = useLocale();
  const handle = useRef<HTMLDivElement>(null);
  const drag = useRef<{
    pointerId: number;
    start: number;
    size: number;
    root: HTMLElement;
  } | null>(null);
  const [range, setRange] = useState({
    minimum: panels[kind].minimum as number,
    maximum: 600,
    size: panels[kind].minimum as number,
  });
  const [dragging, setDragging] = useState(false);
  const spec = panels[kind];
  const vertical = kind === 'tools';

  function resize(size: number) {
    const current = handle.current && geometry(handle.current, kind);
    if (!current) return;
    const bounded = Math.max(current.minimum, Math.min(current.maximum, Math.round(size)));
    current.root.style.setProperty(spec.variable, `${bounded}px`);
    setRange({ minimum: current.minimum, maximum: current.maximum, size: bounded });
  }

  function finish() {
    const active = drag.current;
    drag.current = null;
    active?.root.classList.remove('panel-resizing');
    active?.root.style.removeProperty('--panel-resize-cursor');
    if (active && handle.current?.hasPointerCapture(active.pointerId)) {
      handle.current.releasePointerCapture(active.pointerId);
    }
    setDragging(false);
  }

  function reset() {
    const current = handle.current && geometry(handle.current, kind);
    current?.root.style.removeProperty(spec.variable);
  }

  useEffect(() => {
    const node = handle.current;
    if (!node) return;
    let frame = 0;
    const sync = () => {
      const current = geometry(node, kind);
      if (!current) return;
      // Constrain the rendered panel without overwriting the user's preferred
      // size, so shrinking and restoring the window restores the layout too.
      const limit = kind === 'tools' ? 'maxHeight' : 'maxWidth';
      const maximum = `${current.maximum}px`;
      if (current.panel.style[limit] !== maximum) current.panel.style[limit] = maximum;
      const size = Math.round(
        current.panel.getBoundingClientRect()[kind === 'tools' ? 'height' : 'width'],
      );
      setRange({ minimum: current.minimum, maximum: current.maximum, size });
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(sync);
    });
    const container = node.closest(kind === 'tools' ? '.central-workspace' : '.workspace-frame');
    if (container) {
      observer.observe(container);
      for (const panel of container.querySelectorAll(
        '.project-tree, .inspector-pane, .tool-content',
      )) {
        observer.observe(panel);
      }
    }
    frame = requestAnimationFrame(sync);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      drag.current?.root.classList.remove('panel-resizing');
      drag.current?.root.style.removeProperty('--panel-resize-cursor');
      drag.current = null;
    };
  }, [kind]);

  return (
    <div
      ref={handle}
      className={`panel-resize-handle panel-resize-${kind}${dragging ? ' is-dragging' : ''}`}
      role="separator"
      tabIndex={0}
      aria-label={t('editor.resize.label', { panel: t(spec.label) })}
      aria-orientation={vertical ? 'horizontal' : 'vertical'}
      aria-valuemin={range.minimum}
      aria-valuemax={range.maximum}
      aria-valuenow={range.size}
      aria-valuetext={`${range.size}px`}
      title={t('editor.resize.help', { panel: t(spec.label) })}
      onPointerDown={(event) => {
        if (event.button !== 0 || !event.isPrimary) return;
        const current = geometry(event.currentTarget, kind);
        if (!current) return;
        event.preventDefault();
        event.currentTarget.focus();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = {
          pointerId: event.pointerId,
          start: vertical ? event.clientY : event.clientX,
          size: current.size,
          root: current.root,
        };
        current.root.style.setProperty(
          '--panel-resize-cursor',
          vertical ? 'row-resize' : 'col-resize',
        );
        current.root.classList.add('panel-resizing');
        setDragging(true);
      }}
      onPointerMove={(event) => {
        const active = drag.current;
        if (!active || active.pointerId !== event.pointerId) return;
        const position = vertical ? event.clientY : event.clientX;
        resize(active.size + (position - active.start) * spec.direction);
      }}
      onPointerUp={(event) => {
        if (drag.current?.pointerId === event.pointerId) finish();
      }}
      onPointerCancel={(event) => {
        if (drag.current?.pointerId !== event.pointerId) return;
        resize(drag.current.size);
        finish();
      }}
      onLostPointerCapture={(event) => {
        if (drag.current?.pointerId === event.pointerId) finish();
      }}
      onDoubleClick={reset}
      onKeyDown={(event) => {
        if (event.key === 'Escape' && drag.current) {
          resize(drag.current.size);
          finish();
          event.stopPropagation();
        } else if (event.key === 'Enter') {
          event.preventDefault();
          finish();
          reset();
        } else {
          const current = geometry(event.currentTarget, kind);
          if (!current) return;
          let direction = 0;
          if (event.key === (vertical ? 'ArrowDown' : 'ArrowRight')) direction = 1;
          else if (event.key === (vertical ? 'ArrowUp' : 'ArrowLeft')) direction = -1;
          if (!direction && !['Home', 'End'].includes(event.key)) return;
          event.preventDefault();
          let size = current.size + direction * spec.direction * (event.shiftKey ? 40 : 10);
          if (event.key === 'Home') size = current.minimum;
          else if (event.key === 'End') size = current.maximum;
          resize(size);
        }
      }}
    />
  );
}
