import { isMessage, message, persistLanguagePreference } from '../i18n';
import { native } from './useDelivery';
import type { BuildTarget, ExecutionTools } from '../types';
import { errorText } from './forms';
import type { WorkbenchSession } from './session';
import { stageDefaults } from './useDelivery';

interface Dependencies {
  invalidateOperation: () => void;
}

export function createSettingsActions(session: WorkbenchSession, dependencies: Dependencies) {
  const { invalidateOperation } = dependencies;
  const { call, patchState, running, epoch, stateRef } = session;
  const { stages } = session.state;
  const setNotice = session.field('notice');
  const setSettingsNotice = session.field('settingsNotice');
  const setBusy = session.field('busy');
  const setGenerated = session.field('generated');
  const setBuilt = session.field('built');
  const setVirtualResult = session.field('virtualResult');
  const setGenerationPreview = session.field('generationPreview');
  const setStages = session.field('stages');

  async function changeTarget(target: BuildTarget) {
    invalidateOperation();
    try {
      await call<void>('select_build_target', { target });
      patchState({
        generationPreview: null,
        generated: null,
        built: null,
        virtualResult: null,
        notice: null,
        stages: {
          ...stages,
          generate: stageDefaults.generate,
          build: stageDefaults.build,
          virtual: stageDefaults.virtual,
        },
      });
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  async function configureTools() {
    if (running.current) return;
    patchState({ settingsNotice: null, toolErrors: {} });
    const generation = epoch.current;
    running.current = true;
    setBusy(message('activity.saveTools'));
    try {
      await call<void>('configure_execution_tools', { tools: stateRef.current.toolDraft });
      setGenerated(null);
      setBuilt(null);
      setVirtualResult(null);
      setGenerationPreview(null);
      setNotice(null);
      setStages((previous) => ({
        ...previous,
        generate: stageDefaults.generate,
        build: stageDefaults.build,
        virtual: stageDefaults.virtual,
      }));
      setSettingsNotice({ tone: 'success', text: message('controller.settings.toolsSaved') });
    } catch (error) {
      // The backend names the rejected path; attach it to the field that holds it.
      const draft = stateRef.current.toolDraft;
      const tool =
        isMessage(error) && error.key === 'backend.workbench.configured_tool_path_invalid'
          ? (Object.keys(draft) as (keyof ExecutionTools)[]).find(
              (key) => draft[key] === error.params.path,
            )
          : undefined;
      patchState({
        toolErrors: tool ? { [tool]: errorText(error) } : {},
        settingsNotice: {
          tone: 'error',
          text: tool ? message('controller.settings.toolsRejected') : errorText(error),
          detail: errorText(error),
        },
      });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function configureAppearance() {
    if (running.current) return;
    const generation = epoch.current;
    running.current = true;
    setBusy(message('activity.saveAppearance'));
    try {
      await call<void>('configure_appearance', { appearance: stateRef.current.appearanceDraft });
      setSettingsNotice({ tone: 'success', text: message('controller.settings.appearanceSaved') });
    } catch (error) {
      setSettingsNotice({ tone: 'error', text: errorText(error) });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }
  async function configureLanguage() {
    if (stateRef.current.languageSaving) return;
    const language = stateRef.current.languageDraft;
    patchState({ settingsNotice: null, languageSaving: true });
    try {
      if (native) await call<void>('configure_language', { language });
      else persistLanguagePreference(language);
      patchState({ savedLanguage: language });
      setSettingsNotice({ tone: 'success', text: message('controller.settings.languageSaved') });
    } catch (error) {
      setSettingsNotice({ tone: 'error', text: errorText(error) });
    } finally {
      patchState({ languageSaving: false });
    }
  }
  return {
    changeTarget,
    configureTools,
    configureAppearance,
    configureLanguage,
  };
}
