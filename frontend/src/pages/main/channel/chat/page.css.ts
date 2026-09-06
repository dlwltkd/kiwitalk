import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  position: 'relative',
  width: '100%',
  height: '100%',
  minWidth: 0,
  minHeight: 0,
  display: 'grid',
  gridTemplateRows: 'auto minmax(0, 1fr) auto',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
});

export const channelError = style({
  position: 'absolute',
  top: '44px',
  right: 0,
  left: 0,
  zIndex: vars.layer.head,
  padding: '5px 10px',
  color: vars.color.red400,
  background: vars.color.terminal.surface,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
});

export const commandError = style({
  position: 'absolute',
  right: 0,
  bottom: '46px',
  left: 0,
  zIndex: vars.layer.head,
  padding: '4px 10px',
  color: vars.color.red400,
  background: vars.color.terminal.surface,
  borderTop: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
});
