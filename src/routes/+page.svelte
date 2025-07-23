<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Help from "./Help.svelte";
  import TextCard from "./TextCard.svelte";

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
  let activeItemIndex = $state<number | null>(null);

  function handleItemClick(index: number) {
    activeItemIndex = index;
  }

  onMount(async () => {
    try {
      appData = await invoke<AppData>("get_clipboard_data");
      loading = false;
    } catch (e) {
      error = String(e);
      loading = false;
    }
  });
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
      {#each appData.items as item, index}
        <TextCard
          {item}
          isActive={activeItemIndex === index}
          onCopy={() => handleItemClick(index)}
        />
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
