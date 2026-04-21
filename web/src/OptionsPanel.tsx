import "./style.css";
import type { Context } from "@b9g/crank";
import schema from "../schema.json";
import { SectionHeader } from "./SectionHeader.js";
import type { Mode, State } from "./State.js";

const MODES: { value: Mode; label: string; description: string }[] = [
  {
    value: "analyze",
    label: "Analyze",
    description: "Run the full analysis pipeline and return grouped results.",
  },
  {
    value: "discover",
    label: "Discover",
    description:
      "Scan structure only — outputs a compact schema map. Use this first to understand your data before writing a pipeline.",
  },
  {
    value: "cost-preview",
    label: "Cost Preview",
    description:
      "Show token cost breakdown by field. Helps identify which fields to del() before analyzing.",
  },
];

// ── Options panel ─────────────────────────────────────────────────────────────

export function* OptionsPanel(
  this: Context,
  { state, setState }: { state: State; setState: (u: Partial<State>) => void },
) {
  for ({ state, setState } of this) {
    const selectedMode = MODES.find((m) => m.value === state.mode)!;
    const showPipeline =
      state.mode === "analyze" || state.mode === "cost-preview";
    const showBudget = state.mode === "analyze";
    const showOutputFormat = state.mode === "analyze";

    yield (
      <div class="flex flex-col w-1/3 gap-5 p-4 bg-gray-50 rounded overflow-y-auto min-h-[79vh] max-h-[80vh]">
        {/* ── Mode ── */}
        <div class="flex flex-col gap-2">
          <SectionHeader title="Mode" />
          <select
            class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
            onchange={(e: Event) =>
              setState({ mode: (e.target as HTMLSelectElement).value as Mode })
            }
          >
            {MODES.map((m) => (
              <option selected={state.mode === m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </select>
          <p class="text-xs text-gray-400">{selectedMode.description}</p>
        </div>

        {/* ── Input format ── */}
        <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
          <SectionHeader title="Input Format" />
          <select
            class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
            onchange={(e: Event) =>
              setState({ inputFormat: (e.target as HTMLSelectElement).value })
            }
          >
            {schema.input_formats.map((f) => (
              <option selected={state.inputFormat === f.name} value={f.name}>
                {f.name}
              </option>
            ))}
          </select>
          {(() => {
            const meta = schema.input_formats.find(
              (f) => f.name === state.inputFormat,
            );
            return meta ? (
              <p class="text-xs text-gray-400">{meta.description}</p>
            ) : null;
          })()}
        </div>

        {/* ── Pipeline ── */}
        {showPipeline && (
          <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
            <SectionHeader title="Pipeline" />
            <input
              type="text"
              placeholder="e.g. .items[] | del(.body) | group_by(.type)"
              value={state.pipeline}
              class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm font-mono"
              oninput={(e: Event) =>
                setState({ pipeline: (e.target as HTMLInputElement).value })
              }
            />
            <p class="text-xs text-gray-400">
              Optional. Stages joined by |. Path selection, del(), and a
              terminal verb (summarize, patterns, similar(t), outliers, schemas,
              subtree, group_by(.f)).
            </p>
          </div>
        )}

        {/* ── Budget ── */}
        {showBudget && (
          <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
            <SectionHeader title="Budget" />
            <input
              type="number"
              min="1"
              step="10"
              placeholder="unlimited"
              value={state.budgetLines ?? ""}
              class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
              oninput={(e: Event) => {
                const v = (e.target as HTMLInputElement).value;
                setState({
                  budgetLines: v === "" ? null : Math.max(1, parseInt(v, 10)),
                });
              }}
            />
            <p class="text-xs text-gray-400">
              Maximum output lines. Most important groups shown first.
            </p>
          </div>
        )}

        {/* ── Output format ── */}
        {showOutputFormat && (
          <div class="flex flex-col gap-2 border-t border-gray-200 pt-4">
            <SectionHeader title="Output Format" />
            <select
              class="px-3 py-2 border border-gray-300 rounded-md bg-white text-sm"
              onchange={(e: Event) =>
                setState({
                  outputFormat: (e.target as HTMLSelectElement).value,
                })
              }
            >
              {schema.formatters.map((f) => (
                <option selected={state.outputFormat === f.name} value={f.name}>
                  {f.name}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>
    );
  }
}
