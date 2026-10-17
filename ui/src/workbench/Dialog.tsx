import { isMessage, message, useLocale } from '../i18n';
import type { Text } from '../i18n';
import { useEffect, useEffectEvent, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Check, CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-react';
import { iconSize } from './icons';
import type { Notice } from './forms';

const noticeIcons = {
  error: CircleAlert,
  warning: TriangleAlert,
  info: Info,
  success: CircleCheck,
};

/** One presentation for operation results: sentence, optional copyable detail. */
export function NoticeBanner({
  notice,
  onDismiss,
}: {
  notice: NonNullable<Notice>;
  onDismiss?: () => void;
}) {
  const { t, text } = useLocale();
  const Icon = noticeIcons[notice.tone];
  const full = text(notice.text);
  // Error codes such as STALE_DELIVERY stay in the copyable detail, not in the sentence.
  const headline = full.replace(/\b[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+[:：]\s*/g, '');
  const detail =
    notice.detail === undefined ? (headline === full ? '' : full) : text(notice.detail);
  return (
    <div className={`notice ${notice.tone}`} role={notice.tone === 'error' ? 'alert' : 'status'}>
      <Icon size={iconSize.control} aria-hidden="true" />
      <div className="notice-body">
        <span>{headline.slice(0, 3000)}</span>
        {detail && detail !== headline ? (
          <details>
            <summary>{t('editor.dialog.details')}</summary>
            <pre>{detail.slice(0, 12000)}</pre>
          </details>
        ) : null}
      </div>
      {detail || full.length > 3000 ? (
        <CopyText text={detail || full} label={t('action.copyDetails')} />
      ) : null}
      {onDismiss ? (
        <button
          type="button"
          className="icon-button"
          aria-label={t('editor.dialog.dismiss')}
          title={t('editor.dialog.dismiss')}
          onClick={onDismiss}
        >
          <X size={iconSize.inline} aria-hidden="true" />
        </button>
      ) : null}
    </div>
  );
}

export function rememberDialogOpener() {
  const active = document.activeElement;
  if (
    !(active instanceof HTMLElement) ||
    active === document.body ||
    active.closest('[role="dialog"]')
  )
    return;
  // A menu item leaves with its portalled menu; focus returns to the menu's trigger.
  const menu = active.closest('[role="menu"]')?.getAttribute('aria-labelledby');
  const opener = (menu && document.getElementById(menu)) || active;
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
  notice,
}: {
  title: Text;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  /** Results of actions taken inside the dialog; the page notice is behind the backdrop. */
  notice?: Notice;
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
      // A disabled focused control can leave focus on body in the native view.
      // The top modal still owns Escape and keeps Tab within its own controls.
      if (Array.from(document.querySelectorAll('[role="dialog"]')).at(-1) !== node) return;
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
      if (!node?.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (
        event.shiftKey &&
        (document.activeElement === first || document.activeElement === node)
      ) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener('keydown', trap);
    return () => {
      document.removeEventListener('keydown', trap);
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
          <button type="button" onClick={onClose}>
            {t('action.close')}
          </button>
        </header>
        <div className="dialog-body">{children}</div>
        {notice ? <NoticeBanner notice={notice} /> : null}
        {footer ? <footer>{footer}</footer> : null}
      </section>
    </div>
  );
}

export function CopyText({
  text,
  label = message('action.copy'),
}: {
  text: string | (() => string);
  label?: Text;
}) {
  const { text: renderText } = useLocale();
  const [result, setResult] = useState<Text>('');
  useEffect(() => {
    if (!result) return;
    const timer = window.setTimeout(() => setResult(''), 2000);
    return () => window.clearTimeout(timer);
  }, [result]);
  const copied = isMessage(result) && result.key === 'editor.dialog.copied';
  return (
    <>
      <button
        type="button"
        onClick={() => {
          void navigator.clipboard
            .writeText(typeof text === 'function' ? text() : text)
            .then(() => setResult(message('editor.dialog.copied')))
            .catch(() => setResult(message('editor.dialog.copyFailed')));
        }}
      >
        {copied ? <Check size={iconSize.inline} aria-hidden="true" /> : null}
        {renderText(copied ? result : label)}
      </button>
      <span role="status" className={result && !copied ? 'error-text' : 'visually-hidden'}>
        {renderText(result)}
      </span>
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
        <CopyText text={content} label={t('action.copyLog')} />
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
        <CopyText text={text} />
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
