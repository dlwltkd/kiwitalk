import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
} from 'solid-js';

import { ChannelItem } from '../channel-item';
import type { ChannelListItem } from '@/pages/main/channel/_types';

import * as styles from './channel-list.css';

export type ChannelListProps = {
  activeId?: string;
  setActiveId?: (id: string) => void;
  clearActiveId?: () => void;
  channels: ChannelListItem[];
  loading?: boolean;
  loaded?: boolean;
  error?: string | null;
  onRetry?: () => void;
};

const isEditableTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) return false;

  return target.isContentEditable ||
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.tagName === 'SELECT';
};

export const ChannelList = (props: ChannelListProps) => {
  const [query, setQuery] = createSignal('');
  const [cursorId, setCursorId] = createSignal<string | undefined>(props.activeId);
  let searchInput!: HTMLInputElement;

  const channels = createMemo(() => {
    const needle = query().trim().toLocaleLowerCase();

    return [...props.channels]
      .sort(
        (a, b) =>
          (b.lastChat?.timestamp?.getTime() ?? 0) -
          (a.lastChat?.timestamp?.getTime() ?? 0),
      )
      .filter((channel) => {
        if (!needle) return true;

        return channel.name.toLocaleLowerCase().includes(needle) ||
          channel.lastChat?.content?.toLocaleLowerCase().includes(needle);
      });
  });

  const currentCursorId = () => cursorId() ?? props.activeId ?? channels()[0]?.id;

  const moveCursor = (offset: number) => {
    const items = channels();
    if (items.length === 0) return;

    const currentIndex = items.findIndex((channel) => channel.id === currentCursorId());
    const nextIndex = currentIndex < 0
      ? 0
      : (currentIndex + offset + items.length) % items.length;
    const nextId = items[nextIndex].id;
    setCursorId(nextId);

    requestAnimationFrame(() => {
      document.getElementById(`channel-${nextId}`)?.focus();
    });
  };

  const openCursor = () => {
    const id = currentCursorId();
    if (id) props.setActiveId?.(id);
  };

  createEffect(() => {
    if (props.activeId) setCursorId(props.activeId);
  });

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.isComposing || event.keyCode === 229) return;

      if ((event.key === '/' || (event.ctrlKey && event.key.toLowerCase() === 'k')) &&
        !isEditableTarget(event.target)) {
        event.preventDefault();
        searchInput.focus();
        searchInput.select();
        return;
      }

      if (event.key === 'Escape') {
        if (document.activeElement === searchInput) {
          if (query()) setQuery('');
          else searchInput.blur();
        } else if (!isEditableTarget(event.target) && props.activeId) {
          props.clearActiveId?.();
        }
        return;
      }

      if (isEditableTarget(event.target) || event.ctrlKey || event.altKey || event.metaKey) return;

      if (event.key === 'j' || event.key === 'ArrowDown') {
        event.preventDefault();
        moveCursor(1);
      } else if (event.key === 'k' || event.key === 'ArrowUp') {
        event.preventDefault();
        moveCursor(-1);
      } else if (event.key === 'Enter') {
        event.preventDefault();
        openCursor();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    onCleanup(() => window.removeEventListener('keydown', onKeyDown));
  });

  return (
    <section class={styles.container} aria-label="Channels">
      <header class={styles.header}>
        <div class={styles.pathLine}>
          <span class={styles.prompt}>~</span>
          <span>/channels</span>
          <span class={styles.count}>[{channels().length}]</span>
        </div>
        <label class={styles.search}>
          <span class={styles.searchPrompt}>/</span>
          <input
            ref={searchInput}
            aria-label="Filter channels"
            class={styles.searchInput}
            placeholder="filter channels"
            spellcheck={false}
            value={query()}
            onInput={(event) => {
              setQuery(event.currentTarget.value);
              setCursorId(undefined);
            }}
          />
          <Show when={query()}>
            <button
              class={styles.searchClear}
              type="button"
              aria-label="Clear filter"
              onClick={() => setQuery('')}
            >
              esc
            </button>
          </Show>
        </label>
      </header>

      <div class={styles.columns} aria-hidden="true">
        <span>id</span>
        <span>channel / last activity</span>
        <span>time</span>
      </div>

      <div class={styles.scrollArea}>
        <Show when={props.error}>
          <div class={styles.error} role="alert">
            <span>! {props.error}</span>
            <button class={styles.retry} type="button" onClick={props.onRetry}>
              [retry]
            </button>
          </div>
        </Show>
        <Show when={props.loading && channels().length === 0}>
          <div class={styles.empty}>… loading channels</div>
        </Show>
        <Show
          when={channels().length > 0}
          fallback={(
            <Show when={props.loaded && !props.loading && !props.error}>
              <div class={styles.empty}>no channels match `{query()}`</div>
            </Show>
          )}
        >
          <ul class={styles.list} role="listbox" aria-label="Channel results">
            <For each={channels()}>
              {(channel, index) => (
                <ChannelItem
                  id={channel.id}
                  index={index() + 1}
                  name={channel.name}
                  members={channel.userCount}
                  lastMessage={channel.lastChat?.content}
                  lastMessageTime={channel.lastChat?.timestamp}
                  unreadBadge={channel.unreadCount}
                  silent={channel.silent}
                  selected={channel.id === props.activeId}
                  cursor={channel.id === currentCursorId()}
                  onFocus={() => setCursorId(channel.id)}
                  onClick={() => {
                    setCursorId(channel.id);
                    props.setActiveId?.(channel.id);
                  }}
                />
              )}
            </For>
          </ul>
        </Show>
      </div>
    </section>
  );
};
