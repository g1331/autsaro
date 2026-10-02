import type { Workbench } from '../workbench/useWorkbench';
import type { ExecutionTools } from '../types';

const toolLabels: { key: keyof ExecutionTools; label: string }[] = [
  { key: 'compiler', label: 'C 编译器' },
  { key: 'objdump', label: 'objdump' },
  { key: 'git', label: 'Git' },
  { key: 'python', label: '锁定 CPython' },
];

export function SettingsPage({ controller }: { controller: Workbench }) {
  if (!controller.settingsOpen) return null;
  const {
    capabilities,
    resourceDraft,
    toolDraft,
    settingsNotice,
    setResourceDraft,
    setToolDraft,
    configureResources,
    configureTools,
  } = controller;
  const locked = Boolean(controller.busy);
  function close() {
    controller.setResourceDraft({
      xsdArchive: capabilities?.xsdArchive ?? '',
      modArchive: capabilities?.modArchive ?? '',
    });
    controller.setToolDraft(
      capabilities?.executionTools ?? { compiler: '', objdump: '', git: '', python: '' },
    );
    controller.setSettingsNotice('');
    controller.setSettingsOpen(false);
  }
  return (
    <div className="save-preview-backdrop">
      <section
        className="save-preview-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <header>
          <div>
            <p className="eyebrow">WORKBENCH SETTINGS</p>
            <h2 id="settings-title">规范档案与执行工具</h2>
            <p>官方资料由用户提供；先核对固定摘要，再原子保存设置。取消不会改变当前工作区。</p>
          </div>
          <button className="quiet-button" type="button" onClick={close} disabled={locked}>
            关闭设置
          </button>
        </header>
        <div className="workflow-page">
          <section className="inspector-block">
            <h3>R24-11 校验依赖</h3>
            {capabilities?.resourceError && (
              <p className="notice error" role="alert">
                {capabilities.resourceError}
              </p>
            )}
            <div className="form-fields">
              <label>
                XSD 档案
                <input
                  aria-label="XSD 档案路径"
                  value={resourceDraft.xsdArchive}
                  onChange={(event) =>
                    setResourceDraft({ ...resourceDraft, xsdArchive: event.target.value })
                  }
                  disabled={locked}
                />
              </label>
              <label>
                MOD 档案
                <input
                  aria-label="MOD 档案路径"
                  value={resourceDraft.modArchive}
                  onChange={(event) =>
                    setResourceDraft({ ...resourceDraft, modArchive: event.target.value })
                  }
                  disabled={locked}
                />
              </label>
            </div>
            <button
              className="primary-button compact"
              type="button"
              onClick={() => void configureResources()}
              disabled={locked || !resourceDraft.xsdArchive || !resourceDraft.modArchive}
            >
              核对并保存规范档案
            </button>
          </section>
          <section className="inspector-block">
            <h3>原生执行工具</h3>
            <p>填写绝对可执行文件路径。工具设置就绪不等于编译预检通过。</p>
            {capabilities?.toolError && (
              <p className="notice error" role="alert">
                {capabilities.toolError}
              </p>
            )}
            <div className="form-fields">
              {toolLabels.map(({ key, label }) => (
                <label key={key}>
                  {label}
                  <input
                    aria-label={`${label}路径`}
                    value={toolDraft[key]}
                    onChange={(event) => setToolDraft({ ...toolDraft, [key]: event.target.value })}
                    disabled={locked}
                  />
                </label>
              ))}
            </div>
            <button
              className="primary-button compact"
              type="button"
              onClick={() => void configureTools()}
              disabled={locked || toolLabels.some(({ key }) => !toolDraft[key])}
            >
              保存执行工具
            </button>
          </section>
          {Boolean(capabilities?.environmentOverrides.length) && (
            <p>
              当前环境覆盖：{capabilities?.environmentOverrides.join('、')}
              。环境变量优先于保存设置。
            </p>
          )}
          {settingsNotice && (
            <p role="status" className="page-guidance">
              {settingsNotice}
            </p>
          )}
        </div>
      </section>
    </div>
  );
}
