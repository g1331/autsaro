import { message, useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import {
  editorCategoryKeys,
  editorScopeKeys,
  editorSeverityKeys,
  editorStatusKeys,
} from '../i18n/editors';
import type { Message } from '../i18n';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useMemo } from 'react';
import type { Workbench } from './useWorkbench';
import type { ConfigurationDiagnostic, ToolWindow } from './projectTypes';
import { CopyText, OwnedLog } from './Dialog';
import { PanelResizeHandle } from './PanelResizeHandle';
import { distinct } from './forms';
import { commandLabel } from './labels';
import { iconSize } from './icons';
import { CommandActions } from './CommandActions';
import { commandById } from './commands';
import { Tooltip } from './Tooltip';

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
  const integrationProblems = distinct(c.integrationIssues);
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
        <div className="tool-content" role="region" aria-label={text(toolLabels[c.toolWindow])}>
          <header>
            <strong>{text(toolLabels[c.toolWindow])}</strong>
            <Tooltip label={t('editor.tools.collapse')}>
              <button
                type="button"
                className="icon-button"
                aria-label={t('editor.tools.collapse')}
                onClick={() => c.setToolWindow(null)}
              >
                <ChevronDown size={iconSize.control} aria-hidden="true" />
              </button>
            </Tooltip>
          </header>
          {c.toolWindow === 'problems' ? (
            <div>
              <div className="validation-scopes">
                <p className="field-help">{t('editor.tools.automaticChecks')}</p>
                {c.projection?.validation.map((scope) => (
                  <details key={scope.scope}>
                    <summary>
                      <ChevronRight size={iconSize.inline} aria-hidden="true" />
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
                {c.diagnostics.map((issue, index) => {
                  const resolvable = Boolean(
                    (issue.objectId && targets.objects.has(issue.objectId)) ||
                    (issue.fieldId && targets.fields.has(issue.fieldId)) ||
                    (issue.sourceId && targets.sources.has(issue.sourceId)),
                  );
                  return (
                    <li key={`${issue.code}-${issue.fieldId}-${index}`} className={issue.severity}>
                      <div className="problem-main">
                        <p>
                          <strong>
                            {t(editorSeverityKeys[issue.severity])} ·{' '}
                            {t(editorScopeKeys[issue.scope])} · {issue.code}
                          </strong>{' '}
                          {text(issue.message)}
                        </p>
                        <ProblemMeta
                          path={displayPath(issue.path ?? issue.file ?? '')}
                          remedy={text(issue.remedy)}
                        />
                      </div>
                      <div className="problem-actions">
                        {resolvable ? (
                          <button type="button" onClick={() => void locate(issue)}>
                            {t('editor.tools.locateSource')}
                          </button>
                        ) : (
                          <small>{t('editor.tools.unqualifiedTarget')}</small>
                        )}
                        <CopyText
                          text={JSON.stringify(issue, null, 2)}
                          label={t('editor.common.copyDetails')}
                        />
                      </div>
                    </li>
                  );
                })}
                {c.issues.map((issue, index) => (
                  <li key={`workspace-${issue.code}-${index}`} className={issue.severity}>
                    <div className="problem-main">
                      <p>
                        <strong>
                          {t(editorSeverityKeys[issue.severity])} · {issue.code}
                        </strong>{' '}
                        {text(issue.message)}
                      </p>
                      <ProblemMeta path={displayPath(issue.path ?? issue.file ?? '')} />
                    </div>
                    <div className="problem-actions">
                      <CopyText
                        text={JSON.stringify(issue, null, 2)}
                        label={t('editor.common.copyDetails')}
                      />
                    </div>
                  </li>
                ))}
                {integrationProblems.map((issue, index) => {
                  const object = issue.object ? targets.objectsByPath.get(issue.object) : undefined;
                  const source = issue.file ? targets.sourcesByPath.get(issue.file) : undefined;
                  return (
                    <li key={`standard-${issue.code}-${index}`} className="error">
                      <div className="problem-main">
                        <p>
                          <strong>
                            {issue.code} · {t(editorCategoryKeys[issue.category])}
                          </strong>{' '}
                          {text(issue.message)}
                        </p>
                        <ProblemMeta remedy={text(issue.remedy)} />
                      </div>
                      <div className="problem-actions">
                        {object ? (
                          <button
                            type="button"
                            onClick={() => void c.selectObject(object.objectId)}
                          >
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
                      </div>
                    </li>
                  );
                })}
              </ul>
              {!c.problemCount ? (
                <p className="empty-state">{t('editor.tools.noProblems')}</p>
              ) : null}
            </div>
          ) : null}
          {c.toolWindow === 'generation' ? (
            <div>
              <p role="status">
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
              <p role="status">
                {t(editorStatusKeys[c.stages.build.state])} ·{' '}
                {displayPath(text(c.stages.build.detail))}
              </p>
              <CommandActions
                actions={[
                  ...(c.workspace?.integrationCandidate
                    ? [{ command: commandById(c, 'preflight'), label: t('editor.tools.preflight') }]
                    : []),
                  { command: commandById(c, 'build-ecu'), label: t('editor.tools.build') },
                ]}
              />
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
              <p role="status">
                {t(editorStatusKeys[c.stages.virtual.state])} · {text(c.stages.virtual.detail)}
              </p>
              <CommandActions
                actions={[
                  { command: commandById(c, 'verify-ecu'), label: t('editor.tools.verify') },
                ]}
              />
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
            </div>
          ) : null}
          {c.toolWindow === 'log' ? (
            <div>
              <CopyText
                text={() =>
                  c.operationLog
                    .map(
                      (entry) =>
                        `${commandLabel(entry.command)} (${entry.command}) · ${t(editorStatusKeys[entry.outcome])}\n${text(entry.detail)}`,
                    )
                    .join('\n\n')
                }
                label={t('editor.tools.copySession')}
              />
              <p className="field-help">{t('editor.tools.sessionHelp')}</p>
              {c.operationLog.slice(-100).map((entry, index) => (
                <details key={`${entry.command}-${index}`}>
                  <summary>
                    {commandLabel(entry.command)} · {t(editorStatusKeys[entry.outcome])}
                  </summary>
                  <p className="mono path-text">{entry.command}</p>
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
            className="ghost-button"
            aria-pressed={c.toolWindow === key}
            onClick={() => c.setToolWindow(c.toolWindow === key ? null : key)}
          >
            {text(toolLabels[key])}
            {key === 'problems' ? <small>{c.problemCount}</small> : null}
          </button>
        ))}
      </nav>
    </section>
  );
}

/** The secondary line of a problem row: where it is and what to do. */
function ProblemMeta({ path = '', remedy = '' }: { path?: string; remedy?: string }) {
  return path || remedy ? (
    <small className="problem-meta">
      {path ? <span className="mono path-text">{path}</span> : null}
      {path && remedy ? ' · ' : null}
      {remedy}
    </small>
  ) : null;
}
