import { distinct } from '../shared/ui/forms';
import type { WorkbenchState } from './state';

export function problemSummary(
  state: Pick<WorkbenchState, 'projection' | 'workspace' | 'operationIssues' | 'integrationIssues'>,
) {
  const { workspace, operationIssues } = state;
  const diagnostics = distinct(state.projection?.diagnostics ?? []);

  const issues = distinct([...(workspace?.issues ?? []), ...operationIssues]);

  const integrationProblems = distinct(state.integrationIssues);

  const problemCount = diagnostics.length + issues.length + integrationProblems.length;

  const errorCount =
    diagnostics.filter((issue) => issue.severity === 'error').length +
    issues.filter((issue) => issue.severity === 'error').length +
    integrationProblems.length;

  return { diagnostics, issues, problemCount, errorCount };
}
