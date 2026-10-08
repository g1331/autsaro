import { useLocale } from '../i18n';
import type { SignalFields } from './forms';

export function SignalForm({
  fields,
  onChange,
  disabled,
}: {
  fields: SignalFields;
  onChange: (fields: SignalFields) => void;
  disabled: boolean;
}) {
  const { t } = useLocale();
  return (
    <div className="form-fields">
      <label>
        {t('editor.can.signalName')}
        <input
          value={fields.name}
          onChange={(event) => onChange({ ...fields, name: event.target.value })}
          disabled={disabled}
          autoComplete="off"
        />
      </label>
      <div className="form-pair">
        <label>
          {t('editor.can.startBit')} <small>0–63</small>
          <input
            type="number"
            min="0"
            max="63"
            step="1"
            value={fields.startBit}
            onChange={(event) => onChange({ ...fields, startBit: event.target.value })}
            disabled={disabled}
          />
        </label>
        <label>
          {t('editor.can.length')} <small>1–32 bit</small>
          <input
            type="number"
            min="1"
            max="32"
            step="1"
            value={fields.length}
            onChange={(event) => onChange({ ...fields, length: event.target.value })}
            disabled={disabled}
          />
        </label>
      </div>
      <label>
        {t('editor.can.initial')} <small>{t('editor.can.rawUnsigned')}</small>
        <input
          type="number"
          min="0"
          step="1"
          value={fields.initialValue}
          onChange={(event) => onChange({ ...fields, initialValue: event.target.value })}
          disabled={disabled}
        />
      </label>
      <p className="field-help">{t('editor.can.signalHelp')}</p>
    </div>
  );
}
