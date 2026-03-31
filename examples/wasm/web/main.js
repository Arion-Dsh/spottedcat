import init from "../pkg/wasm_demo.js";

async function main() {
  console.log("[spot][js] main() start");
  console.log("[spot][js] isSecureContext=", globalThis.isSecureContext);
  console.log("[spot][js] navigator.gpu=", navigator.gpu);
  try {
    const wasm = await init();
    console.log("[spot][js] wasm init() resolved");
    if (wasm && typeof wasm.run_demo === "function") {
      if (!globalThis.isSecureContext || !navigator.gpu) {
        console.error(
          "[spot][js] WebGPU unavailable. Open via http://localhost:8000/examples/wasm/web/ (or https) in a WebGPU-enabled browser.",
        );
        return;
      }

      console.log("[spot][js] calling wasm.run_demo()");
      try {
        wasm.run_demo();
      } catch (e) {
        const msg = String(e && e.message ? e.message : e);
        if (msg.includes("Using exceptions for control flow")) {
          return;
        }
        throw e;
      }
    } else {
      console.error("[spot][js] wasm.run_demo() not found", wasm);
    }
  } catch (e) {
    console.error("[spot][js] wasm init() failed", e);
  }
}

main();
