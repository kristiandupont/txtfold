// eslint-disable-next-line @typescript-eslint/no-explicit-any
export let wasmModule: any = null;

export async function loadCore() {
  if (!wasmModule) {
    try {
      wasmModule = await import("./wasm/txtfold.js");
      await wasmModule.default();
    } catch (e) {
      console.error("Failed to load WASM:", e);
      throw e;
    }
  }
  return wasmModule;
} // WASM module will be loaded dynamically
