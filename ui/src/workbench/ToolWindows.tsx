import { message, useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import {
  editorCategoryKeys,
  editorScopeKeys,
  editorSeverityKeys,
  editorStatusKeys,
} from '../i18n/editors';
import type { Message } from '../i18n';
import { ChevronDown } from 'lucide-react';
import { useMemo } from 'react';
import type { Workbench } from './useWorkbench';
import type { ConfigurationDiagnostic, ToolWindow } from './projectTypes';
import { CopyText, OwnedLog } from './Dialog';
import { VirtualPage } from '../pages/VirtualPage';
import { issueTarget } from './forms';
import { PanelResizeHandle } from './PanelResizeHandle';

export const toolLabels: Record<ToolWindow, Message> = {
  problems: message('editor.tools.tab.problems'),
  generation: message('editor.tools.tab.generation'),
  build: message('editor.tools.tab.build'),
  host: message('editor.tools.tab.host'),
  log: message('editor.tools.tab.log'),
};

export function ToolWindows({ controller: c }: { controller: Workbench }) {
  const { t, text } = useLocale();
  const targets = useMemo(
    () => ({
      objects: new Set(c.projection?.objects.map((object) => object.objectId)),
      fields: new Map(c.projection?.fields.map((field) => [field.fieldId, field.objectId])),
      sources: new Set(c.projection?.sources.map((source) => source.sourceId)),
      objectsByPath: new Map(c.projection?.objects.map((object) => [object.path, object])),
      sourcesByPath: new Map(c.projection?.sources.map((source) => [source.path, source])),
    }),
    [c.projection],
  );
  async function locate(issue: ConfigurationDiagnostic) {
    await c.guardContext(message('editor.tools.locateIssue'), async () => {
      const objectId =
        issue.objectId ?? (issue.fieldId ? targets.fields.get(issue.fieldId) : undefined);
      c.setTreeVisible(true);
      c.setInspectorVisible(true);
      c.setInspectorTab('properties');
      c.setTreeFilter('');
      c.setObjectFilter('');
      if (objectId && targets.objects.has(objectId)) {
        await c.selectObject(objectId);
        requestAnimationFrame(() => {
          const row = Array.from(document.querySelectorAll<HTMLElement>('[data-object-row]')).find(
            (element) => element.dataset.objectRow === objectId,
          );
          row?.scrollIntoView({ block: 'nearest' });
          const field = issue.fieldId
            ? Array.from(document.querySelectorAll<HTMLElement>('[data-field-id]')).find(
                (element) => element.dataset.fieldId === issue.fieldId,
              )
            : null;
          const title = Array.from(
            document.querySelectorAll<HTMLElement>('[data-object-title]'),
          ).find((element) => element.dataset.objectTitle === objectId);
          (field ?? title)?.focus();
        });
      } else if (issue.sourceId && targets.sources.has(issue.sourceId))
        await c.readSource(issue.sourceId);
    });
  }
  return (
    <section className="bottom-tools" aria-label={t('editor.tools.heading')}>
      {c.toolWindow ? <PanelResizeHandle kind="tools" /> : null}
      {c.toolWindow ? (
        <div className="tool-content" role="tabpanel" aria-label={text(toolLabels[c.toolWindow])}>
          <header>
            <strong>{text(toolLabels[c.toolWindow])}</strong>
            <button
              type="button"
              className="panel-icon-button"
              aria-label={t('editor.tools.collapse')}
              title={t('editor.tools.collapse')}
              onClick={() => c.setToolWindow(null)}
            >
              <ChevronDown size={16} aria-hidden="true" />
            </button>
          </header>
          {c.toolWindow === 'problems' ? (
            <div>
              <div className="validation-scopes">
                {c.projection?.validation.map((scope) => (
                  <details key={scope.scope}>
                    <summary>
                      {t(editorScopeKeys[scope.scope])} · {t(editorStatusKeys[scope.status])}
                    </summary>
                    {scope.coverage.map((coverage) => (
                      <p key={coverage.ruleId}>
                        {coverage.ruleId} ·{' '}
                        {coverage.supported
                          ? t('editor.tools.supported')
                          : t('editor.tools.unsupported')}{' '}
                        · {text(coverage.reason ?? '')}
                      </p>
                    ))}
                  </details>
                ))}
              </div>
              <ul className="problem-list">
                {c.projection?.diagnostics.map((issue, index) => {
                  const resolvable = Boolean(
                    (issue.objectId && targets.objects.has(issue.objectId)) ||
                    (issue.fieldId && targets.fields.has(issue.fieldId)) ||
                    (issue.sourceId && targets.sources.has(issue.sourceId)),
                  );
                  return (
                    <li key={`${issue.code}-${issue.fieldId}-${index}`} className={issue.severity}>
                      <strong>
                        {t(editorSeverityKeys[issue.severity])} · {t(editorScopeKeys[issue.scope])}{' '}
                        · {issue.code}
                      </strong>
                      <p>{text(issue.message)}</p>
                      <span className="mono path-text">
                        {displayPath(issue.path ?? issue.file ?? '')}
                      </span>
                      <p>{text(issue.remedy)}</p>
                      {resolvable ? (
                        <button type="button" onClick={() => void locate(issue)}>
                          {t('editor.tools.locateSource')}
                        </button>
                      ) : (
                        <span>{t('editor.tools.unqualifiedTarget')}</span>
                      )}
                      <CopyText
                        text={JSON.stringify(issue, null, 2)}
                        label={t('editor.common.copyDetails')}
                      />
                    </li>
                  );
                })}
                {c.issues.map((issue, index) => {
                  const target = c.workspace ? issueTarget(issue, c.workspace) : null;
                  return (
                    <li key={`legacy-${issue.code}-${index}`} className={issue.severity}>
                      <strong>
                        {t(editorSeverityKeys[issue.severity])} · {issue.code}
                      </strong>
                      <p>{text(issue.message)}</p>
                      <span className="mono path-text">
                        {displayPath(issue.path ?? issue.file ?? '')}
                      </span>
                      {target ? (
                        <button type="button" onClick={() => void c.choose(target)}>
                          {t('editor.common.locateObject')}
                        </button>
                      ) : null}
                      <CopyText
                        text={JSON.stringify(issue, null, 2)}
                        label={t('editor.common.copyDetails')}
                      />
                    </li>
                  );
                })}
                {c.integrationIssues.map((issue, index) => {
                  const object = issue.object ? targets.objectsByPath.get(issue.object) : undefined;
                  const source = issue.file ? targets.sourcesByPath.get(issue.file) : undefined;
                  return (
                    <li key={`standard-${issue.code}-${index}`} className="error">
                      <strong>
                        {issue.code} · {t(editorCategoryKeys[issue.category])}
                      </strong>
                      <p>{text(issue.message)}</p>
                      <p>{text(issue.remedy)}</p>
                      {object ? (
                        <button type="button" onClick={() => void c.selectObject(object.objectId)}>
                          {t('editor.common.locateObject')}
                        </button>
                      ) : source ? (
                        <button type="button" onClick={() => void c.readSource(source.sourceId)}>
                          {t('editor.common.viewSource')}
                        </button>
                      ) : null}
                      <CopyText
                        text={JSON.stringify(issue, null, 2)}
                        label={t('editor.common.copyDetails')}
                      />
                    </li>
                  );
                })}
              </ul>
              {!c.projection?.diagnostics.length &&
              !c.issues.length &&
              !c.integrationIssues.length ? (
                <p className="empty-state">{t('editor.tools.noProblems')}</p>
              ) : null}
            </div>
          ) : null}
          {c.toolWindow === 'generation' ? (
            <div>
              <p>
                {t(editorStatusKeys[c.stages.generate.state])} ·{' '}
                {displayPath(text(c.stages.generate.detail))}
              </p>
              {c.generated ? (
                <>
                  <p className="mono path-text">{displayPath(c.generated.outputDirectory)}</p>
                  <CopyText
                    text={displayPath(c.generated.outputDirectory)}
                    label={t('editor.tools.copyOutput')}
                  />
                  <ul>
                    {c.generated.files.map((file) => (
                      <li key={file} className="mono">
                        {file}
                      </li>
                    ))}
                  </ul>
                </>
              ) : (
                <p>{t('editor.tools.noGeneration')}</p>
              )}
            </div>
          ) : null}
          {c.toolWindow === 'build' ? (
            <div>
              <p>
                {t(editorStatusKeys[c.stages.build.state])} ·{' '}
                {displayPath(text(c.stages.build.detail))}
              </p>
              {c.workspace?.integrationCandidate ? (
                <button
                  type="button"
                  disabled={c.executionDisabled || c.unapplied || Boolean(c.workspace?.dirty)}
                  onClick={c.preflightEcu}
                >
                  {t('editor.tools.preflight')}
                </button>
              ) : null}
              <button
                type="button"
                disabled={c.executionDisabled || !c.generated || !c.buildDirectory.trim()}
                onClick={c.workspace?.integrationCandidate ? c.buildEcu : c.buildProject}
              >
                {t('editor.tools.build')}
              </button>
              <p className="field-help">{text(c.executionReason)}</p>
              {c.preflight ? (
                <OwnedLog
                  text={c.preflight.logs.map(text).join('\n')}
                  label={t('editor.tools.preflightLabel', {
                    status: t(editorStatusKeys[c.preflight.status]),
                  })}
                />
              ) : null}
              {c.built ? (
                <>
                  <p className="mono path-text">{displayPath(c.built.binaryPath)}</p>
                  <CopyText
                    text={displayPath(c.built.binaryPath)}
                    label={t('editor.tools.copyBinary')}
                  />
                  <OwnedLog text={c.built.log} label={t('editor.tools.buildLog')} />
                </>
              ) : null}
            </div>
          ) : null}
          {c.toolWindow === 'host' ? (
            <div>
              {c.workspace?.integrationCandidate ? (
                <>
                  <button
                    type="button"
                    disabled={c.executionDisabled || !c.built}
                    onClick={c.verifyEcu}
                  >
                    {t('editor.tools.verify')}
                  </button>
                  <p>{text(c.executionReason)}</p>
                  {c.virtualResult ? (
                    <OwnedLog
                      text={c.virtualResult.log}
                      label={
                        c.virtualResult.passed
                          ? t('editor.tools.hostPassed')
                          : t('editor.tools.hostFailed')
                      }
                    />
                  ) : null}
                </>
              ) : (
                <VirtualPage controller={c} />
              )}
            </div>
          ) : null}
          {c.toolWindow === 'log' ? (
            <div>
              <CopyText
                text={() =>
                  c.operationLog
                    .map(
                      (entry) =>
                        `${entry.command} · ${t(editorStatusKeys[entry.outcome])}\n${text(entry.detail)}`,
                    )
                    .join('\n\n')
                }
                label={t('editor.tools.copySession')}
              />
              <p className="field-help">{t('editor.tools.sessionHelp')}</p>
              {c.operationLog.slice(-100).map((entry, index) => (
                <details key={`${entry.command}-${index}`}>
                  <summary>
                    {entry.command} · {t(editorStatusKeys[entry.outcome])}
                  </summary>
                  <OwnedLog text={entry.detail} label={t('editor.tools.replyDetails')} />
                </details>
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
      <nav className="tool-tabs" aria-label={t('editor.tools.bottom')}>
        {(Object.keys(toolLabels) as ToolWindow[]).map((key) => (
          <button
            key={key}
            type="button"
            aria-pressed={c.toolWindow === key}
            onClick={() => c.setToolWindow(c.toolWindow === key ? null : key)}
          >
            {text(toolLabels[key])}
            {key === 'problems' ? (
              <small>
                {(c.projection?.diagnostics.length ?? 0) +
                  c.issues.length +
                  c.integrationIssues.length}
              </small>
            ) : null}
          </button>
        ))}
      </nav>
    </section>
  );
}
