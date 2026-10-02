import type { Workbench } from '../workbench/useWorkbench';
import { CircleAlert, CircleCheck, FolderOpen, MonitorPlay } from 'lucide-react';
import { stageLabels } from '../workbench/useDelivery';

export function VirtualPage({ controller }: { controller: Workbench }) {
  const {
    unapplied,
    stages,
    built,
    setPage,
    peerDirectory,
    chooseDirectory,
    setPeerDirectory,
    setPeerBinaryPath,
    setVirtualResult,
    disabled,
    peerBinaryPath,
    chooseBinary,
    runVirtual,
    workspace,
    runDiagnostic,
    virtualResult,
    virtualKind,
  } = controller;
  if (!workspace) return null;
  return (
    <div className="workflow-page virtual-view">
      <div className="section-header">
        <div>
          <p className="eyebrow">HOST VIRTUAL BUS</p>
          <h2>主机虚拟运行</h2>
          <p>双 ECU 信号闭环与独立测试器诊断验证分别运行；结果不代表真实硬件符合性。</p>
        </div>
      </div>
      <div
        className={`virtual-status stage ${unapplied && stages.virtual.state === 'done' ? 'stale' : stages.virtual.state}`}
      >
        <span className="stage-number">05</span>
        <div className="stage-copy">
          <strong>主机虚拟验证</strong>
          <small>
            {unapplied && stages.virtual.state === 'done'
              ? '草稿未应用，结果已过期'
              : stages.virtual.detail}
          </small>
        </div>
        <span className="stage-pill">
          {!unapplied && stages.virtual.state === 'done' && (
            <CircleCheck aria-hidden="true" size={13} />
          )}
          {unapplied && stages.virtual.state === 'done'
            ? stageLabels.stale
            : stageLabels[stages.virtual.state]}
        </span>
      </div>
      {(!built || stages.build.state !== 'done' || unapplied) && (
        <div className="page-guidance">
          当前配置尚未完成可运行的主机目标构建。
          <button type="button" onClick={() => setPage(unapplied ? 'editor' : 'build')}>
            前往{unapplied ? '配置' : '生成与构建'}
          </button>
        </div>
      )}
      <div className="peer-section">
        <h3>对端 ECU 工程</h3>
        <p>选择对端封存源码目录及独立构建的实际二进制；两份真实 ECU 在虚拟总线上运行。</p>
        <div className="path-picker">
          <input
            readOnly
            value={peerDirectory}
            placeholder="选择对端生成工程目录"
            aria-label="对端生成工程目录"
          />
          <button
            type="button"
            onClick={() =>
              void chooseDirectory((path) => {
                setPeerDirectory(path);
                setPeerBinaryPath('');
                setVirtualResult(null);
              })
            }
            disabled={disabled || stages.build.state !== 'done'}
          >
            <FolderOpen aria-hidden="true" size={15} />
            选择对端目录
          </button>
        </div>
        <div className="path-picker">
          <input
            aria-label="对端主机二进制"
            value={peerBinaryPath}
            disabled={disabled}
            placeholder="选择对端独立构建的 ecu_host"
            onChange={(event) => {
              setPeerBinaryPath(event.target.value);
              setVirtualResult(null);
            }}
          />
          <button
            type="button"
            disabled={disabled || stages.build.state !== 'done'}
            onClick={() =>
              void chooseBinary((path) => {
                setPeerBinaryPath(path);
                setVirtualResult(null);
              })
            }
          >
            选择对端二进制
          </button>
        </div>
        <button
          type="button"
          className="primary-button compact"
          onClick={runVirtual}
          title={controller.executionReason}
          disabled={
            controller.executionDisabled ||
            unapplied ||
            stages.build.state !== 'done' ||
            !peerDirectory ||
            !peerBinaryPath
          }
        >
          <MonitorPlay aria-hidden="true" size={15} />
          运行虚拟闭环
        </button>
      </div>
      {workspace.diagnostic && (
        <div className="peer-section">
          <h3>诊断独立测试器</h3>
          <p>
            对当前生成 ECU 注入物理诊断 CAN 帧，核对会话、0xF186 活动会话 DID、实时 DID、多 DID
            顺序、流控、超时与故障恢复；无需对端 ECU 工程。
            {workspace.diagnostic.dtc &&
              '配置故障记忆时，还核对 Rx 超时 DTC、0x19 状态查询及支持的 DTC 列表、扩展会话 0x14/0xFFFFFF 清除，以及 0x85/0x02 暂停记录、0x85/0x01 恢复记录和隔离主机存储的跨进程重启持久化。'}
          </p>
          <button
            type="button"
            className="primary-button compact"
            onClick={runDiagnostic}
            title={controller.executionReason}
            disabled={controller.executionDisabled || unapplied || stages.build.state !== 'done'}
          >
            <MonitorPlay aria-hidden="true" size={15} />
            验证诊断连接
          </button>
        </div>
      )}
      {virtualResult &&
        !unapplied &&
        !workspace.dirty &&
        (stages.virtual.state === 'done' || stages.virtual.state === 'failed') && (
          <div className="result-section">
            <h3>
              {virtualKind === 'diagnostic' ? '诊断独立测试器' : '双 ECU 信号闭环'} ·{' '}
              {virtualResult.passed ? '通过' : '未通过'}
            </h3>
            <ul className="events-list">
              {virtualResult.events.map((event, index) => (
                <li key={index} className="mono">
                  {event}
                </li>
              ))}
            </ul>
            <details>
              <summary>完整运行日志</summary>
              <pre>{virtualResult.log}</pre>
            </details>
          </div>
        )}
      <div className="hardware-note">
        <CircleAlert aria-hidden="true" size={16} />
        <span>真实硬件未验证；主机虚拟运行结果不代表已上板。</span>
      </div>
    </div>
  );
}
