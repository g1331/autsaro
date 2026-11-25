import type { Text } from '../../i18n';

export type DocumentKind =
  'configuration' | 'integration' | 'delivery' | 'source' | 'project-entry';
export type DocumentTab = { kind: DocumentKind; sourceId?: string };
export type DraftGuard = { kind: 'context' | 'project'; title: Text };
/** An extension definition catalog waiting for the user's decision; `subject` is its path or ID. */
export type CatalogConfirmation = { kind: 'accept' | 'remove'; subject: string };
