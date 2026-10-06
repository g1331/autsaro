import { CopyText, OwnedLog } from './workbench/Dialog';
import type { BuildTarget } from './types';
import type { Workbench } from './workbench/useWorkbench';

export function IntegrationDelivery({
  controller,
  active,
}: {
  controller: Workbench;
  active: boolean;
}) {
  const {
    workspace,
    native,
    ecuOutputDirectory: output,
    buildDirectory,
    ecuImportDirectory: importDirectory,
    legacyTarget: target,
    generationPreview: preview,
    generationPreviewPath: selected,
    generated,
    built,
    virtualResult: verified,
    changeEcuOutput: changeOutput,
    changeEcuBuildDirectory: changeBuildDirectory,
    setEcuImportDirectory: setImportDirectory,
    setGenerationPreviewPath: setSelected,
    chooseDirectory,
    importHandoffDirectory: onImport,
  } = controller;
  if (!workspace) return null;
  const busy = Boolean(controller.busy);
  const locked = busy;
  const sourceReady = !workspace.dirty && !controller.projection?.dirty && !controller.unapplied;
  const disabled = !native || busy || !sourceReady || Boolean(controller.capabilities?.ruleError);
  const executionReason = controller.capabilities?.nativeExecution
    ? (controller.capabilities.toolError ?? '')
    : '未执行：本机不支持所选目标执行';
  const executionDisabled =
    disabled ||
    !controller.capabilities?.nativeExecution ||
    Boolean(controller.capabilities.toolError);
  const notice = controller.notice?.text ?? '请选择新的交付目录；构建产物写入独立空目录。';
  const handoff = controller.generationKind === 'handoff';
  const file = preview?.files.find((item) => item.path === selected);
  const labels = {
    pending: '未执行',
    running: '执行中',
    done: '已通过',
    failed: '失败',
    stale: '已失效',
  };
  const stages = [
    ['保存', sourceReady ? '已保存' : '未保存／草稿未应用'],
    ['校验', sourceReady ? labels[controller.stages.validate.state] : '需重新校验'],
    ['生成', sourceReady ? labels[controller.stages.generate.state] : '已失效'],
    ['构建', sourceReady ? labels[controller.stages.build.state] : '已失效'],
    ['主机行为', sourceReady ? labels[controller.stages.virtual.state] : '已失效'],
    ['完整 SC1 工程等级复验', '当前工程未验证'],
    ['实机', '未验证'],
  ];
  return (
    <div className="workflow-page delivery-view" hidden={!active} data-testid="ecu-delivery">
      <div className="section-header">
        <div>
          <p className="eyebrow">ECU SOURCE DELIVERY</p>
          <h2>新目标生成、构建与主机验证</h2>
          <p>
            从已保存的标准输入交付 Windows／Linux x64 ECU
            源码。编译预检显式执行，主机行为仅覆盖本次实际向量。
          </p>
        </div>
      </div>
      <section className="document-page">
        <h3>用户 live 应用源</h3>
        <p>
          预览由实际组件契约提供槽、入口符号和
          seed；确认只创建不存在的源并更新成员清单，生成输出只封存其快照，不写回 live 树。
        </p>
        <button
          type="button"
          disabled={disabled || Boolean(controller.actionReason('edit'))}
          onClick={() => void controller.previewApplicationInitialization()}
        >
          预览初始化用户应用…
        </button>
        {controller.actionReason('edit') ? (
          <p className="field-help">{controller.actionReason('edit')}</p>
        ) : null}
        {controller.applicationWarnings.map((warning, index) => (
          <p key={index} className="notice info">
            {warning}
          </p>
        ))}
        {controller.applicationRecoveryFiles.length ? (
          <>
            <p className="error-text">后台保留以下恢复文件；勿删除或覆盖，先按实际警告处理。</p>
            <ul>
              {controller.applicationRecoveryFiles.map((path) => (
                <li key={path} className="mono path-text">
                  {path}
                  <CopyText text={path} label="复制恢复路径" />
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </section>
      <ol className="stage-list">
        {stages.map(([name, state], index) => (
          <li className="stage" key={name}>
            <span className="stage-number">{index + 1}</span>
            <div className="stage-copy">
              <strong>{name}</strong>
            </div>
            <span className="stage-pill" data-stage={name}>
              {state}
            </span>
          </li>
        ))}
      </ol>
      <div className="form-fields ecu-delivery-fields">
        <label>
          ECU 构建目标
          <select
            aria-label="ECU 构建目标"
            value={target}
            disabled={busy || locked}
            onChange={(event) => void controller.changeTarget(event.target.value as BuildTarget)}
          >
            <option value="windows-x64-controlled-v1">Windows x64 controlled v1</option>
            <option value="linux-x64-controlled-v1">Linux x64 controlled v1</option>
          </select>
        </label>
        <label>
          ECU 输出目录
          <input
            aria-label="ECU 输出目录"
            value={output}
            disabled={busy || locked}
            onChange={(e) => changeOutput(e.target.value)}
          />
        </label>
        <button
          className="outline-button small"
          type="button"
          disabled={busy || locked || !native}
          onClick={() =>
            void chooseDirectory((path) => {
              changeOutput(path);
            })
          }
        >
          选择 ECU 输出目录
        </button>
        <label>
          <input
            type="checkbox"
            checked={handoff}
            disabled={busy || locked}
            onChange={(e) => {
              controller.setGenerationKind(e.target.checked ? 'handoff' : 'project');
              changeOutput(output);
            }}
          />
          包含版本化交接元数据
        </label>
        <label>
          独立构建目录
          <input
            aria-label="ECU 构建目录"
            value={buildDirectory}
            disabled={busy || locked}
            onChange={(e) => changeBuildDirectory(e.target.value)}
          />
        </label>
        <button
          className="outline-button small"
          type="button"
          disabled={busy || locked || !native}
          onClick={() => void chooseDirectory(changeBuildDirectory)}
        >
          选择 ECU 构建目录
        </button>
      </div>
      <div className="section-actions">
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !output.trim()}
          onClick={controller.previewEcu}
        >
          预览 ECU 交付
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled}
          title={executionReason}
          onClick={controller.preflightEcu}
        >
          显式编译预检
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !preview}
          onClick={controller.generateEcu}
        >
          确认生成 ECU
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled || !generated || !buildDirectory.trim()}
          onClick={controller.buildEcu}
        >
          构建 ECU
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled || !built}
          onClick={controller.verifyEcu}
        >
          验证 ECU 主机行为
        </button>
      </div>
      <div className="form-fields ecu-delivery-fields">
        <label>
          重导入 ECU 目录
          <input
            aria-label="重导入 ECU 目录"
            value={importDirectory}
            disabled={busy || locked}
            onChange={(e) => setImportDirectory(e.target.value)}
          />
        </label>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !importDirectory.trim()}
          onClick={() => onImport(importDirectory)}
        >
          重导入 ECU 交接包
        </button>
      </div>
      <p role="status" className="page-guidance">
        {busy ? '后台处理真实目标，请等待完成。' : notice}
      </p>
      {preview && (
        <div className="generation-preview">
          <label>
            交付文件
            <select
              aria-label="ECU 预览文件"
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              {preview.files.map((item) => (
                <option key={item.path} value={item.path}>
                  {item.status} · {item.path}
                </option>
              ))}
            </select>
          </label>
          <pre aria-label="ECU 文件预览">{file?.after ?? '二进制或不可显示文件'}</pre>
        </div>
      )}
      {built && <OwnedLog text={built.log} label="ECU 构建日志" />}
      {verified && <OwnedLog text={verified.log} label="ECU 行为日志" />}
    </div>
  );
}
