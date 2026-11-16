import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { checkSources } from "../scripts/source-comments.mjs";

checkSources();

export default defineConfig({
  root: import.meta.dirname,
  plugins: [vue()],
  build: {
    target: "chrome101",
    cssTarget: "chrome101",
    rollupOptions: { output: { entryFileNames: "app.js", assetFileNames: "[name][extname]", codeSplitting: false } },
  },
});
