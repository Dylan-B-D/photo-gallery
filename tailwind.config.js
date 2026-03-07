module.exports = {
  content: ["./templates/**/*.html"],
  theme: {
    extend: {
      screens: {
        "3xl": "2000px",
      },
      colors: {
        primary: "#18181b",
        secondary: "#27272a",
        accent: "#3f3f46",
        background: "#09090b",
      },
      fontFamily: {
        sans: ["Nunito", "serif"],
        merienda: ["Cinzel", "serif"],
      },
      backdropBlur: {
        xs: '2px',
      },
      dropShadow: {
        'emerald': '0 4px 6px rgba(16, 185, 129, 0.5)',
      },
    },
  },
  plugins: [],
};
