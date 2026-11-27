import { useState } from 'react';
import { useLocale } from '../../i18n';
import type { VariantEditor as VariantView } from '../../ipc/bindings';

export function VariantEditor({
  layers,
  preview,
  draft,
  update,
  disabled,
}: {
  layers: VariantView;
  preview: (show: (rows: VariantView['differences']) => void) => Promise<void>;
  draft: Record<string, string[]> | null;
  update: (next: Record<string, string[]> | null) => void;
  disabled: boolean;
}) {
  const [comparison, setComparison] = useState<{
    selection: typeof draft;
    rows: VariantView['differences'];
  } | null>(null);
  const compared = comparison && comparison.selection === draft ? comparison.rows : null;
  const { t } = useLocale();
  const original = Object.fromEntries(
    Object.entries(layers.groups).map(([name, group]) => [name, group.selected]),
  );
  const current = draft ?? original;
  function select(group: string, values: string[]) {
    const next = { ...current, [group]: values.sort() };
    update(
      Object.keys(original).some((name) => next[name].join('\n') !== original[name].join('\n'))
        ? next
        : null,
    );
  }
  return (
    <section className="inspector-block">
      <h3>{t('shell.integration.variants')}</h3>
      <p>
        {layers.base} · {layers.revision}
      </p>
      <details>
        <summary>{t('shell.integration.overlay')}</summary>
        <p className="mono path-text">{layers.baseDigest}</p>
        {layers.projectFields.map((field, index) => (
          <p key={index} className="mono path-text">
            {field}
          </p>
        ))}
        {layers.projectMembers.map((member) => (
          <p key={member} className="mono path-text">
            {member}
          </p>
        ))}
      </details>
      {Object.entries(layers.groups).map(([name, group]) => (
        <fieldset disabled={disabled} key={name}>
          <legend>{name}</legend>
          {group.options.map((option) => (
            <label key={option}>
              <input
                type={group.exactlyOne ? 'radio' : 'checkbox'}
                name={name}
                checked={current[name].includes(option)}
                onChange={(event) =>
                  select(
                    name,
                    group.exactlyOne
                      ? [option]
                      : event.target.checked
                        ? [...current[name], option]
                        : current[name].filter((item) => item !== option),
                  )
                }
              />
              {option}
            </label>
          ))}
        </fieldset>
      ))}
      {draft && (
        <button
          disabled={disabled}
          onClick={() => void preview((rows) => setComparison({ selection: draft, rows }))}
        >
          {t('shell.integration.compareVariants')}
        </button>
      )}
      {compared?.length === 0 && <p role="status">{t('shell.integration.noDifferences')}</p>}
      {(compared ?? (draft ? [] : layers.differences)).map((row) => (
        <p key={JSON.stringify([row.object, row.field])}>
          {row.object} · {row.field} ·{' '}
          {row.changes
            .map((change) => t(`shell.integration.diff.${change}`))
            .join(t('shell.listSeparator'))}{' '}
          · {row.before.join(', ')} → {row.after.join(', ')}
        </p>
      ))}
    </section>
  );
}
