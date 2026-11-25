import { readFileSync } from 'node:fs';
import { join } from 'node:path';

export function workbenchStyles() {
  const source = join(import.meta.dirname, '../../src');
  return readFileSync(join(source, 'styles.css'), 'utf8').replace(
    /@import '([^']+)';/g,
    (_, stylesheet) => readFileSync(join(source, stylesheet), 'utf8'),
  );
}
