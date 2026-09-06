import { Accessor, createEffect, createSignal } from 'solid-js';

import { Channel, loadChannel } from '@/api/client';
import { useReady } from '@/pages/main/_hooks';

const channelLoaders = new Map<string, Promise<Channel>>();

const loadChannelOnce = (id: string) => {
  const cached = channelLoaders.get(id);
  if (cached) return cached;

  const request = loadChannel(id);
  channelLoaders.set(id, request);
  const clear = () => {
    if (channelLoaders.get(id) === request) channelLoaders.delete(id);
  };
  void request.then(clear, clear);

  return request;
};

export const useChannel = (id: Accessor<string | null>) => {
  const isReady = useReady();
  const [channel, setChannel] = createSignal<Channel | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let generation = 0;

  createEffect(() => {
    const ready = isReady();
    const channelId = id();
    const requestGeneration = ++generation;

    setChannel(null);
    setError(null);
    if (!ready || typeof channelId !== 'string') {
      setLoading(false);
      return;
    }

    setLoading(true);
    void loadChannelOnce(channelId)
      .then((result) => {
        if (requestGeneration === generation) setChannel(result);
      })
      .catch(() => {
        if (requestGeneration === generation) {
          setError('channel metadata could not be loaded');
        }
      })
      .finally(() => {
        if (requestGeneration === generation) setLoading(false);
      });
  });

  return { channel, loading, error };
};
