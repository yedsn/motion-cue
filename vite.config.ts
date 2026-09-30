import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  root: "src-ui",
  clearScreen: false,
  server: { port: 5191, strictPort: true },
  build: { outDir: "../dist", emptyOutDir: true },
  test: { environment: "jsdom", include: ["src/**/*.test.ts"] },
});

