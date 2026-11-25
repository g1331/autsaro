import type { WorkspaceView } from '../../ipc/types';
import type { CatalogConfirmation, DocumentTab, DraftGuard } from './projectTypes';
import type { ProjectCreationPreview, ProjectProjection } from '../../ipc/types';
import { fieldState } from './projectGroups';

export type ProjectState = {
  workspace: WorkspaceView | null;
  projection: ProjectProjection | null;
  memberProjectPath: string | null;
  /** Field states of the last clean projection, by field ID. */
  savedValues: Record<string, string> | null;
  source: 'empty' | 'import' | 'save-as';
  projectName: string;
  projectDirectory: string;
  importPaths: string[];
  importPathText: string;
  projectPreview: ProjectCreationPreview | null;
  projectPreviewKind: 'create' | 'save-as';
  objectSelection: string[];
  activeObjectId: string | null;
  /** Parameter whose grid row has focus; the inspector describes it. */
  activeFieldId: string | null;
  activeSourceId: string | null;
  sourceText: string | null;
  tabs: DocumentTab[];
  activeDocument: DocumentTab;
  guard: DraftGuard | null;
  catalogConfirmation: CatalogConfirmation | null;
  replacementReady: boolean;
  savingForReplacement: boolean;
};

export function initialProjectState(): ProjectState {
  return {
    workspace: null,
    projection: null,
    memberProjectPath: null,
    savedValues: null,
    source: 'empty',
    projectName: '',
    projectDirectory: '',
    importPaths: [],
    importPathText: '',
    projectPreview: null,
    projectPreviewKind: 'create',
    objectSelection: [],
    activeObjectId: null,
    activeFieldId: null,
    activeSourceId: null,
    sourceText: null,
    tabs: [{ kind: 'configuration' }],
    activeDocument: { kind: 'configuration' },
    guard: null,
    catalogConfirmation: null,
    replacementReady: false,
    savingForReplacement: false,
  };
}

/** A clean projection becomes the baseline for modified marks in the tree. */
export function projectionBaseline(projection: ProjectProjection | null | undefined) {
  if (projection === null) return { savedValues: null };
  if (projection && !projection.dirty)
    return {
      savedValues: Object.fromEntries(
        projection.fields.map((field) => [field.fieldId, fieldState(field)]),
      ),
    };
  return {};
}
