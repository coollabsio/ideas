/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: ['./src/**/*.{astro,html,js,jsx,ts,tsx,md,mdx}'],
  theme: {
    extend: {
      colors: {
        // Coolify brand
        coollabs: {
          DEFAULT: '#6b16ed',
          50: '#f5f0ff',
          100: '#7317ff',
          200: '#5a12c7',
          300: '#4a0fa3',
        },
        // Dark-mode accent
        warning: {
          DEFAULT: '#fcd452',
          50: '#fefce8',
          100: '#fef9c3',
          200: '#fef08a',
          300: '#fde047',
          400: '#fcd452',
          500: '#facc15',
          600: '#ca8a04',
          700: '#a16207',
          800: '#854d0e',
          900: '#713f12',
        },
        // Dark surfaces
        base: '#101010',
        coolgray: {
          100: '#181818',
          200: '#202020',
          300: '#242424',
          400: '#282828',
          500: '#323232',
        },
        success: '#22C55E',
        error: '#dc2626',
      },
      fontFamily: {
        sans: ['"Geist Sans"', 'Inter', 'sans-serif'],
        mono: ['"Geist Mono"', 'SFMono-Regular', 'Consolas', 'Liberation Mono', 'Menlo', 'monospace'],
      },
      borderRadius: {
        sm: '0.125rem',
        DEFAULT: '0.25rem',
        lg: '0.5rem',
      },
    },
  },
  plugins: [],
};
