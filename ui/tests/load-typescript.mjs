import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Execute the actual source; erase types and supply explicit platform adapters.
export async function loadSource(relative, adapters = {}) {
  const source = await readFile(new URL(relative, import.meta.url), 'utf8');
  let { outputText } = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  });
  for (const [specifier, implementation] of Object.entries(adapters)) {
    const url = `data:text/javascript;base64,${Buffer.from(implementation).toString('base64')}`;
    outputText = outputText.replaceAll(`'${specifier}'`, JSON.stringify(url));
  }
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
}
