import type {
  ConfigurationChange,
  ConfigurationDiagnostic,
  FieldDescriptor,
  ObjectProjection,
} from './projectTypes';
import type { PlanDiagnostic } from '../types';

/** First-level object view groups, in display order (ADR 0020). */
export const objectGroups = [
  'modules',
  'application',
  'communication',
  'types',
  'bsw',
  'other',
] as const;
export type ObjectGroup = (typeof objectGroups)[number];

type GroupedObject = Pick<
  ObjectProjection,
  'objectId' | 'parentId' | 'kind' | 'definitionId' | 'shortName'
>;

/** The ECUC module a configuration object belongs to, from its definition. */
export function moduleName(object: { definitionId: string | null }): string | null {
  return object.definitionId?.match(/\/EcucDefs\/([^/]+)/)?.[1] ?? null;
}

/** Tree label: a module configuration shows its module name, any other object its short name. */
export function moduleLabel(object: Pick<GroupedObject, 'kind' | 'definitionId' | 'shortName'>) {
  return object.kind === 'ECUC-MODULE-CONFIGURATION-VALUES'
    ? (moduleName(object) ?? object.shortName)
    : object.shortName;
}

/** Group of a top-level element, read from its kind like the object role colour. */
export function objectGroup(object: Pick<GroupedObject, 'kind'>): ObjectGroup {
  const kind = object.kind;
  if (kind === 'ECUC-MODULE-CONFIGURATION-VALUES') return 'modules';
  if (kind.startsWith('BSW-')) return 'bsw';
  if (
    /(DATA-TYPE|BASE-TYPE|MODE-DECLARATION-GROUP$|^SW-ADDR-METHOD$|^COMPU-METHOD$|^UNIT$)/.test(
      kind,
    )
  )
    return 'types';
  if (/(COMPONENT|PORT-PROTOTYPE|INTERFACE|RUNNABLE|CONNECTOR)/.test(kind)) return 'application';
  if (/(^SYSTEM|^ECU-INSTANCE$|^CAN-|FRAME|PDU|I-SIGNAL|CLUSTER)/.test(kind))
    return 'communication';
  return 'other';
}

export type TreeNode =
  | { type: 'group'; key: string; group: ObjectGroup; children: TreeNode[] }
  | { type: 'object'; key: string; object: GroupedObject; children: TreeNode[] };

/**
 * Builds the grouped object view: packages are not nodes; each element directly
 * under a package (or at the root) joins its group, and keeps its own nesting.
 * BSW modules sort by module name, other members by name.
 */
export function groupedObjectTree(objects: readonly GroupedObject[]): TreeNode[] {
  const byId = new Map(objects.map((object) => [object.objectId, object]));
  const children = new Map<string, GroupedObject[]>();
  const members = new Map<ObjectGroup, GroupedObject[]>();
  const isPackage = (object: GroupedObject) => object.kind === 'AR-PACKAGE';
  for (const object of objects) {
    if (isPackage(object)) continue;
    const parent = object.parentId ? byId.get(object.parentId) : undefined;
    if (parent && !isPackage(parent)) {
      const list = children.get(parent.objectId) ?? [];
      list.push(object);
      children.set(parent.objectId, list);
      continue;
    }
    const group = objectGroup(object);
    const list = members.get(group) ?? [];
    list.push(object);
    members.set(group, list);
  }
  const node = (object: GroupedObject): TreeNode => ({
    type: 'object',
    key: object.objectId,
    object,
    children: (children.get(object.objectId) ?? []).map(node),
  });
  return objectGroups.flatMap((group) => {
    const list = members.get(group);
    if (!list?.length) return [];
    const sorted = [...list].sort(
      (left, right) =>
        moduleLabel(left).localeCompare(moduleLabel(right)) ||
        left.shortName.localeCompare(right.shortName),
    );
    return [{ type: 'group' as const, key: `group:${group}`, group, children: sorted.map(node) }];
  });
}

export type ObjectMarks = {
  errors: number;
  warnings: number;
  modified: boolean;
  unapplied: boolean;
};

/** Serialized current state of a field; equal strings mean an unchanged field. */
export function fieldState(field: Pick<FieldDescriptor, 'current' | 'reference'>): string {
  return JSON.stringify([
    field.current,
    field.reference?.state === 'explicit' ? field.reference.rawPath : null,
  ]);
}

/**
 * Marks per tree node, summed over its subtree: diagnostics (by object, or by
 * object path for standard input findings), fields that differ from the saved
 * baseline, and staged drafts.
 */
export function objectMarks(
  tree: readonly TreeNode[],
  input: {
    objects: readonly Pick<ObjectProjection, 'objectId' | 'path'>[];
    fields: readonly Pick<FieldDescriptor, 'fieldId' | 'objectId' | 'current' | 'reference'>[];
    diagnostics: readonly Pick<ConfigurationDiagnostic, 'objectId' | 'fieldId' | 'severity'>[];
    integrationIssues: readonly Pick<PlanDiagnostic, 'object'>[];
    saved: Readonly<Record<string, string>> | null;
    changes: readonly ConfigurationChange[];
  },
): Map<string, ObjectMarks> {
  const own = new Map<string, ObjectMarks>();
  const mark = (objectId: string) => {
    let entry = own.get(objectId);
    if (!entry)
      own.set(objectId, (entry = { errors: 0, warnings: 0, modified: false, unapplied: false }));
    return entry;
  };
  const fieldObject = new Map(input.fields.map((field) => [field.fieldId, field.objectId]));
  for (const issue of input.diagnostics) {
    const objectId = issue.objectId ?? (issue.fieldId ? fieldObject.get(issue.fieldId) : undefined);
    if (!objectId) continue;
    if (issue.severity === 'error') mark(objectId).errors++;
    else if (issue.severity === 'warning') mark(objectId).warnings++;
  }
  const byPath = new Map(input.objects.map((object) => [object.path, object.objectId]));
  for (const issue of input.integrationIssues) {
    const objectId = issue.object ? byPath.get(issue.object) : undefined;
    if (objectId) mark(objectId).errors++;
  }
  if (input.saved)
    for (const field of input.fields)
      if (input.saved[field.fieldId] !== fieldState(field)) mark(field.objectId).modified = true;
  for (const change of input.changes) {
    const objectId =
      'field' in change
        ? change.field.kind === 'existing'
          ? fieldObject.get(change.field.fieldId)
          : change.field.object.kind === 'existing'
            ? change.field.object.objectId
            : undefined
        : 'parent' in change
          ? change.parent.kind === 'existing'
            ? change.parent.objectId
            : undefined
          : change.object.kind === 'existing'
            ? change.object.objectId
            : undefined;
    if (objectId) mark(objectId).unapplied = true;
  }
  const result = new Map<string, ObjectMarks>();
  const visit = (node: TreeNode): ObjectMarks => {
    const total = { errors: 0, warnings: 0, modified: false, unapplied: false };
    const add = (marks: ObjectMarks | undefined) => {
      if (!marks) return;
      total.errors += marks.errors;
      total.warnings += marks.warnings;
      total.modified ||= marks.modified;
      total.unapplied ||= marks.unapplied;
    };
    if (node.type === 'object') add(own.get(node.object.objectId));
    for (const child of node.children) add(visit(child));
    result.set(node.key, total);
    return total;
  };
  tree.forEach(visit);
  return result;
}
