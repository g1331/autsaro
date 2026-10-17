import { useMemo } from 'react';
import { CircleX, TriangleAlert } from 'lucide-react';
import { translate as t } from '../i18n';
import type { Workbench } from './useWorkbench';
import { iconSize } from './icons';
import { objectMarks } from './projectGroups';
import type { ObjectMarks, TreeNode } from './projectGroups';

type MarkSources = Pick<Workbench, 'projection' | 'integrationIssues' | 'savedValues' | 'changes'>;

/** Problem and change marks of every node in a grouped tree, summed over subtrees. */
export function useObjectMarks(c: MarkSources, tree: readonly TreeNode[]) {
  return useMemo(
    () =>
      objectMarks(tree, {
        objects: c.projection?.objects ?? [],
        fields: c.projection?.fields ?? [],
        diagnostics: c.projection?.diagnostics ?? [],
        integrationIssues: c.integrationIssues,
        saved: c.savedValues,
        changes: c.changes,
      }),
    [tree, c.projection, c.integrationIssues, c.savedValues, c.changes],
  );
}

/** The marks as words, for accessible descriptions and titles. */
export function markLabels(mark: ObjectMarks | undefined): string[] {
  return [
    mark?.errors ? t('editor.tree.errors', { count: mark.errors }) : '',
    mark?.warnings ? t('editor.tree.warnings', { count: mark.warnings }) : '',
    mark?.modified ? t('editor.tree.modified') : '',
    mark?.unapplied ? t('editor.tree.unapplied') : '',
  ].filter(Boolean);
}

/** Visible marks; the same information must also be given as text by `markLabels`. */
export function MarkIcons({ mark }: { mark: ObjectMarks | undefined }) {
  if (!mark || (!mark.errors && !mark.warnings && !mark.modified && !mark.unapplied)) return null;
  return (
    <small className="tree-marks" aria-hidden="true">
      {mark.errors ? (
        <span className="tree-mark-error">
          <CircleX size={iconSize.inline} />
          {mark.errors}
        </span>
      ) : null}
      {mark.warnings ? (
        <span className="tree-mark-warning">
          <TriangleAlert size={iconSize.inline} />
          {mark.warnings}
        </span>
      ) : null}
      {mark.unapplied ? <span className="tree-mark-unapplied">◆</span> : null}
      {mark.modified ? <span className="tree-mark-modified">●</span> : null}
    </small>
  );
}
