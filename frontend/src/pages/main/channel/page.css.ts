import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateColumns: 'minmax(260px, 32ch) minmax(0, 1fr)',
  background: vars.color.terminal.background,

  '@media': {
    'screen and (max-width: 720px)': {
      gridTemplateColumns: 'minmax(0, 1fr)',
    },
  },
});

const listBase = style({
  minWidth: 0,
  minHeight: 0,
  borderRight: `1px solid ${vars.color.terminal.border}`,
});

export const list = styleVariants({
  channelList: [listBase],
  channelOpen: [listBase, {
    '@media': {
      'screen and (max-width: 720px)': {
        display: 'none',
      },
    },
  }],
});

const detailBase = style({
  minWidth: 0,
  minHeight: 0,
  overflow: 'hidden',
});

export const detail = styleVariants({
  channelOpen: [detailBase],
  channelList: [detailBase, {
    '@media': {
      'screen and (max-width: 720px)': {
        display: 'none',
      },
    },
  }],
});
