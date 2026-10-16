import { isMessage, message, useLocale } from '../i18n';
import type { Text } from '../i18n';
import { useEffect, useId, useState } from 'react';
import { AlertDialog, Dialog as RadixDialog } from 'radix-ui';
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
  const detail = notice.detail === undefined ? '' : text(notice.detail);
  return (
    <div className={`notice ${notice.tone}`} role={notice.tone === 'error' ? 'alert' : 'status'}>
      <Icon size={iconSize.control} aria-hidden="true" />
      <div className="notice-body">
        <span>{text(notice.text).slice(0, 3000)}</span>
        {detail && detail !== text(notice.text) ? (
          <details>
            <summary>{t('editor.dialog.details')}</summary>
            <pre>{detail.slice(0, 12000)}</pre>
          </details>
        ) : null}
      </div>
      {detail || text(notice.text).length > 3000 ? (
        <CopyText text={detail || text(notice.text)} label={t('editor.common.copyDetails')} />
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
    active.closest('[role="dialog"], [role="alertdialog"]')
  )
    return;
  const opener = active.closest('.menu-root')?.querySelector<HTMLElement>('button') ?? active;
  document
    .querySelectorAll('[data-dialog-opener]')
    .forEach((element) => element.removeAttribute('data-dialog-opener'));
  opener.setAttribute('data-dialog-opener', 'true');
}

/** Where focus returns on close; a menu item that opened the dialog is gone by then. */
function dialogOpener() {
  const active = document.activeElement;
  return active instanceof HTMLElement &&
    active !== document.body &&
    !active.closest('[role="dialog"], [role="alertdialog"]')
    ? active
    : (document.querySelector<HTMLElement>('[data-dialog-opener]') ??
        document.querySelector<HTMLElement>('.menubar button'));
}

/** Radix `onCloseAutoFocus`: back to the opener, unless a following dialog already took focus. */
export function useFocusReturn() {
  const [opener] = useState(dialogOpener);
  return (event: Event) => {
    event.preventDefault();
    if (document.activeElement?.closest('[role="dialog"], [role="alertdialog"]')) return;
    if (opener?.isConnected) opener.focus();
  };
}

/** DESIGN.md Layout: confirmation 480, form 720, preview 1100. */
export type DialogSize = 'confirm' | 'form' | 'preview';

export function Dialog({
  title,
  children,
  onClose,
  footer,
  notice,
  size = 'form',
  alert = false,
  busy,
  onCancelBusy,
  className,
}: {
  title: Text;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  /** Results of actions taken inside the dialog; the page notice is behind the backdrop. */
  notice?: Notice;
  size?: DialogSize;
  /** A decision that interrupts the task (draft guards): Radix AlertDialog semantics. */
  alert?: boolean;
  /** The running operation; closing waits for it, and the dialog says so. */
  busy?: Text | null;
  onCancelBusy?: () => void;
  className?: string;
}) {
  const { t, text } = useLocale();
  const returnFocus = useFocusReturn();
  const busyId = useId();
  const Primitive = alert ? AlertDialog : RadixDialog;
  const blocked = Boolean(busy);
  const body = <div className="dialog-body">{children}</div>;
  return (
    <Primitive.Root
      open
      onOpenChange={(open) => {
        if (!open && !blocked) onClose();
      }}
    >
      <Primitive.Portal>
        <Primitive.Overlay className="modal-backdrop">
          <Primitive.Content
            className={['workbench-dialog', size, className].filter(Boolean).join(' ')}
            onOpenAutoFocus={(event) => {
              const initial = (event.currentTarget as HTMLElement).querySelector<HTMLElement>(
                '[data-initial-focus]',
              );
              if (!initial) return;
              event.preventDefault();
              initial.focus();
            }}
            onCloseAutoFocus={returnFocus}
            {...(alert
              ? {}
              : {
                  // Only alert dialogs carry a description element.
                  'aria-describedby': undefined,
                  // Drafts and previews must not vanish on a stray click outside.
                  onInteractOutside: (event: Event) => event.preventDefault(),
                })}
          >
            <header>
              <Primitive.Title>{text(title)}</Primitive.Title>
              <button
                type="button"
                className="icon-button"
                disabled={blocked}
                aria-describedby={blocked ? busyId : undefined}
                aria-label={t('editor.dialog.closeLabel', { title: text(title) })}
                title={t('editor.dialog.closeLabel', { title: text(title) })}
                onClick={onClose}
              >
                <X size={iconSize.control} aria-hidden="true" />
              </button>
            </header>
            {alert ? <AlertDialog.Description asChild>{body}</AlertDialog.Description> : body}
            {notice ? <NoticeBanner notice={notice} /> : null}
            {blocked ? (
              <p className="dialog-busy" id={busyId} role="status">
                <span className="busy-indicator" aria-hidden="true" />
                <span>{t('editor.dialog.closeBlocked', { operation: text(busy ?? '') })}</span>
                {onCancelBusy ? (
                  <button type="button" onClick={onCancelBusy}>
                    {t('shell.command.cancel')}
                  </button>
                ) : null}
              </p>
            ) : null}
            {footer ? <footer>{footer}</footer> : null}
          </Primitive.Content>
        </Primitive.Overlay>
      </Primitive.Portal>
    </Primitive.Root>
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
        aria-label={renderText(label)}
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
