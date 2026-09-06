import { createSignal, onCleanup, onMount } from 'solid-js';
import { Outlet, useLocation, useNavigate } from '@solidjs/router';

import { KiwiTalkEvent, LogoutReason } from '@/api';
import { created, destroy } from '@/api/client/client';

import { Sidebar } from './_components/sidebar';
import { ReadyProvider, EventContext, useSidebar } from './_hooks';

import * as styles from './page.css';

export const MainPage = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const [isReady, setIsReady] = createSignal(false);
  const [listeners, setListeners] = createSignal<((event: KiwiTalkEvent) => void)[]>([]);
  const [sidebarEvent, setSidebarEvent] = createSignal<KiwiTalkEvent | null>(null);

  const sidebar = useSidebar(isReady, sidebarEvent);

  const activeTab = () => location.pathname.match(/main\/([^/]+)/)?.[1] ?? 'chat';
  const setActiveTab = (tab: string) => {
    navigate(`/main/${tab}`, { replace: true });
  };

  const onLogout = async (reason: LogoutReason) => {
    const sessionError = reason.type === 'Kickout'
      ? 'the chat session was closed by the server; sign in again to reconnect'
      : reason.type === 'Disconnected'
        ? 'the chat transport disconnected; sign in again to reconnect'
        : 'the native chat connection failed; sign in again to retry';

    try {
      navigate('/login/list', {
        resolve: false,
        replace: true,
        state: { sessionError },
      });
    } finally {
      if (await created().catch(() => false)) {
        await destroy().catch(() => undefined);
      }
    }
  };
  const onEvent = (event: KiwiTalkEvent) => {
    setSidebarEvent(event);
    listeners().forEach((listener) => listener(event));
  };

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.altKey || event.metaKey) return;
      if (event.key === '1') {
        event.preventDefault();
        setActiveTab('chat');
      } else if (event.key === '2') {
        event.preventDefault();
        setActiveTab('friends');
      }
    };

    window.addEventListener('keydown', onKeyDown);
    onCleanup(() => window.removeEventListener('keydown', onKeyDown));
  });

  return (
    <EventContext.Provider value={{
      addEvent: (listener) => {
        setListeners([...listeners(), listener]);
      },
      removeEvent: (listener) => {
        const newList = listeners().filter((l) => l !== listener);

        setListeners(newList);
      },
    }}>
      <ReadyProvider onLogout={onLogout} onEvent={onEvent} onReadyChange={setIsReady}>
        <main class={styles.container}>
          <Sidebar
            collapsed={false}
            activePath={activeTab()}
            setActivePath={setActiveTab}
            chatBadges={sidebar.badges()?.chat}
            notificationActive={sidebar.notificationActive()}
            onNotificationActive={sidebar.setNotificationActive}
          />
          <div class={styles.workspace}>
            <Outlet />
          </div>
          <footer class={styles.statusLine}>
            <span class={styles.connection[isReady() ? 'ready' : 'pending']}>
              {isReady() ? '● online' : '○ connecting'}
            </span>
            <span>android/subdevice</span>
            <span class={styles.shortcuts}>[/] filter  [j/k] move  [enter] open  [i] compose  [ctrl+1/2] section</span>
          </footer>
        </main>
      </ReadyProvider>
    </EventContext.Provider>
  );
};
