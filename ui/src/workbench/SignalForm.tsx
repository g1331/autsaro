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
  return (
    <div className="form-fields">
      <label>
        信号名称
        <input
          value={fields.name}
          onChange={(event) => onChange({ ...fields, name: event.target.value })}
          disabled={disabled}
          autoComplete="off"
        />
      </label>
      <div className="form-pair">
        <label>
          起始位 <small>0–63</small>
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
          长度 <small>1–32 bit</small>
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
        初始值 <small>原始无符号值</small>
        <input
          type="number"
          min="0"
          step="1"
          value={fields.initialValue}
          onChange={(event) => onChange({ ...fields, initialValue: event.target.value })}
          disabled={disabled}
        />
      </label>
      <p className="field-help">按 little-endian 位序写入所属 CAN 帧；位范围不可超过帧 DLC。</p>
    </div>
  );
}
