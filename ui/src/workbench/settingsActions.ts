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
          validate: { state: 'stale', detail: '规范档案已变化，须重新校验' },
          generate: stageDefaults.generate,
          build: stageDefaults.build,
          virtual: stageDefaults.virtual,
        },
      });
      setSettingsNotice('规范档案已核对固定摘要并保存；须重新校验。');
    } catch (error) {
      setSettingsNotice(errorText(error));
    }
  }

  async function configureTools() {
    if (running.current) return;
    setSettingsNotice('');
    const generation = epoch.current;
    running.current = true;
    setBusy('保存执行工具');
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
      setSettingsNotice('执行工具已保存；原生预检尚未执行。');
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
    setBusy('保存外观');
    try {
      await call<void>('configure_appearance', { appearance: stateRef.current.appearanceDraft });
      setSettingsNotice('外观已保存；工程输入保持。');
    } catch (error) {
      setSettingsNotice(errorText(error));
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }
  return { changeTarget, configureResources, configureTools, configureAppearance };
}
