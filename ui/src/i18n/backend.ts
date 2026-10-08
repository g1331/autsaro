import catalog from '../../../core/src/messages.json';

// Shared with Rust; normal Cargo builds generate its static English lookup.
export const backendResources: { 'zh-CN': Record<string, string>; en: Record<string, string> } =
  catalog;
