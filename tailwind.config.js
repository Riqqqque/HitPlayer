/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['"Segoe UI"', "Inter", "system-ui", "sans-serif"],
      },
      colors: {
        ink: {
          950: "#090b10",
          900: "#0d1118",
          850: "#111722",
          800: "#171d28",
          700: "#242c3a",
        },
        hit: {
          500: "#38bdf8",
          400: "#67e8f9",
          300: "#8ddff8",
        },
      },
      boxShadow: {
        panel: "0 20px 60px rgba(0, 0, 0, 0.32)",
      },
    },
  },
  plugins: [],
};
