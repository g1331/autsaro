(function () {
  'use strict';

  const categories = [
    { key: 'appearance', label: '外观', icon: 'sun' },
    { key: 'resources', label: '官方资源', icon: 'folder' },
    { key: 'tools', label: '执行工具', icon: 'terminal' },
  ];
  const resourceFields = [
    {
      key: 'xsd',
      label: 'XSD 档案路径',
      hint: 'R24-11 XML Schema 档案，用于 ARXML 结构校验。',
    },
    {
      key: 'mod',
      label: 'MOD 档案路径',
      hint: 'R24-11 模块定义档案，为规范依赖与生成提供依据。',
    },
  ];
  const toolFields = [
    {
      key: 'compiler',
      label: 'C 编译器路径',
      hint: '编译生成的 C99 主机工程；应与执行目标匹配。',
    },
    {
      key: 'objdump',
      label: 'objdump 路径',
      hint: '读取编译产物的目标格式与符号信息。',
    },
    {
      key: 'git',
      label: 'Git 路径',
      hint: '准备工程构建所需的固定版本源码依赖。',
    },
    {
      key: 'python',
      label: '锁定 CPython 路径',
      hint: '用于受管执行与取消；填写目标要求的 CPython 可执行文件。',
    },
  ];

  function pathField(field, values, helpers) {
    const id = 'setting-' + field.key;
    return `
      <div class="field field-wide">
        <label for="${id}">${field.label}</label>
        <div class="dependency-row">
          <input id="${id}" class="mono" type="text"
            data-setting="${field.key}" data-focus="${field.key}"
            value="${helpers.escape(values[field.key])}"
            aria-describedby="${id}-hint" autocomplete="off" spellcheck="false">
          <button class="secondary" type="button" data-action="browse-setting"
            data-value="${field.key}" aria-label="浏览${field.label}">
            ${helpers.icon('folder')}浏览
          </button>
        </div>
        <p id="${id}-hint" class="hint">${field.hint}</p>
      </div>`;
  }

  function render(state, helpers) {
    const draft = state.settingsDraft;
    const category = state.settingsTab;
    let content;

    if (category === 'appearance') {
      content = `
        <h3 class="section-title">工作台主题 <span class="role-tag">拟新增偏好</span></h3>
        <p class="muted">选择适合当前光照的界面。主题不改变工程配置或已有结果。</p>
        <div class="form-grid">
          <div class="field field-wide">
            <label for="setting-theme">界面主题</label>
            <select id="setting-theme" data-setting="theme" data-focus="theme"
              aria-describedby="setting-theme-hint">
              <option value="light"${draft.theme === 'light' ? ' selected' : ''}>浅色</option>
              <option value="dark"${draft.theme === 'dark' ? ' selected' : ''}>深色</option>
              <option value="system"${draft.theme === 'system' ? ' selected' : ''}>跟随系统</option>
            </select>
            <p id="setting-theme-hint" class="hint">点击“应用”后切换；取消保留当前主题。</p>
          </div>
        </div>
        <div class="dependency-row">${helpers.icon('sun')}<div><strong>浅色</strong><p class="hint">适合明亮办公环境，便于比较表格与参数。</p></div></div>
        <div class="dependency-row">${helpers.icon('moon')}<div><strong>深色</strong><p class="hint">适合低照度下长时操作，保留相同的信息层级。</p></div></div>
        <div class="dependency-row">${helpers.icon('settings')}<div><strong>跟随系统</strong><p class="hint">随操作系统的浅色或深色偏好切换。</p></div></div>`;
    } else if (category === 'resources') {
      content = `
        <h3 class="section-title">R24-11 规范档案</h3>
        <p class="muted">使用合法取得的官方档案。资源不随工作台分发，两项均需填写。</p>
        <p class="status-inline">${helpers.icon('info')}${state.resources.ready ? '当前已配置资源路径（演示）' : '尚未配置完整资源路径'}</p>
        ${state.resources.error ? `<p class="inline-notice" role="alert">${helpers.escape(state.resources.error)}</p>` : ''}
        <div class="form-grid">${resourceFields
          .map(function (field) {
            return pathField(field, draft.resources, helpers);
          })
          .join('')}</div>
        <p class="hint">应用资源后须重新校验，旧的生成、构建与主机验证结果将失效。</p>
        <p class="hint">本原型的浏览按钮填入示例路径，不读取档案或核对摘要。</p>`;
    } else {
      content = `
        <h3 class="section-title">原生主机执行工具</h3>
        <p class="muted">填写四项可执行文件的绝对路径，用于预检、构建与主机验证。</p>
        <p class="status-inline">${helpers.icon('info')}${state.tools.ready ? '当前已配置工具路径（演示）' : '尚未配置完整工具路径'}；不代表预检通过。</p>
        <div class="form-grid">${toolFields
          .map(function (field) {
            return pathField(field, draft.tools, helpers);
          })
          .join('')}</div>
        <p class="hint">应用工具后须重新生成并执行预检；旧构建与主机验证结果将失效。配置和源码预览不要求执行工具。</p>
        <p class="hint">本原型不探测文件或工具版本。正式软件中，环境变量优先于保存设置。</p>`;
    }

    return `
      <header class="dialog-header">
        <div><h2 id="dialog-title">工作台设置</h2><p class="hint">应用当前分类；取消丢弃尚未应用的修改。</p></div>
        <button class="icon-button" type="button" data-action="close-dialog" aria-label="关闭设置">${helpers.icon('close')}</button>
      </header>
      <div class="settings-layout">
        <nav class="settings-nav" aria-label="设置分类">
          ${categories
            .map(function (item) {
              return `<button class="text-button${category === item.key ? ' active' : ''}" type="button"
              data-action="settings-tab" data-value="${item.key}"
              aria-pressed="${category === item.key}" aria-controls="settings-content">
              ${helpers.icon(item.icon)}${item.label}
            </button>`;
            })
            .join('')}
        </nav>
        <section id="settings-content" class="settings-main" aria-label="${
          categories.find(function (item) {
            return item.key === category;
          }).label
        }">
          ${content}
          ${state.notice ? `<p class="inline-notice" role="status">${helpers.escape(state.notice)}</p>` : ''}
        </section>
      </div>
      <footer class="dialog-footer">
        <p class="hint">仅在本次评审中生效，不写入磁盘。</p>
        <button class="secondary" type="button" data-action="close-dialog">取消</button>
        <button class="primary" type="button" data-action="settings-save" data-value="${category}">${helpers.icon('check')}应用</button>
      </footer>`;
  }

  function change(state, field, value) {
    const draft = state.settingsDraft;
    if (field === 'theme') {
      return { ...draft, theme: value };
    }
    if (field === 'xsd' || field === 'mod') {
      return { ...draft, resources: { ...draft.resources, [field]: value } };
    }
    return { ...draft, tools: { ...draft.tools, [field]: value } };
  }

  function save(state, category) {
    const draft = state.settingsDraft;
    if (category === 'appearance') {
      return { theme: draft.theme, notice: '主题已应用，仅在本次评审中生效。' };
    }
    if (category === 'resources') {
      const xsd = draft.resources.xsd.trim();
      const mod = draft.resources.mod.trim();
      const missing = resourceFields.filter(function (field) {
        return !draft.resources[field.key].trim();
      });
      if (missing.length) {
        return {
          notice:
            '请填写' +
            missing
              .map(function (field) {
                return field.label;
              })
              .join('、') +
            '，或点击对应的“浏览”填入示例路径；尚未应用。',
        };
      }
      return {
        resources: {
          ...state.resources,
          xsd: xsd,
          mod: mod,
          ready: true,
          error: '',
        },
        notice: '资源路径已应用（演示），未读取档案或核对摘要；须重新校验。',
      };
    }
    const missing = toolFields.filter(function (field) {
      return !draft.tools[field.key].trim();
    });
    if (missing.length) {
      return {
        notice:
          '请填写' +
          missing
            .map(function (field) {
              return field.label;
            })
            .join('、') +
          '，或点击对应的“浏览”填入示例路径；尚未应用。',
      };
    }
    return {
      tools: {
        ...state.tools,
        compiler: draft.tools.compiler.trim(),
        objdump: draft.tools.objdump.trim(),
        git: draft.tools.git.trim(),
        python: draft.tools.python.trim(),
        ready: true,
      },
      notice:
        '工具路径已应用（演示），未探测文件或版本；须重新生成并执行预检。',
    };
  }

  window.PrototypeSettings = { render: render, change: change, save: save };
})();
