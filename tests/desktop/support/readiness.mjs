// A momentarily idle backend does not prove React's automatic inspection has
// started. This editor appears only after its successful reply is accepted.
export function standardInspectionReady(document) {
  return Boolean(
    document.querySelector(
      '[aria-label="标准输入工作区"] .integration-fields label:first-child input:not(:disabled)',
    ),
  );
}

// Workspace tabs are published before the asynchronous object projection.
// Scenarios expecting object rows must observe their publication, not just tabs.
export function projectObjectsReady(document) {
  return Boolean(
    !document.querySelector('.start-page') &&
    document.querySelector('.document-tabs') &&
    document.querySelector('.tree-content [data-object-id]'),
  );
}

// Closing the confirmation starts a transaction; it does not publish its reply.
export function catalogResultReady(settings, count) {
  const accept = [...(settings?.querySelectorAll('button') ?? [])].find(
    (button) => button.textContent.trim() === '接纳扩展定义…',
  );
  return Boolean(
    settings?.querySelectorAll('.catalog-row').length === count &&
    accept &&
    !accept.disabled &&
    accept.getAttribute('aria-disabled') !== 'true',
  );
}
// Serialized into the WebView; keep synchronous predicates and their DOM
// evidence in one turn, and preserve the existing await for async predicates.
export async function sampleDomQuery(expression) {
  let value = (0, eval)(expression);
  if (value && typeof value.then === 'function') value = await value;
  return {
    value: Boolean(value),
    at: Date.now(),
    uiOperation: document.querySelector('#operation-status')?.textContent ?? null,
    statuses: [...document.querySelectorAll('[role=status]')].map((node) => node.textContent),
    editors: [...document.querySelectorAll('.integration-fields input')].map((node) => ({
      label: node.closest('label')?.textContent,
      disabled: node.disabled,
      value: node.value,
    })),
  };
}
