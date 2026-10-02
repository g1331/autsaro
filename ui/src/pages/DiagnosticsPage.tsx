import type { Workbench } from '../workbench/useWorkbench';
import { ListChecks } from 'lucide-react';
import { issueTarget } from '../workbench/forms';

export function DiagnosticsPage({ controller }: { controller: Workbench }) {
  const {
    validateProject,
    disabled,
    unapplied,
    setPage,
    errorCount,
    issues,
    workspace,
    choose,
    stages,
  } = controller;
  if (!workspace) return null;
  return (
    <div className="workflow-page diagnostic-view">
      <div className="section-header">
        <div>
          <p className="eyebrow">VALIDATION REPORT</p>
          <h2>诊断</h2>
          <p>按来源文件与配置对象定位问题。校验结果只代表当前项目状态。</p>
        </div>
        <button
          type="button"
          className="primary-button compact"
          onClick={validateProject}
          disabled={disabled || unapplied}
        >
          <ListChecks aria-hidden="true" size={15} />
          运行校验
        </button>
      </div>
      {unapplied && (
        <div className="page-guidance">
          配置页有未应用的更改。
          <button type="button" onClick={() => setPage('editor')}>
            返回配置
          </button>
        </div>
      )}
      <div className={`diagnostic-summary${errorCount ? ' has-errors' : ''}`}>
        <strong>{errorCount}</strong> 个错误 <span>·</span> {issues.length - errorCount} 个警告 /
        提示
      </div>
      {issues.length ? (
        <ul className="issue-list">
          {issues.map((issue, index) => {
            const target = issueTarget(issue, workspace);
            return (
              <li
                key={`${issue.code}-${issue.path ?? issue.file ?? ''}-${index}`}
                className={issue.severity}
              >
                <span className="issue-icon" aria-hidden="true">
                  {issue.severity === 'error' ? '!' : issue.severity === 'warning' ? '△' : 'i'}
                </span>
                <div>
                  <div className="issue-meta">
                    {issue.severity.toUpperCase()} <span>/</span> {issue.code}
                  </div>
                  <p>{issue.message}</p>
                  {(issue.path || issue.file) && (
                    <button
                      type="button"
                      className="issue-location"
                      onClick={() => {
                        if (target) choose(target);
                      }}
                      disabled={!target}
                    >
                      {issue.file && <span>{issue.file}</span>}
                      {issue.path && <span>{issue.path}</span>}
                    </button>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      ) : (
        <p className="empty-message">
          {stages.validate.state === 'done' ? '本次校验完成，未发现诊断。' : '尚未运行校验。'}
        </p>
      )}
    </div>
  );
}
