import schema from "../schema.json";

export type Mode = "analyze" | "discover" | "cost-preview";

export interface State {
  mode: Mode;
  inputFormat: string;
  pipeline: string;
  budgetLines: number | null;
  outputFormat: string;
  input: string;
  output: string;
  error: string;
  processing: boolean;
}

export const initialState: State = {
  mode: "analyze",
  inputFormat: "line",
  pipeline: "",
  budgetLines: null,
  outputFormat: schema.formatters[0].name,
  input: "",
  output: "",
  error: "",
  processing: false,
};
