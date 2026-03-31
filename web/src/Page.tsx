import { App } from "./App";

export function Page() {
  return (
    <div class="h-full w-full bg-white overflow-hidden flex flex-col px-16 py-4 gap-4">
      <div class="flex-row border rounded-2xl border-gray-500 px-4 py-2 items-center">
        <div class="font-black tracking-tighter text-2xl">txtfold</div>
      </div>
      <div class="flex-row border rounded-2xl border-gray-500 px-4 py-2 items-center">
        <App />
      </div>
    </div>
  );
}
