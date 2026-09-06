import { createResource } from 'solid-js';

import { FriendProfile, updateFriends } from '@/api';

import { useReady } from '../../_hooks';

export const useFriendList = () => {
  const isReady = useReady();

  const [friends] = createResource(isReady, async (ready) => {
    if (!ready) return [] as FriendProfile[];

    const list: FriendProfile[] = [];

    const res = await updateFriends(list.map((friend) => friend.userId));

    for (const removed of res.removedIds) {
      const index = list.findIndex((friend) => friend.userId === removed);
      list.splice(index, 1);
    }

    for (const added of res.added) {
      list.push(added);
    }

    return list;
  });

  return {
    all: (() => friends() ?? []),
    pinned: () => [], // TODO: implement
    nearBirthday: () => [], // TODO: implement
  };
};
