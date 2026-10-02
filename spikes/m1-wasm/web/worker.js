// Dedicated Worker: OPFS sahpool needs createSyncAccessHandle(), which is Worker-only.
import init, { init as forgeInit, dispatch } from './pkg/forge_m1_spike.js';

let ready;

self.onmessage = async (event) => {
  const { id, op, args } = event.data;
  try {
    let result;
    if (op === 'init') {
      const t0 = performance.now();
      await init(); // instantiate the wasm module
      const msWasmInstantiate = performance.now() - t0;
      const info = JSON.parse(await forgeInit());
      info.ms_wasm_instantiate = msWasmInstantiate;
      ready = true;
      result = info;
    } else if (op === 'dispatch') {
      if (!ready) throw new Error('not initialised');
      result = JSON.parse(await dispatch(args.method, args.path, args.body ?? ''));
    } else {
      throw new Error('unknown op ' + op);
    }
    self.postMessage({ id, ok: true, result });
  } catch (e) {
    self.postMessage({ id, ok: false, error: String(e && e.stack ? e.stack : e) });
  }
};
