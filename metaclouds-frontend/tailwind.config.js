/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    // 与 src/theme/breakpoints.ts 同步（R1 单一真相源）
    screens: {
      xs: '0px',
      sm: '576px',
      md: '768px',
      lg: '1024px',
      xl: '1280px',
      '2xl': '1440px',
      '3xl': '1920px',
      '4xl': '2560px',
    },
    extend: {
      colors: {
        brand: {
          DEFAULT: '#2f6bff',
          600: '#1f52e0',
          400: '#5b8bff',
          100: '#dce8ff',
          50: '#eef4ff',
        },
      },
    },
  },
  // 关闭 preflight，避免重置 antd 5（CSS-in-JS）的基础样式导致组件走样。
  // 仅启用 components / utilities，使 PrivateRoute 等处的 Tailwind 类名真正生效（R10）。
  corePlugins: {
    preflight: false,
  },
  plugins: [],
};
