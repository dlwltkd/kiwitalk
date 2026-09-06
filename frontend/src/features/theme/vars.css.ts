import { createTheme, style } from '@vanilla-extract/css';

const [themeClass, baseVars] = createTheme({
  color: {
    red400: '#ff5345',
    yellow400: '#e5c736',
    blue100: '#111c18',
    blue200: '#182720',
    blue400: '#509475',
    blue500: '#63b07a',

    terminal: {
      background: '#090f0d',
      surface: '#0c1512',
      surfaceRaised: '#111c18',
      surfaceHover: '#182720',
      border: '#23372b',
      borderStrong: '#53685b',
      foreground: '#c1c497',
      foregroundBright: '#f7e8b2',
      foregroundMuted: '#81b8a8',
      accent: '#509475',
      accentBright: '#2dd5b7',
      selection: '#32473b',
    },

    neutral: {
      black: '#000000',
      white: '#ffffff',
      grey800: '#c1c497',
      lightAlpha050: 'rgba(193, 196, 151, .05)',
      lightAlpha100: 'rgba(193, 196, 151, .1)',
      lightAlpha300: 'rgba(193, 196, 151, .3)',
      lightAlpha500: 'rgba(193, 196, 151, .62)',
      darkAlpha200: 'rgba(9, 15, 13, .78)',
      darkAlpha500: 'rgba(9, 15, 13, .94)',
    },
  },
  blur: {
    regular: 'none',
    large: 'none',
  },
  shadow: {
    regular: 'none',
  },
  radius: {
    extraSmall: '2px',
    small: '3px',
    regular: '4px',
    large: '6px',
    full: '999px',
  },
  layer: {
    hidden: '-100',
    base: '0',
    above: '1',
    below: '-1',
    head: '100',
    backdrop: '500',
    modal: '1000',
    tooltip: '2000',
    windowFrame: '10000',
  },
  opacity: {
    hover: '0.82',
  },
  easing: {
    background: 'ease-out 100ms',
    fill: 'ease-out 100ms',
    transform: 'ease-out 100ms',
    linear: 'linear 100ms',
  },
  font: {
    ui: '"JetBrainsMono Nerd Font", "JetBrains Mono", "Pretendard Variable", monospace',
  },
});

type Surface = {
  background: string;
  fillPrimary: string;
  fillSecondary: string;
  fillTertiary?: string;
  elevated?: string;
  attention?: string;
};

const vars = {
  ...baseVars,

  color: {
    ...baseVars.color,
    primary: {
      background: baseVars.color.terminal.accent,
      fillPrimary: baseVars.color.terminal.background,
      fillSecondary: baseVars.color.terminal.surface,
      elevated: baseVars.color.terminal.selection,
    } satisfies Surface,
    secondary: {
      background: baseVars.color.terminal.surface,
      fillPrimary: baseVars.color.terminal.foreground,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
      attention: baseVars.color.terminal.accentBright,
      elevated: baseVars.color.terminal.surfaceHover,
    } satisfies Surface,
    solidPrimary: {
      background: baseVars.color.terminal.background,
      fillPrimary: baseVars.color.terminal.foreground,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
    } satisfies Surface,
    solidSecondary: {
      background: baseVars.color.terminal.surfaceRaised,
      fillPrimary: baseVars.color.terminal.foreground,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
      attention: baseVars.color.terminal.accentBright,
    } satisfies Surface,
    glassPrimary: {
      background: baseVars.color.terminal.surface,
      fillPrimary: baseVars.color.terminal.foreground,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
      fillTertiary: baseVars.color.terminal.border,
      attention: baseVars.color.terminal.accent,
    } satisfies Surface,
    glassSecondary: {
      background: baseVars.color.terminal.background,
      fillPrimary: baseVars.color.terminal.foreground,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
      fillTertiary: baseVars.color.terminal.border,
      attention: baseVars.color.terminal.accentBright,
    } satisfies Surface,
    overlay: {
      background: baseVars.color.terminal.surfaceHover,
      fillPrimary: baseVars.color.terminal.foregroundBright,
      fillSecondary: baseVars.color.terminal.foregroundMuted,
    } satisfies Surface,
  },
};

const themeRoot = style([themeClass, {
  fontFamily: vars.font.ui,
  color: vars.color.terminal.foreground,
  colorScheme: 'dark',
}]);

export { themeRoot, vars };
