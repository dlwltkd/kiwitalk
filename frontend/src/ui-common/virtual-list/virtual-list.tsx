import {
  Accessor,
  batch,
  ComponentProps,
  createEffect,
  createMemo,
  createSignal,
  For,
  mergeProps,
  on,
  onCleanup,
  onMount,
  splitProps,
  untrack,
  ValidComponent,
} from 'solid-js';
import { assignInlineVars } from '@vanilla-extract/dynamic';
import { Dynamic, DynamicProps } from 'solid-js/web';

import { calculateVisibleRange } from './calculate-visible-range';
import * as styles from './virtual-list.css';

import type { JSX } from 'solid-js/jsx-runtime';
import { Range } from './types';

const DEFAULT_HEIGHT = 50;

type RequiredKey<T, K extends keyof T> = Omit<T, K> & Required<Pick<T, K>>;

export interface VirtualListRef {
  scrollToIndex: (index: number, options?: ScrollToOptions) => void;
  refresh: () => void;
  range: () => Range;
  topPadding: () => number;
  bottomPadding: () => number;

  element: HTMLElement;
  innerElement: Accessor<HTMLElement | null>;
}
export type VirtualListProps<T> = {
  ref?: (ref: VirtualListRef) => void;

  items: T[];
  children: (item: T, index: Accessor<number>) => JSX.Element;

  overscan?: number;
  itemHeight?: number | ((index: number) => number);
  estimatedItemHeight?: number;
  topMargin?: number;
  bottomMargin?: number;
  reverse?: boolean;
  alignToBottom?: boolean;

  innerStyle?: JSX.HTMLAttributes<HTMLDivElement>['style'];
  innerClass?: JSX.HTMLAttributes<HTMLDivElement>['class'];
}

export const VirtualList = <
  Item,
  T extends ValidComponent,
  P = ComponentProps<T>
