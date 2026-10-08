import { message, useLocale } from '../i18n';
import type { Text } from '../i18n';
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
  title: Text;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
}) {
  const { t, text } = useLocale();
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
        aria-label={text(title)}
        tabIndex={-1}
      >
        <header>
          <h2>{text(title)}</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label={t('editor.dialog.closeLabel', { title: text(title) })}
          >
            {t('editor.dialog.close')}
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
  label = message('editor.dialog.copy'),
}: {
  text: string | (() => string);
  label?: Text;
}) {
  const { text: renderText } = useLocale();
  const [error, setError] = useState<Text>('');
  return (
    <>
      <button
        type="button"
        onClick={() => {
          void navigator.clipboard
            .writeText(typeof text === 'function' ? text() : text)
            .then(() => setError(''))
            .catch(() => setError(message('editor.dialog.copyFailed')));
        }}
      >
        {renderText(label)}
      </button>
      {error ? (
        <span role="status" className="error-text">
          {renderText(error)}
        </span>
      ) : null}
    </>
  );
}
export function OwnedLog({ text, label }: { text: Text; label: Text }) {
  const { t, text: renderText } = useLocale();
  const content = renderText(text);
  return (
    <section className="owned-log">
      <div className="log-header">
        <strong>{renderText(label)}</strong>
        <CopyText text={content} label={t('editor.dialog.copyLog')} />
      </div>
      {content.length > 12000 ? <p className="field-help">{t('editor.dialog.logTail')}</p> : null}
      <pre aria-label={renderText(label)}>{content.slice(-12000)}</pre>
    </section>
  );
}

export function TextSnapshot({ text, label }: { text: string; label: Text }) {
  const { t, text: renderText } = useLocale();
  const [offset, setOffset] = useState(0);
  const pageSize = 40000;
  const start = Math.min(offset, Math.floor(Math.max(0, text.length - 1) / pageSize) * pageSize);
  return (
    <section className="text-snapshot">
      <div className="log-header">
        <CopyText text={text} label={t('editor.dialog.copyLabel', { label: renderText(label) })} />
        {text.length > pageSize ? (
          <div>
            <button
              type="button"
              disabled={!start}
              onClick={() => setOffset(Math.max(0, start - pageSize))}
            >
              {t('editor.dialog.previous')}
            </button>
            <button
              type="button"
              disabled={start + pageSize >= text.length}
              onClick={() => setOffset(start + pageSize)}
            >
              {t('editor.dialog.next')}
            </button>
          </div>
        ) : null}
      </div>
      {text.length > pageSize ? (
        <p className="field-help">
          {t('editor.dialog.segmentHelp', {
            start: start + 1,
            end: Math.min(start + pageSize, text.length),
          })}
        </p>
      ) : null}
      <pre aria-label={renderText(label)}>{text.slice(start, start + pageSize)}</pre>
    </section>
  );
}
