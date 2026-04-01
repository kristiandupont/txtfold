import { App } from "./App";
import { InstallGuide } from "./InstallGuide";

export function Page() {
  return (
    <div class="h-full w-full bg-white overflow-hidden flex flex-col items-center px-16 py-4 gap-24">
      <div class="flex-row border rounded-2xl border-gray-500 p-4 items-center w-full">
        <div class="font-black tracking-tighter text-2xl select-none">
          txtfold
        </div>
      </div>

      <div class="w-2/3 text-center text-gray-700 text-lg">
        <span class="font-black tracking-tighter">txtfold</span> helps you
        summarize large text files with repetitive data and interesting outliers
        (e.g. logs, JSON) for human or LLM consumption.
      </div>

      <InstallGuide />

      <App />
    </div>
  );
}
