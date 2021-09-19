const colors = require('tailwindcss/colors')

module.exports = {
  mode: 'jit',
  purge: ['./src/**/*.{html,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        teal: colors.teal,
      },
      translate: {
        '85p': '85%',
      },
    },
  },
  plugins: [],
}