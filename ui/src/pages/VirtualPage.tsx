import { CopyText, OwnedLog } from '../workbench/Dialog';
import type { Workbench } from '../workbench/useWorkbench';
import { CircleAlert, CircleCheck, FolderOpen, MonitorPlay } from 'lucide-react';
import { stageLabels } from '../workbench/useDelivery';
import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';

export function VirtualPage({ controller }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const {
    unapplied,
    stages,
    built,
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
          <p className="eyebrow">{t('shell.virtual.eyebrow')}</p>
          <h2>{t('shell.virtual.title')}</h2>
          <p>{t('shell.virtual.description')}</p>
        </div>
      </div>
      <div
        className={`virtual-status stage ${unapplied && stages.virtual.state === 'done' ? 'stale' : stages.virtual.state}`}
      >
        <span className="stage-number">05</span>
        <div className="stage-copy">
          <strong>{t('shell.virtual.stage')}</strong>
          <small>
            {unapplied && stages.virtual.state === 'done'
              ? t('shell.virtual.draftStale')
              : text(stages.virtual.detail)}
          </small>
        </div>
        <span className="stage-pill">
          {!unapplied && stages.virtual.state === 'done' && (
            <CircleCheck aria-hidden="true" size={13} />
          )}
          {unapplied && stages.virtual.state === 'done'
            ? text(stageLabels.stale)
            : text(stageLabels[stages.virtual.state])}
        </span>
      </div>
      {(!built || stages.build.state !== 'done' || unapplied) && (
        <div className="page-guidance">
          {t('shell.virtual.guidance')}
          <button
            type="button"
            onClick={() =>
              controller.openDocument({ kind: unapplied ? 'communication' : 'delivery' })
            }
          >
            {unapplied ? t('shell.go.configuration') : t('shell.go.build')}
          </button>
        </div>
      )}
      <div className="peer-section">
        <h3>{t('shell.virtual.peer')}</h3>
        <p>{t('shell.virtual.peerHelp')}</p>
        <div className="path-picker">
          <input
            readOnly
            value={displayPath(peerDirectory)}
            placeholder={t('shell.virtual.peerPlaceholder')}
            aria-label={t('shell.virtual.peerDirectory')}
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
            {t('shell.virtual.choosePeer')}
          </button>
        </div>
        <div className="path-picker">
          <input
            aria-label={t('shell.virtual.peerBinary')}
            value={displayPath(peerBinaryPath)}
            disabled={disabled}
            placeholder={t('shell.virtual.binaryPlaceholder')}
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
            {t('shell.virtual.chooseBinary')}
          </button>
        </div>
        <button
          type="button"
          className="primary-button compact"
          onClick={runVirtual}
          title={text(controller.executionReason)}
          disabled={
            controller.executionDisabled ||
            unapplied ||
            stages.build.state !== 'done' ||
            !peerDirectory ||
            !peerBinaryPath
          }
        >
          <MonitorPlay aria-hidden="true" size={15} />
          {t('shell.virtual.run')}
        </button>
      </div>
      {workspace.diagnostic && (
        <div className="peer-section">
          <h3>{t('shell.virtual.tester')}</h3>
          <p>
            {t('shell.virtual.diagnosticHelp')}
            {workspace.diagnostic.dtc && t('shell.virtual.dtcHelp')}
          </p>
          <button
            type="button"
            className="primary-button compact"
            onClick={runDiagnostic}
            title={text(controller.executionReason)}
            disabled={controller.executionDisabled || unapplied || stages.build.state !== 'done'}
          >
            <MonitorPlay aria-hidden="true" size={15} />
            {t('shell.virtual.verifyDiagnostic')}
          </button>
        </div>
      )}
      {virtualResult &&
        !unapplied &&
        !workspace.dirty &&
        (stages.virtual.state === 'done' || stages.virtual.state === 'failed') && (
          <div className="result-section">
            <h3>
              {virtualKind === 'diagnostic'
                ? t('shell.virtual.tester')
                : t('shell.virtual.signalLoop')}{' '}
              · {virtualResult.passed ? t('shell.virtual.passed') : t('shell.virtual.failed')}
            </h3>
            <ul className="events-list">
              {virtualResult.events.slice(-100).map((event, index) => (
                <li key={index} className="mono">
                  {text(event)}
                </li>
              ))}
            </ul>
            {virtualResult.events.length > 100 && (
              <p className="field-help">{t('shell.virtual.recentEvents')}</p>
            )}
            <CopyText
              text={virtualResult.events.map((event) => text(event)).join('\n')}
              label={t('shell.virtual.copyEvents')}
            />
            <details>
              <summary>{t('shell.virtual.fullLog')}</summary>
              <OwnedLog text={virtualResult.log} label={t('shell.virtual.realLog')} />
            </details>
          </div>
        )}
      <div className="hardware-note">
        <CircleAlert aria-hidden="true" size={16} />
        <span>{t('shell.virtual.hardware')}</span>
      </div>
    </div>
  );
}
