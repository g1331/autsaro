import { useId, useState } from 'react';
import type { ReactNode } from 'react';
import { Menubar } from 'radix-ui';
import { useLocale } from '../i18n';
import type { Command } from './commands';

const menus = ['file', 'edit', 'view', 'tools', 'help'] as const;

/**
 * Radix Menubar owns focus and keys (←→ between menus, ↓ opens, Esc closes); unavailable
 * items stay focusable with `aria-disabled` so their reason can be read (ADR 0019).
 */
export function WorkbenchMenubar({
  label,
  commands,
  run,
}: {
  label: string;
  commands: Command[];
  run: (id: string) => void;
}) {
  const { t } = useLocale();
  const [open, setOpen] = useState('');
  return (
    <Menubar.Root
      className="menubar-menus"
      aria-label={label}
      value={open}
      onValueChange={setOpen}
      loop
    >
      {menus.map((menu) => (
        <Menubar.Menu key={menu} value={menu}>
          <Menubar.Trigger className="ghost-button">{t(`shell.menu.${menu}`)}</Menubar.Trigger>
          <Menubar.Portal>
            <Menubar.Content
              className="menu-popup"
              align="start"
              sideOffset={1}
              loop
              onKeyDown={(event) => {
                // Radix keeps Tab inside an open menu; the menubar pattern closes it.
                if (event.key === 'Tab') setOpen('');
              }}
              onCloseAutoFocus={(event) => {
                // A command that opened a dialog keeps focus there.
                if (document.activeElement?.closest('[role="dialog"]')) event.preventDefault();
              }}
            >
              <MenuRows commands={commands.filter((command) => command.menu === menu)} run={run} />
            </Menubar.Content>
          </Menubar.Portal>
        </Menubar.Menu>
      ))}
    </Menubar.Root>
  );
}

/** Menu rows: separators between groups; a disabled reason shared within a group appears once at its end. */
function MenuRows({ commands, run }: { commands: Command[]; run: (id: string) => void }) {
  const { text } = useLocale();
  const id = useId();
  const rows: ReactNode[] = [];
  commands.forEach((command, index) => {
    const group = commands.filter((entry) => entry.group === command.group);
    const reasons = [...new Set(group.map((entry) => text(entry.reason)).filter(Boolean))];
    const users = (reason: string) => group.filter((entry) => text(entry.reason) === reason).length;
    const reasonId = (reason: string) => `${id}-${command.group}-${reasons.indexOf(reason)}`;
    const reason = text(command.reason);
    if (index > 0 && commands[index - 1].group !== command.group)
      rows.push(<Menubar.Separator key={`${command.id}-separator`} />);
    rows.push(
      <Menubar.Item
        key={command.id}
        className="menu-item"
        textValue={text(command.label)}
        aria-disabled={reason ? true : undefined}
        aria-describedby={reason ? reasonId(reason) : undefined}
        onSelect={(event) => {
          if (reason) event.preventDefault();
          else run(command.id);
        }}
      >
        <span>{text(command.label)}</span>
        <kbd>{command.shortcut}</kbd>
      </Menubar.Item>,
    );
    if (reason && users(reason) === 1)
      rows.push(
        <small key={`${command.id}-reason`} id={reasonId(reason)}>
          {reason}
        </small>,
      );
    if (commands[index + 1]?.group !== command.group)
      for (const shared of reasons.filter((entry) => users(entry) > 1))
        rows.push(
          <small key={`${command.id}-${shared}`} id={reasonId(shared)}>
            {shared}
          </small>,
        );
  });
  return rows;
}
