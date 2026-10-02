import type { Workbench } from '../workbench/useWorkbench';
import { IntegrationPanel } from '../IntegrationPanel';

export function IntegrationPage({ controller }: { controller: Workbench }) {
  const { page, workspace } = controller;
  if (!workspace) return null;
  return (
    <div hidden={page !== 'integration'}>
      <IntegrationPanel controller={controller} />
    </div>
  );
}
