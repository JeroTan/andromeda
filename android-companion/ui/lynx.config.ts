import { defineConfig } from "@lynx-js/rspeedy";
import { pluginLynxConfig } from "@lynx-js/config-rsbuild-plugin";
import { pluginReactLynx } from "@lynx-js/react-rsbuild-plugin";
export default defineConfig({
  source: { entry: { main: "./src/index.tsx" } },
  plugins: [
    pluginLynxConfig({ enableA11y: true, enableAccessibilityElement: false }),
    pluginReactLynx({ engineVersion: "3.9" }),
  ],
  output: { filename: { bundle: "main.lynx.bundle" }, sourceMap: false },
  tools: { rspack: { optimization: { splitChunks: false } } },
});
