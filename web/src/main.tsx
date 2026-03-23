/** @jsxImportSource @b9g/crank */

import "./style.css";
import { renderer } from "@b9g/crank/dom";
import type { Context } from "@b9g/crank";
import { loadCore } from "./loadCore.js";
import schema from "../../schema.json";

// ── Types derived from schema ─────────────────────────────────────────────────

type AlgorithmMeta = (typeof schema.algorithms)[number];
type ParameterMeta = AlgorithmMeta["parameters"][number];

// ── Schema helpers ────────────────────────────────────────────────────────────

function paramDefault(p: ParameterMeta): number {
  const d = p.default as { Float?: number; USize?: number };
  return d.Float ?? d.USize ?? 0;
}

function paramRange(p: ParameterMeta): { min: number; max: number; step: number } {
  type Bound = { min: number; max: number };
  const r = p.range as { Float?: Bound; USize?: Bound };
  if (r.Float !== undefined) return { min: r.Float.min, max: r.Float.max, step: 0.05 };
  if (r.USize !== undefined) return { min: r.USize.min, max: r.USize.max, step: 1 };
  return { min: 0, max: 1, step: 0.1 };
}

function defaultParams(algoName: string): Record<string, number> {
  const algo = schema.algorithms.find((a) => a.name === algoName);
  if (!algo) return {};
  return Object.fromEntries(algo.parameters.map((p) => [p.name, paramDefault(p)]));
}

function compatibleAlgorithms(inputFormat: string): typeof schema.algorithms {
  if (inputFormat === "auto") return schema.algorithms;
  const typeMap: Record<string, string> = {
    text: "Text",
    "json-array": "JsonArray",
    "json-map": "JsonMap",
  };
  const type = typeMap[inputFormat];
  if (type === undefined) return schema.algorithms;
  return schema.algorithms.filter((a) => (a.input_types as string[]).includes(type));
}

// ── State ─────────────────────────────────────────────────────────────────────

interface State {
  inputFormat: string;
  subOptions: Record<string, string>;
  algorithm: string;
  params: Record<string, number>;
  outputFormat: string;
  input: string;
  output: string;
  error: string;
  processing: boolean;
}

// ── Shared UI primitives ──────────────────────────────────────────────────────

function SectionHeader({ title }: { title: string }) {
  return (
    <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400">
      {title}
    </h2>
  );
}

function ParamControl({
  param,
  value,
  onchange,
}: {
  param: ParameterMeta;
  value: number;
  onchange: (v: number) => void;
}) {
  const { min, max, step } = paramRange(param);
  const isFloat = "Float" in param.range;
  const display = isFloat ? value.toFixed(2) : String(Math.round(value));
  const specialLabel = (param.special_values as Array<[number, string]>).find(
    ([v]) => v === value,
  )?.[1];

  return (
    <div class="flex flex-col gap-1">
      <div class="flex justify-between items-baseline">
        <label class="text-sm font-medium capitalize">
          {param.name.replace(/_/g, " ")}
        </label>
        <span class="text-xs font-mono text-gray-500">
          {specialLabel !== undefined ? `${display} — ${specialLabel}` : display}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        class="w-full accent-blue-600"
        oninput={(e: Event) =>
          onchange(parseFloat((e.target as HTMLInputElement).value))
        }
      />
      <p class="text-xs text-gray-400">{param.description}</p>
    </div>
  );
}

// ── Options panel ─────────────────────────────────────────────────────────────

