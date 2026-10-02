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
  return (
    <div className="reference-view">
      <h3>引用与依赖</h3>
      <p>以下关系来自当前配置模型，不推断未显示的跨模块引用。</p>
      <div className="reference-chain">
        {signal && (
          <>
            <div>
              <small>信号</small>
              <strong>{signal.path}</strong>
            </div>
            <span aria-hidden="true">↓</span>
          </>
        )}
        <div>
          <small>所属 CAN 帧</small>
          <strong>{frame.path}</strong>
        </div>
        <span aria-hidden="true">↓</span>
        <div>
          <small>总线标识</small>
          <strong>
            标准 11-bit · 0x{frame.id.toString(16).toUpperCase().padStart(3, '0')} ·{' '}
            {frame.direction.toUpperCase()}
          </strong>
        </div>
      </div>
      {signals && (
        <p className="reference-note">
          关联信号：{signals.length ? signals.map((item) => item.name).join('、') : '无'}
        </p>
      )}
    </div>
  );
}
