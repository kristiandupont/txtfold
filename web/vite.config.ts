import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({
  base: process.env.VITE_BASE ?? "/",
  plugins: [tailwindcss()],
  optimizeDeps: {
    exclude: ["txtfold"],
  },
  server: {
    fs: {
      allow: [".."],
    },
  },
});
