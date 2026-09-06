import { classes, vars } from '@/features/theme';
import { style } from '@vanilla-extract/css';

export const container = style([classes.typography.title, {
  color: vars.color.terminal.foregroundMuted,

  padding: '3px 0',
}]);
