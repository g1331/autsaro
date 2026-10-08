import { useLocale } from '../i18n';
import { useEffect, useEffectEvent, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Copy, Minus, Square, X } from 'lucide-react';

type Props = {
  onClose: () => Promise<void>;
  onError: (error: unknown) => void;
};

export function WindowControls({ onClose, onError }: Props) {
  const { t } = useLocale();
  const [maximized, setMaximized] = useState(false);
  const reportError = useEffectEvent(onError);
  useEffect(() => {
    const nativeWindow = getCurrentWindow();
    let live = true;
    let stop: (() => void) | undefined;
    const refresh = async () => {
      try {
        const value = await nativeWindow.isMaximized();
        if (live) setMaximized(value);
      } catch (error) {
        if (live) reportError(error);
      }
    };
    void nativeWindow
      .onResized(() => void refresh())
      .then((unlisten) => {
        if (live) {
          stop = unlisten;
          void refresh();
        } else unlisten();
      })
      .catch((error: unknown) => {
        if (live) reportError(error);
      });
    return () => {
      live = false;
      stop?.();
    };
  }, []);

  const maximizeLabel = maximized ? t('editor.window.restore') : t('editor.window.maximize');
  return (
    <div className="window-controls" role="group" aria-label={t('editor.window.actions')}>
      <button
        type="button"
        className="window-control"
        aria-label={t('editor.window.minimize')}
        title={t('editor.window.minimize')}
        onClick={() => void getCurrentWindow().minimize().catch(onError)}
      >
        <Minus size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="window-control"
        aria-label={maximizeLabel}
        title={maximizeLabel}
        onClick={() => void getCurrentWindow().toggleMaximize().catch(onError)}
      >
        {maximized ? (
          <Copy size={13} aria-hidden="true" />
        ) : (
          <Square size={12} aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        className="window-control window-close"
        aria-label={t('editor.window.close')}
        title={t('editor.window.close')}
        onClick={() => void onClose().catch(onError)}
      >
        <X size={16} aria-hidden="true" />
      </button>
    </div>
  );
}
