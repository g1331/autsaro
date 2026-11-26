import { previewDelta } from '../../shared/ui/forms';
import type { DeliveryState } from './state';

export function deliveryPreviews({
  savePreview,
  previewPath,
  generationPreview,
  generationPreviewPath,
}: Pick<
  DeliveryState,
  'savePreview' | 'previewPath' | 'generationPreview' | 'generationPreviewPath'
>) {
  const previewFile = savePreview?.files.find((file) => file.path === previewPath);

  const delta =
    previewFile?.before !== null &&
    previewFile?.after !== null &&
    previewFile?.before !== undefined &&
    previewFile?.after !== undefined
      ? previewDelta(previewFile.before, previewFile.after)
      : null;

  const generationPreviewFile = generationPreview?.files.find(
    (file) => file.path === generationPreviewPath,
  );

  const generationDelta =
    generationPreviewFile?.status === 'changed' &&
    generationPreviewFile.before !== null &&
    generationPreviewFile.after !== null
      ? previewDelta(generationPreviewFile.before, generationPreviewFile.after, false)
      : null;

  return { previewFile, delta, generationPreviewFile, generationDelta };
}
