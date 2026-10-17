import { ContextMenu } from 'radix-ui';
import { useId } from 'react';
import type { ReactNode } from 'react';
import { message, useLocale } from '../i18n';
import type { Workbench } from './useWorkbench';
import { commandById } from './commands';
import type { InstanceDefinitionOption, ObjectProjection } from './projectTypes';

type MenuController = Pick<
  Workbench,
  | 'projection'
  | 'changes'
  | 'activeObjectId'
  | 'objectSelection'
  | 'actionReason'
  | 'stageChange'
  | 'guardContext'
  | 'selectObject'
  | 'setInspectorTab'
  | 'commands'
>;

/** Atomic software components, whose user application code the workbench initializes. */
const isAtomicComponent = (object: ObjectProjection) =>
  /SW-COMPONENT-TYPE$/.test(object.kind) && object.kind !== 'COMPOSITION-SW-COMPONENT-TYPE';

export const definitionName = (definitionId: string) =>
  definitionId.split('/').pop() ?? definitionId;

/** Instances of a child definition under a parent, counting staged creations. */
export function childCount(
  c: Pick<Workbench, 'projection' | 'changes'>,
  parentId: string,
  definitionId: string,
) {
  const existing =
    c.projection?.objects.filter(
      (object) => object.parentId === parentId && object.definitionId === definitionId,
    ).length ?? 0;
  const staged = c.changes.filter(
    (change) =>
      change.op === 'create-instance' &&
      change.parent.kind === 'existing' &&
      change.parent.objectId === parentId &&
      change.definitionId === definitionId,
  ).length;
  return existing + staged;
}

/** Child definitions the definition allows under the object and whose multiplicity is not yet full. */
export function addableDefinitions(
  c: Pick<Workbench, 'projection' | 'changes'>,
  object: ObjectProjection,
): InstanceDefinitionOption[] {
  if (!object.writable) return [];
  return (c.projection?.instanceDefinitions ?? []).filter(
    (option) =>
      option.parentId === object.objectId &&
      option.definition.writable &&
      (option.definition.upperMultiplicity === null ||
        childCount(c, object.objectId, option.definition.definitionId) <
          option.definition.upperMultiplicity),
  );
}

function focusLater(selector: string) {
  requestAnimationFrame(() => document.querySelector<HTMLElement>(selector)?.focus());
}

/** Runs an action with the object current, through the draft guard when it is not. */
function onObject(c: MenuController, object: ObjectProjection, action: () => void) {
  c.setInspectorTab('properties');
  if (c.activeObjectId === object.objectId && c.objectSelection.length <= 1) action();
  else
    void c.guardContext(message('activity.chooseConfiguration'), async () => {
      await c.selectObject(object.objectId);
      action();
    });
}

/** Stages a new child instance; its name is entered in the inspector's structure editor. */
export function addChild(c: MenuController, object: ObjectProjection, definitionId: string) {
  const changeId = crypto.randomUUID();
  onObject(c, object, () => {
    c.stageChange({
      op: 'create-instance',
      changeId,
      parent: { kind: 'existing', objectId: object.objectId },
      sourceId: object.sourceId,
      definitionId,
      shortName: '',
    });
    focusLater(`[data-creation-id="${changeId}"]`);
  });
}

export function removeObject(c: MenuController, object: ObjectProjection) {
  c.stageChange({
    op: 'remove-instance',
    changeId: object.objectId,
    object: { kind: 'existing', objectId: object.objectId },
    expectedShortName: object.shortName,
  });
}

/**
 * Context menu of an object in the tree or the sub-container table (ADR 0020 D9): add an
 * allowed child, rename or remove; each only stages a draft for the batch preview.
 */
export function ObjectContextMenu({
  controller,
  object,
  children,
}: {
  /** A getter reads the controller when the menu opens, for callers that memoize their rows. */
  controller: MenuController | (() => MenuController);
  object: ObjectProjection;
  children: ReactNode;
}) {
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild disabled={!object.writable && !isAtomicComponent(object)}>
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content className="menu-popup" loop>
          <ObjectMenuItems controller={controller} object={object} />
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}

/** Unavailable items stay focusable with `aria-disabled` and the shared reason, as in the menubar. */
function ObjectMenuItems({
  controller,
  object,
}: {
  controller: MenuController | (() => MenuController);
  object: ObjectProjection;
}) {
  // Mounted only while open, so a getter is read at opening time.
  const c = typeof controller === 'function' ? controller() : controller;
  const { t, text } = useLocale();
  const reasonId = useId();
  const reason = text(c.actionReason('prepare-change'));
  const item = (
    key: string,
    label: string,
    run: () => void,
    blocked = reason,
    describedBy = reasonId,
  ) => (
    <ContextMenu.Item
      key={key}
      className="menu-item"
      aria-disabled={blocked ? true : undefined}
      aria-describedby={blocked ? describedBy : undefined}
      onSelect={(event) => {
        if (blocked) event.preventDefault();
        else run();
      }}
    >
      <span>{label}</span>
    </ContextMenu.Item>
  );
  // ADR 0020 D7: user application initialization starts from the component node.
  const initialize = isAtomicComponent(object) ? commandById(c, 'initialize-application') : null;
  const initializeReason = initialize ? text(initialize.reason) : '';
  const initializeItem = initialize ? (
    <>
      {item(
        'initialize',
        text(initialize.label),
        () => void initialize.run(),
        initializeReason,
        `${reasonId}-initialize`,
      )}
      {initializeReason ? <small id={`${reasonId}-initialize`}>{initializeReason}</small> : null}
    </>
  ) : null;
  if (!object.writable) return initializeItem;
  const options = addableDefinitions(c, object);
  return (
    <>
      {initializeItem}
      {initializeItem ? <ContextMenu.Separator /> : null}
      {options.map((option) =>
        item(
          option.definition.definitionId,
          t('editor.container.add', { name: definitionName(option.definition.definitionId) }),
          () => addChild(c, object, option.definition.definitionId),
        ),
      )}
      {options.length ? <ContextMenu.Separator /> : null}
      {item('rename', t('editor.container.rename'), () =>
        onObject(c, object, () => focusLater('[data-rename-input]')),
      )}
      {item('remove', t('editor.objects.removeInstance'), () => removeObject(c, object))}
      {reason ? <small id={reasonId}>{reason}</small> : null}
    </>
  );
}
