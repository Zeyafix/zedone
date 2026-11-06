import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  root: import.meta.dirname,
  plugins: [vue()],
  build: {
    // The prepared Android 13 AOSP image ships WebView 101. Keep both JS
    // and CSS compatible; newer media-range syntax would hide mobile rules.
    target: "chrome101",
    cssTarget: "chrome101",
    // Stable names let the small Rust host embed the exact built UI.
    rollupOptions: { output: { entryFileNames: "app.js", assetFileNames: "[name][extname]", codeSplitting: false } },
  },
});
