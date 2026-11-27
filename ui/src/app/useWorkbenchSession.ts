import { useEffect, useEffectEvent, useRef, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { composedMessage, message, ProductError } from '../i18n';
import type { Text } from '../i18n';
import { invoke } from '../ipc';
import { native } from '../ipc/native';
import type { CallArgs, Value, WorkbenchCommand } from '../ipc';
import type { CommandMap } from '../ipc/bindings';
import type { WorkbenchCapabilities } from '../ipc/types';
import type { Stage } from '../features/delivery/stages';
import { errorText } from '../shared/ui/forms';
import { useLanguagePreview } from '../features/settings/useLanguagePreview';
import { useWindowTitle } from './useWindowTitle';
import { initialDomains, reduceWorkbench, workbenchSnapshot } from './state';
import type { WorkbenchState } from './state';
import type { WorkbenchSession } from './session';

export function useWorkbenchSession(): WorkbenchSession & {
  invalidateOperation(): void;
  cancelOperation(): Promise<void>;
} {
  const [domains, setDomains] = useState(initialDomains);
  const domainsRef = useRef(domains);
  const state = workbenchSnapshot(domains);
  const stateRef = useRef(state);

  function patchState(patch: Partial<WorkbenchState>) {
    // Commands and their continuations can read the accepted transaction before React renders.
    const next = reduceWorkbench(domainsRef.current, patch);
    domainsRef.current = next;
    stateRef.current = workbenchSnapshot(next);
    setDomains(next);
  }

  useLanguagePreview(state.languageDraft);
  useWindowTitle(patchState);

  const capabilitiesRef = useRef<WorkbenchCapabilities | null>(null);

  const epoch = useRef(0);

  const running = useRef(false);

  const pendingGuard = useRef<(() => void | Promise<void>) | null>(null);
  const pendingReplacement = useRef<(() => void | Promise<void>) | null>(null);
  const changeConfirmation = useRef<((applied: boolean) => void) | null>(null);
  const catalogDecision = useRef<((accepted: boolean) => void) | null>(null);

  useEffect(() => {
    if (!native) return;
    let live = true;
    const operationEpoch = epoch;
    void invoke('workbench_capabilities')
      .then((capabilities) => {
        if (!live) return;
        capabilitiesRef.current = capabilities;
        patchState({
          capabilities,
          buildTarget: capabilities.target,
          appearanceDraft: capabilities.appearance,
          languageDraft: capabilities.language ?? 'system',
          savedLanguage: capabilities.language ?? 'system',
          toolDraft: capabilities.configuredExecutionTools ?? {
            compiler: '',
            objdump: '',
            git: '',
            python: '',
          },
        });
      })
      .catch((error) => {
        if (live) patchState({ notice: { tone: 'error', text: errorText(error) } });
      });
    return () => {
      live = false;
      operationEpoch.current++;
    };
  }, []);

  const resumeReplacement = useEffectEvent(async () => {
    if (!stateRef.current.replacementReady || stateRef.current.busy || running.current) return;
    const action = pendingReplacement.current;
    pendingReplacement.current = null;
    patchState({ replacementReady: false, savingForReplacement: false });
    await action?.();
  });
  useEffect(() => {
    void resumeReplacement();
  }, [state.replacementReady, state.busy]);

  function call<C extends WorkbenchCommand>(command: C, ...args: CallArgs<C>): Promise<Value<C>>;
  async function call(command: WorkbenchCommand, payload: Record<string, unknown> = {}) {
    const capabilities = capabilitiesRef.current;
    if (!capabilities) throw new ProductError(message('controller.error.capabilitiesNotReady'));
    const generation = epoch.current;
    const fingerprint = capabilities.fingerprint;
    try {
      const reply = await invoke(command, {
        ...payload,
        fingerprint,
      } as CommandMap[WorkbenchCommand]['args']);
      if (generation !== epoch.current || reply.inputFingerprint !== fingerprint) {
        throw new ProductError(message('controller.error.staleDelivery'));
      }
      const previous = capabilitiesRef.current;
      capabilitiesRef.current = reply.capabilities;
      patchState({
        capabilities: reply.capabilities,
        buildTarget: reply.capabilities.target,
        changePreview:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.changePreview,
        preparedChangeSet:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.preparedChangeSet,
        applicationPreview:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.applicationPreview,
      });
      const value = typeof reply.value === 'object' && reply.value !== null ? reply.value : null;
      const detail: Text =
        value && 'log' in value
          ? errorText(value.log)
          : value && 'logs' in value && Array.isArray(value.logs)
            ? (value.logs as Text[])
            : command;
      const failed = Boolean(
        value &&
        (('status' in value && value.status === 'failed') ||
          ('passed' in value && value.passed === false) ||
          ('exitCode' in value && typeof value.exitCode === 'number' && value.exitCode !== 0)),
      );
      field('operationLog')((logs) => [
        ...logs,
        { command, outcome: failed ? 'failed' : 'done', detail },
      ]);
      return reply.value;
    } catch (error) {
      if (generation === epoch.current)
        field('operationLog')((logs) => [
          ...logs,
          { command, outcome: 'failed', detail: errorText(error) },
        ]);
      if (generation === epoch.current) {
        const current = await invoke('workbench_capabilities');
        if (generation === epoch.current) {
          capabilitiesRef.current = current;
          const presentationOnly =
            command === 'configure_language' || command === 'configure_appearance';
          patchState({
            capabilities: current,
            buildTarget: current.target,
            ...(presentationOnly
              ? {}
              : { changePreview: null, preparedChangeSet: null, applicationPreview: null }),
          });
        }
      }
      throw error;
    }
  }

  function invalidateOperation() {
    epoch.current++;
    running.current = false;
    changeConfirmation.current?.(false);
    changeConfirmation.current = null;
    catalogDecision.current?.(false);
    catalogDecision.current = null;
    pendingGuard.current = null;
    pendingReplacement.current = null;
    patchState({
      guard: null,
      catalogConfirmation: null,
      changePreview: null,
      preparedChangeSet: null,
      applicationPreview: null,
      savingForReplacement: false,
      replacementReady: false,
    });
    field('stages')((previous) => {
      const next = { ...previous };
      for (const key of Object.keys(next) as Stage[]) {
        if (next[key].state === 'running') {
          next[key] = { state: 'stale', detail: message('controller.operation.cancelledResult') };
        }
      }
      return next;
    });
    patchState({ busy: null, integrationProcessing: false, operationGeneration: epoch.current });
  }

  async function cancelOperation() {
    invalidateOperation();
    try {
      await call('cancel_operation');
      patchState({ notice: { tone: 'info', text: message('controller.operation.cancelled') } });
    } catch (error) {
      field('notice')({
        tone: 'error',
        text: composedMessage('controller.operation.cancelFailed', { error: errorText(error) }),
      });
    }
  }

  function field<K extends keyof WorkbenchState>(
    key: K,
  ): Dispatch<SetStateAction<WorkbenchState[K]>> {
    return (value) => {
      const previous = stateRef.current;
      const resolved =
        typeof value === 'function'
          ? (value as (previous: WorkbenchState[K]) => WorkbenchState[K])(previous[key])
          : value;
      const patch = {
        [key]: resolved,
        ...([
          'connectionDrafts',
          'variantSelection',
          'signalDrafts',
          'integrationIds',
          'integrationPeriod',
          'changes',
        ].includes(key as string)
          ? { changePreview: null, preparedChangeSet: null, applicationPreview: null }
          : {}),
      } as Partial<WorkbenchState>;
      patchState(patch);
    };
  }

  return {
    state,
    stateRef,
    capabilitiesRef,
    epoch,
    running,
    pendingGuard,
    pendingReplacement,
    changeConfirmation,
    catalogDecision,
    field,
    patchState,
    call,
    invalidateOperation,
    cancelOperation,
  };
}
