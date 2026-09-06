import { createEffect } from 'solid-js';
import { Outlet, useNavigate, useParams } from '@solidjs/router';

import { ChannelList } from './_components/channel-list';
import { useChannelList } from './_hooks';

import * as styles from './page.css';

export const ChannelListPage = () => {
  const navigate = useNavigate();
  const param = useParams();
  const channelList = useChannelList();

  const activeId = () => param.channelId;
  const setActiveId = (id: string) => {
    navigate(`/main/chat/${id}`);
  };
  const clearActiveId = () => navigate('/main/chat');

  createEffect(() => {
    const id = activeId();
    if (!id || !channelList.loaded()) return;

    if (!channelList.channels().some((channel) => channel.id === id)) clearActiveId();
  });

  return (
    <div class={styles.container}>
      <div class={styles.list[activeId() ? 'channelOpen' : 'channelList']}>
        <ChannelList
          channels={channelList.channels()}
          activeId={activeId()}
          setActiveId={setActiveId}
          clearActiveId={clearActiveId}
          loading={channelList.loading()}
          loaded={channelList.loaded()}
          error={channelList.error()}
          onRetry={channelList.retry}
        />
      </div>
      <div class={styles.detail[activeId() ? 'channelOpen' : 'channelList']}>
        <Outlet />
      </div>
    </div>
  );
};
