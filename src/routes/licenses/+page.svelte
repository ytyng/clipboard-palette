<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onMount } from "svelte";
  import { applyTheme } from "$lib/theme";

  // Shown by "Third-Party Licenses" in the app menu (src-tauri/src/notices.rs)
  let notices = $state("");
  let error = $state<string | null>(null);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      void getCurrentWebviewWindow().close();
    }
  }

  onMount(async () => {
    applyTheme("auto");
    try {
      notices = await invoke<string>("third_party_notices");
    } catch (e) {
      error = String(e);
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="flex h-screen flex-col gap-3 bg-gray-100 p-4 dark:bg-gray-900">
  <p class="text-sm text-gray-700 dark:text-gray-300">
    clipboard-palette bundles the open source libraries listed below, under the
    licenses shown.
  </p>
  {#if error}
    <p class="text-sm text-red-600 dark:text-red-400">Error: {error}</p>
  {:else}
    <pre
      class="min-h-0 flex-1 select-text overflow-auto whitespace-pre-wrap rounded border border-gray-300 bg-white p-3 font-mono text-[11px] leading-snug text-gray-800 dark:border-gray-700 dark:bg-gray-950 dark:text-gray-200">{notices}</pre>
  {/if}
</main>