>(props: DynamicProps<T, P> & VirtualListProps<Item>): JSX.Element => {
  const [local, classProps, componentProps, leftProps] = splitProps(
    mergeProps(
      {
        overscan: 5,
        component: 'div',
        class: '',
        classList: {},
        reverse: false,
        alignToBottom: props.reverse ?? false,
        estimatedItemHeight: DEFAULT_HEIGHT,
      },
      props,
    ) as DynamicProps<T, P> &
      RequiredKey<
        VirtualListProps<Item>,
        'overscan' | 'reverse' | 'estimatedItemHeight' | 'alignToBottom'
      > & {
      class: string;
      classList: Record<string, boolean>;
      onScroll?: JSX.EventHandler<T, Event>;
    },
    [
      'component',
      'items',
      'overscan',
      'itemHeight',
      'topMargin',
      'bottomMargin',
      'reverse',
      'estimatedItemHeight',
      'alignToBottom',
    ],
    [
      'class',
      'classList',
      'innerStyle',
      'innerClass',
    ],
    [
      'children',
      'onScroll',
    ],
  );

  const [alignToBottom, setAlignToBottom] = createSignal<boolean>(local.alignToBottom);
  const [topPadding, setTopPadding] = createSignal(0);
  const [bottomPadding, setBottomPadding] = createSignal(0);
  const [range, setRange] = createSignal<[number, number]>([0, 30]);
  const [items, setItems] = createSignal<Item[]>([]);

  let frameRef: HTMLElement | undefined;
  let parentRef: HTMLDivElement | undefined;

  const defaultItemHeight: number = (
    typeof local.itemHeight === 'function' ?
      local.estimatedItemHeight :
      (local.itemHeight ?? local.estimatedItemHeight)
  );
  const itemHeights = new Map<number, number>();

  const getHeight = (index: number) => {
    const defaultValue = typeof local.itemHeight === 'function' ?
      local.itemHeight(index) :
      defaultItemHeight;

    return Number(itemHeights.get(index) ?? defaultValue);
  };

  const measureRenderedItems = () => {
    if (!parentRef) return;

    const [start, end] = untrack(() => range());
    const children = Array.from(parentRef.children);

    for (let i = start; i < end; i++) {
      const child = children[i - start + 1];
      if (!child || itemHeights.has(i)) continue;

      itemHeights.set(i, child.getBoundingClientRect().height || defaultItemHeight);
    }
  };

  const calculateRange = (scroll: number, height: number, measure = true) => {
    if (measure) measureRenderedItems();

    const [start, end] = untrack(() => range());
    const [newStart, newEnd] = calculateVisibleRange(
      scroll,
      height,
      { getHeight, overscan: local.overscan, length: items().length },
    );

    let newTop = 0;
    let newBottom = 0;

    for (let i = 0; i < newStart; i++) {
      newTop += itemHeights.get(i) ?? defaultItemHeight;
    }
    for (let i = newEnd; i < items().length; i++) {
      newBottom += itemHeights.get(i) ?? defaultItemHeight;
    }

    batch(() => {
      if (start !== newStart || end !== newEnd) setRange([newStart, newEnd]);
      setTopPadding(newTop);
      setBottomPadding(newBottom);
    });
  };

  let ignoreAlignScroll = false;
  const onScroll: JSX.EventHandlerUnion<T, Event> = (event) => {
    if (!ignoreAlignScroll) setAlignToBottom(false);

    const scroll = event.target.scrollTop;
    const height = event.target.clientHeight;

    calculateRange(scroll, height);

    componentProps.onScroll?.(event);
  };

  const scrollToIndex = (index: number, options: ScrollToOptions = {}) => {
    const rawIndex = local.reverse ? items().length - index - 1 : index;

    let top = 0;

    for (let i = 0; i < rawIndex; i++) {
      top += itemHeights.get(i) ?? defaultItemHeight;
    }

    frameRef?.scrollTo({
      ...options,
      top: top + (options?.top ?? 0),
    });
  };

  let cancelAlignScroll: number | null = null;
  const tryAlignToBottom = () => {
    if (alignToBottom()) {
      if (typeof cancelAlignScroll === 'number') cancelAnimationFrame(cancelAlignScroll);
      ignoreAlignScroll = true;

      frameRef?.scrollTo({
        top: frameRef.scrollHeight,
        behavior: 'instant',
      });

      cancelAlignScroll = requestAnimationFrame(() => {
        ignoreAlignScroll = false;
      });
    }
  };
  onMount(() => {
    tryAlignToBottom();
  });

  createEffect(() => {
    if (local.reverse) setItems(local.items.toReversed());
    else setItems(local.items as Item[]);
  });

  createEffect(on(items, () => {
    if (!parentRef || !frameRef) return;

    itemHeights.clear();
    const scroll = frameRef.scrollTop;
    const height = frameRef.clientHeight;

    calculateRange(scroll, height, false);
    if (local.alignToBottom) setAlignToBottom(true);

    tryAlignToBottom();
  }));

  let resizeFrame: number | null = null;
  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const index = Number(entry.target.getAttribute('data-sorted-index'));

      if (Number.isFinite(index)) {
        itemHeights.set(index, entry.target.getBoundingClientRect().height || defaultItemHeight);
        tryAlignToBottom();
      }
    }

    if (resizeFrame === null) {
      resizeFrame = requestAnimationFrame(() => {
        resizeFrame = null;
        if (frameRef) calculateRange(frameRef.scrollTop, frameRef.clientHeight, false);
      });
    }
  });
  createEffect(() => {
    const [start, end] = range();
    items();
    if (!parentRef) return;

    const children = Array.from(parentRef.children);
    resizeObserver.disconnect();

    for (let i = start; i < end; i++) {
      const child = children[i - start + 1];
      if (!child) continue;

      const index = local.reverse ? items().length - i - 1 : i;

      child.setAttribute('data-sorted-index', i.toString());
      child.setAttribute('data-index', index.toString());
      resizeObserver.observe(child);
    }
  });

  onCleanup(() => {
    resizeObserver.disconnect();
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);
    if (cancelAlignScroll !== null) cancelAnimationFrame(cancelAlignScroll);
  });

  const outerClassList = () => {
    const list: Record<string, boolean> = {
      [styles.outer]: true,
    };

    if (classProps.class) list[classProps.class] = true;
    if (classProps.classList) Object.assign(list, classProps.classList);

    return list;
  };

  const onRegisterFrame = (element: HTMLElement) => {
    frameRef = element;

    const ref: VirtualListRef = {
      scrollToIndex,
      refresh: () => {
        if (!frameRef) return;

        const scroll = frameRef.scrollTop;
        const height = frameRef.clientHeight;

        calculateRange(scroll, height);
      },
      range: () => {
        const [start, end] = range();

        if (local.reverse) return [items().length - end, items().length - start];
        return [start, end];
      },
      topPadding,
      bottomPadding,

      element,
      innerElement: () => parentRef ?? null,
    };
    props.ref?.(ref);
  };

  return (
    <Dynamic
      {...leftProps}
      component={local.component}
      ref={onRegisterFrame}
      classList={outerClassList()}
      onScroll={onScroll}
    >
      <div
        ref={(el) => parentRef = el}
        class={`${styles.inner} ${classProps.innerClass}`}
        style={classProps.innerStyle}
      >
        <div
          class={styles.placeholer}
          style={assignInlineVars({
            [styles.gap]: `${(local.topMargin ?? 0) + topPadding() || 0}px`,
          })}
        />
        <For each={items().slice(...range())}>
          {(item, index) => componentProps.children(
            item,
            createMemo(() => {
              const arrayIndex = index() + range()[0];

              if (local.reverse) return items().length - arrayIndex - 1;
              return arrayIndex;
            }),
          )}
        </For>
        <div
          class={styles.placeholer}
          style={assignInlineVars({
            [styles.gap]: `${(local.bottomMargin ?? 0) + bottomPadding() || 0}px`,
          })}
        />
      </div>
    </Dynamic>
  );
};
