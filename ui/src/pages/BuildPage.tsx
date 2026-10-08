import { OwnedLog } from '../workbench/Dialog';
import type { Workbench } from '../workbench/useWorkbench';
import { Boxes, CircleCheck, Hammer } from 'lucide-react';
import { buildSteps, stageLabels } from '../workbench/useDelivery';
import type { BuildTarget } from '../types';
import { useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';

export function BuildPage({ controller }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const {
    unapplied,
    stages,
    workspace,
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
          <p className="eyebrow">{t('shell.build.eyebrow')}</p>
          <h2>{t('shell.build.title')}</h2>
          <p>{t('shell.build.description')}</p>
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
                <strong>{text(step.label)}</strong>
                <small title={displayPath(text(stages[step.key].detail))}>
                  {state === 'stale' && unapplied
                    ? t('shell.build.draftStale')
                    : displayPath(text(stages[step.key].detail))}
                </small>
              </div>
              <span className="stage-pill">
                {state === 'done' && <CircleCheck aria-hidden="true" size={13} />}
                {text(stageLabels[state])}
              </span>
            </li>
          );
        })}
      </ol>
      {(unapplied || workspace.dirty || stages.validate.state !== 'done') && (
        <div className="page-guidance">
          {t('shell.build.guidance')}
          <button
            type="button"
            onClick={() =>
              unapplied || workspace.dirty || controller.projection?.dirty
                ? controller.openDocument({ kind: 'communication' })
                : controller.setToolWindow('problems')
            }
          >
            {unapplied || workspace.dirty ? t('shell.go.configuration') : t('shell.go.diagnostics')}
          </button>
        </div>
      )}
      <div className="form-fields ecu-delivery-fields">
        <label>
          {t('shell.build.target')}
          <select
            aria-label={t('shell.build.target')}
            value={legacyTarget}
            disabled={!controller.native}
            onChange={(event) => void changeTarget(event.target.value as BuildTarget)}
          >
            <option value="windows-x64-controlled-v1">Windows x64 controlled v1</option>
            <option value="linux-x64-controlled-v1">Linux x64 controlled v1</option>
          </select>
        </label>
        <label>
          {t('shell.build.directory')}
          <input
            aria-label={t('shell.build.directory')}
            value={displayPath(buildDirectory)}
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
          {t('shell.build.chooseDirectory')}
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
          {t('shell.build.preview')}
        </button>
        <button
          type="button"
          className="outline-button"
          onClick={() => generateProject(true)}
          disabled={disabled || unapplied || workspace.dirty || stages.validate.state !== 'done'}
        >
          {t('shell.build.handoff')}
        </button>
        <button
          type="button"
          className="outline-button"
          onClick={buildProject}
          title={text(controller.executionReason)}
          disabled={
            controller.executionDisabled ||
            unapplied ||
            stages.generate.state !== 'done' ||
            !buildDirectory.trim()
          }
        >
          <Hammer aria-hidden="true" size={15} />
          {t('shell.build.build')}
        </button>
      </div>
      {generated && !unapplied && !workspace.dirty && stages.generate.state === 'done' && (
        <div className="result-section">
          <h3>{t('shell.build.location')}</h3>
          <p className="mono path-text">{displayPath(generated.outputDirectory)}</p>
          <p>{handoffGenerated ? t('shell.build.handoffHelp') : t('shell.build.projectHelp')}</p>
          <details>
            <summary>{t('shell.build.files', { count: generated.files.length })}</summary>
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
          {t('shell.build.previousLocation')}
          <span className="mono path-text">{displayPath(generated.previousOutputDirectory)}</span>
          {t('shell.build.previousHelp')}
        </div>
      )}
      {built && !unapplied && !workspace.dirty && stages.build.state === 'done' && (
        <div className="result-section">
          <h3>{t('shell.build.binary')}</h3>
          <p className="mono path-text">{displayPath(built.binaryPath)}</p>
          <details>
            <summary>{t('shell.build.log')}</summary>
            <OwnedLog text={built.log} label={t('shell.build.realLog')} />
          </details>
        </div>
      )}
    </div>
  );
}
