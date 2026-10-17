import { Command as Palette } from 'cmdk';
import { Search } from 'lucide-react';
import { Dialog, VisuallyHidden } from 'radix-ui';
import { useLocale } from '../i18n';
import type { Command } from './commands';
import { useFocusReturn } from './Dialog';
import { iconSize } from './icons';

/**
 * Ctrl+K: every command with its reason when unavailable; cmdk owns filtering, ↑↓/Enter
 * (unavailable rows are skipped) and the empty state.
 */
export function CommandPalette({
  commands,
  onRun,
  onClose,
}: {
  commands: Command[];
  onRun: (id: string) => void;
  onClose: () => void;
}) {
  const { t, text } = useLocale();
  const returnFocus = useFocusReturn();
  return (
    <Dialog.Root open onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="modal-backdrop palette-backdrop">
          <Dialog.Content
            className="workbench-dialog command-palette"
            aria-describedby={undefined}
            onCloseAutoFocus={returnFocus}
          >
            <VisuallyHidden.Root asChild>
              <Dialog.Title>{t('shell.app.commandSearch')}</Dialog.Title>
            </VisuallyHidden.Root>
            <Palette label={t('shell.app.searchPlaceholder')} loop vimBindings={false}>
              <label className="search-field">
                <Search size={iconSize.control} aria-hidden="true" />
                <Palette.Input
                  aria-label={t('shell.app.searchPlaceholder')}
                  placeholder={t('shell.app.searchPlaceholder')}
                />
              </label>
              <Palette.List className="command-results" label={t('shell.app.commandSearch')}>
                <Palette.Empty className="empty-state">{t('shell.app.noCommands')}</Palette.Empty>
                {commands.map((command) => (
                  <Palette.Item
                    key={command.id}
                    value={text(command.label)}
                    disabled={Boolean(command.reason)}
                    onSelect={() => onRun(command.id)}
                  >
                    <span>{text(command.label)}</span>
                    {command.reason ? <small>{text(command.reason)}</small> : null}
                    {command.shortcut ? <kbd>{command.shortcut}</kbd> : null}
                  </Palette.Item>
                ))}
              </Palette.List>
            </Palette>
          </Dialog.Content>
        </Dialog.Overlay>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
