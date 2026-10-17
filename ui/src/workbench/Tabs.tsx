import type { KeyboardEvent, ReactNode } from 'react';
import { Tooltip } from './Tooltip';

export type TabItem<K extends string> = {
  key: K;
  label: ReactNode;
  /** Pointer close button beside the tab, out of the tab order; Delete closes from the keyboard. */
  onClose?: () => void;
  closeLabel?: string;
  closeIcon?: ReactNode;
};

export const tabId = (id: string, key: string) => `${id}-tab-${key}`;
export const tabPanelId = (id: string) => `${id}-panel`;

/** Props for the single panel that shows the selected tab's content. */
export function tabPanelProps(id: string, selected: string) {
  return {
    id: tabPanelId(id),
    role: 'tabpanel',
    'aria-labelledby': tabId(id, selected),
  } as const;
}

/**
 * WAI-ARIA tab list with one tab stop, arrow keys, Home and End. Selection
 * follows focus unless `manual`, where Enter or Space activates the tab; use
 * it when activation does more than swap visible content.
 */
export function Tabs<K extends string>({
  id,
  label,
  items,
  selected,
  onSelect,
  orientation = 'horizontal',
  manual = false,
  className,
}: {
  id: string;
  label: string;
  items: TabItem<K>[];
  selected: K;
  onSelect: (key: K) => void;
  orientation?: 'horizontal' | 'vertical';
  manual?: boolean;
  className?: string;
}) {
  const [previous, next] =
    orientation === 'vertical' ? ['ArrowUp', 'ArrowDown'] : ['ArrowLeft', 'ArrowRight'];
  function keyDown(event: KeyboardEvent<HTMLDivElement>) {
    const tabs = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
    );
    const index = tabs.indexOf(event.target as HTMLButtonElement);
    if (index < 0) return;
    if (event.key === 'Delete' && items[index].onClose) {
      event.preventDefault();
      items[index].onClose();
      return;
    }
    const target =
      event.key === previous
        ? (index - 1 + tabs.length) % tabs.length
        : event.key === next
          ? (index + 1) % tabs.length
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? tabs.length - 1
              : -1;
    if (target < 0) return;
    event.preventDefault();
    tabs[target].focus();
    if (!manual) onSelect(items[target].key);
  }
  return (
    <div
      role="tablist"
      aria-label={label}
      aria-orientation={orientation}
      className={className}
      onKeyDown={keyDown}
    >
      {items.map((item) => {
        const active = item.key === selected;
        return (
          <span key={item.key} role="presentation" className={active ? 'active' : undefined}>
            <button
              type="button"
              role="tab"
              className="ghost-button"
              id={tabId(id, item.key)}
              aria-selected={active}
              aria-controls={tabPanelId(id)}
              aria-keyshortcuts={item.onClose ? 'Delete' : undefined}
              tabIndex={active ? 0 : -1}
              onClick={() => onSelect(item.key)}
            >
              {item.label}
            </button>
            {item.onClose ? (
              <Tooltip label={item.closeLabel ?? ''}>
                <button
                  type="button"
                  className="icon-button tab-close"
                  tabIndex={-1}
                  aria-label={item.closeLabel}
                  onClick={item.onClose}
                >
                  {item.closeIcon}
                </button>
              </Tooltip>
            ) : null}
          </span>
        );
      })}
    </div>
  );
}
