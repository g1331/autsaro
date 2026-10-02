import type { Workbench } from '../workbench/useWorkbench';
import { Boxes, CircleCheck, Hammer } from 'lucide-react';
import { buildSteps, stageLabels } from '../workbench/useDelivery';
import type { BuildTarget } from '../types';

export function BuildPage({ controller }: { controller: Workbench }) {
  const {
    unapplied,
    stages,
    workspace,
    setPage,
    legacyTarget,
    disabled,
    changeTarget,
    buildDirectory,
    changeEcuBuildDirectory,
    chooseDirectory,
    generateProject,
    buildProject,
    generated,
    handoffGenerated,
    built,
  } = controller;
  if (!workspace) return null;
  return (
    <div className="workflow-page delivery-view">
      <div className="section-header">
        <div>
          <p className="eyebrow">GENERATION / BUILD</p>
          <h2>生成与构建</h2>
          <p>
            从已保存、无阻断错误的配置生成独立 C99
            工程，再在独立空目录构建主机目标。封存源码不会被构建修改；已有产物不覆盖。
          </p>
        </div>
      </div>
      <ol className="stage-list">
        {buildSteps.map((step) => {
          const state =
            unapplied && stages[step.key].state === 'done' ? 'stale' : stages[step.key].state;
          return (
            <li key={step.key} className={`stage ${state}`}>
              <span className="stage-number">{step.number}</span>
              <div className="stage-copy">
                <strong>{step.label}</strong>
                <small title={stages[step.key].detail}>
                  {state === 'stale' && unapplied
                    ? '草稿未应用，需重新保存并校验'
                    : stages[step.key].detail}
                </small>
              </div>
              <span className="stage-pill">
                {state === 'done' && <CircleCheck aria-hidden="true" size={13} />}
                {stageLabels[state]}
              </span>
            </li>
          );
        })}
      </ol>
      {(unapplied || workspace.dirty || stages.validate.state !== 'done') && (
        <div className="page-guidance">
          生成前须应用更改、保存配置并完成无阻断错误的校验。
          <button
            type="button"
            onClick={() => setPage(unapplied || workspace.dirty ? 'editor' : 'diagnostics')}
          >
            前往{unapplied || workspace.dirty ? '配置' : '诊断'}
          </button>
        </div>
      )}
      <div className="form-fields ecu-delivery-fields">
        <label>
          主机构建目标
          <select
            aria-label="主机构建目标"
            value={legacyTarget}
            disabled={!controller.native}
            onChange={(event) => void changeTarget(event.target.value as BuildTarget)}
          >
            <option value="windows-x64-controlled-v1">Windows x64 controlled v1</option>
            <option value="linux-x64-controlled-v1">Linux x64 controlled v1</option>
          </select>
        </label>
        <label>
          独立主机构建目录
          <input
            aria-label="独立主机构建目录"
            value={buildDirectory}
            disabled={disabled}
            onChange={(event) => changeEcuBuildDirectory(event.target.value)}
          />
        </label>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled}
          onClick={() => void chooseDirectory(changeEcuBuildDirectory)}
        >
          选择独立主机构建目录
        </button>
      </div>
      <div className="delivery-actions">
        <button
          type="button"
          className="primary-button compact"
          onClick={() => generateProject()}
          disabled={disabled || unapplied || workspace.dirty || stages.validate.state !== 'done'}
        >
          <Boxes aria-hidden="true" size={15} />
          选择目录并预览工程
        </button>
        <button
          type="button"
          className="outline-button"
          onClick={() => generateProject(true)}
          disabled={disabled || unapplied || workspace.dirty || stages.validate.state !== 'done'}
        >
          导出可重建主机交付包
        </button>
        <button
          type="button"
          className="outline-button"
          onClick={buildProject}
          title={controller.executionReason}
          disabled={
            controller.executionDisabled ||
            unapplied ||
            stages.generate.state !== 'done' ||
            !buildDirectory.trim()
          }
        >
          <Hammer aria-hidden="true" size={15} />
          构建生成工程
        </button>
      </div>
      {generated && !unapplied && !workspace.dirty && stages.generate.state === 'done' && (
        <div className="result-section">
          <h3>生成工程位置</h3>
          <p className="mono path-text">{generated.outputDirectory}</p>
          <p>
            {handoffGenerated
              ? '交付包含源 ARXML 与 handoff.json；README.md 列明重新导入依赖。生成成功不代表主机行为已验证。'
              : '交付目录内的 README.md 与 tools/ecu-tool.py 提供独立构建和启动方法；源 ARXML 未随普通工程交付。'}
          </p>
          <details>
            <summary>工程文件 · {generated.files.length}</summary>
            <ul>
              {generated.files.map((file, index) => (
                <li key={`${file}-${index}`} className="mono">
                  {file}
                </li>
              ))}
            </ul>
          </details>
        </div>
      )}
      {generated?.previousOutputDirectory && (
        <div className="page-guidance" role="status">
          旧生成工程保留位置：
          <span className="mono path-text">{generated.previousOutputDirectory}</span>
          。工具不会自动清理；确认不再需要后由文件所有者自行移走或删除。
        </div>
      )}
      {built && !unapplied && !workspace.dirty && stages.build.state === 'done' && (
        <div className="result-section">
          <h3>构建二进制</h3>
          <p className="mono path-text">{built.binaryPath}</p>
          <details>
            <summary>构建日志</summary>
            <pre>{built.log}</pre>
          </details>
        </div>
      )}
    </div>
  );
}
