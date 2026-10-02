import { isTauri } from '@tauri-apps/api/core';
import type { WorkspaceView } from '../types';
import type { Stage, StageRecord, StageState } from './forms';

export const native = isTauri();

export const steps: { key: Stage; label: string; number: string }[] = [
  { key: 'save', label: '保存配置', number: '01' },
  { key: 'validate', label: '校验项目', number: '02' },
  { key: 'generate', label: '生成 C99 工程', number: '03' },
  { key: 'build', label: '构建主机目标', number: '04' },
  { key: 'virtual', label: '主机虚拟闭环', number: '05' },
];

export const buildSteps = steps.slice(0, 4);

export const stageDefaults: Record<Stage, StageRecord> = {
  save: { state: 'pending', detail: '尚未保存' },
  validate: { state: 'pending', detail: '尚未校验' },
  generate: { state: 'pending', detail: '尚未生成' },
  build: { state: 'pending', detail: '尚未构建' },
  virtual: { state: 'pending', detail: '尚未运行' },
};

export const stageLabels: Record<StageState, string> = {
  pending: '待执行',
  running: '进行中',
  done: '已完成',
  failed: '未通过',
  stale: '已过期',
};

export function importedSaveStage(view: WorkspaceView): StageRecord {
  if (
    view.issues.some((issue) => issue.code.startsWith('PDU_') || issue.code === 'DIAG_UNSUPPORTED')
  ) {
    return {
      state: 'failed',
      detail: '导入的配置不属于当前支持范围；来源文件未被修改，不能保存或生成',
    };
  }
  return view.dirty ? stageDefaults.save : { state: 'done', detail: '项目配置已保存' };
}
