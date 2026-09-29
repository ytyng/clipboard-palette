<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";

  interface ClipboardItem {
    label: string;
    text: string;
    open_kind: "url" | "path" | null;
  }

  interface Props {
    item: ClipboardItem;
    index: number;
    // True while Shift is held
    openMode: boolean;
    isActive: boolean;
    onCopy: () => void;
  }

  let { item, index, openMode, isActive, onCopy }: Props = $props();

  // Only cards holding a URL or a path change in open mode
  let opens = $derived(openMode && item.open_kind !== null);

  async function openItem() {
    try {
      // Only the index is sent: the app reads the text back and checks it again
      await invoke("open_item", { index });
    } catch (e) {
      console.error("Failed to open:", e);
    }
  }

  function handleClick(event: MouseEvent) {
    // The click itself decides, so a Shift released just before the click
    // still copies
    if (event.shiftKey && item.open_kind !== null) {
      openItem();
    } else {
      copyToClipboard(item.text);
    }
  }

  let isClicked = $state(false);
  let showSuccessOverlay = $state(false);

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      console.log("Text copied to clipboard:", text);
      onCopy(); // Notify the parent component
      isClicked = true;
      showSuccessOverlay = true;
      setTimeout(() => {
        showSuccessOverlay = false;
      }, 2000); // Hide overlay after 2 seconds
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
    }
  }

  // Decide the background color
  function getBackgroundClass() {
    if (isActive) {
      return "bg-emerald-100 dark:bg-emerald-900/30"; // Light green (active)
    } else if (isClicked) {
      return "bg-gray-200 dark:bg-gray-600"; // Light gray (already clicked)
    } else {
      return "bg-white dark:bg-gray-800"; // Default
    }
  }
</script>

<!-- select-none in open mode: Shift+click would otherwise extend a text
     selection across the cards -->
<button
  class="{getBackgroundClass()} rounded-lg shadow-md p-4 relative cursor-pointer hover:bg-indigo-50 dark:hover:bg-gray-700 transition-colors text-left"
  class:select-none={opens}
  onclick={handleClick}
>
  {#if item.label == item.text}
    <pre
      class="text-sm whitespace-pre-wrap text-gray-900 dark:text-gray-50"
      class:underline={opens}>{item.text}</pre>
  {:else}
    <h2 class="text-sm font-semibold text-gray-500 dark:text-gray-400 mb-2">
      {item.label}
    </h2>
    <pre
      class="whitespace-pre-wrap text-gray-900 dark:text-gray-50 line-clamp-10"
      class:underline={opens}>{item.text}</pre>
  {/if}
  <div class="absolute top-2 right-2 z-10">
    {#if opens && item.open_kind === "url"}
      <!-- External link: opens in the default browser -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-5 w-5 text-indigo-500 dark:text-indigo-300"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
        />
      </svg>
    {:else if opens}
      <!-- Folder: shows the file in Finder -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-5 w-5 text-indigo-500 dark:text-indigo-300"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"
        />
      </svg>
    {:else}
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
    {/if}
  </div>
  {#if showSuccessOverlay}
    <div
      transition:fade={{ duration: 150 }}
      class="absolute inset-0 bg-indigo-200/70 dark:bg-indigo-800/70 rounded-lg z-20 flex items-center justify-center backdrop-blur-xs"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="currentColor"
        class="bi bi-check h-12 w-12 text-indigo-700 dark:text-indigo-200"
        viewBox="0 0 16 16"
      >
        <path
          d="M10.97 4.97a.75.75 0 0 1 1.07 1.05l-3.99 4.99a.75.75 0 0 1-1.08.02L4.324 8.384a.75.75 0 1 1 1.06-1.06l2.094 2.093 3.473-4.425z"
        />
      </svg>
    </div>
  {/if}
</button>
