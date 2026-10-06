import { ChevronDown } from 'lucide-react';
import type { Workbench } from './useWorkbench';
import type { ConfigurationDiagnostic, ToolWindow } from './projectTypes';
import { CopyText, OwnedLog } from './Dialog';
import { VirtualPage } from '../pages/VirtualPage';
import { issueTarget } from './forms';
import { PanelResizeHandle } from './PanelResizeHandle';

export const toolLabels: Record<ToolWindow, string> = {
  problems: '问题',
  generation: '生成',
  build: '构建',
  host: '主机验证',
  log: '操作日志',
};

export function ToolWindows({ controller: c }: { controller: Workbench }) {
  async function locate(issue: ConfigurationDiagnostic) {
    await c.guardContext('定位问题', async () => {
      const objectId =
        issue.objectId ??
        c.projection?.fields.find((field) => field.fieldId === issue.fieldId)?.objectId;
      c.setTreeVisible(true);
      c.setInspectorVisible(true);
      c.setInspectorTab('properties');
      c.setTreeFilter('');
      c.setObjectFilter('');
      if (objectId && c.projection?.objects.some((object) => object.objectId === objectId)) {
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
      } else if (
        issue.sourceId &&
        c.projection?.sources.some((source) => source.sourceId === issue.sourceId)
      )
        await c.readSource(issue.sourceId);
    });
  }
  return (
    <section className="bottom-tools" aria-label="工具窗口">
      {c.toolWindow ? <PanelResizeHandle kind="tools" /> : null}
      {c.toolWindow ? (
        <div className="tool-content" role="tabpanel" aria-label={toolLabels[c.toolWindow]}>
          <header>
            <strong>{toolLabels[c.toolWindow]}</strong>
            <button
              type="button"
              className="panel-icon-button"
              aria-label="收起工具窗口"
              title="收起工具窗口"
              onClick={() => c.setToolWindow(null)}
            >
              <ChevronDown size={16} aria-hidden="true" />
            </button>
          </header>
          <div hidden={c.toolWindow !== 'problems'}>
            <div className="validation-scopes">
              {c.projection?.validation.map((scope) => (
                <details key={scope.scope}>
                  <summary>
                    {scope.scope} · {scope.status}
                  </summary>
                  {scope.coverage.map((coverage) => (
                    <p key={coverage.ruleId}>
                      {coverage.ruleId} · {coverage.supported ? '支持' : '不支持'} ·{' '}
                      {coverage.reason}
                    </p>
                  ))}
                </details>
              ))}
            </div>
            <ul className="problem-list">
              {c.projection?.diagnostics.map((issue, index) => {
                const resolvable = Boolean(
                  (issue.objectId &&
                    c.projection?.objects.some((object) => object.objectId === issue.objectId)) ||
                  (issue.fieldId &&
                    c.projection?.fields.some((field) => field.fieldId === issue.fieldId)) ||
                  (issue.sourceId &&
                    c.projection?.sources.some((source) => source.sourceId === issue.sourceId)),
                );
                return (
                  <li key={`${issue.code}-${issue.fieldId}-${index}`} className={issue.severity}>
                    <strong>
                      {issue.severity.toUpperCase()} · {issue.scope} · {issue.code}
                    </strong>
                    <p>{issue.message}</p>
                    <span className="mono path-text">{issue.path ?? issue.file}</span>
                    <p>{issue.remedy}</p>
                    {resolvable ? (
                      <button type="button" onClick={() => void locate(issue)}>
                        定位真实来源
                      </button>
                    ) : (
                      <span>无法核定定位目标</span>
                    )}
                    <CopyText text={JSON.stringify(issue, null, 2)} label="复制详情" />
                  </li>
                );
              })}
              {c.issues.map((issue, index) => {
                const target = c.workspace ? issueTarget(issue, c.workspace) : null;
                return (
                  <li key={`legacy-${issue.code}-${index}`} className={issue.severity}>
                    <strong>
                      {issue.severity.toUpperCase()} · {issue.code}
                    </strong>
                    <p>{issue.message}</p>
                    <span className="mono path-text">{issue.path ?? issue.file}</span>
                    {target ? (
                      <button type="button" onClick={() => void c.choose(target)}>
                        定位对象
                      </button>
                    ) : null}
                    <CopyText text={JSON.stringify(issue, null, 2)} label="复制详情" />
                  </li>
                );
              })}
              {c.integrationIssues.map((issue, index) => {
                const object = c.projection?.objects.find((item) => item.path === issue.object);
                const source = c.projection?.sources.find((item) => item.path === issue.file);
                return (
                  <li key={`standard-${issue.code}-${index}`} className="error">
                    <strong>
                      {issue.code} · {issue.category}
                    </strong>
                    <p>{issue.message}</p>
                    <p>{issue.remedy}</p>
                    {object ? (
                      <button type="button" onClick={() => void c.selectObject(object.objectId)}>
                        定位对象
                      </button>
                    ) : source ? (
                      <button type="button" onClick={() => void c.readSource(source.sourceId)}>
                        查看来源
                      </button>
                    ) : null}
                    <CopyText text={JSON.stringify(issue, null, 2)} label="复制详情" />
                  </li>
                );
              })}
            </ul>
            {!c.projection?.diagnostics.length &&
            !c.issues.length &&
            !c.integrationIssues.length ? (
              <p className="empty-state">当前后台没有返回问题；各校验域的实际执行状态见上方。</p>
            ) : null}
          </div>
          <div hidden={c.toolWindow !== 'generation'}>
            <p>
              {c.stages.generate.state} · {c.stages.generate.detail}
            </p>
            {c.generated ? (
              <>
                <p className="mono path-text">{c.generated.outputDirectory}</p>
                <CopyText text={c.generated.outputDirectory} label="复制输出路径" />
                <ul>
                  {c.generated.files.map((file) => (
                    <li key={file} className="mono">
                      {file}
                    </li>
                  ))}
                </ul>
              </>
            ) : (
              <p>尚无本次生成结果；从源码交付文档预览。</p>
            )}
          </div>
          <div hidden={c.toolWindow !== 'build'}>
            <p>
              {c.stages.build.state} · {c.stages.build.detail}
            </p>
            {c.workspace?.integrationCandidate ? (
              <button
                type="button"
                disabled={c.executionDisabled || c.unapplied || Boolean(c.workspace?.dirty)}
                onClick={c.preflightEcu}
              >
                显式编译预检
              </button>
            ) : null}
            <button
              type="button"
              disabled={c.executionDisabled || !c.generated || !c.buildDirectory.trim()}
              onClick={c.workspace?.integrationCandidate ? c.buildEcu : c.buildProject}
            >
              独立构建
            </button>
            <p className="field-help">{c.executionReason}</p>
            {c.preflight ? (
              <OwnedLog
                text={c.preflight.logs.join('\n')}
                label={`编译预检 ${c.preflight.status}`}
              />
            ) : null}
            {c.built ? (
              <>
                <p className="mono path-text">{c.built.binaryPath}</p>
                <CopyText text={c.built.binaryPath} label="复制二进制路径" />
                <OwnedLog text={c.built.log} label="真实构建日志" />
              </>
            ) : null}
          </div>
          <div hidden={c.toolWindow !== 'host'}>
            {c.workspace?.integrationCandidate ? (
              <>
                <button
                  type="button"
                  disabled={c.executionDisabled || !c.built}
                  onClick={c.verifyEcu}
                >
                  验证本次 CAN / DID / N_Cr
                </button>
                <p>{c.executionReason}</p>
                {c.virtualResult ? (
                  <OwnedLog
                    text={c.virtualResult.log}
                    label={c.virtualResult.passed ? '本次主机行为通过' : '本次主机行为失败'}
                  />
                ) : null}
              </>
            ) : (
              <VirtualPage controller={c} />
            )}
          </div>
          <div hidden={c.toolWindow !== 'log'}>
            <CopyText
              text={() =>
                c.operationLog
                  .map((entry) => `${entry.command} · ${entry.outcome}\n${entry.detail}`)
                  .join('\n\n')
              }
              label="复制完整会话操作日志"
            />
            <p className="field-help">摘要显示最近 100 次真实 IPC 返回，不是持久审计记录。</p>
            {c.operationLog.slice(-100).map((entry, index) => (
              <details key={`${entry.command}-${index}`}>
                <summary>
                  {entry.command} · {entry.outcome}
                </summary>
                <OwnedLog text={entry.detail} label="后台返回详情" />
              </details>
            ))}
          </div>
        </div>
      ) : null}
      <nav className="tool-tabs" aria-label="底部工具窗口">
        {(Object.keys(toolLabels) as ToolWindow[]).map((key) => (
          <button
            key={key}
            type="button"
            aria-pressed={c.toolWindow === key}
            onClick={() => c.setToolWindow(c.toolWindow === key ? null : key)}
          >
            {toolLabels[key]}
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
