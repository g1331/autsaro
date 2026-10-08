import { CopyText, OwnedLog } from './workbench/Dialog';
import { useLocale } from './i18n';
import type { BuildTarget } from './types';
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
    ? text(controller.capabilities.toolError ?? '')
    : t('shell.delivery.noExecution');
  const executionDisabled =
    disabled ||
    !controller.capabilities?.nativeExecution ||
    Boolean(controller.capabilities.toolError);
  const notice = controller.notice?.text ?? t('shell.delivery.directoryHint');
  const handoff = controller.generationKind === 'handoff';
  const file = preview?.files.find((item) => item.path === selected);
  const labels = {
    pending: t('shell.stage.pending'),
    running: t('shell.stage.running'),
    done: t('shell.stage.done'),
    failed: t('shell.stage.failed'),
    stale: t('shell.stage.stale'),
  };
  const stages = [
    [t('shell.stage.save'), sourceReady ? t('shell.stage.saved') : t('shell.stage.unsavedDraft')],
    [
      t('shell.stage.validate'),
      sourceReady ? labels[controller.stages.validate.state] : t('shell.stage.revalidate'),
    ],
    [
      t('shell.stage.generate'),
      sourceReady ? labels[controller.stages.generate.state] : t('shell.stage.stale'),
    ],
    [
      t('shell.stage.build'),
      sourceReady ? labels[controller.stages.build.state] : t('shell.stage.stale'),
    ],
    [
      t('shell.stage.host'),
      sourceReady ? labels[controller.stages.virtual.state] : t('shell.stage.stale'),
    ],
    [t('shell.stage.sc1'), t('shell.stage.projectUnverified')],
    [t('shell.stage.hardware'), t('shell.stage.unverified')],
  ];
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
        <button
          type="button"
          disabled={disabled || Boolean(controller.actionReason('edit'))}
          onClick={() => void controller.previewApplicationInitialization()}
        >
          {t('shell.command.initialize')}
        </button>
        {controller.actionReason('edit') ? (
          <p className="field-help">{text(controller.actionReason('edit'))}</p>
        ) : null}
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
                  {path}
                  <CopyText text={path} label={t('shell.delivery.copyRecovery')} />
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
          {t('shell.delivery.target')}
          <select
            aria-label={t('shell.delivery.target')}
            value={target}
            disabled={busy || locked}
            onChange={(event) => void controller.changeTarget(event.target.value as BuildTarget)}
          >
            <option value="windows-x64-controlled-v1">Windows x64 controlled v1</option>
            <option value="linux-x64-controlled-v1">Linux x64 controlled v1</option>
          </select>
        </label>
        <label>
          {t('shell.delivery.output')}
          <input
            aria-label={t('shell.delivery.output')}
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
          {t('shell.delivery.chooseOutput')}
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
            aria-label={t('shell.delivery.buildInput')}
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
          {t('shell.delivery.chooseBuildDirectory')}
        </button>
      </div>
      <div className="section-actions">
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !output.trim()}
          onClick={controller.previewEcu}
        >
          {t('shell.delivery.preview')}
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled}
          title={executionReason}
          onClick={controller.preflightEcu}
        >
          {t('shell.command.preflight')}
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={disabled || !preview}
          onClick={controller.generateEcu}
        >
          {t('shell.delivery.generate')}
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled || !generated || !buildDirectory.trim()}
          onClick={controller.buildEcu}
        >
          {t('shell.delivery.build')}
        </button>
        <button
          className="outline-button small"
          type="button"
          disabled={executionDisabled || !built}
          onClick={controller.verifyEcu}
        >
          {t('shell.delivery.verify')}
        </button>
      </div>
      <div className="form-fields ecu-delivery-fields">
        <label>
          {t('shell.delivery.importDirectory')}
          <input
            aria-label={t('shell.delivery.importDirectory')}
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
          {t('shell.delivery.reimport')}
        </button>
      </div>
      <p role="status" className="page-guidance">
        {busy ? t('shell.delivery.busy') : text(notice)}
      </p>
      {preview && (
        <div className="generation-preview">
          <label>
            {t('shell.delivery.files')}
            <select
              aria-label={t('shell.delivery.previewFile')}
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
