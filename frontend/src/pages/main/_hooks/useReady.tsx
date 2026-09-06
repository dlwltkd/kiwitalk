import {
  Accessor,
  ParentProps,
  createContext,
  createResource,
  createSignal,
  onCleanup,
  useContext,
} from 'solid-js';

import {
  created,
  create,
  createMainEventStream,
  reconnect,
} from '@/api/client/client';
import { KiwiTalkEvent, LogoutReason } from '@/api';

const CHANGE_SERVER_REASON = 2;
const RECONNECT_DELAYS_MS = [0, 500, 1500] as const;
const STABLE_CONNECTION_MS = 30_000;

const ReadyContext = createContext<Accessor<boolean>>(() => false);
export const useReady = () => useContext(ReadyContext);

export type ReadyProviderProps = ParentProps<{
  onReadyChange?: (ready: boolean) => void;
  onLogout?: (reason: LogoutReason) => void | Promise<void>;
  onEvent?: (event: KiwiTalkEvent) => void;
}>;
export const ReadyProvider = (props: ReadyProviderProps) => {
  const [isReady, setIsReady] = createSignal(false);
  let cancelled = false;
  let reconnectTask: Promise<boolean> | null = null;
  let connectedSince = 0;
  let reconnectAttemptsUsed = 0;

  const updateReady = (ready: boolean) => {
    setIsReady(ready);
    props.onReadyChange?.(ready);
  };

  const wait = (delay: number) => new Promise<void>((resolve) => {
    window.setTimeout(resolve, delay);
  });

  const reconnectWithBackoff = () => {
    if (reconnectTask) return reconnectTask;

    reconnectTask = (async () => {
      updateReady(false);

      if (Date.now() - connectedSince >= STABLE_CONNECTION_MS) {
        reconnectAttemptsUsed = 0;
      }

      while (reconnectAttemptsUsed < RECONNECT_DELAYS_MS.length) {
        const delay = RECONNECT_DELAYS_MS[reconnectAttemptsUsed];
        reconnectAttemptsUsed += 1;

        if (delay > 0) await wait(delay);
        if (cancelled) return false;

        try {
          await reconnect('Unlocked');
          if (cancelled) return false;

          connectedSince = Date.now();
          updateReady(true);
          return true;
        } catch {
          // Retry only within the bounded schedule above.
        }
      }

      return false;
    })().finally(() => {
      reconnectTask = null;
    });

    return reconnectTask;
  };

  onCleanup(() => {
    cancelled = true;
  });

  createResource(async () => {
    try {
      if (!await created()) {
        await create('Unlocked');
      }
      connectedSince = Date.now();
      updateReady(true);

      while (!cancelled) {
        let reconnectRequested = false;

        try {
          const stream = createMainEventStream();
          for await (const event of stream) {
            if (event.type === 'SwitchServer' || (
              event.type === 'Kickout' && event.content.reason === CHANGE_SERVER_REASON
            )) {
              reconnectRequested = true;
              break;
            }

            if (event.type === 'Kickout') {
              await props.onLogout?.({ type: 'Kickout', reasonId: event.content.reason });
              return;
            }

            props.onEvent?.(event);
          }

          // A closed event queue is treated like a transport failure.
          reconnectRequested = true;
        } catch {
          // Transport failures and event-queue inconsistencies are recoverable.
          reconnectRequested = true;
        }

        if (!reconnectRequested || cancelled) return;

        if (!await reconnectWithBackoff()) {
          if (!cancelled) {
            await props.onLogout?.({
              type: 'Error',
              err: new Error('chat reconnect attempts were exhausted'),
            });
          }
          return;
        }
      }
    } catch (err) {
      if (!cancelled) await props.onLogout?.({ type: 'Error', err });
    } finally {
      if (!cancelled) updateReady(false);
    }
  });

  return (
    <ReadyContext.Provider value={isReady}>
      {props.children}
    </ReadyContext.Provider>
  );
};
