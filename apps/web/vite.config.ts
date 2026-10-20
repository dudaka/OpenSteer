import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// `npm run dev` proxies the WebSocket to a steerd running on 127.0.0.1:7400.
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: { "/ws": { target: "ws://127.0.0.1:7400", ws: true } },
  },
});
