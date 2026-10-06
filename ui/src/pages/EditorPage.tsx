import type { Workbench } from '../workbench/useWorkbench';
import { Plus, Save } from 'lucide-react';
import { diagnosticFields, dtcFields, draftFor, labelFromPath } from '../workbench/forms';
import { FrameForm } from '../workbench/FrameForm';
import { SignalForm } from '../workbench/SignalForm';
import { ReferenceView } from '../workbench/ReferenceView';

export function EditorPage({
  controller,
  section,
}: {
  controller: Workbench;
  section: 'communication' | 'diagnostic';
}) {
  const {
    requestSave,
    disabled,
    native,
    busy,
    unsupportedIssue,
    unapplied,
    openCreator,
    workspace,
    focusedFrame,
    choose,
    currentSignal,
    diagnosticDraft,
    setDiagnosticDraft,
    diagnosticSignal,
    setDiagnosticSignal,
    eligibleSignals,
    diagnosticError,
    configureDiagnostic,
    diagnosticUnapplied,
    dtcUnapplied,
    frameUnapplied,
    setDiagnosticError,
    clearDiagnostic,
    dtcDraft,
    setDtcDraft,
    setDtcError,
    dtcError,
    eligibleMonitorFrames,
    configureDtc,
    clearDtc,
  } = controller;
  if (!workspace) return null;
  return (
    <>
      <div className="legacy-editor-content" hidden={section !== 'communication'}>
        <div className="section-header">
          <div>
            <p className="eyebrow">CAN COMMUNICATION</p>
            <h2>帧与信号</h2>
            <p>仅支持标准 11-bit CAN、DLC 1–8、原始无符号小端信号。</p>
          </div>
          <div className="section-actions">
            <button
              type="button"
              className="outline-button small"
              onClick={requestSave}
              disabled={!native || Boolean(busy) || Boolean(unsupportedIssue) || unapplied}
            >
              <Save aria-hidden="true" size={15} />
              查看并保存 ARXML
            </button>
            <button
              type="button"
              className="outline-button small"
              onClick={() => openCreator('frame')}
              disabled={disabled}
            >
              <Plus aria-hidden="true" size={15} />
              添加帧
            </button>
          </div>
        </div>
        {unsupportedIssue && (
          <div className="page-guidance" role="alert">
            {unsupportedIssue.message}。原 ARXML
            保持不变；请在“诊断”页查看问题，当前不能修改、保存或生成。
          </div>
        )}
        <div className="table-wrap">
          <table>
            <caption>CAN 帧配置</caption>
            <thead>
              <tr>
                <th scope="col">帧名称</th>
                <th scope="col">CAN ID</th>
                <th scope="col">DLC</th>
                <th scope="col">方向</th>
                <th scope="col">周期 / 超时</th>
                <th scope="col">信号</th>
              </tr>
            </thead>
            <tbody>
              {workspace.frames.map((frame) => (
                <tr
                  key={frame.path}
                  className={focusedFrame?.path === frame.path ? 'selected-row' : ''}
                  onClick={() => choose({ kind: 'frame', path: frame.path })}
                >
                  <td>
                    <button
                      type="button"
                      className="table-link"
                      onClick={(event) => {
                        event.stopPropagation();
                        choose({ kind: 'frame', path: frame.path });
                      }}
                    >
                      {frame.name}
                    </button>
                  </td>
                  <td className="mono">0x{frame.id.toString(16).toUpperCase().padStart(3, '0')}</td>
                  <td className="mono">{frame.dlc}</td>
                  <td>
                    <span className={`direction ${frame.direction}`}>
                      {frame.direction.toUpperCase()}
                    </span>
                  </td>
                  <td className="mono">
                    {frame.direction === 'tx'
                      ? `${frame.periodMs ?? '—'} ms`
                      : `${frame.timeoutMs ?? '—'} ms`}
                  </td>
                  <td className="mono">
                    {workspace.signals.filter((signal) => signal.framePath === frame.path).length}
                  </td>
                </tr>
              ))}
              {!workspace.frames.length && (
                <tr>
                  <td colSpan={6} className="empty-cell">
                    项目尚无 CAN 帧。使用“添加帧”开始配置。
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="section-header secondary">
          <div>
            <p className="eyebrow">FRAME MAPPING</p>
            <h2>{focusedFrame ? `${focusedFrame.name} · 信号` : '全部信号'}</h2>
            <p>
              {focusedFrame ? `帧路径：${focusedFrame.path}` : '选择一帧可查看信号映射与引用。'}
            </p>
          </div>
          <button
            type="button"
            className="outline-button small"
            onClick={() => openCreator('signal')}
            disabled={disabled || !focusedFrame}
          >
            <Plus aria-hidden="true" size={15} />
            添加信号
          </button>
        </div>
        <div className="table-wrap">
          <table>
            <caption>信号配置</caption>
            <thead>
              <tr>
                <th scope="col">信号名称</th>
                <th scope="col">所属帧</th>
                <th scope="col">起始位</th>
                <th scope="col">长度</th>
                <th scope="col">初始值</th>
                <th scope="col">编码</th>
              </tr>
            </thead>
            <tbody>
              {workspace.signals
                .filter((signal) => !focusedFrame || signal.framePath === focusedFrame.path)
                .map((signal) => (
                  <tr
                    key={signal.path}
                    className={currentSignal?.path === signal.path ? 'selected-row' : ''}
                    onClick={() => choose({ kind: 'signal', path: signal.path })}
                  >
                    <td>
                      <button
                        type="button"
                        className="table-link"
                        onClick={(event) => {
                          event.stopPropagation();
                          choose({ kind: 'signal', path: signal.path });
                        }}
                      >
                        {signal.name}
                      </button>
                    </td>
                    <td>
                      {workspace.frames.find((frame) => frame.path === signal.framePath)?.name ??
                        signal.framePath}
                    </td>
                    <td className="mono">{signal.startBit}</td>
                    <td className="mono">{signal.length} bit</td>
                    <td className="mono">{signal.initialValue}</td>
                    <td>uint / LE</td>
                  </tr>
                ))}
              {!workspace.signals.some(
                (signal) => !focusedFrame || signal.framePath === focusedFrame.path,
              ) && (
                <tr>
                  <td colSpan={6} className="empty-cell">
                    当前范围内暂无信号。
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
      <section
        className="diagnostic-editor"
        aria-labelledby="diagnostic-editor-title"
        hidden={section !== 'diagnostic'}
      >
        <div className="section-header secondary">
          <div>
            <p className="eyebrow">HOST VIRTUAL / DoCAN</p>
            <h2 id="diagnostic-editor-title">诊断通信配置</h2>
            <p>单条 11-bit 物理连接；扩展会话中一个 DID 按顺序读取实时 32-bit Tx 信号。</p>
          </div>
          <span className="diagnostic-state">{workspace.diagnostic ? '已配置' : '未配置'}</span>
        </div>
        {workspace.diagnostic && (
          <p className="diagnostic-path">
            配置路径 <span className="mono path-text">{workspace.diagnostic.path}</span>
          </p>
        )}
        <div className="diagnostic-fields form-fields">
          <div className="form-pair">
            <label>
              请求 CAN ID <small>0–2047 · 十进制或 0x 十六进制</small>
              <input
                value={diagnosticDraft.requestId}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    requestId: event.target.value,
                  })
                }
                placeholder="例如 0x700"
                disabled={disabled}
                autoComplete="off"
              />
            </label>
            <label>
              响应 CAN ID <small>不与 Com 帧冲突</small>
              <input
                value={diagnosticDraft.responseId}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    responseId: event.target.value,
                  })
                }
                placeholder="例如 0x708"
                disabled={disabled}
                autoComplete="off"
              />
            </label>
          </div>
          <div className="diagnostic-timers">
            <label>
              S3 <small>ms · 5000–2147483647</small>
              <input
                type="number"
                min="5000"
                max="2147483647"
                step="1"
                value={diagnosticDraft.s3Ms}
                onChange={(event) =>
                  setDiagnosticDraft({ ...diagnosticDraft, s3Ms: event.target.value })
                }
                disabled={disabled}
              />
            </label>
            <label>
              N_As <small>ms · 1–2147483647</small>
              <input
                type="number"
                min="1"
                max="2147483647"
                step="1"
                value={diagnosticDraft.nAsMs}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    nAsMs: event.target.value,
                  })
                }
                disabled={disabled}
              />
            </label>
            <label>
              N_Bs <small>ms · 1–2147483647</small>
              <input
                type="number"
                min="1"
                max="2147483647"
                step="1"
                value={diagnosticDraft.nBsMs}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    nBsMs: event.target.value,
                  })
                }
                disabled={disabled}
              />
            </label>
            <label>
              N_Cr <small>ms · 1–2147483647</small>
              <input
                type="number"
                min="1"
                max="2147483647"
                step="1"
                value={diagnosticDraft.nCrMs}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    nCrMs: event.target.value,
                  })
                }
                disabled={disabled}
              />
            </label>
          </div>
          <label>
            DID <small>0–65535 · 0xF186 保留</small>
            <input
              value={diagnosticDraft.did}
              onChange={(event) =>
                setDiagnosticDraft({ ...diagnosticDraft, did: event.target.value })
              }
              placeholder="例如 0xF190"
              disabled={disabled}
              autoComplete="off"
            />
          </label>
          <div className="signal-picker">
            <label htmlFor="diagnostic-signal">
              DID 信号顺序 <small>1–8 个 32-bit Tx 信号；每项占 4 字节</small>
            </label>
            <div className="signal-picker-controls">
              <select
                id="diagnostic-signal"
                value={diagnosticSignal}
                onChange={(event) => setDiagnosticSignal(event.target.value)}
                disabled={
                  disabled ||
                  diagnosticDraft.signalPaths.length >= 8 ||
                  !eligibleSignals.some(
                    (signal) => !diagnosticDraft.signalPaths.includes(signal.path),
                  )
                }
              >
                <option value="">选择 32-bit Tx 信号</option>
                {eligibleSignals
                  .filter((signal) => !diagnosticDraft.signalPaths.includes(signal.path))
                  .map((signal) => (
                    <option key={signal.path} value={signal.path}>
                      {signal.name} ·{' '}
                      {workspace.frames.find((frame) => frame.path === signal.framePath)?.name} ·{' '}
                      {signal.path}
                    </option>
                  ))}
              </select>
              <button
                type="button"
                className="outline-button small"
                onClick={() => {
                  if (diagnosticSignal) {
                    setDiagnosticDraft({
                      ...diagnosticDraft,
                      signalPaths: [...diagnosticDraft.signalPaths, diagnosticSignal],
                    });
                    setDiagnosticSignal('');
                  }
                }}
                disabled={disabled || !diagnosticSignal || diagnosticDraft.signalPaths.length >= 8}
              >
                <Plus aria-hidden="true" size={14} />
                添加
              </button>
            </div>
            {!eligibleSignals.length && (
              <p className="field-help">先在 Tx CAN 帧中配置至少一个 32-bit 信号。</p>
            )}
            <ol className="diagnostic-signal-list">
              {diagnosticDraft.signalPaths.map((path, index) => {
                const signal = workspace.signals.find((item) => item.path === path);
                return (
                  <li key={path}>
                    <span className="signal-order">{String(index + 1).padStart(2, '0')}</span>
                    <span className="signal-description">
                      <strong>{signal?.name ?? '信号不可用'}</strong>
                      <small className="mono path-text">{path}</small>
                    </span>
                    <div className="signal-order-actions">
                      <button
                        type="button"
                        aria-label={`上移 ${signal?.name ?? path}`}
                        onClick={() => {
                          const paths = [...diagnosticDraft.signalPaths];
                          [paths[index - 1], paths[index]] = [paths[index], paths[index - 1]];
                          setDiagnosticDraft({
                            ...diagnosticDraft,
                            signalPaths: paths,
                          });
                        }}
                        disabled={disabled || index === 0}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={`下移 ${signal?.name ?? path}`}
                        onClick={() => {
                          const paths = [...diagnosticDraft.signalPaths];
                          [paths[index], paths[index + 1]] = [paths[index + 1], paths[index]];
                          setDiagnosticDraft({
                            ...diagnosticDraft,
                            signalPaths: paths,
                          });
                        }}
                        disabled={disabled || index === diagnosticDraft.signalPaths.length - 1}
                      >
                        ↓
                      </button>
                      <button
                        type="button"
                        aria-label={`移除 ${signal?.name ?? path}`}
                        onClick={() =>
                          setDiagnosticDraft({
                            ...diagnosticDraft,
                            signalPaths: diagnosticDraft.signalPaths.filter(
                              (item) => item !== path,
                            ),
                          })
                        }
                        disabled={disabled}
                      >
                        移除
                      </button>
                    </div>
                  </li>
                );
              })}
            </ol>
          </div>
          <label className="diagnostic-write">
            <input
              type="checkbox"
              checked={diagnosticDraft.writeEnabled}
              onChange={(event) =>
                setDiagnosticDraft({
                  ...diagnosticDraft,
                  writeEnabled: event.target.checked,
                  resetRoutineId: event.target.checked ? diagnosticDraft.resetRoutineId : '',
                })
              }
              disabled={disabled}
            />
            <span>
              允许扩展会话写入此 DID (0x2E)
              <small>
                仅修改主机虚拟运行的应用状态；重启后恢复初始值。关闭写入也会关闭下方复位例程。
              </small>
            </span>
          </label>
          <label className="diagnostic-routine">
            复位例程 RID (0x31/0x01){' '}
            <small>可选 · 0–65535 · 十进制或 0x 十六进制；留空即关闭</small>
            <input
              value={diagnosticDraft.resetRoutineId}
              onChange={(event) =>
                setDiagnosticDraft({
                  ...diagnosticDraft,
                  resetRoutineId: event.target.value,
                })
              }
              placeholder="例如 0x0201"
              disabled={disabled || !diagnosticDraft.writeEnabled}
              autoComplete="off"
            />
            <small>
              仅在扩展会话中，将当前可写 DID 的 Tx 信号恢复为配置初始值；仅主机虚拟易失状态，不写入
              Flash/NvM；启用 0x27 档案时须先解锁。
            </small>
          </label>
          <label className="diagnostic-write">
            <input
              type="checkbox"
              checked={diagnosticDraft.securityEnabled}
              onChange={(event) =>
                setDiagnosticDraft({
                  ...diagnosticDraft,
                  securityEnabled: event.target.checked,
                })
              }
              disabled={disabled}
            />
            <span>
              用 0x27 保护状态更改
              <small>
                可选 · 单级 seed/key；保护 0x2E、0x31、0x14 和 0x85。须先启用写入或配置
                DTC。密钥在运行时从独立文件提供，不保存在 ARXML 中；仅限 Windows 主机虚拟目标。
              </small>
            </span>
          </label>
        </div>
        {diagnosticError && (
          <p className="diagnostic-error" role="alert">
            {diagnosticError}
          </p>
        )}
        <div className="diagnostic-actions">
          <button
            type="button"
            className="primary-button compact"
            onClick={configureDiagnostic}
            disabled={disabled || !diagnosticUnapplied || dtcUnapplied || frameUnapplied}
          >
            {workspace.diagnostic ? '应用诊断更改' : '创建诊断配置'}
          </button>
          <button
            type="button"
            className="quiet-button"
            onClick={() => {
              setDiagnosticDraft(diagnosticFields(workspace.diagnostic));
              setDiagnosticSignal('');
              setDiagnosticError('');
            }}
            disabled={disabled || !diagnosticUnapplied}
          >
            还原草稿
          </button>
          {workspace.diagnostic && (
            <button
              type="button"
              className="quiet-button"
              onClick={clearDiagnostic}
              disabled={disabled || diagnosticUnapplied || dtcUnapplied || frameUnapplied}
            >
              移除诊断配置
            </button>
          )}
          {diagnosticUnapplied && <span role="status">未应用的诊断草稿</span>}
        </div>
        <section className="dtc-editor" aria-labelledby="dtc-editor-title">
          <div className="dtc-heading">
            <div>
              <p className="eyebrow">HOST VIRTUAL / DEM + NVM</p>
              <h3 id="dtc-editor-title">
                故障记忆 <span className="dtc-optional">可选 · 单个 DTC</span>
              </h3>
            </div>
            <span className="diagnostic-state">
              {workspace.diagnostic?.dtc ? '已配置' : '未配置'}
            </span>
          </div>
          <p className="dtc-copy">
            有效 Rx 帧首次到达后，若该帧超时，Dem 记录故障状态并由主机 NvM 持久化。支持
            0x19/0x01、0x19/0x02 状态掩码查询、0x19/0x0A 读取支持的 DTC
            列表（无故障时也可发现监测项）、扩展会话 0x14/0xFFFFFF 清除，以及扩展会话 0x85/0x02
            暂停、0x85/0x01 恢复 DTC 设置。暂停时已有故障仍可读取和清除；切回默认会话（含 S3
            超时）或重启后自动恢复记录。不代表完整 Dem/NvM、硬件或标准符合性。
          </p>
          {!workspace.diagnostic ? (
            <p className="field-help">先应用上方 DoCAN 配置，再添加故障记忆。</p>
          ) : (
            <>
              {workspace.diagnostic.dtc && (
                <p className="diagnostic-path">
                  配置路径 <span className="mono path-text">{workspace.diagnostic.dtc.path}</span>
                </p>
              )}
              <div className="dtc-fields form-fields">
                <label>
                  DTC 代码 <small>24-bit · 0x000100–0xFFFFFE · 十进制或 0x 十六进制</small>
                  <input
                    value={dtcDraft.code}
                    onChange={(event) => {
                      setDtcDraft({ ...dtcDraft, code: event.target.value });
                      setDtcError('');
                    }}
                    placeholder="例如 0x123456"
                    disabled={disabled}
                    autoComplete="off"
                    aria-invalid={Boolean(dtcError)}
                    aria-describedby={dtcError ? 'dtc-error' : undefined}
                  />
                </label>
                <label>
                  监测 Rx CAN 帧 <small>至少一个信号 · 接收超时大于 0</small>
                  <select
                    value={dtcDraft.monitorFramePath}
                    onChange={(event) => {
                      setDtcDraft({
                        ...dtcDraft,
                        monitorFramePath: event.target.value,
                      });
                      setDtcError('');
                    }}
                    disabled={disabled || !eligibleMonitorFrames.length}
                    aria-invalid={Boolean(dtcError)}
                    aria-describedby={dtcError ? 'dtc-error' : undefined}
                  >
                    <option value="">选择监测帧</option>
                    {dtcDraft.monitorFramePath &&
                      !eligibleMonitorFrames.some(
                        (frame) => frame.path === dtcDraft.monitorFramePath,
                      ) && (
                        <option value={dtcDraft.monitorFramePath}>
                          原监测帧已不符合条件 · 请重新选择
                        </option>
                      )}
                    {eligibleMonitorFrames.map((frame) => (
                      <option key={frame.path} value={frame.path}>
                        {frame.name} · 0x
                        {frame.id.toString(16).toUpperCase().padStart(3, '0')} · {frame.timeoutMs}{' '}
                        ms
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              {!eligibleMonitorFrames.length && (
                <p className="field-help">
                  先添加一条接收超时大于 0 的 Rx 帧，并在帧中添加至少一个信号。
                </p>
              )}
              {dtcError && (
                <p id="dtc-error" className="diagnostic-error" role="alert">
                  {dtcError}
                </p>
              )}
              <div className="diagnostic-actions">
                <button
                  type="button"
                  className="primary-button compact"
                  onClick={configureDtc}
                  disabled={disabled || !dtcUnapplied || diagnosticUnapplied || frameUnapplied}
                >
                  {workspace.diagnostic.dtc ? '应用故障记忆更改' : '配置故障记忆'}
                </button>
                <button
                  type="button"
                  className="quiet-button"
                  onClick={() => {
                    setDtcDraft(dtcFields(workspace.diagnostic?.dtc ?? null));
                    setDtcError('');
                  }}
                  disabled={disabled || !dtcUnapplied}
                >
                  还原草稿
                </button>
                {workspace.diagnostic.dtc && (
                  <button
                    type="button"
                    className="quiet-button"
                    onClick={clearDtc}
                    disabled={disabled || unapplied}
                  >
                    移除故障记忆
                  </button>
                )}
                {dtcUnapplied && <span role="status">未应用的故障记忆草稿</span>}
              </div>
            </>
          )}
        </section>
      </section>
    </>
  );
}

export function EditorInspector({ controller }: { controller: Workbench }) {
  const {
    creating,
    focusedFrame,
    frameInput,
    setFrameInput,
    disabled,
    signalInput,
    setSignalInput,
    addFrame,
    addSignal,
    setCreating,
    currentFile,
    currentFrame,
    draft,
    setDraft,
    updateSelected,
    unapplied,
    workspace,
    selection,
    currentSignal,
    signalFrame,
    issues,
  } = controller;
  if (!workspace) return null;
  return (
    <aside className="inspector" aria-label="对象检查器">
      <div className="pane-overline">OBJECT INSPECTOR</div>
      {creating ? (
        <>
          <p className="eyebrow">CREATE / {creating.toUpperCase()}</p>
          <h2>{creating === 'frame' ? '添加 CAN 帧' : '添加信号'}</h2>
          <p className="inspector-intro">
            {creating === 'frame'
              ? '标准 11-bit ID，最多 8 字节。'
              : `所属帧：${focusedFrame?.name ?? '未选择'}`}
          </p>
          {creating === 'frame' ? (
            <FrameForm fields={frameInput} onChange={setFrameInput} disabled={disabled} />
          ) : (
            <SignalForm fields={signalInput} onChange={setSignalInput} disabled={disabled} />
          )}
          <div className="inspector-actions">
            <button
              type="button"
              className="primary-button compact"
              onClick={creating === 'frame' ? addFrame : addSignal}
              disabled={disabled || (creating === 'signal' && !focusedFrame)}
            >
              添加{creating === 'frame' ? '帧' : '信号'}
            </button>
            <button type="button" className="quiet-button" onClick={() => setCreating(null)}>
              取消
            </button>
          </div>
        </>
      ) : currentFile ? (
        <>
          <p className="eyebrow">SOURCE FILE</p>
          <h2 title={currentFile.path}>{labelFromPath(currentFile.path)}</h2>
          <p className="inspector-intro">源文件属于当前项目配置集合。</p>
          <dl className="property-list">
            <div>
              <dt>绝对路径</dt>
              <dd className="mono path-text">{currentFile.path}</dd>
            </div>
            <div>
              <dt>访问</dt>
              <dd>{currentFile.readonly ? '只读' : '可写'}</dd>
            </div>
            <div>
              <dt>保留项</dt>
              <dd>{currentFile.retainedCount}</dd>
            </div>
          </dl>
          <div className="inspector-block">
            <h3>保留策略</h3>
            <p>
              未支持内容保持原文件语义；编辑关联对象时由内核判断引用安全。不提供保留项的直接编辑。
            </p>
          </div>
        </>
      ) : currentFrame && draft?.kind === 'frame' ? (
        <>
          <p className="eyebrow">CAN FRAME / {currentFrame.direction.toUpperCase()}</p>
          <h2>{currentFrame.name}</h2>
          <p className="inspector-intro mono path-text">{currentFrame.path}</p>
          <FrameForm
            fields={draft.fields}
            onChange={(fields) => setDraft({ ...draft, fields })}
            disabled={disabled}
          />
          <div className="inspector-actions">
            <button
              type="button"
              className="primary-button compact"
              onClick={updateSelected}
              disabled={disabled || !unapplied}
            >
              应用更改
            </button>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDraft(draftFor(workspace, selection))}
              disabled={!unapplied}
            >
              还原
            </button>
          </div>
          <ReferenceView
            frame={currentFrame}
            signals={workspace.signals.filter((signal) => signal.framePath === currentFrame.path)}
          />
        </>
      ) : currentSignal && draft?.kind === 'signal' ? (
        <>
          <p className="eyebrow">CAN SIGNAL / UINT LE</p>
          <h2>{currentSignal.name}</h2>
          <p className="inspector-intro mono path-text">{currentSignal.path}</p>
          <SignalForm
            fields={draft.fields}
            onChange={(fields) => setDraft({ ...draft, fields })}
            disabled={disabled}
          />
          <div className="inspector-actions">
            <button
              type="button"
              className="primary-button compact"
              onClick={updateSelected}
              disabled={disabled || !unapplied}
            >
              应用更改
            </button>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDraft(draftFor(workspace, selection))}
              disabled={!unapplied}
            >
              还原
            </button>
          </div>
          {signalFrame && <ReferenceView frame={signalFrame} signal={currentSignal} />}
        </>
      ) : (
        <div className="inspector-empty">
          <strong>选择对象</strong>
          <p>在工程树或表格中选择文件、帧或信号，以检查属性及真实引用。</p>
        </div>
      )}
      {selection &&
        issues.some((issue) => issue.path === selection.path || issue.file === selection.path) && (
          <div className="inspector-issues">
            <h3>相关诊断</h3>
            {issues
              .filter((issue) => issue.path === selection.path || issue.file === selection.path)
              .map((issue, index) => (
                <p key={`${issue.code}-${index}`} className={issue.severity}>
                  {issue.code} · {issue.message}
                </p>
              ))}
          </div>
        )}
    </aside>
  );
}
