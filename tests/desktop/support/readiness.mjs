// A momentarily idle backend does not prove React's automatic inspection has
// started. This editor appears only after its successful reply is accepted.
export function standardInspectionReady(document) {
  return Boolean(
    document.querySelector(
      '[aria-label="标准输入工作区"] .integration-fields label:first-child input:not(:disabled)',
    ),
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
