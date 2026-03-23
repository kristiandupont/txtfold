import type { Context } from "@b9g/crank";
import type { State } from "./State";

export function* InputPanel(
  this: Context,
  {
    state,
    setState,
    onProcess,
  }: {
    state: State;
    setState: (u: Partial<State>) => void;
    onProcess: () => void;
  },
) {
  for ({ state, onProcess, setState } of this) {
    yield (
      <div class="flex flex-col h-full overflow-y-auto">
        <div class="flex items-center justify-between p-4 border-b border-gray-200">
          <h2 class="font-bold text-lg">Input</h2>
          <button
            class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
            disabled={state.processing || !state.input.trim()}
            onclick={onProcess}
          >
            {state.processing ? "Processing…" : "Process"}
          </button>
        </div>
        <textarea
          class="flex-1 p-4 font-mono text-sm resize-none focus:outline-none"
          placeholder="Paste your log entries or JSON data here…"
          value={state.input}
          oninput={(e: Event) =>
            setState({ input: (e.target as HTMLTextAreaElement).value })
          }
        />
      </div>
    );
  }
}
