import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { requestConfirmation } from './confirmation';
import type {
  BuildResult,
  BuildTarget,
  PreflightReport,
  GenerateResult,
  GenerationPreview,
  IntegrationInspection,
  VirtualResult,
  WorkspaceView,
} from './types';

type Props = {
  workspace: WorkspaceView;
  native: boolean;
  active: boolean;
  locked: boolean;
  unapplied: boolean;
  onBusyChange: (busy: boolean) => void;
  chooseDirectory: (onChoose: (path: string) => void) => Promise<void>;
  onImport: (directory: string) => void;
};
type Status = '未执行' | '执行中' | '已通过' | '失败';

function message(error: unknown): string {
  if (Array.isArray(error)) {
    return error.map((item) => `${item.code}: ${item.message}\n${item.remedy}`).join('\n');
  }
  return String(error);
}

export function IntegrationDelivery({
  workspace,
  native,
  active,
  locked,
  unapplied,
  onBusyChange,
  chooseDirectory,
  onImport,
}: Props) {
  const [output, setOutput] = useState('');
  const [buildDirectory, setBuildDirectory] = useState('');
  const [importDirectory, setImportDirectory] = useState('');
  const [handoff, setHandoff] = useState(true);
  const [target, setTarget] = useState<BuildTarget>('windows-x64-controlled-v1');
  const [preview, setPreview] = useState<GenerationPreview | null>(null);
  const [selected, setSelected] = useState('');
  const [generated, setGenerated] = useState<GenerateResult | null>(null);
  const [built, setBuilt] = useState<BuildResult | null>(null);
  const [verified, setVerified] = useState<VirtualResult | null>(null);
  const [validation, setValidation] = useState<Status>('未执行');
  const [generation, setGeneration] = useState<Status>('未执行');
  const [build, setBuild] = useState<Status>('未执行');
  const [behavior, setBehavior] = useState<Status>('未执行');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('请选择新的交付目录；构建产物写入独立空目录。');

  const [source, setSource] = useState(workspace);
  if (source !== workspace) {
    setSource(workspace);
    setPreview(null);
    setGenerated(null);
    setBuilt(null);
    setVerified(null);
    setValidation('未执行');
    setGeneration('未执行');
    setBuild('未执行');
    setBehavior('未执行');
    setNotice('输入已变化或重新打开；请重新校验、生成、构建和验证。');
  }

  const sourceReady = !workspace.dirty && !unapplied;
  const disabled = !native || busy || locked || !sourceReady;
  function changeOutput(path: string) {
    setOutput(path);
    setPreview(null);
    setGenerated(null);
    setBuilt(null);
    setVerified(null);
    setGeneration('未执行');
    setBuild('未执行');
    setBehavior('未执行');
  }
  function changeBuildDirectory(path: string) {
    setBuildDirectory(path);
    setBuilt(null);
    setVerified(null);
    setBuild('未执行');
    setBehavior('未执行');
  }
  async function perform(action: () => Promise<void>) {
    if (disabled) return;
    setBusy(true);
    onBusyChange(true);
    try {
      await action();
    } catch (error) {
      setNotice(message(error));
    } finally {
      setBusy(false);
      onBusyChange(false);
    }
  }
  async function prepare() {
    setPreview(null);
    setGenerated(null);
    setBuilt(null);
    setVerified(null);
    setValidation('执行中');
    setGeneration('未执行');
    setBuild('未执行');
    setBehavior('未执行');
    try {
      const inspection = await invoke<IntegrationInspection>('inspect_integration');
      if (!inspection.description) throw inspection.diagnostics;
    } catch (error) {
      setValidation('失败');
      throw error;
    }
    setValidation('已通过');
    setGeneration('执行中');
    try {
      const value = await invoke<GenerationPreview>('preview_ecu_project', {
        outputDirectory: output,
        handoff,
        target,
      });
      setPreview(value);
      setSelected(value.files[0]?.path ?? '');
      setGeneration('未执行');
      setNotice(`纯源码预览 ${value.files.length} 个文件；编译预检未执行，尚未写入输出目录。`);
    } catch (error) {
      setGeneration('失败');
      throw error;
    }
  }
  async function generate() {
    if (!preview) return;
    if (
      !(await requestConfirmation(
        `确认将 ${preview.files.length} 个文件写入 ${preview.outputDirectory}？`,
      ))
    ) {
      setNotice('已取消生成，输出目录未改动。');
      return;
    }
    setGeneration('执行中');
    try {
      const value = await invoke<GenerateResult>('generate_ecu_project', {
        outputDirectory: preview.outputDirectory,
        handoff,
        revision: preview.revision,
        target,
      });
      setGenerated(value);
      setPreview(null);
      setGeneration('已通过');
      setNotice(`已生成 ${value.files.length} 个文件：${value.outputDirectory}`);
    } catch (error) {
      setGeneration('失败');
      throw error;
    }
  }
  async function preflight() {
    const report = await invoke<PreflightReport>('preflight_ecu', { target, handoff });
    const status = { not_run: '未执行（非本机目标）', passed: '已通过', failed: '失败' }[
      report.status
    ];
    setNotice(`编译预检：${status}；源码身份：${report.fingerprint}。${report.logs.join('\n')}`);
  }
  async function compile() {
    if (!generated) return;
    setBuilt(null);
    setVerified(null);
    setBuild('执行中');
    setBehavior('未执行');
    try {
      const value = await invoke<BuildResult>('build_ecu', {
        outputDirectory: generated.outputDirectory,
        buildDirectory,
      });
      setBuilt(value);
      setBuild('已通过');
      setNotice(`实际主机二进制：${value.binaryPath}`);
    } catch (error) {
      setBuild('失败');
      throw error;
    }
  }
  async function verify() {
    if (!generated || !built) return;
    setVerified(null);
    setBehavior('执行中');
    try {
      const value = await invoke<VirtualResult>('verify_ecu', {
        outputDirectory: generated.outputDirectory,
      });
      setVerified(value);
      setBehavior(value.passed ? '已通过' : '失败');
      setNotice(
        value.passed
          ? 'CAN/DID、真实 N_Cr 超时恢复与非法批次拒绝通过。'
          : '主机行为检查失败；请查看实际日志。',
      );
    } catch (error) {
      setBehavior('失败');
      throw error;
    }
  }
  const file = preview?.files.find((item) => item.path === selected);
  const stages = [
    ['保存', sourceReady ? '已保存' : '未保存／草稿未应用'],
    ['校验', sourceReady ? validation : '需重新校验'],
    ['生成', sourceReady ? generation : '已失效'],
    ['构建', sourceReady ? build : '已失效'],
    ['主机行为', sourceReady ? behavior : '已失效'],
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
            onChange={(event) => {
              setTarget(event.target.value as BuildTarget);
              changeOutput(output);
              setNotice('目标已变化；编译预检未执行，请重新预览和生成。');
            }}
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
              setHandoff(e.target.checked);
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
          onClick={() => void perform(prepare)}
        >
          预览 ECU 交付
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled}
          onClick={() => void perform(preflight)}
        >
          显式编译预检
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !preview}
          onClick={() => void perform(generate)}
        >
          确认生成 ECU
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !generated || !buildDirectory.trim()}
          onClick={() => void perform(compile)}
        >
          构建 ECU
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !built}
          onClick={() => void perform(verify)}
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
      {built && <pre aria-label="ECU 构建日志">{built.log}</pre>}
      {verified && <pre aria-label="ECU 行为日志">{verified.log}</pre>}
    </div>
  );
}
