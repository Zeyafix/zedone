import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const proxy = { "/prototype": "http://127.0.0.1:4317" };
export default defineConfig({
  plugins: [vue()],
  server: { host: "127.0.0.1", proxy },
  preview: { host: "127.0.0.1", proxy },
});
