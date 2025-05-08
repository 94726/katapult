import { defineConfig } from "vite";
import { dirname, resolve } from "node:path";
import solid from "vite-plugin-solid";
import { fileURLToPath } from "node:url";
import UnoCSS from "unocss/vite";
import Icons from "unplugin-icons/vite";
import { compression } from "vite-plugin-compression2";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export default defineConfig({
  plugins: [
    solid(),
    UnoCSS(),
    Icons({ compiler: "solid" }),
    compression({
      // deleteOriginalAssets: true,
    }),
  ],
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
  server: {
    proxy: {
      "/api": {
        // the mdns domain is much much slower for some reason
        // target: 'http://katapult.local',
        target: "http://10.42.0.1",
      },
      "/api/ws": {
        target: "ws://10.42.0.1",
        ws: true,
        rewriteWsOrigin: true,
      },
    },
  },
});
