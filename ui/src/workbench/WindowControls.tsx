import { useEffect, useEffectEvent, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Copy, Minus, Square, X } from 'lucide-react';

type Props = {
  onClose: () => Promise<void>;
  onError: (error: unknown) => void;
};

export function WindowControls({ onClose, onError }: Props) {
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

  const maximizeLabel = maximized ? '还原窗口' : '最大化窗口';
  return (
    <div className="window-controls" role="group" aria-label="窗口操作">
      <button
        type="button"
        className="window-control"
        aria-label="最小化窗口"
        title="最小化窗口"
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
        aria-label="关闭工作台"
        title="关闭工作台"
        onClick={() => void onClose().catch(onError)}
      >
        <X size={16} aria-hidden="true" />
      </button>
    </div>
  );
}
