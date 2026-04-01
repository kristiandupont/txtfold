import type { Context } from "@b9g/crank";

const installers = [
  // {
  //   name: "Apt",
  //   command: "sudo apt install txtfold",
  // },
  // {
  //   name: "Homebrew",
  //   command: "brew install txtfold",
  // },
  {
    name: "Rust",
    command: "cargo add txtfold",
  },
  {
    name: "JS/TS",
    command: "npm install txtfold",
  },
  {
    name: "Python",
    command: "pip install txtfold",
  },
  {
    name: "Build CLI",
    command:
      "git clone https://github.com/kristiandupont/txtfold.git && cd txtfold && cargo install",
  },
];

function ClipboardIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      stroke-width="1.5"
      stroke="currentColor"
      class="size-6"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 0 1-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 0 1 1.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 0 0-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 0 1-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 0 0-3.375-3.375h-1.5a1.125 1.125 0 0 1-1.125-1.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H9.75"
      />
    </svg>
  );
}

export function* InstallGuide(this: Context) {
  let selectedInstallerIndex = 0;

  const selectInstaller = (installerIndex: number) =>
    this.refresh(() => {
      selectedInstallerIndex = installerIndex;
    });

  for ({} of this) {
    const selectedInstaller = installers[selectedInstallerIndex];
    yield (
      <div class="flex flex-col w-2/3 justify-center items-start gap-2 bg-gray-50 border-3 border-gray-300 rounded-xl text-gray-700 p-4 shadow-lg">
        <ul class="flex flex-row gap-2">
          {installers.map((installer, index) => (
            <li
              class={`px-3 py-1 rounded-lg text-sm cursor-pointer ${
                index === selectedInstallerIndex
                  ? "bg-gray-500 text-white"
                  : "bg-white text-gray-500 hover:bg-gray-50"
              }`}
              onclick={() => selectInstaller(index)}
            >
              {installer.name}
            </li>
          ))}
        </ul>
        <pre class="font-mono text-sm bg-gray-800 text-gray-100 p-2 rounded w-full overflow-x-auto relative pl-5">
          <span class="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400 text-xs">
            $
          </span>
          {selectedInstaller.command}
          <button
            class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 text-xs hover:text-gray-200 cursor-pointer"
            onclick={() =>
              navigator.clipboard.writeText(selectedInstaller.command)
            }
          >
            <ClipboardIcon />
          </button>
        </pre>
      </div>
    );
  }
}
