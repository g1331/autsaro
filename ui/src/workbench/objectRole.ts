export type ObjectRole = 'blue' | 'teal' | 'purple' | 'gold';

const communication = new Set([
  'Com',
  'PduR',
  'CanIf',
  'CanTp',
  'Can',
  'CanSM',
  'ComM',
  'Nm',
  'CanNm',
]);
const diagnostics = new Set(['Dcm', 'Dem', 'FiM']);

/**
 * Object-role colour from DESIGN.md: ECUC objects take the role of their
 * module, other elements the role of their kind. Packages and unknown kinds
 * stay neutral.
 */
export function objectRole(object: {
  kind: string;
  definitionId: string | null;
}): ObjectRole | null {
  const module = object.definitionId?.match(/\/EcucDefs\/([^/]+)/)?.[1];
  if (module) return communication.has(module) ? 'blue' : diagnostics.has(module) ? 'gold' : 'teal';
  const kind = object.kind;
  if (kind.startsWith('ECUC-')) return 'teal';
  if (kind.startsWith('DIAGNOSTIC-')) return 'gold';
  if (/(^I-SIGNAL|PDU|FRAME|^CAN-|CLUSTER)/.test(kind)) return 'blue';
  if (/(COMPONENT|PORT|INTERFACE|DATA-TYPE|BASE-TYPE|RUNNABLE)/.test(kind)) return 'purple';
  if (/^(ECU-INSTANCE|SYSTEM)$/.test(kind)) return 'teal';
  return null;
}

export function objectRoleClass(object: { kind: string; definitionId: string | null }) {
  const role = objectRole(object);
  return role ? `role-${role}` : undefined;
}
