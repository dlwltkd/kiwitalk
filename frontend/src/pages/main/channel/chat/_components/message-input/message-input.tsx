import { createSignal, onCleanup, onMount } from 'solid-js';

import * as styles from './message-input.css';

export type MessageInputProps = {
  placeholder?: string;
  disabled?: boolean;
  onSubmit?: (message: string) => void | Promise<void>;
};

export const MessageInput = (props: MessageInputProps) => {
  const [sending, setSending] = createSignal(false);
  let textarea!: HTMLTextAreaElement;

  const resize = () => {
    textarea.style.height = '0';
    textarea.style.height = `${Math.min(textarea.scrollHeight, 104)}px`;
  };

  const submit = async (event: Event) => {
    event.preventDefault();
    const value = textarea.value.trim();
    if (!value || sending() || props.disabled) return;

    setSending(true);
    try {
      await props.onSubmit?.(value);
      textarea.value = '';
      resize();
    } catch {
      textarea.focus();
    } finally {
      setSending(false);
    }
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.isComposing || event.keyCode === 229) return;

    if (event.key === 'Escape') {
      textarea.blur();
      return;
    }

    if (!event.shiftKey && event.key === 'Enter') void submit(event);
  };

  onMount(() => {
    const focusComposer = (event: KeyboardEvent) => {
      if (event.isComposing || event.keyCode === 229) return;
      const target = event.target;
      const editable = target instanceof HTMLElement && (
        target.isContentEditable || target.tagName === 'INPUT' || target.tagName === 'TEXTAREA'
      );

      if (!editable && event.key.toLocaleLowerCase() === 'i') {
        event.preventDefault();
        textarea.focus();
      }
    };

    window.addEventListener('keydown', focusComposer);
    onCleanup(() => window.removeEventListener('keydown', focusComposer));
  });

  return (
    <form class={styles.container} onSubmit={submit}>
      <span class={styles.prompt}>❯</span>
      <textarea
        ref={textarea}
        aria-label={props.placeholder ?? 'Message'}
        class={styles.input}
        disabled={props.disabled || sending()}
        placeholder={props.placeholder ?? 'write a message'}
        rows={1}
        onInput={resize}
        onKeyDown={onKeyDown}
      />
      <button class={styles.button} disabled={props.disabled || sending()} type="submit">
        {sending() ? '[...]' : '[send ↵]'}
      </button>
    </form>
  );
};
