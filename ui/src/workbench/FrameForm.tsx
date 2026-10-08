import { useLocale } from '../i18n';
import type { FrameFields } from './forms';

export function FrameForm({
  fields,
  onChange,
  disabled,
}: {
  fields: FrameFields;
  onChange: (fields: FrameFields) => void;
  disabled: boolean;
}) {
  const { t } = useLocale();
  return (
    <div className="form-fields">
      <label>
        {t('editor.can.frameName')}
        <input
          value={fields.name}
          onChange={(event) => onChange({ ...fields, name: event.target.value })}
          disabled={disabled}
          autoComplete="off"
        />
      </label>
      <div className="form-pair">
        <label>
          {t('editor.can.standardId')} <small>{t('editor.can.decimalId')}</small>
          <input
            type="number"
            min="0"
            max="2047"
            step="1"
            value={fields.id}
            onChange={(event) => onChange({ ...fields, id: event.target.value })}
            disabled={disabled}
          />
        </label>
        <label>
          DLC <small>{t('editor.can.bytes')}</small>
          <input
            type="number"
            min="1"
            max="8"
            step="1"
            value={fields.dlc}
            onChange={(event) => onChange({ ...fields, dlc: event.target.value })}
            disabled={disabled}
          />
        </label>
      </div>
      <label>
        {t('editor.can.direction')}
        <select
          value={fields.direction}
          onChange={(event) =>
            onChange({
              ...fields,
              direction: event.target.value as 'tx' | 'rx',
              periodMs: event.target.value === 'tx' ? fields.periodMs || '100' : '',
              timeoutMs: event.target.value === 'rx' ? fields.timeoutMs || '500' : '',
            })
          }
          disabled={disabled}
        >
          <option value="tx">{t('editor.can.tx')}</option>
          <option value="rx">{t('editor.can.rx')}</option>
        </select>
      </label>
      {fields.direction === 'tx' ? (
        <label>
          {t('editor.can.period')} <small>ms</small>
          <input
            type="number"
            min="1"
            step="1"
            value={fields.periodMs}
            onChange={(event) => onChange({ ...fields, periodMs: event.target.value })}
            disabled={disabled}
          />
        </label>
      ) : (
        <label>
          {t('editor.can.timeout')} <small>ms</small>
          <input
            type="number"
            min="1"
            step="1"
            value={fields.timeoutMs}
            onChange={(event) => onChange({ ...fields, timeoutMs: event.target.value })}
            disabled={disabled}
          />
        </label>
      )}
    </div>
  );
}
