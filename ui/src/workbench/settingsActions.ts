import { message, persistLanguagePreference } from '../i18n';
import { native } from './useDelivery';
import type { BuildTarget } from '../types';
import { errorText } from './forms';
import type { WorkbenchSession } from './session';
import { stageDefaults } from './useDelivery';

interface Dependencies {
  invalidateOperation: () => void;
}

export function createSettingsActions(session: WorkbenchSession, dependencies: Dependencies) {
  const { invalidateOperation } = dependencies;
  const { call, patchState, running, state, epoch, stateRef } = session;
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

  async function configureResources() {
    if (running.current) return;
    setSettingsNotice('');
    try {
      await call<void>('configure_validation_resources', state.resourceDraft);
      patchState({
        generationPreview: null,
        generated: null,
        built: null,
        virtualResult: null,
        notice: null,
        stages: {
          ...stages,
          validate: { state: 'stale', detail: message('controller.settings.resourcesChanged') },
          generate: stageDefaults.generate,
          build: stageDefaults.build,
          virtual: stageDefaults.virtual,
        },
      });
      setSettingsNotice(message('controller.settings.resourcesSaved'));
    } catch (error) {
      setSettingsNotice(errorText(error));
    }
  }

  async function configureTools() {
    if (running.current) return;
    setSettingsNotice('');
    const generation = epoch.current;
    running.current = true;
    setBusy(message('controller.settings.saveTools'));
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
      setSettingsNotice(message('controller.settings.toolsSaved'));
    } catch (error) {
      setSettingsNotice(errorText(error));
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
    setBusy(message('controller.settings.saveAppearance'));
    try {
      await call<void>('configure_appearance', { appearance: stateRef.current.appearanceDraft });
      setSettingsNotice(message('controller.settings.appearanceSaved'));
    } catch (error) {
      setSettingsNotice(errorText(error));
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
    setSettingsNotice('');
    patchState({ languageSaving: true });
    try {
      if (native) await call<void>('configure_language', { language });
      else persistLanguagePreference(language);
      patchState({ savedLanguage: language });
      setSettingsNotice(message('controller.settings.languageSaved'));
    } catch (error) {
      setSettingsNotice(errorText(error));
    } finally {
      patchState({ languageSaving: false });
    }
  }
  return {
    changeTarget,
    configureResources,
    configureTools,
    configureAppearance,
    configureLanguage,
  };
}
