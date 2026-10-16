import type { ReactNode } from 'react';

export type Command = {
  id: string;
  label: string;
  menu?: 'file' | 'edit' | 'view' | 'tools' | 'help';
  /** Menu separators fall between consecutive commands of different groups. */
  group?: string;
  shortcut?: string;
  reason: string;
  run: () => void | Promise<unknown>;
};

/** Menu rows: separators between groups; a disabled reason shared within a group appears once at its end. */
export function menuRows(commands: Command[], run: (id: string) => void) {
  const rows: ReactNode[] = [];
  commands.forEach((command, index) => {
    const group = commands.filter((entry) => entry.group === command.group);
    const shared = (reason: string) => group.filter((entry) => entry.reason === reason).length > 1;
    if (index > 0 && commands[index - 1].group !== command.group)
      rows.push(<hr key={`${command.id}-separator`} />);
    rows.push(
      <button
        key={command.id}
        type="button"
        role="menuitem"
        className="ghost-button"
        disabled={Boolean(command.reason)}
        onClick={() => run(command.id)}
      >
        <span>{command.label}</span>
        <kbd>{command.shortcut}</kbd>
      </button>,
    );
    if (command.reason && !shared(command.reason))
      rows.push(<small key={`${command.id}-reason`}>{command.reason}</small>);
    if (commands[index + 1]?.group !== command.group)
      for (const reason of new Set(group.map((entry) => entry.reason)))
        if (reason && shared(reason))
          rows.push(<small key={`${command.group}-${reason}`}>{reason}</small>);
  });
  return rows;
}
