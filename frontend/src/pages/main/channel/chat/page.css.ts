import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minWidth: 0,
  minHeight: 0,
  display: 'grid',
  gridTemplateRows: 'auto auto minmax(0, 1fr) auto auto',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
});

export const channelError = style({
  padding: '5px 10px',
  color: vars.color.red400,
  background: vars.color.terminal.surface,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
});

export const commandError = style({
  padding: '4px 10px',
  color: vars.color.red400,
  background: vars.color.terminal.surface,
  borderTop: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
});
