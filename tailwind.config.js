// Preserve opacity modifiers for semantic CSS colors (for example bg-accent/15).
const themeColor = (name) => ({ opacityValue }) => opacityValue === undefined
  ? `var(${name})`
  : `color-mix(in srgb, var(${name}) calc(${opacityValue} * 100%), transparent)`;

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        subtle: themeColor('--border-subtle'),
        // Minimalist theme colors
        primary: {
          DEFAULT: themeColor('--color-primary'),
          light: themeColor('--color-primary-light'),
          dark: themeColor('--color-primary-dark'),
          dim: themeColor('--color-primary-dim'),
        },
        'primary-foreground': themeColor('--text-on-primary'),
        accent: {
          DEFAULT: themeColor('--color-accent'),
          light: themeColor('--color-accent-light'),
          dark: themeColor('--color-accent-dark'),
          dim: themeColor('--color-accent-dim'),
        },
        success: {
          DEFAULT: themeColor('--color-success'),
          dim: themeColor('--color-success-dim'),
        },
        error: {
          DEFAULT: themeColor('--color-error'),
          dim: themeColor('--color-error-dim'),
        },
        warning: {
          DEFAULT: themeColor('--color-warning'),
          dim: themeColor('--color-warning-dim'),
        },
        info: {
          DEFAULT: themeColor('--color-info'),
          dim: themeColor('--color-info-dim'),
        },
        // Background colors
        bg: {
          base: themeColor('--bg-base'),
          primary: themeColor('--bg-primary'),
          secondary: themeColor('--bg-secondary'),
          tertiary: themeColor('--bg-tertiary'),
          elevated: themeColor('--bg-elevated'),
          overlay: themeColor('--bg-overlay'),
        },
        // Text colors
        text: {
          primary: themeColor('--text-primary'),
          secondary: themeColor('--text-secondary'),
          muted: themeColor('--text-muted'),
          disabled: themeColor('--text-disabled'),
          'on-primary': themeColor('--text-on-primary'),
          'on-accent': themeColor('--text-on-accent'),
          'on-success': themeColor('--text-on-success'),
          'on-error': themeColor('--text-on-error'),
        },
        // Border colors
        border: {
          primary: themeColor('--border-primary'),
          secondary: themeColor('--border-secondary'),
          accent: themeColor('--border-accent'),
          subtle: themeColor('--border-subtle'),
        },
      },
      fontFamily: {
        sans: 'var(--font-sans)',
        mono: 'var(--font-mono)',
        display: 'var(--font-display)',
      },
      fontSize: {
        'xs': 'var(--text-xs)',
        'sm': 'var(--text-sm)',
        'base': 'var(--text-base)',
        'lg': 'var(--text-lg)',
        'xl': 'var(--text-xl)',
        '2xl': 'var(--text-2xl)',
        '3xl': 'var(--text-3xl)',
      },
      spacing: {
        'xs': 'var(--spacing-xs)',
        'sm': 'var(--spacing-sm)',
        'md': 'var(--spacing-md)',
        'lg': 'var(--spacing-lg)',
        'xl': 'var(--spacing-xl)',
        '2xl': 'var(--spacing-2xl)',
        '3xl': 'var(--spacing-3xl)',
      },
      borderRadius: {
        'xs': 'var(--radius-xs)',
        'sm': 'var(--radius-sm)',
        'md': 'var(--radius-md)',
        'lg': 'var(--radius-lg)',
        'xl': 'var(--radius-xl)',
        '2xl': 'var(--radius-2xl)',
      },
      boxShadow: {
        'xs': 'var(--shadow-xs)',
        'sm': 'var(--shadow-sm)',
        'md': 'var(--shadow-md)',
        'lg': 'var(--shadow-lg)',
        'xl': 'var(--shadow-xl)',
        'inner': 'var(--shadow-inner)',
      },
      animation: {
        'fade-in': 'fade-in 0.25s ease-out',
        'scale-in': 'scale-in 0.25s ease-out',
        'slide-up': 'slide-up 0.35s ease-out',
        'slide-in-right': 'slide-in-right 0.35s ease-out',
      },
      keyframes: {
        'fade-in': {
          'from': {
            opacity: '0',
            transform: 'translateY(8px)',
          },
          'to': {
            opacity: '1',
            transform: 'translateY(0)',
          },
        },
        'scale-in': {
          'from': {
            transform: 'scale(0.98)',
            opacity: '0',
          },
          'to': {
            transform: 'scale(1)',
            opacity: '1',
          },
        },
        'slide-up': {
          'from': {
            opacity: '0',
            transform: 'translateY(16px)',
          },
          'to': {
            opacity: '1',
            transform: 'translateY(0)',
          },
        },
        'slide-in-right': {
          'from': {
            opacity: '0',
            transform: 'translateX(16px)',
          },
          'to': {
            opacity: '1',
            transform: 'translateX(0)',
          },
        },
      },
      transitionDuration: {
        'fast': 'var(--transition-fast)',
        'normal': 'var(--transition-normal)',
        'slow': 'var(--transition-slow)',
        'slower': 'var(--transition-slower)',
      },
      zIndex: {
        'dropdown': 'var(--z-dropdown)',
        'sticky': 'var(--z-sticky)',
        'fixed': 'var(--z-fixed)',
        'modal-backdrop': 'var(--z-modal-backdrop)',
        'modal': 'var(--z-modal)',
        'popover': 'var(--z-popover)',
        'tooltip': 'var(--z-tooltip)',
      },
    },
  },
  plugins: [],
}
