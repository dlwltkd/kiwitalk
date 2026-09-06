import { classes, vars } from '@/features/theme';
import { style } from '@vanilla-extract/css';

export const error = style([classes.typography.body, {
  color: vars.color.red400,

  display: 'flex',
  flexDirection: 'row',
  justifyContent: 'flex-start',
  alignItems: 'center',
  gap: '8px',

  alignSelf: 'flex-start',
  padding: '5px 7px',
  border: `1px solid ${vars.color.red400}`,
  background: vars.color.terminal.background,
}]);

export const errorIcon = style([classes.typography.body, {
  width: '16px',
  height: '16px',

  display: 'flex',
  justifyContent: 'center',
  alignItems: 'center',

  color: vars.color.red400,
}]);
