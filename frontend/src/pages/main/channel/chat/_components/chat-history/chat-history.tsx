import { For, JSX, Show, createMemo, createResource, createSignal, onCleanup } from 'solid-js';

import { ChatArchive, importChatArchive, loadChatArchive } from '@/api/client';
import { VirtualList } from '@/ui-common/virtual-list';

import * as styles from './chat-history.css';
import * as listStyles from '../message-list/message-list.css';

type ChatHistoryProps = {
  channelId: string;
  ready: boolean;
  logonId?: string;
  onViewChange: (saved: boolean) => void;
  children: JSX.Element;
};

const SavedChat = (props: { archive: ChatArchive; logonId?: string }) => {
  const rows = createMemo(() => props.archive.entries.map((entry, index, entries) => ({
    ...entry,
    showDate: index === 0 || entry.date !== entries[index - 1].date,
  })).toReversed());
  let scroller: HTMLElement | undefined;

  return (
    <section class={styles.saved} aria-label="Saved chat">
      <div class={styles.source}>
        <span>{props.archive.sourceName} · {props.archive.messageCount} messages</span>
        <span>Saved {props.archive.savedAt} · Times as written in the file · Read-only</span>
        <div class={styles.jumpButtons}>
          <button type="button" class={listStyles.historyButton} onClick={() => scroller?.scrollTo({ top: 0 })}>First message</button>
          <button type="button" class={listStyles.historyButton} onClick={() => scroller?.scrollTo({ top: scroller.scrollHeight })}>Latest saved message</button>
        </div>
      </div>
      <VirtualList
        reverse
        component="ol"
        aria-label="Messages from saved file"
        tabIndex={0}
        ref={(ref) => { scroller = ref.element; }}
        items={rows()}
        class={listStyles.virtualList.outer}
        innerClass={listStyles.virtualList.inner}
        estimatedItemHeight={70}
        topMargin={8}
        bottomMargin={8}
      >
        {(entry) => (
          <li class={styles.entry}>
            <Show when={entry.showDate}>
              <div class={styles.date}>{entry.date}</div>
            </Show>
            <Show when={entry.sender} fallback={
              <div class={styles.event}>{entry.content} <span>(time not included in export)</span></div>
            }>
              <div class={styles.sender}>
                <time>{entry.time}</time>
                <span>{entry.sender}</span>
                <Show when={entry.senderId !== null && entry.senderId === props.logonId}><span>you</span></Show>
              </div>
              <div class={styles.content}>{entry.content}</div>
            </Show>
          </li>
        )}
      </VirtualList>
    </section>
  );
};

export const ChatHistory = (props: ChatHistoryProps) => {
  const [archive, { mutate }] = createResource(
    () => props.ready ? props.channelId : null,
    loadChatArchive,
  );
  const [saved, setSaved] = createSignal(false);
  const [importing, setImporting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let input: HTMLInputElement | undefined;
  let disposed = false;
  onCleanup(() => { disposed = true; });

  const chooseView = (value: boolean) => {
    setSaved(value);
    props.onViewChange(value);
  };

  const importFile: JSX.EventHandler<HTMLInputElement, Event> = async (event) => {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = '';
    if (!file || importing()) return;
    if (file.size > 8 * 1024 * 1024) {
      setError('Choose a chat export smaller than 8 MiB.');
      return;
    }
    const id = props.channelId;
    setError(null);
    setImporting(true);
    try {
      const bytes = await file.arrayBuffer();
      const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
      const imported = await importChatArchive(id, file.name, text, -new Date().getTimezoneOffset());
      if (disposed || props.channelId !== id) return;
      mutate(imported);
      chooseView(true);
    } catch (cause) {
      if (disposed || props.channelId !== id) return;
      setError(typeof cause === 'string' ? cause : 'Could not import this file. Choose a UTF-8 KakaoTalk desktop export.');
    } finally {
      if (!disposed && props.channelId === id) setImporting(false);
    }
  };

  return (
    <div class={styles.container}>
      <div class={styles.toolbar}>
        <button type="button" class={listStyles.historyButton} aria-pressed={!saved()} onClick={() => chooseView(false)}>Live messages</button>
        <Show when={!archive.error && archive()}>
          <button type="button" class={listStyles.historyButton} aria-pressed={saved()} onClick={() => chooseView(true)}>Saved chat ({archive()!.messageCount})</button>
        </Show>
        <button type="button" class={listStyles.historyButton} disabled={!props.ready || importing() || archive.loading} onClick={() => input?.click()}>
          {importing() ? 'Importing…' : (!archive.error && archive()) ? 'Replace saved chat…' : 'Import chat .txt…'}
        </button>
        <input ref={input} hidden type="file" accept=".txt,text/plain" onChange={importFile} />
        <Show when={error() || archive.error}>
          <span role="alert">{error() ?? 'Saved chat could not be loaded.'}</span>
        </Show>
      </div>
      <Show when={saved() && !archive.error && archive()} fallback={props.children} keyed>
        {(data) => <SavedChat archive={data} logonId={props.logonId} />}
      </Show>
    </div>
  );
};
