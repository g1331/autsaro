import {
  CircleCheck,
  CircleDashed,
  CircleX,
  FolderOpen,
  History,
  LoaderCircle,
} from 'lucide-react';
import { CopyText, OwnedLog } from './workbench/Dialog';
import { CommandActions } from './workbench/CommandActions';
import { commandById } from './workbench/commands';
import { useLocale } from './i18n';
import type { StageState } from './workbench/forms';
import { targetLabel } from './workbench/labels';
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
    generated,
    built,
    virtualResult: verified,
    changeEcuOutput: changeOutput,
    changeEcuBuildDirectory: changeBuildDirectory,
    setEcuImportDirectory: setImportDirectory,
    chooseDirectory,
    importHandoffDirectory: onImport,
  } = controller;
  if (!workspace) return null;
  const busy = Boolean(controller.busy);
  const sourceReady = !workspace.dirty && !controller.projection?.dirty && !controller.unapplied;
  const command = (id: string) => commandById(controller, id);
  const handoff = controller.generationKind === 'handoff';
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
  const choose = (label: string, onChoose: (path: string) => void) => (
    <button
      type="button"
      aria-label={label}
      disabled={busy || !native}
      onClick={() => void chooseDirectory(onChoose)}
    >
      <FolderOpen size={iconSize.control} aria-hidden="true" />
      {t('editor.common.select')}
    </button>
  );
  return (
    <div className="workflow-page delivery-view" hidden={!active} data-testid="ecu-delivery">
      <div className="section-header">
        <div>
          <p className="eyebrow">{t('shell.delivery.eyebrow')}</p>
          <h2>{t('shell.delivery.title')}</h2>
          <p>{t('shell.delivery.description')}</p>
        </div>
      </div>
      <ol className="stage-list">
        {stages.map(([name, state, label], index) => {
          const Icon = icons[state];
          return (
            <li className={`stage ${state}`} key={name} title={`${name} · ${label}`}>
              <span className="stage-number">{index + 1}</span>
              <div className="stage-copy">
                <strong>{name}</strong>
              </div>
              <span className="stage-pill" data-stage={name}>
                <Icon size={iconSize.inline} aria-hidden="true" />
                <span>{label}</span>
              </span>
            </li>
          );
        })}
      </ol>
      <section className="delivery-step">
        <h3>{t('shell.delivery.liveSource')}</h3>
        <p className="field-help">{t('shell.delivery.liveDescription')}</p>
        <CommandActions
          actions={[
            { command: command('initialize-application'), label: t('shell.command.initialize') },
          ]}
        />
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
                  <CopyText text={displayPath(path)} label={t('shell.delivery.copyRecovery')} />
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </section>
      <section className="delivery-step">
        <h3>{t('shell.stage.generate')}</h3>
        <div className="form-fields">
          <label>
            {t('shell.delivery.target')}
            <select
              aria-label={t('shell.delivery.target')}
              value={target}
              disabled={busy}
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
            <div className="path-picker">
              <input
                aria-label={t('shell.delivery.output')}
                value={displayPath(output)}
                disabled={busy}
                onChange={(e) => changeOutput(e.target.value)}
              />
              {choose(t('shell.delivery.chooseOutput'), changeOutput)}
            </div>
          </label>
          <label className="checkbox-field">
            <input
              type="checkbox"
              checked={handoff}
              disabled={busy}
              onChange={(e) => {
                controller.setGenerationKind(e.target.checked ? 'handoff' : 'project');
                changeOutput(output);
              }}
            />
            {t('shell.delivery.metadata')}
          </label>
        </div>
        {/* Generation itself is confirmed in the preview dialog, once. */}
        <CommandActions
          actions={[
            {
              command: command('preview'),
              label: t('shell.delivery.preview'),
              className: 'primary-button',
            },
            { command: command('preflight'), label: t('shell.command.preflight') },
          ]}
        />
      </section>
      <section className="delivery-step">
        <h3>{t('shell.stage.build')}</h3>
        <div className="form-fields">
          <label>
            {t('shell.delivery.buildDirectory')}
            <div className="path-picker">
              <input
                aria-label={t('shell.delivery.buildInput')}
                value={displayPath(buildDirectory)}
                disabled={busy}
                onChange={(e) => changeBuildDirectory(e.target.value)}
              />
              {choose(t('shell.delivery.chooseBuildDirectory'), changeBuildDirectory)}
            </div>
          </label>
        </div>
        <CommandActions
          actions={[
            {
              command: command('build-ecu'),
              label: t('shell.delivery.build'),
              className: 'primary-button',
            },
          ]}
        />
      </section>
      <section className="delivery-step">
        <h3>{t('shell.stage.host')}</h3>
        <CommandActions
          actions={[
            {
              command: command('verify-ecu'),
              label: t('shell.delivery.verify'),
              className: 'primary-button',
            },
          ]}
        />
      </section>
      <section className="delivery-step">
        <h3>{t('shell.delivery.reimport')}</h3>
        <div className="form-fields">
          <label>
            {t('shell.delivery.importDirectory')}
            <div className="path-picker">
              <input
                aria-label={t('shell.delivery.importDirectory')}
                value={displayPath(importDirectory)}
                disabled={busy}
                onChange={(e) => setImportDirectory(e.target.value)}
              />
              {choose(t('shell.delivery.chooseImportDirectory'), setImportDirectory)}
            </div>
          </label>
        </div>
        <CommandActions
          actions={[
            {
              command: command('ecu-package'),
              label: t('shell.delivery.reimport'),
              run: () => onImport(importDirectory),
            },
          ]}
        />
      </section>
      <div className={`page-guidance${failed ? ' error' : ''}`}>
        <span role="status">{guidance}</span>
        {!busy && !failed && (built || generated) ? (
          <CopyText
            text={displayPath(built ? built.binaryPath : generated!.outputDirectory)}
            label={built ? t('editor.tools.copyBinary') : t('editor.tools.copyOutput')}
          />
        ) : null}
      </div>
      {built && <OwnedLog text={built.log} label={t('shell.delivery.buildLog')} />}
      {verified && <OwnedLog text={verified.log} label={t('shell.delivery.behaviorLog')} />}
    </div>
  );
}
