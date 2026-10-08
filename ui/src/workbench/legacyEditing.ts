import { message, ProductError } from '../i18n';
import type { Text } from '../i18n';
import type { Frame, WorkspaceView } from '../types';
import type { DiagnosticChanges, FrameChanges, Selection, SignalChanges, Stage } from './forms';
import { diagnosticChanges, dtcChanges, errorText, frameChanges, signalChanges } from './forms';
import type { WorkbenchOperation } from './operation';
import type { WorkbenchSession } from './session';

interface Dependencies {
  run: <T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) => Promise<void>;
  invalidateAfterEdit: () => void;
  acceptView: (view: WorkspaceView, requested?: Selection | null) => void;
  focusedFrame: Frame | undefined;
  diagnosticUnapplied: boolean;
  frameUnapplied: boolean;
  unapplied: boolean;
  confirmAction: (message: Text) => Promise<boolean>;
  dtcUnapplied: boolean;
}

export function createLegacyEditing(session: WorkbenchSession, dependencies: Dependencies) {
  const {
    run,
    invalidateAfterEdit,
    acceptView,
    focusedFrame,
    diagnosticUnapplied,
    frameUnapplied,
    unapplied,
    confirmAction,
    dtcUnapplied,
  } = dependencies;
  const { call } = session;
  const { frameInput, signalInput, draft, workspace, diagnosticDraft, dtcDraft } = session.state;
  const setNotice = session.field('notice');
  const setCreating = session.field('creating');
  const setDiagnosticError = session.field('diagnosticError');
  const setDtcError = session.field('dtcError');

  function addFrame() {
    let values: FrameChanges;
    try {
      values = frameChanges(frameInput);
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
      return;
    }
    void run(
      { kind: 'action', label: message('workflow.action.addFrame') },
      () => call<WorkspaceView>('add_frame', values),
      (view) => {
        invalidateAfterEdit();
        acceptView(view, {
          kind: 'frame',
          path: view.frames.find((frame) => frame.name === values.name)?.path ?? '',
        });
        setCreating(null);
      },
      undefined,
      'frame',
    );
  }

  function addSignal() {
    if (!focusedFrame) return;
    let values: SignalChanges;
    try {
      values = signalChanges(signalInput, focusedFrame);
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
      return;
    }
    const framePath = focusedFrame.path;
    void run(
      { kind: 'action', label: message('workflow.action.addSignal') },
      () => call<WorkspaceView>('add_signal', { framePath, ...values }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view, {
          kind: 'signal',
          path:
            view.signals.find(
              (signal) => signal.name === values.name && signal.framePath === framePath,
            )?.path ?? '',
        });
        setCreating(null);
      },
      undefined,
      'frame',
    );
  }

  function updateSelected() {
    if (!draft || !workspace) return;
    try {
      if (draft.kind === 'frame') {
        const changes = frameChanges(draft.fields);
        const oldPath = draft.path;
        void run(
          { kind: 'action', label: message('workflow.action.updateFrame') },
          () => call<WorkspaceView>('update_frame', { path: oldPath, changes }),
          (view) => {
            invalidateAfterEdit();
            acceptView(view, {
              kind: 'frame',
              path:
                view.frames.find((frame) => frame.path === oldPath)?.path ??
                view.frames.find((frame) => frame.name === changes.name)?.path ??
                '',
            });
          },
          undefined,
          'frame',
        );
      } else {
        const owner = workspace.frames.find(
          (frame) =>
            frame.path === workspace.signals.find((item) => item.path === draft.path)?.framePath,
        );
        if (!owner) throw new ProductError(message('workflow.error.updateSignalFrameMissing'));
        const changes = signalChanges(draft.fields, owner);
        const oldPath = draft.path;
        void run(
          { kind: 'action', label: message('workflow.action.updateSignal') },
          () => call<WorkspaceView>('update_signal', { path: oldPath, changes }),
          (view) => {
            invalidateAfterEdit();
            acceptView(view, {
              kind: 'signal',
              path:
                view.signals.find((signal) => signal.path === oldPath)?.path ??
                view.signals.find(
                  (signal) => signal.name === changes.name && signal.framePath === owner.path,
                )?.path ??
                '',
            });
          },
          undefined,
          'frame',
        );
      }
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  function configureDiagnostic() {
    if (!workspace) return;
    let values: DiagnosticChanges;
    try {
      values = diagnosticChanges(diagnosticDraft, workspace);
      setDiagnosticError('');
    } catch (error) {
      setDiagnosticError(errorText(error));
      return;
    }
    void run(
      { kind: 'action', label: message('workflow.action.configureDiagnostic') },
      () =>
        call<WorkspaceView>('configure_diagnostic', { settings: values }).catch((error) => {
          setDiagnosticError(errorText(error));
          throw error;
        }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'diagnostic',
    );
  }

  function configureDtc() {
    if (!workspace?.diagnostic || diagnosticUnapplied || frameUnapplied) return;
    let values: { code: number; monitorFramePath: string };
    try {
      values = dtcChanges(dtcDraft, workspace);
      setDtcError('');
    } catch (error) {
      setDtcError(errorText(error));
      return;
    }
    void run(
      { kind: 'action', label: message('workflow.action.configureDtc') },
      () =>
        call<WorkspaceView>('configure_dtc', values).catch((error) => {
          setDtcError(errorText(error));
          throw error;
        }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'dtc',
    );
  }

  async function clearDtc() {
    if (!workspace?.diagnostic?.dtc || unapplied) return;
    if (!(await confirmAction(message('workflow.confirm.removeDtc')))) return;
    void run(
      { kind: 'action', label: message('workflow.action.removeDtc') },
      () => call<WorkspaceView>('clear_dtc'),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
    );
  }

  async function clearDiagnostic() {
    if (!workspace?.diagnostic || diagnosticUnapplied || dtcUnapplied || frameUnapplied) return;
    if (!(await confirmAction(message('workflow.confirm.removeDiagnostic')))) return;
    void run(
      { kind: 'action', label: message('workflow.action.removeDiagnostic') },
      () => call<WorkspaceView>('clear_diagnostic'),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'diagnostic',
    );
  }
  return {
    addFrame,
    addSignal,
    updateSelected,
    configureDiagnostic,
    configureDtc,
    clearDtc,
    clearDiagnostic,
  };
}
