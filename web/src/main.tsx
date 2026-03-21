/** @jsxImportSource @b9g/crank */

import "./style.css";
import { renderer } from "@b9g/crank/dom";
import type { Context } from "@b9g/crank";

// WASM module will be loaded dynamically
let wasmModule: any = null;

interface State {
  algorithm: string;
  threshold: number;
  ngramSize: number;
  outlierThreshold: number;
  format: string;
  input: string;
  output: string;
  error: string;
  processing: boolean;
}

async function loadWasm() {
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
}

function* OptionsPanel(
  this: Context,
  {
    state,
    setState,
  }: { state: State; setState: (updates: Partial<State>) => void },
) {
  for ({ state, setState } of this) {
    yield (
      <div class="flex flex-col gap-4 p-4 bg-gray-50 border-r border-gray-200">
        <h2 class="font-bold text-lg">Options</h2>

        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">Algorithm</label>
          <select
            class="px-3 py-2 border border-gray-300 rounded-md"
            value={state.algorithm}
            onchange={(e: Event) =>
              setState({ algorithm: (e.target as HTMLSelectElement).value })
            }
          >
            <option value="auto">Auto</option>
            <option value="template">Template</option>
            <option value="clustering">Clustering</option>
            <option value="ngram">N-gram</option>
            <option value="schema">Schema</option>
          </select>
        </div>

        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">Threshold</label>
          <input
            type="number"
            min="0"
            max="1"
            step="0.1"
            class="px-3 py-2 border border-gray-300 rounded-md"
            value={state.threshold}
            oninput={(e: Event) =>
              setState({
                threshold: parseFloat((e.target as HTMLInputElement).value),
              })
            }
          />
          <span class="text-xs text-gray-500">
            0.0 - 1.0 (for clustering/schema)
          </span>
        </div>

        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">N-gram Size</label>
          <input
            type="number"
            min="1"
            max="10"
            class="px-3 py-2 border border-gray-300 rounded-md"
            value={state.ngramSize}
            oninput={(e: Event) =>
              setState({
                ngramSize: parseInt((e.target as HTMLInputElement).value),
              })
            }
          />
          <span class="text-xs text-gray-500">For n-gram algorithm</span>
        </div>

        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">Outlier Threshold</label>
          <input
            type="number"
            min="0"
            step="0.1"
            class="px-3 py-2 border border-gray-300 rounded-md"
            value={state.outlierThreshold}
            oninput={(e: Event) =>
              setState({
                outlierThreshold: parseFloat(
                  (e.target as HTMLInputElement).value,
                ),
              })
            }
          />
          <span class="text-xs text-gray-500">0 for auto-detection</span>
        </div>

        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">Output Format</label>
          <select
            class="px-3 py-2 border border-gray-300 rounded-md"
            value={state.format}
            onchange={(e: Event) =>
              setState({ format: (e.target as HTMLSelectElement).value })
            }
          >
            <option value="markdown">Markdown</option>
            <option value="json">JSON</option>
          </select>
        </div>
      </div>
    );
  }
}

function* InputPanel(
  this: Context,
  {
    state,
    setState,
    onProcess,
  }: {
    state: State;
    setState: (updates: Partial<State>) => void;
    onProcess: () => void;
  },
) {
  for ({} of this) {
    yield (
      <div class="flex flex-col h-full">
        <div class="flex items-center justify-between p-4 border-b border-gray-200">
          <h2 class="font-bold text-lg">Input</h2>
          <button
            class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
            disabled={state.processing || !state.input.trim()}
            onclick={onProcess}
          >
            {state.processing ? "Processing..." : "Process"}
          </button>
        </div>
        <textarea
          class="flex-1 p-4 font-mono text-sm resize-none focus:outline-none"
          placeholder="Paste your log entries or JSON data here..."
          value={state.input}
          oninput={(e: Event) =>
            setState({ input: (e.target as HTMLTextAreaElement).value })
          }
        />
      </div>
    );
  }
}

function* OutputPanel(this: Context, { state }: { state: State }) {
  for ({} of this) {
    yield (
      <div class="flex flex-col h-full">
        <div class="p-4 border-b border-gray-200">
          <h2 class="font-bold text-lg">Output</h2>
        </div>
        <div class="flex-1 p-4 overflow-auto">
          {state.error ? (
            <div class="p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
              <strong>Error:</strong> {state.error}
            </div>
          ) : state.output ? (
            <pre class="font-mono text-sm whitespace-pre-wrap">
              {state.output}
            </pre>
          ) : (
            <div class="text-gray-400 italic">Output will appear here...</div>
          )}
        </div>
      </div>
    );
  }
}

function* App(this: Context) {
  const state: State = {
    algorithm: "auto",
    threshold: 0.8,
    ngramSize: 2,
    outlierThreshold: 0.0,
    format: "markdown",
    input: "",
    output: "",
    error: "",
    processing: false,
  };

  const setState = (updates: Partial<State>) => {
    Object.assign(state, updates);
    this.refresh();
  };

  const processText = async () => {
    setState({ processing: true, error: "", output: "" });

    try {
      const wasm = await loadWasm();
      const result = wasm.process_text(
        state.input,
        state.algorithm,
        state.threshold,
        state.ngramSize,
        state.outlierThreshold,
        state.format,
      );
      setState({ output: result, processing: false });
    } catch (e: any) {
      setState({ error: e.toString(), processing: false });
    }
  };

  for ({} of this) {
    yield (
      <div class="h-screen w-screen bg-white overflow-hidden flex flex-col">
        {/* Header */}
        <div class="bg-gray-900 text-white px-6 py-4">
          <h1 class="text-2xl font-black tracking-tight">txtfold</h1>
          <p class="text-sm text-gray-400">
            Deterministic text compression for log analysis
          </p>
        </div>

        {/* Main content */}
        <div class="flex-1 grid grid-cols-[300px_1fr_1fr] overflow-hidden">
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