function* OptionsPanel(
  this: Context,
  { state, setState }: { state: State; setState: (u: Partial<State>) => void },
) {
  // Defined before the loop so they are created once; they close over `state`
  // which is reassigned on each iteration, so they always see the latest value.
  const handleInputFormatChange = (fmt: string) => {
    const compatible = compatibleAlgorithms(fmt);
    const algoStillValid =
      state.algorithm === "auto" ||
      compatible.some((a) => a.name === state.algorithm);
    setState({
      inputFormat: fmt,
      subOptions: {},
      algorithm: algoStillValid ? state.algorithm : "auto",
      params: algoStillValid ? state.params : {},
    });
  };

  const handleAlgoChange = (algo: string) => {
    setState({ algorithm: algo, params: defaultParams(algo) });
  };

  for ({ state, setState } of this) {
    const algos = compatibleAlgorithms(state.inputFormat);
    const selectedAlgo = schema.algorithms.find((a) => a.name === state.algorithm);
    const inputFormatMeta = schema.input_formats.find(
      (f) => f.name === state.inputFormat,
    );

    yield (
      <div class="flex flex-col gap-5 p-4 bg-gray-50 border-r border-gray-200 overflow-y-auto">
        {/* ── Input format ── */}
        <div class="flex flex-col gap-2">
          <SectionHeader title="Input Format" />
          <select
            class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
            value={state.inputFormat}
            onchange={(e: Event) =>
              handleInputFormatChange((e.target as HTMLSelectElement).value)
            }
          >
            <option value="auto">Auto-detect</option>
            {schema.input_formats.map((f) => (
              <option value={f.name}>{f.name}</option>
            ))}
          </select>
          {inputFormatMeta !== undefined && (
            <p class="text-xs text-gray-400">{inputFormatMeta.description}</p>
          )}
        </div>

        {/* ── Sub-options (e.g. entry-mode for text) ── */}
        {(inputFormatMeta?.sub_options ?? []).map((sub) => (
          <div class="flex flex-col gap-2">
            <label class="text-sm font-medium capitalize">
              {sub.name.replace(/-/g, " ")}
            </label>
            <select
              class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
              value={state.subOptions[sub.name] ?? sub.default}
              onchange={(e: Event) =>
                setState({
                  subOptions: {
                    ...state.subOptions,
                    [sub.name]: (e.target as HTMLSelectElement).value,
                  },
                })
              }
            >
              {sub.values.map((v) => (
                <option value={v}>{v}</option>
              ))}
            </select>
            <p class="text-xs text-gray-400">{sub.description}</p>
          </div>
        ))}

        {/* ── Algorithm ── */}
        <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
          <SectionHeader title="Algorithm" />
          <select
            class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
            value={state.algorithm}
            onchange={(e: Event) =>
              handleAlgoChange((e.target as HTMLSelectElement).value)
            }
          >
            <option value="auto">Auto</option>
            {algos.map((a) => (
              <option value={a.name}>{a.name}</option>
            ))}
          </select>
          {selectedAlgo !== undefined && (
            <p class="text-xs text-gray-400">{selectedAlgo.best_for}</p>
          )}
        </div>

        {/* ── Parameters (only when a specific algorithm is selected) ── */}
        {selectedAlgo !== undefined && selectedAlgo.parameters.length > 0 && (
          <div class="flex flex-col gap-4 border-t border-gray-200 pt-4">
            <SectionHeader title="Parameters" />
            {selectedAlgo.parameters.map((p) => (
              <ParamControl
                param={p}
                value={state.params[p.name] ?? paramDefault(p)}
                onchange={(v) =>
                  setState({ params: { ...state.params, [p.name]: v } })
                }
              />
            ))}
          </div>
        )}

        {/* ── Output format ── */}
        <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
          <SectionHeader title="Output Format" />
          <select
            class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
            value={state.outputFormat}
            onchange={(e: Event) =>
              setState({ outputFormat: (e.target as HTMLSelectElement).value })
            }
          >
            {schema.formatters.map((f) => (
              <option value={f.name}>{f.name}</option>
            ))}
          </select>
        </div>
      </div>
    );
  }
}

// ── Input / Output panels ──────────────────────────────────────────────────────

function* InputPanel(
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

function* OutputPanel(this: Context, { state }: { state: State }) {
  for ({ state } of this) {
    yield (
      <div class="flex flex-col h-full overflow-y-auto">
        <div class="p-4 border-b border-gray-200">
          <h2 class="font-bold text-lg">Output</h2>
        </div>
        <div class="flex-1 p-4 overflow-auto">
          {state.error ? (
            <div class="p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
              <strong>Error:</strong> {state.error}
            </div>
          ) : state.output ? (
            <pre class="font-mono text-sm whitespace-pre-wrap">{state.output}</pre>
          ) : (
            <div class="text-gray-400 italic">Output will appear here…</div>
          )}
        </div>
      </div>
    );
  }
}

// ── App ────────────────────────────────────────────────────────────────────────

function* App(this: Context) {
  const state: State = {
    inputFormat: "auto",
    subOptions: {},
    algorithm: "auto",
    params: {},
    outputFormat: schema.formatters[0].name,
    input: "",
    output: "",
    error: "",
    processing: false,
  };

  const setState = (updates: Partial<State>) =>
    this.refresh(() => {
      Object.assign(state, updates);
    });

  const processText = async () => {
    setState({ processing: true, error: "", output: "" });
    try {
      const wasm = await loadCore();
      // Map dynamic params back to the WASM positional API.
      // Params not relevant to the selected algorithm use their defaults.
      const result = wasm.process_text(
        state.input,
        state.algorithm,
        state.params["threshold"] ?? 0.8,
        Math.round(state.params["ngram_size"] ?? 2),
        state.params["outlier_threshold"] ?? 0.0,
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
          <InputPanel state={state} setState={setState} onProcess={processText} />
          <OutputPanel state={state} />
        </div>
      </div>
    );
  }
}

(async () => {
  await renderer.render(<App />, document.body);
})();
