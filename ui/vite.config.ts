import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // Dev server proxies API calls to the WasmCloud host so the React dev server
  // can run on a different port without CORS issues during development.
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
      "/auth": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
  build: {
    // Output goes to dist/; the api-gateway actor embeds these at compile time.
    outDir: "dist",
    emptyOutDir: true,
    // Produce a single JS chunk + CSS file for simpler embedding.
    rollupOptions: {
      output: {
        manualChunks: undefined,
      },
    },
  },
});
