<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";

  interface ClipboardItem {
    label: string;
    text: string;
  }

  interface AppData {
    items: ClipboardItem[];
    mode: string;
    is_default_data: boolean;
  }

  let appData = $state<AppData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      appData = await invoke<AppData>("get_clipboard_data");
      loading = false;
    } catch (e) {
      error = String(e);
      loading = false;
    }
  });

  async function copyToClipboard(text: string) {
    try {
      await writeText(text);
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
    }
  }
</script>

<main class="min-h-screen bg-gray-100 dark:bg-gray-900 p-8">
  <div class="max-w-4xl mx-auto">
    <h1 class="text-3xl font-bold text-gray-800 dark:text-gray-100 mb-8 text-center">
      Clipboard Palette
    </h1>

    {#if loading}
      <div class="text-center text-gray-600 dark:text-gray-400">
        Loading...
      </div>
    {:else if error}
      <div class="text-center text-red-600 dark:text-red-400">
        Error: {error}
      </div>
    {:else if appData && appData.items.length > 0}
      <div class="space-y-4">
        {#each appData.items as item}
          <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4">
            <div class="flex items-start justify-between gap-4">
              <div class="flex-1">
                <h3 class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-2">
                  {item.label}
                </h3>
                {#if item.label !== item.text}
                  <pre class="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap bg-gray-50 dark:bg-gray-700 p-3 rounded">{item.text}</pre>
                {/if}
              </div>
              <button
                onclick={() => copyToClipboard(item.text)}
                class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors duration-200 flex items-center gap-2 shrink-0"
                data-annotate="button-copy"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                Copy
              </button>
            </div>
          </div>
        {/each}
      </div>

      {#if appData.is_default_data}
        <div class="mt-12 bg-blue-50 dark:bg-blue-900/30 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
          <h2 class="text-xl font-semibold text-blue-800 dark:text-blue-200 mb-4">使用方法</h2>
          <div class="text-sm text-blue-700 dark:text-blue-300 space-y-3">
            <p class="font-medium">標準入力からデータを渡してアプリを起動できます：</p>
            
            <div class="space-y-2">
              <div>
                <span class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs">通常モード:</span>
                <code class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded">echo 'コピーしたいテキスト' | clipboard-palette</code>
              </div>

              <div>
                <span class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs">複数行モード:</span>
                <code class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded">echo -e 'テキスト1\nテキスト2\nテキスト3' | clipboard-palette --multiline</code>
              </div>

              <div>
                <span class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs">空行区切りモード:</span>
                <code class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded">cat file.txt | clipboard-palette --split-empty-line</code>
              </div>

              <div>
                <span class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs">JSONモード:</span>
                <code class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded">
                  echo '[{"{"}label": "ラベル", "text": "テキスト"{"}"}]' | clipboard-palette --json
                </code>
              </div>
            </div>

            <p class="text-xs mt-4 italic">各ボタンをクリックするとテキストがクリップボードにコピーされます。</p>
          </div>
        </div>
      {/if}
    {:else}
      <div class="text-center text-gray-600 dark:text-gray-400">
        No data to display
      </div>
    {/if}
  </div>
</main>