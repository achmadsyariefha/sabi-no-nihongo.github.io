/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./crates/web/src/**/*.rs",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Soft warm off-white for light-mode cards (less glare than pure white)
        paper: '#f5f3ef',
        // Dark-mode greys from the "Dimmed interface" palette (octet.design).
        // Same keys as zinc so `dark:*-zinc-N` classes map 1:1 to `dark:*-dim-N`.
        dim: {
          50: '#F0F2F0',
          100: '#F0F2F0',
          200: '#D5D8D5',
          300: '#B0B3B0',
          400: '#999C99',
          500: '#7E817E',
          600: '#5B5B5B',
          700: '#3E3F3E',
          800: '#2C2D2C',
          900: '#1E1E1E',
          950: '#060C06',
        },
        sakura: {
          50: '#fff5f7',
          100: '#ffe8ee',
          200: '#ffd0de',
          300: '#ffa8c1',
          400: '#ff709b',
          500: '#f43f75',
          600: '#e11d5a',
          700: '#be1245',
          800: '#9e123c',
          900: '#851437',
          950: '#4a0a1f',
        },
        zen: {
          50: '#f8f8f7',
          100: '#efefec',
          200: '#e0ded7',
          300: '#cbc7bb',
          400: '#b1ab9a',
          500: '#9b927e',
          600: '#847966',
          700: '#6c6253',
          800: '#585044',
          900: '#484239',
          950: '#26221d',
        },
        matcha: {
          50: '#f3f8f2',
          100: '#e3efe2',
          200: '#c7e0c4',
          300: '#9dc898',
          400: '#6fa969',
          500: '#4e8d48',
          600: '#3c7137',
          700: '#315a2d',
          800: '#2a4827',
          900: '#243d22',
        },
        indigoPaper: {
          50: '#f0f4f9',
          100: '#dee7f2',
          200: '#c2d4e7',
          300: '#98b9d7',
          400: '#6998c3',
          500: '#497dae',
          600: '#386392',
          700: '#2e4f76',
          800: '#294363',
          900: '#263b53',
          950: '#192637',
        }
      },
      fontFamily: {
        japanese: ['"Noto Sans JP"', '"Hiragino Kaku Gothic ProN"', '"Meiryo"', 'sans-serif'],
        serifJapanese: ['"Noto Serif JP"', '"Hiragino Mincho ProN"', '"Yu Mincho"', 'serif'],
      }
    },
  },
  plugins: [],
}
