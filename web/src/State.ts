import schema from "../schema.json";

export interface State {
  inputFormat: string;
  subOptions: Record<string, string>;
  algorithm: string;
  params: Record<string, number>;
  budget: number | null;
  outputFormat: string;
  input: string;
  output: string;
  error: string;
  processing: boolean;
}

export const initialState: State = {
  inputFormat: "auto",
  subOptions: {},
  algorithm: "auto",
  params: {},
  budget: null,
  outputFormat: schema.formatters[0].name,
  input: "",
  output: "",
  error: "",
  processing: false,
};
