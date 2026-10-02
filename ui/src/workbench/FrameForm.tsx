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
  return (
    <div className="form-fields">
      <label>
        帧名称
        <input
          value={fields.name}
          onChange={(event) => onChange({ ...fields, name: event.target.value })}
          disabled={disabled}
          autoComplete="off"
        />
      </label>
      <div className="form-pair">
        <label>
          标准 CAN ID <small>0–2047 · 十进制</small>
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
          DLC <small>字节</small>
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
        方向
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
          <option value="tx">TX · 周期发送</option>
          <option value="rx">RX · 接收超时</option>
        </select>
      </label>
      {fields.direction === 'tx' ? (
        <label>
          发送周期 <small>ms</small>
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
          接收超时 <small>ms</small>
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
