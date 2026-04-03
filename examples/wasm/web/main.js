import init from "../pkg/wasm_demo.js";

function setupCanvas() {
  const canvas = document.getElementById("spot-canvas");
  if (!canvas) {
    console.error("[spot][js] #spot-canvas not found");
    return;
  }

  const resizeCanvas = () => {
    const dpr = globalThis.devicePixelRatio || 1;
    const cssWidth = globalThis.innerWidth || document.documentElement.clientWidth || 300;
    const cssHeight = globalThis.innerHeight || document.documentElement.clientHeight || 150;

    canvas.style.width = "100vw";
    canvas.style.height = "100vh";
    canvas.style.display = "block";

    const width = Math.max(1, Math.round(cssWidth * dpr));
    const height = Math.max(1, Math.round(cssHeight * dpr));

    if (canvas.width !== width) {
      canvas.width = width;
    }
    if (canvas.height !== height) {
      canvas.height = height;
    }
  };

  resizeCanvas();
  globalThis.addEventListener("resize", resizeCanvas);
}

async function main() {
  setupCanvas();
  try {
    const wasm = await init();
    if (wasm && typeof wasm.run_demo === "function") {
      if (!globalThis.isSecureContext || !navigator.gpu) {
        console.error(
          "[spot][js] WebGPU unavailable. Open via http://localhost:8000/examples/wasm/web/ (or https) in a WebGPU-enabled browser.",
        );
        return;
      }

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
