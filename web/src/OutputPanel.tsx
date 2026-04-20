import type { Context } from "@b9g/crank";
import { Markdown } from "./markdown/Markdown";
import type { State } from "./State";

export function* OutputPanel(this: Context, { state }: { state: State }) {
  let rendered = true;

  const toggleRendered = () =>
    this.refresh(() => {
      rendered = !rendered;
    });

  for ({ state } of this) {
    const isMarkdown = state.outputFormat === "markdown";
    yield (
      <div class="relative flex flex-1 flex-col w-1/2 h-full min-h-[79vh] max-h-[80vh] ">
        <div class="p-4 flex items-center justify-between absolute top-0 right-0">
          {isMarkdown && state.output && (
            <div class="flex text-xs rounded-full border border-gray-300 overflow-hidden">
              <button
                class={`px-3 py-1 ${rendered ? "bg-gray-900 text-white" : "bg-white text-gray-500 hover:bg-gray-50"}`}
                onclick={rendered ? undefined : toggleRendered}
              >
                Rendered
              </button>
              <button
                class={`px-3 py-1 ${!rendered ? "bg-gray-900 text-white" : "bg-white text-gray-500 hover:bg-gray-50"}`}
                onclick={!rendered ? undefined : toggleRendered}
              >
                Raw
              </button>
            </div>
          )}
        </div>
        <div class="flex-1 p-4 overflow-auto border-gray-200 border rounded">
          {state.error ? (
            <div class="p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
              <strong>Error:</strong> {state.error}
            </div>
          ) : state.output ? (
            isMarkdown && rendered ? (
              <Markdown content={state.output} />
            ) : (
              <pre class="font-mono text-sm whitespace-pre-wrap">
                {state.output}
              </pre>
            )
          ) : (
            <div class="text-gray-400 italic">Output will appear here…</div>
          )}
        </div>
      </div>
    );
  }
}
