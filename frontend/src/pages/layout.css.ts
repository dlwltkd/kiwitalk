import { vars } from '@/features/theme';
import { style } from '@vanilla-extract/css';

export const container = style({
  width: '100%',
  height: '100%',

  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'flex-start',
  alignItems: 'stretch',

  backgroundColor: vars.color.terminal.background,
});

export const viewport = style({
  position: 'relative',
  width: '100%',
  minHeight: 0,
  flex: '1 1 auto',
  overflow: 'hidden',
});
