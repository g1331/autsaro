import { useLocale } from '../i18n';
import type { Frame, Signal } from '../types';

export function ReferenceView({
  frame,
  signal,
  signals,
}: {
  frame: Frame;
  signal?: Signal;
  signals?: Signal[];
}) {
  const { t } = useLocale();
  return (
    <div className="reference-view">
      <h3>{t('editor.references.heading')}</h3>
      <p>{t('editor.references.intro')}</p>
      <div className="reference-chain">
        {signal && (
          <>
            <div>
              <small>{t('editor.references.signal')}</small>
              <strong>{signal.path}</strong>
            </div>
            <span aria-hidden="true">↓</span>
          </>
        )}
        <div>
          <small>{t('editor.references.frame')}</small>
          <strong>{frame.path}</strong>
        </div>
        <span aria-hidden="true">↓</span>
        <div>
          <small>{t('editor.references.bus')}</small>
          <strong>
            {t('editor.references.busValue', {
              id: `0x${frame.id.toString(16).toUpperCase().padStart(3, '0')}`,
              direction: frame.direction.toUpperCase(),
            })}
          </strong>
        </div>
      </div>
      {signals && (
        <p className="reference-note">
          {t('editor.references.relatedSignals', {
            names: signals.length
              ? signals.map((item) => item.name).join(', ')
              : t('editor.common.none'),
          })}
        </p>
      )}
    </div>
  );
}
