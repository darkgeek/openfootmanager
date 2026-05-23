/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'Avenir', 'Helvetica', 'Arial', 'sans-serif'],
        heading: ['Barlow Condensed', 'Inter', 'Arial Narrow', 'sans-serif'],
      },
      colors: {
        primary: {
          50: '#ecfdf5',
          100: '#d1fae5',
          200: '#a7f3d0',
          300: '#6ee7b7',
          400: '#34d399',
          500: '#10b981',
          600: '#059669',
          700: '#047857',
          800: '#065f46',
          900: '#064e3b',
        },
        accent: {
          50: '#fffbeb',
          100: '#fef3c7',
          200: '#fde68a',
          300: '#fcd34d',
          400: '#ffd60a',
          500: '#eab308',
          600: '#b8960a',
          700: '#a16207',
          800: '#854d0e',
          900: '#713f12',
        },
        success: {
          400: '#06d6a0',
          500: '#05a87d',
          600: '#059669',
        },
        navy: {
          100: '#7a9ad1',
          200: '#5f7fb8',
          300: '#4a6599',
          400: '#3a5080',
          500: '#2d3f6b',
          600: '#243054',
          700: '#1a2340',
          800: '#131b33',
          900: '#0a1128',
        },
      },
      minHeight: {
        '18': '4.5rem',    // 72px
        '22': '5.5rem',    // 88px
        '30': '7.5rem',    // 120px
        '36': '9rem',      // 144px
        '40': '10rem',     // 160px
        '44': '11rem',     // 176px
        '48': '12rem',     // 192px
        '52': '13rem',     // 208px
        '56': '14rem',     // 224px
        '60': '15rem',     // 240px
        '64': '16rem',     // 256px
        '72': '18rem',     // 288px
        '80': '20rem',     // 320px
        '88': '22rem',     // 352px
        '96': '24rem',     // 384px
        '100': '25rem',    // 400px
        '108': '27rem',    // 432px
        '115': '28.75rem', // 460px
        '120': '30rem',    // 480px
        '128': '32rem',    // 512px
        '130': '32.5rem',  // 520px
        '140': '35rem',    // 560px
        '150': '37.5rem',  // 600px
        '160': '40rem',    // 640px
      },
      width: {
        '18': '4.5rem',    // 72px
        '20': '5rem',      // 80px
        '20.5': '5.125rem', // 82px
        '22': '5.5rem',    // 88px
      },
    },
  },
  plugins: [],
};
