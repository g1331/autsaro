import { useId } from 'react';
import type { ComponentProps, ReactNode } from 'react';
import { Info } from 'lucide-react';
import { useLocale } from '../i18n';
import type { Command } from './commands';
import { iconSize } from './icons';

type CommandButtonProps = Omit<ComponentProps<'button'>, 'onClick' | 'disabled'> & {
  command: Command;
  /** Runs instead of `command.run`, e.g. with a value from the page's own field. */
  run?: () => unknown;
  /** Id of the visible element that states `command.reason`. */
  reasonId?: string;
};

/**
 * An unavailable command stays focusable (`aria-disabled`) so keyboard and screen-reader
 * users reach the reason it is described by; a click does nothing.
 */
export function CommandButton({
  command,
  run = command.run,
  reasonId,
  ...props
}: CommandButtonProps) {
  const unavailable = Boolean(command.reason);
  return (
    <button
      type="button"
      {...props}
      aria-disabled={unavailable || undefined}
      aria-describedby={unavailable ? reasonId : undefined}
      onClick={() => {
        if (!unavailable) void run();
      }}
    />
  );
}

/** States each distinct reason of a group once, with the command that resolves it. */
export function CommandReasons({
  commands,
  id,
  className = 'command-reasons',
}: {
  commands: Command[];
  id: string;
  className?: string;
}) {
  const { text } = useLocale();
  const reasons = distinctReasons(commands, text);
  if (!reasons.length) return null;
  return (
    <ul className={className}>
      {reasons.map(({ reason, remedy }, index) => (
        <li key={reason}>
          <Info size={iconSize.inline} aria-hidden="true" />
          <span id={`${id}-${index}`}>{reason}</span>
          {remedy && !remedy.reason ? (
            <button type="button" className="ghost-button" onClick={() => void remedy.run()}>
              {text(remedy.label)}
            </button>
          ) : null}
        </li>
      ))}
    </ul>
  );
}

export function reasonIdFor(
  commands: Command[],
  id: string,
  text: (value: Command['reason']) => string,
) {
  const reasons = distinctReasons(commands, text).map((entry) => entry.reason);
  return (command: Command) => `${id}-${reasons.indexOf(text(command.reason))}`;
}

function distinctReasons(commands: Command[], text: (value: Command['reason']) => string) {
  const reasons: { reason: string; remedy?: Command }[] = [];
  for (const command of commands) {
    const reason = text(command.reason);
    if (reason && !reasons.some((entry) => entry.reason === reason))
      reasons.push({ reason, remedy: command.remedy });
  }
  return reasons;
}

/** One action group: its buttons, then the reasons its unavailable buttons are described by. */
export function CommandActions({
  actions,
  className = 'section-actions',
}: {
  actions: { command: Command; label: ReactNode; run?: () => unknown; className?: string }[];
  className?: string;
}) {
  const { text } = useLocale();
  const id = useId();
  const commands = actions.map((action) => action.command);
  const reasonId = reasonIdFor(commands, id, text);
  return (
    <>
      <div className={className}>
        {actions.map(({ command, label, run, className: buttonClass }) => (
          <CommandButton
            key={command.id}
            command={command}
            run={run}
            className={buttonClass}
            reasonId={reasonId(command)}
          >
            {label}
          </CommandButton>
        ))}
      </div>
      <CommandReasons commands={commands} id={id} />
    </>
  );
}
