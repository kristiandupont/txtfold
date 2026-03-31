import type { Context } from "@b9g/crank";
import { InputPanel } from "./InputPanel";
import { processFormatted } from "./loadCore.js";
import { OptionsPanel } from "./OptionsPanel";
import { OutputPanel } from "./OutputPanel";
import { initialState, type State } from "./State";

function PlayIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      class="size-6"
    >
      <path d="M8 5V19L19 12L8 5Z" fill="currentColor" />
    </svg>
  );
}

function ProcessingIcon() {
  return <div class="animate-pulse">...</div>;
}

export function* App(this: Context) {
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
      <div class="min-h-[80vh] flex flex-col">
        <div class="flex-1 flex flex-col sm:flex-row gap-4">
          <OptionsPanel state={state} setState={setState} />
          <div class="flex flex-col sm:flex-row gap-4 w-full relative">
            <InputPanel state={state} setState={setState} />
            <OutputPanel state={state} />
            <button
              class="absolute top-1/2 left-1/2 -ml-8 -mt-8 rounded-full size-16 flex items-center justify-center border-2 bg-gray-50 border-gray-500 shadow-lg"
              disabled={state.processing || !state.input.trim()}
              onclick={processText}
            >
              {state.processing ? <ProcessingIcon /> : <PlayIcon />}
            </button>
          </div>
        </div>
      </div>
    );
  }
}
