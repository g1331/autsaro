import { useCallback, useLayoutEffect, useRef } from 'react';
import type { Workbench } from './useWorkbench';

export function selectWorkbench<K extends keyof Workbench>(
  controller: Workbench,
  fields: readonly K[],
): Pick<Workbench, K> {
  return Object.fromEntries(fields.map((field) => [field, controller[field]])) as Pick<
    Workbench,
    K
  >;
}

export function sameWorkbenchView<V extends object>(previous: V, next: V) {
  return (Object.keys(previous) as (keyof V)[]).every(
    (key) =>
      Object.is(previous[key], next[key]) ||
      (typeof previous[key] === 'function' && typeof next[key] === 'function'),
  );
}

type Action = (...args: never[]) => unknown;

/** Memoized views keep their handlers while actions start from the latest committed render. */
export function useWorkbenchView<K extends keyof Workbench>(
  controller: Workbench,
  fields: readonly K[],
): Pick<Workbench, K> {
  const current = useRef(controller);
  useLayoutEffect(() => {
    current.current = controller;
  }, [controller]);
  const run = useCallback(
    (field: K, args: never[]) => (current.current[field] as Action)(...args),
    [],
  );
  return Object.fromEntries(
    fields.map((field) => [
      field,
      typeof controller[field] === 'function'
        ? (...args: never[]) => run(field, args)
        : controller[field],
    ]),
  ) as Pick<Workbench, K>;
}
