// A momentarily idle backend does not prove React's automatic inspection has
// started. This editor appears only after its successful reply is accepted.
export function standardInspectionReady(document) {
  return Boolean(
    document.querySelector(
      '[aria-label="标准输入工作区"] .integration-fields label:first-child input:not(:disabled)',
    ),
  );
}
