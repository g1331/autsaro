import { message, useLocale } from '../i18n';
import { displayPath } from '../pathDisplay';
import { editorScopeKeys, editorStatusKeys } from '../i18n/editors';
import type { Message } from '../i18n';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import type { ToolWindow } from './projectTypes';
import { ProblemsTable, RejectionDetails } from './Problems';
import { CopyText, OwnedLog } from './Dialog';
import { PanelResizeHandle } from './PanelResizeHandle';
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
              <RejectionDetails controller={c} />
              <ProblemsTable controller={c} />
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
                    label={t('action.copyPath')}
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
                    ? [{ command: commandById(c, 'preflight') }]
                    : []),
                  { command: commandById(c, 'build-ecu') },
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
                  <CopyText text={displayPath(c.built.binaryPath)} label={t('action.copyPath')} />
                  <OwnedLog text={c.built.log} label={t('shell.delivery.buildLog')} />
                </>
              ) : null}
            </div>
          ) : null}
          {c.toolWindow === 'host' ? (
            <div>
              <p role="status">
                {t(editorStatusKeys[c.stages.virtual.state])} · {text(c.stages.virtual.detail)}
              </p>
              <CommandActions actions={[{ command: commandById(c, 'verify-ecu') }]} />
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
                label={t('action.copyLog')}
              />
              <p className="field-help">{t('editor.tools.sessionHelp')}</p>
              {c.operationLog.slice(-100).map((entry, index) => (
                <details key={`${entry.command}-${index}`}>
                  <summary>
                    {commandLabel(entry.command)} · {t(editorStatusKeys[entry.outcome])}
                  </summary>
                  <OwnedLog text={entry.detail} label={t('editor.dialog.details')} />
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
