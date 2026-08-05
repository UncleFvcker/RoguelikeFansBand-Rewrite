import { defineConfig } from "vite";

export default defineConfig({
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "pixi-rendering",
              test: /node_modules[\\/]pixi\.js[\\/]lib[\\/]rendering[\\/]/,
              priority: 40,
            },
            {
              name: "pixi-support",
              test: /node_modules[\\/](?:pixi\.js|@pixi)[\\/]/,
              priority: 30,
            },
            {
              name: "fluent",
              test: /node_modules[\\/]@fluent[\\/]/,
              priority: 20,
            },
            {
              name: "tauri",
              test: /node_modules[\\/]@tauri-apps[\\/]/,
              priority: 10,
            },
          ],
        },
      },
    },
  },
});
