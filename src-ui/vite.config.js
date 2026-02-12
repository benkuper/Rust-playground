import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import path from "node:path";

export default defineConfig({
  plugins: [sveltekit()],
  resolve: {
    alias: {
      $golden_ui: path.resolve(__dirname, "../crates/golden_ui/src/lib")
    }
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
