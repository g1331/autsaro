import { useEffect, useEffectEvent, useRef, useState } from 'react';
import type { ReactNode } from 'react';

export function rememberDialogOpener() {
  const active = document.activeElement;
  if (
    !(active instanceof HTMLElement) ||
    active === document.body ||
    active.closest('[role="dialog"]')
  )
    return;
  const opener = active.closest('.menu-root')?.querySelector<HTMLElement>('button') ?? active;
  document
    .querySelectorAll('[data-dialog-opener]')
    .forEach((element) => element.removeAttribute('data-dialog-opener'));
  opener.setAttribute('data-dialog-opener', 'true');
}

export function Dialog({
  title,
  children,
  onClose,
  footer,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
}) {
  const dialog = useRef<HTMLElement>(null);
  const close = useEffectEvent(onClose);
  useEffect(() => {
    const active = document.activeElement;
    const opener =
      active instanceof HTMLElement && active !== document.body && !dialog.current?.contains(active)
        ? active
        : (document.querySelector<HTMLElement>('[data-dialog-opener]') ??
          document.querySelector<HTMLElement>('.menubar button'));
    const node = dialog.current;
    const focusable = () =>
      Array.from(
        node?.querySelectorAll<HTMLElement>(
          'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"], summary',
        ) ?? [],
      ).filter((item) => item.getClientRects().length > 0);
    (node?.querySelector<HTMLElement>('[data-initial-focus]') ?? focusable()[0] ?? node)?.focus();
    function trap(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        close();
      }
      if (event.key !== 'Tab') return;
      const items = focusable();
      if (!items.length) {
        event.preventDefault();
        node?.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && (document.activeElement === first || document.activeElement === node)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    node?.addEventListener('keydown', trap);
    return () => {
      node?.removeEventListener('keydown', trap);
      if (opener?.isConnected) opener.focus();
    };
  }, []);
  return (
    <div className="modal-backdrop">
      <section
        ref={dialog}
        className="workbench-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
      >
        <header>
          <h2>{title}</h2>
          <button type="button" onClick={onClose} aria-label={`关闭${title}`}>
            关闭
          </button>
        </header>
        <div className="dialog-body">{children}</div>
        {footer ? <footer>{footer}</footer> : null}
      </section>
    </div>
  );
}

export function CopyText({
  text,
  label = '复制',
}: {
  text: string | (() => string);
  label?: string;
}) {
  const [error, setError] = useState('');
  return (
    <>
      <button
        type="button"
        onClick={() => {
          void navigator.clipboard
            .writeText(typeof text === 'function' ? text() : text)
            .then(() => setError(''))
            .catch(() => setError('复制失败，请选择文本手动复制'));
        }}
      >
        {label}
      </button>
      {error ? (
        <span role="status" className="error-text">
          {error}
        </span>
      ) : null}
    </>
  );
}
export function OwnedLog({ text, label }: { text: string; label: string }) {
  return (
    <section className="owned-log">
      <div className="log-header">
        <strong>{label}</strong>
        <CopyText text={text} label="复制完整日志" />
      </div>
      {text.length > 12000 ? (
        <p className="field-help">显示末尾 12000 字符；完整 owned 返回日志可复制。</p>
      ) : null}
      <pre aria-label={label}>{text.slice(-12000)}</pre>
    </section>
  );
}

export function TextSnapshot({ text, label }: { text: string; label: string }) {
  const [offset, setOffset] = useState(0);
  const pageSize = 40000;
  const start = Math.min(offset, Math.floor(Math.max(0, text.length - 1) / pageSize) * pageSize);
  return (
    <section className="text-snapshot">
      <div className="log-header">
        <CopyText text={text} label={`复制${label}`} />
        {text.length > pageSize ? (
          <div>
            <button
              type="button"
              disabled={!start}
              onClick={() => setOffset(Math.max(0, start - pageSize))}
            >
              上一段
            </button>
            <button
              type="button"
              disabled={start + pageSize >= text.length}
              onClick={() => setOffset(start + pageSize)}
            >
              下一段
            </button>
          </div>
        ) : null}
      </div>
      {text.length > pageSize ? (
        <p className="field-help">
          显示第 {start + 1}–{Math.min(start + pageSize, text.length)}{' '}
          个字符；完整后台原文保持并可复制。
        </p>
      ) : null}
      <pre aria-label={label}>{text.slice(start, start + pageSize)}</pre>
    </section>
  );
}
