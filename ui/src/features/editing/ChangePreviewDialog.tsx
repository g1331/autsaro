import type { useLocale } from '../../i18n';
import type { Workbench } from '../../app/useWorkbench';
import { Dialog } from '../../shared/ui/Dialog';
import { editorScopeKeys } from '../../i18n/editors';

export function renderChangePreview(
  c: Pick<
    Workbench,
    | 'applyChanges'
    | 'busy'
    | 'cancelChangePreview'
    | 'cancelOperation'
    | 'changePreview'
    | 'notice'
    | 'preparedChangeSet'
  >,
  locale: ReturnType<typeof useLocale>,
) {
  const { t, text } = locale;
  const busy = { busy: c.busy, onCancelBusy: c.cancelOperation };
  if (c.changePreview)
    return (
      <Dialog
        notice={c.notice}
        size="preview"
        {...busy}
        title={t('workflow.batch.title')}
        onClose={() => c.cancelChangePreview()}
        footer={
          <>
            <span>{t('workflow.batch.count', { count: c.changePreview.impacts.length })}</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.cancelChangePreview()}
            >
              {t('action.cancel')}
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy) || !c.preparedChangeSet}
              onClick={() => void c.applyChanges()}
            >
              {t('action.apply')}
            </button>
          </>
        }
      >
        <p className="mono path-text">
          {t('workflow.batch.inputIdentity', { fingerprint: c.changePreview.inputFingerprint })}
          <br />
          {t('workflow.batch.definitionIdentity', {
            fingerprint: c.changePreview.definitionFingerprint,
          })}
          <br />
          {t('workflow.batch.confirmationIdentity', { revision: c.changePreview.changeRevision })}
        </p>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t('workflow.batch.objectChangeId')}</th>
                <th>{t('workflow.batch.before')}</th>
                <th>{t('workflow.batch.after')}</th>
                <th>{t('workflow.batch.impact')}</th>
              </tr>
            </thead>
            <tbody>
              {c.changePreview.impacts.map((impact, index) => (
                <tr key={`${impact.changeId}-${index}`}>
                  <td className="mono path-text">
                    {impact.path}
                    <small>{impact.changeId}</small>
                  </td>
                  <td className="mono">{impact.before}</td>
                  <td className="mono">{impact.after}</td>
                  <td>
                    {impact.incoming ? t('workflow.batch.incoming') : t('workflow.batch.explicit')}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {c.changePreview.diagnostics.map((issue, index) => (
          <p
            key={`${issue.code}-${index}`}
            className={issue.severity === 'error' ? 'error-text' : 'field-help'}
          >
            {t(editorScopeKeys[issue.scope])} · {issue.code} · {text(issue.message)} ·{' '}
            {text(issue.remedy)}
          </p>
        ))}
      </Dialog>
    );
  return null;
}
