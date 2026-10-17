import { CircleCheck, CircleDashed, CircleX, History, LoaderCircle } from 'lucide-react';
import { CopyText, OwnedLog } from './workbench/Dialog';
import { CommandActions } from './workbench/CommandActions';
import { commandById } from './workbench/commands';
import { useLocale } from './i18n';
import type { StageState } from './workbench/forms';
import { fileStatusLabel, targetLabel } from './workbench/labels';
import { displayPath } from './pathDisplay';
import type { BuildTarget } from './types';
import { iconSize } from './workbench/icons';
import type { Workbench } from './workbench/useWorkbench';

export function IntegrationDelivery({
  controller,
  active,
}: {
  controller: Workbench;
  active: boolean;
}) {
  const { t, text } = useLocale();
  const {
    workspace,
    native,
    ecuOutputDirectory: output,
    buildDirectory,
    ecuImportDirectory: importDirectory,
    buildTarget: target,
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
  const command = (id: string) => commandById(controller, id);
  const handoff = controller.generationKind === 'handoff';
  const file = preview?.files.find((item) => item.path === selected);
  const labels: Record<StageState, string> = {
    pending: t('shell.stage.pending'),
    running: t('shell.stage.running'),
    done: t('shell.stage.done'),
    failed: t('shell.stage.failed'),
    stale: t('shell.stage.stale'),
  };
  const icons = {
    pending: CircleDashed,
    running: LoaderCircle,
    done: CircleCheck,
    failed: CircleX,
    stale: History,
  };
  const downstream = (state: StageState): StageState => (sourceReady ? state : 'stale');
  const stages: [string, StageState, string][] = [
    [
      t('shell.stage.save'),
      sourceReady ? 'done' : 'pending',
      sourceReady ? t('shell.stage.saved') : t('shell.stage.unsavedDraft'),
    ],
    [
      t('shell.stage.validate'),
      downstream(controller.stages.validate.state),
      sourceReady ? labels[controller.stages.validate.state] : t('shell.stage.revalidate'),
    ],
    ...(
      [
        ['shell.stage.generate', controller.stages.generate.state],
        ['shell.stage.build', controller.stages.build.state],
        ['shell.stage.host', controller.stages.virtual.state],
      ] as const
    ).map(([name, state]): [string, StageState, string] => [
      t(name),
      downstream(state),
      labels[downstream(state)],
    ]),
    [t('shell.stage.sc1'), 'pending', t('shell.stage.projectUnverified')],
    [t('shell.stage.hardware'), 'pending', t('shell.stage.unverified')],
  ];
  // The latest result of this page's own steps; never a hint left over from before.
  const failed = (['virtual', 'build', 'generate'] as const).find(
    (key) => controller.stages[key].state === 'failed',
  );
  const guidance = busy
    ? t('shell.busy', { operation: text(controller.busy) })
    : failed
      ? t('shell.delivery.failedStage', { detail: text(controller.stages[failed].detail) })
      : verified
        ? verified.passed
          ? t('shell.delivery.verified')
          : t('shell.delivery.verifyFailed')
        : built
          ? t('shell.delivery.built', { path: displayPath(built.binaryPath) })
          : generated
            ? t('shell.delivery.generated', {
                count: generated.files.length,
                directory: displayPath(generated.outputDirectory),
              })
            : preview
              ? t('shell.delivery.previewed', { count: preview.files.length })
              : t('shell.delivery.directoryHint');
  return (
    <div className="workflow-page delivery-view" hidden={!active} data-testid="ecu-delivery">
      <div className="section-header">
        <div>
          <p className="eyebrow">{t('shell.delivery.eyebrow')}</p>
          <h2>{t('shell.delivery.title')}</h2>
          <p>{t('shell.delivery.description')}</p>
        </div>
      </div>
      <section className="document-page">
        <h3>{t('shell.delivery.liveSource')}</h3>
        <p>{t('shell.delivery.liveDescription')}</p>
        <CommandActions actions={[{ command: command('initialize-application') }]} />
        {controller.applicationWarnings.map((warning, index) => (
          <p key={index} className="notice info">
            {text(warning)}
          </p>
        ))}
        {controller.applicationRecoveryFiles.length ? (
          <>
            <p className="error-text">{t('shell.delivery.recoveryWarning')}</p>
            <ul>
              {controller.applicationRecoveryFiles.map((path) => (
                <li key={path} className="mono path-text">
                  {displayPath(path)}
                  <CopyText text={displayPath(path)} label={t('action.copyPath')} />
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </section>
      <ol className="stage-list">
        {stages.map(([name, state, label], index) => {
          const Icon = icons[state];
          return (
            <li className={`stage ${state}`} key={name}>
              <span className="stage-number">{index + 1}</span>
              <div className="stage-copy">
                <strong>{name}</strong>
              </div>
              <span className="stage-pill" data-stage={name}>
                <Icon size={iconSize.inline} aria-hidden="true" />
                {label}
              </span>
            </li>
          );
        })}
      </ol>
      <div className="form-fields ecu-delivery-fields">
        <label>
          {t('shell.delivery.target')}
          <select
            value={target}
            disabled={busy || locked}
            onChange={(event) => void controller.changeTarget(event.target.value as BuildTarget)}
          >
            {(['windows-x64-controlled-v1', 'linux-x64-controlled-v1'] as const).map((value) => (
              <option key={value} value={value}>
                {targetLabel(value)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t('shell.delivery.output')}
          <input
            value={displayPath(output)}
            disabled={busy || locked}
            onChange={(e) => changeOutput(e.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={busy || locked || !native}
          onClick={() =>
            void chooseDirectory((path) => {
              changeOutput(path);
            })
          }
        >
          {t('action.browse')}
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
          {t('shell.delivery.metadata')}
        </label>
        <label>
          {t('shell.delivery.buildDirectory')}
          <input
            value={displayPath(buildDirectory)}
            disabled={busy || locked}
            onChange={(e) => changeBuildDirectory(e.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={busy || locked || !native}
          onClick={() => void chooseDirectory(changeBuildDirectory)}
        >
          {t('action.browse')}
        </button>
      </div>
      <CommandActions
        actions={[
          { command: command('preview') },
          { command: command('preflight') },
          { command: command('generate') },
          { command: command('build-ecu') },
          { command: command('verify-ecu') },
        ]}
      />
      <div className="form-fields ecu-delivery-fields">
        <label>
          {t('shell.delivery.importDirectory')}
          <input
            value={displayPath(importDirectory)}
            disabled={busy || locked}
            onChange={(e) => setImportDirectory(e.target.value)}
          />
        </label>
      </div>
      <CommandActions
        actions={[
          {
            command: command('ecu-package'),
            run: () => onImport(importDirectory),
          },
        ]}
      />
      <div className={`page-guidance${failed ? ' error' : ''}`}>
        <span role="status">{guidance}</span>
        {!busy && !failed && (built || generated) ? (
          <CopyText
            text={displayPath(built ? built.binaryPath : generated!.outputDirectory)}
            label={t('action.copyPath')}
          />
        ) : null}
      </div>
      {preview && (
        <div className="generation-preview">
          <label>
            {t('shell.delivery.files')}
            <select value={selected} onChange={(e) => setSelected(e.target.value)}>
              {preview.files.map((item) => (
                <option key={item.path} value={item.path}>
                  {fileStatusLabel(item.status)} · {item.path}
                </option>
              ))}
            </select>
          </label>
          <pre aria-label={t('shell.delivery.filePreview')}>
            {file?.after ?? t('shell.delivery.binaryHidden')}
          </pre>
        </div>
      )}
      {built && <OwnedLog text={built.log} label={t('shell.delivery.buildLog')} />}
      {verified && <OwnedLog text={verified.log} label={t('shell.delivery.behaviorLog')} />}
    </div>
  );
}
