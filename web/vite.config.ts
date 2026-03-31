import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({
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
