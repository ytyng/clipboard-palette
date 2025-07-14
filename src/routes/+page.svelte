<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";
  import Help from "./Help.svelte";

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

<main class="min-h-screen bg-gray-100 dark:bg-gray-900">
  {#if loading}
    <div class="text-gray-600 dark:text-gray-400">Loading...</div>
  {:else if error}
    <div class="text-red-600 dark:text-red-400">
      Error: {error}
    </div>
  {:else if appData && appData.items.length > 0}
    <div class="flex flex-col p-4 gap-4">
      {#each appData.items as item}
        <button
          class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4 relative cursor-pointer hover:bg-indigo-50 dark:hover:bg-gray-700 transition-colors text-left"
          onclick={() => copyToClipboard(item.text)}
        >
          {#if item.label == item.text}
            <pre
              class="text-sm whitespace-pre-wrap text-gray-700">{item.text}</pre>
          {:else}
            <h2
              class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-2"
            >
              {item.label}
            </h2>
            <pre
              class="text-xs whitespace-pre-wrap text-gray-500">{item.text}</pre>
          {/if}
          <div class="absolute top-2 right-2">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="h-5 w-5 text-gray-500"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
              />
            </svg>
          </div>
        </button>
      {/each}
    </div>

    {#if appData.is_default_data}
      <div class="p-4">
        <Help />
      </div>
    {/if}
  {:else}
    <div class="text-center text-gray-600 dark:text-gray-400">
      No data to display
    </div>
  {/if}
</main>
