import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateColumns: 'minmax(260px, 32ch) minmax(0, 1fr)',
  background: vars.color.terminal.background,
});

export const list = style({
  minWidth: 0,
  minHeight: 0,
  borderRight: `1px solid ${vars.color.terminal.border}`,
});
