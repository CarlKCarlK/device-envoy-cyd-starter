import init, { start } from "../pkg/device_envoy_cyd_starter_wasm.js";
import { mountCydSimulator } from "./cyd-simulator.js";

await mountCydSimulator({
  wasm: { init, start },
  app: {
    orientation: "landscape",
    // A physical touchscreen samples continuously while held. A few synthetic
    // browser samples preserve the same pressed-button feedback behavior.
    touchDownSamples: 9,
    // This starter has no demo gallery; its links point at this repository.
    galleryUrl: null,
    repoUrl: "https://github.com/CarlKCarlK/device-envoy-cyd-starter",
  },
});

