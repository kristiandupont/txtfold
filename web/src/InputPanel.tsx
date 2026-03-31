import type { Context } from "@b9g/crank";
import type { State } from "./State";

export function* InputPanel(
  this: Context,
  {
    state,
    setState,
  }: {
    state: State;
    setState: (u: Partial<State>) => void;
  },
) {
  for ({ state, setState } of this) {
    yield (
      <div class="flex flex-1 flex-col h-full overflow-y-auto">
        <textarea
          class="flex-1 p-4 font-mono text-xs rounded-sm bg-black text-green-200 resize-none focus:outline-none"
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
