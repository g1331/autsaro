import { useLocale } from '../i18n';
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
  const { t, text } = useLocale();
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
            <p className="eyebrow">{t('editor.can.overline')}</p>
            <h2>{t('editor.can.heading')}</h2>
            <p>{t('editor.can.support')}</p>
          </div>
          <div className="section-actions">
            <button
              type="button"
              className="outline-button small"
              onClick={requestSave}
              disabled={!native || Boolean(busy) || Boolean(unsupportedIssue) || unapplied}
            >
              <Save aria-hidden="true" size={15} />
              {t('editor.can.save')}
            </button>
            <button
              type="button"
              className="outline-button small"
              onClick={() => openCreator('frame')}
              disabled={disabled}
            >
              <Plus aria-hidden="true" size={15} />
              {t('editor.can.addFrame')}
            </button>
          </div>
        </div>
        {unsupportedIssue && (
          <div className="page-guidance" role="alert">
            {text(unsupportedIssue.message)}
            {t('editor.can.unsupportedHelp')}
          </div>
        )}
        <div className="table-wrap">
          <table>
            <caption>{t('editor.can.caption')}</caption>
            <thead>
              <tr>
                <th scope="col">{t('editor.can.frameName')}</th>
                <th scope="col">CAN ID</th>
                <th scope="col">DLC</th>
                <th scope="col">{t('editor.can.direction')}</th>
                <th scope="col">{t('editor.can.timing')}</th>
                <th scope="col">{t('editor.can.signals')}</th>
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
                    {t('editor.can.emptyFrames')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="section-header secondary">
          <div>
            <p className="eyebrow">{t('editor.can.mappingOverline')}</p>
            <h2>
              {focusedFrame
                ? t('editor.can.frameSignals', { name: focusedFrame.name })
                : t('editor.can.allSignals')}
            </h2>
            <p>
              {focusedFrame
                ? t('editor.can.framePath', { path: focusedFrame.path })
                : t('editor.can.chooseFrame')}
            </p>
          </div>
          <button
            type="button"
            className="outline-button small"
            onClick={() => openCreator('signal')}
            disabled={disabled || !focusedFrame}
          >
            <Plus aria-hidden="true" size={15} />
            {t('editor.can.addSignal')}
          </button>
        </div>
        <div className="table-wrap">
          <table>
            <caption>{t('editor.can.signalCaption')}</caption>
            <thead>
              <tr>
                <th scope="col">{t('editor.can.signalName')}</th>
                <th scope="col">{t('editor.can.parentFrame')}</th>
                <th scope="col">{t('editor.can.startBit')}</th>
                <th scope="col">{t('editor.can.length')}</th>
                <th scope="col">{t('editor.can.initial')}</th>
                <th scope="col">{t('editor.can.encoding')}</th>
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
                    {t('editor.can.emptySignals')}
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
            <p className="eyebrow">{t('editor.diagnostic.overline')}</p>
            <h2 id="diagnostic-editor-title">{t('editor.diagnostic.heading')}</h2>
            <p>{t('editor.diagnostic.intro')}</p>
          </div>
          <span className="diagnostic-state">
            {workspace.diagnostic
              ? t('editor.common.configured')
              : t('editor.common.notConfigured')}
          </span>
        </div>
        {workspace.diagnostic && (
          <p className="diagnostic-path">
            {t('editor.common.path')}{' '}
            <span className="mono path-text">{workspace.diagnostic.path}</span>
          </p>
        )}
        <div className="diagnostic-fields form-fields">
          <div className="form-pair">
            <label>
              {t('editor.diagnostic.requestId')} <small>{t('editor.diagnostic.idHelp')}</small>
              <input
                value={diagnosticDraft.requestId}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    requestId: event.target.value,
                  })
                }
                placeholder={t('editor.diagnostic.exampleRequest')}
                disabled={disabled}
                autoComplete="off"
              />
            </label>
            <label>
              {t('editor.diagnostic.responseId')}{' '}
              <small>{t('editor.diagnostic.responseHelp')}</small>
              <input
                value={diagnosticDraft.responseId}
                onChange={(event) =>
                  setDiagnosticDraft({
                    ...diagnosticDraft,
                    responseId: event.target.value,
                  })
                }
                placeholder={t('editor.diagnostic.exampleResponse')}
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
            DID <small>{t('editor.diagnostic.didHelp')}</small>
            <input
              value={diagnosticDraft.did}
              onChange={(event) =>
                setDiagnosticDraft({ ...diagnosticDraft, did: event.target.value })
              }
              placeholder={t('editor.diagnostic.exampleDid')}
              disabled={disabled}
              autoComplete="off"
            />
          </label>
          <div className="signal-picker">
            <label htmlFor="diagnostic-signal">
              {t('editor.diagnostic.signalOrder')}{' '}
              <small>{t('editor.diagnostic.signalHelp')}</small>
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
                <option value="">{t('editor.diagnostic.chooseSignal')}</option>
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
                {t('editor.common.add')}
              </button>
            </div>
            {!eligibleSignals.length && (
              <p className="field-help">{t('editor.diagnostic.needSignal')}</p>
            )}
            <ol className="diagnostic-signal-list">
              {diagnosticDraft.signalPaths.map((path, index) => {
                const signal = workspace.signals.find((item) => item.path === path);
                return (
                  <li key={path}>
                    <span className="signal-order">{String(index + 1).padStart(2, '0')}</span>
                    <span className="signal-description">
                      <strong>{signal?.name ?? t('editor.diagnostic.signalUnavailable')}</strong>
                      <small className="mono path-text">{path}</small>
                    </span>
                    <div className="signal-order-actions">
                      <button
                        type="button"
                        aria-label={t('editor.diagnostic.moveUp', { name: signal?.name ?? path })}
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
                        aria-label={t('editor.diagnostic.moveDown', { name: signal?.name ?? path })}
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
                        aria-label={t('editor.diagnostic.removeSignal', {
                          name: signal?.name ?? path,
                        })}
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
                        {t('editor.diagnostic.remove')}
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
              {t('editor.diagnostic.allowWrite')}
              <small>{t('editor.diagnostic.writeHelp')}</small>
            </span>
          </label>
          <label className="diagnostic-routine">
            {t('editor.diagnostic.resetRid')} <small>{t('editor.diagnostic.ridHelp')}</small>
            <input
              value={diagnosticDraft.resetRoutineId}
              onChange={(event) =>
                setDiagnosticDraft({
                  ...diagnosticDraft,
                  resetRoutineId: event.target.value,
                })
              }
              placeholder={t('editor.diagnostic.exampleRid')}
              disabled={disabled || !diagnosticDraft.writeEnabled}
              autoComplete="off"
            />
            <small>{t('editor.diagnostic.resetHelp')}</small>
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
              {t('editor.diagnostic.security')}
              <small>{t('editor.diagnostic.securityHelp')}</small>
            </span>
          </label>
        </div>
        {diagnosticError && (
          <p className="diagnostic-error" role="alert">
            {text(diagnosticError)}
          </p>
        )}
        <div className="diagnostic-actions">
          <button
            type="button"
            className="primary-button compact"
            onClick={configureDiagnostic}
            disabled={disabled || !diagnosticUnapplied || dtcUnapplied || frameUnapplied}
          >
            {workspace.diagnostic ? t('editor.diagnostic.apply') : t('editor.diagnostic.create')}
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
            {t('editor.common.restoreDraft')}
          </button>
          {workspace.diagnostic && (
            <button
              type="button"
              className="quiet-button"
              onClick={clearDiagnostic}
              disabled={disabled || diagnosticUnapplied || dtcUnapplied || frameUnapplied}
            >
              {t('editor.diagnostic.removeConfig')}
            </button>
          )}
          {diagnosticUnapplied && <span role="status">{t('editor.diagnostic.draft')}</span>}
        </div>
        <section className="dtc-editor" aria-labelledby="dtc-editor-title">
          <div className="dtc-heading">
            <div>
              <p className="eyebrow">{t('editor.dtc.overline')}</p>
              <h3 id="dtc-editor-title">
                {t('editor.dtc.heading')}{' '}
                <span className="dtc-optional">{t('editor.dtc.optional')}</span>
              </h3>
            </div>
            <span className="diagnostic-state">
              {workspace.diagnostic?.dtc
                ? t('editor.common.configured')
                : t('editor.common.notConfigured')}
            </span>
          </div>
          <p className="dtc-copy">{t('editor.dtc.help')}</p>
          {!workspace.diagnostic ? (
            <p className="field-help">{t('editor.dtc.needDiagnostic')}</p>
          ) : (
            <>
              {workspace.diagnostic.dtc && (
                <p className="diagnostic-path">
                  {t('editor.common.path')}{' '}
                  <span className="mono path-text">{workspace.diagnostic.dtc.path}</span>
                </p>
              )}
              <div className="dtc-fields form-fields">
                <label>
                  {t('editor.dtc.code')} <small>{t('editor.dtc.codeHelp')}</small>
                  <input
                    value={dtcDraft.code}
                    onChange={(event) => {
                      setDtcDraft({ ...dtcDraft, code: event.target.value });
                      setDtcError('');
                    }}
                    placeholder={t('editor.dtc.example')}
                    disabled={disabled}
                    autoComplete="off"
                    aria-invalid={Boolean(dtcError)}
                    aria-describedby={dtcError ? 'dtc-error' : undefined}
                  />
                </label>
                <label>
                  {t('editor.dtc.monitor')} <small>{t('editor.dtc.monitorHelp')}</small>
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
                    <option value="">{t('editor.dtc.choose')}</option>
                    {dtcDraft.monitorFramePath &&
                      !eligibleMonitorFrames.some(
                        (frame) => frame.path === dtcDraft.monitorFramePath,
                      ) && (
                        <option value={dtcDraft.monitorFramePath}>
                          {t('editor.dtc.invalidFrame')}
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
                <p className="field-help">{t('editor.dtc.needFrame')}</p>
              )}
              {dtcError && (
                <p id="dtc-error" className="diagnostic-error" role="alert">
                  {text(dtcError)}
                </p>
              )}
              <div className="diagnostic-actions">
                <button
                  type="button"
                  className="primary-button compact"
                  onClick={configureDtc}
                  disabled={disabled || !dtcUnapplied || diagnosticUnapplied || frameUnapplied}
                >
                  {workspace.diagnostic.dtc ? t('editor.dtc.apply') : t('editor.dtc.configure')}
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
                  {t('editor.common.restoreDraft')}
                </button>
                {workspace.diagnostic.dtc && (
                  <button
                    type="button"
                    className="quiet-button"
                    onClick={clearDtc}
                    disabled={disabled || unapplied}
                  >
                    {t('editor.dtc.remove')}
                  </button>
                )}
                {dtcUnapplied && <span role="status">{t('editor.dtc.draft')}</span>}
              </div>
            </>
          )}
        </section>
      </section>
    </>
  );
}

export function EditorInspector({ controller }: { controller: Workbench }) {
  const { t, text } = useLocale();
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
    <aside className="inspector" aria-label={t('editor.inspector.label')}>
      <div className="pane-overline">{t('editor.inspector.overline')}</div>
      {creating ? (
        <>
          <p className="eyebrow">
            {t(
              creating === 'frame'
                ? 'editor.inspector.createFrameOverline'
                : 'editor.inspector.createSignalOverline',
            )}
          </p>
          <h2>{creating === 'frame' ? t('editor.can.addCanFrame') : t('editor.can.addSignal')}</h2>
          <p className="inspector-intro">
            {creating === 'frame'
              ? t('editor.inspector.frameIntro')
              : t('editor.inspector.parentFrame', {
                  name: focusedFrame?.name ?? t('editor.common.noSelection'),
                })}
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
              {creating === 'frame' ? t('editor.can.addFrame') : t('editor.can.addSignal')}
            </button>
            <button type="button" className="quiet-button" onClick={() => setCreating(null)}>
              {t('editor.common.cancel')}
            </button>
          </div>
        </>
      ) : currentFile ? (
        <>
          <p className="eyebrow">{t('editor.inspector.sourceOverline')}</p>
          <h2 title={currentFile.path}>{labelFromPath(currentFile.path)}</h2>
          <p className="inspector-intro">{t('editor.inspector.sourceIntro')}</p>
          <dl className="property-list">
            <div>
              <dt>{t('editor.inspector.absolutePath')}</dt>
              <dd className="mono path-text">{currentFile.path}</dd>
            </div>
            <div>
              <dt>{t('editor.common.access')}</dt>
              <dd>
                {currentFile.readonly ? t('editor.common.readonly') : t('editor.common.writable')}
              </dd>
            </div>
            <div>
              <dt>{t('editor.inspector.retained')}</dt>
              <dd>{currentFile.retainedCount}</dd>
            </div>
          </dl>
          <div className="inspector-block">
            <h3>{t('editor.inspector.policy')}</h3>
            <p>{t('editor.inspector.policyHelp')}</p>
          </div>
        </>
      ) : currentFrame && draft?.kind === 'frame' ? (
        <>
          <p className="eyebrow">
            {t('editor.inspector.frameOverline', {
              direction: currentFrame.direction.toUpperCase(),
            })}
          </p>
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
              {t('editor.common.apply')}
            </button>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDraft(draftFor(workspace, selection))}
              disabled={!unapplied}
            >
              {t('editor.common.restore')}
            </button>
          </div>
          <ReferenceView
            frame={currentFrame}
            signals={workspace.signals.filter((signal) => signal.framePath === currentFrame.path)}
          />
        </>
      ) : currentSignal && draft?.kind === 'signal' ? (
        <>
          <p className="eyebrow">{t('editor.inspector.signalOverline')}</p>
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
              {t('editor.common.apply')}
            </button>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDraft(draftFor(workspace, selection))}
              disabled={!unapplied}
            >
              {t('editor.common.restore')}
            </button>
          </div>
          {signalFrame && <ReferenceView frame={signalFrame} signal={currentSignal} />}
        </>
      ) : (
        <div className="inspector-empty">
          <strong>{t('editor.inspector.select')}</strong>
          <p>{t('editor.inspector.empty')}</p>
        </div>
      )}
      {selection &&
        issues.some((issue) => issue.path === selection.path || issue.file === selection.path) && (
          <div className="inspector-issues">
            <h3>{t('editor.inspector.diagnostics')}</h3>
            {issues
              .filter((issue) => issue.path === selection.path || issue.file === selection.path)
              .map((issue, index) => (
                <p key={`${issue.code}-${index}`} className={issue.severity}>
                  {issue.code} · {text(issue.message)}
                </p>
              ))}
          </div>
        )}
    </aside>
  );
}
