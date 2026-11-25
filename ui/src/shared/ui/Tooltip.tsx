import { Tooltip as Primitive } from 'radix-ui';
import type { ReactElement } from 'react';

/** The hint of an icon button; the button keeps its own accessible name. */
export function Tooltip({ label, children }: { label: string; children: ReactElement }) {
  return (
    <Primitive.Provider delayDuration={500}>
      <Primitive.Root>
        <Primitive.Trigger asChild>{children}</Primitive.Trigger>
        <Primitive.Portal>
          <Primitive.Content className="tooltip" side="bottom" sideOffset={4} collisionPadding={6}>
            {label}
          </Primitive.Content>
        </Primitive.Portal>
      </Primitive.Root>
    </Primitive.Provider>
  );
}
