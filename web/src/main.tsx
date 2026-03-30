/** @jsxImportSource @b9g/crank */

import "./style.css";
import { renderer } from "@b9g/crank/dom";
import type { Context } from "@b9g/crank";
import { processFormatted } from "./loadCore.js";
import { initialState, type State } from "./State.js";
import { OptionsPanel } from "./OptionsPanel.js";
import { InputPanel } from "./InputPanel.js";
import { OutputPanel } from "./OutputPanel.js";

function* App(this: Context) {
  const state = initialState;

  const setState = (updates: Partial<State>) =>
    this.refresh(() => {
      Object.assign(state, updates);
    });

  const processText = async () => {
    setState({ processing: true, error: "", output: "" });
    try {
      const result = await processFormatted(
        state.input,
        {
          algorithm: state.algorithm,
          threshold: state.params["threshold"] ?? 0.8,
          ngramSize: Math.round(state.params["ngram_size"] ?? 2),
          outlierThreshold: state.params["outlier_threshold"] ?? 0.0,
          budgetLines: state.budget ?? undefined,
        },
        state.outputFormat,
      );
      setState({ output: result, processing: false });
    } catch (e: unknown) {
      setState({ error: (e as Error).toString(), processing: false });
    }
  };

  for ({} of this) {
    yield (
      <div class="h-screen w-screen bg-white overflow-hidden flex flex-col">
        <div class="bg-gray-900 text-white px-6 py-4">
          <h1 class="text-2xl font-black tracking-tighter">txtfold</h1>
          <p class="text-sm text-gray-400">
            Deterministic text compression for log analysis
          </p>
        </div>

        <div class="flex-1 grid grid-cols-[280px_1fr_1fr] overflow-hidden">
          <OptionsPanel state={state} setState={setState} />
          <InputPanel
            state={state}
            setState={setState}
            onProcess={processText}
          />
          <OutputPanel state={state} />
        </div>
      </div>
    );
  }
}

(async () => {
  await renderer.render(<App />, document.body);
})();
