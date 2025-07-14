<script lang="ts">
  import { fade } from "svelte/transition";

  interface ClipboardItem {
    label: string;
    text: string;
  }

  interface Props {
    item: ClipboardItem;
  }

  let { item }: Props = $props();

  let showSuccessOverlay = $state(false);

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      console.log("Text copied to clipboard:", text);
      showSuccessOverlay = true;
      setTimeout(() => {
        showSuccessOverlay = false;
      }, 2000); // Hide overlay after 2 seconds
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
    }
  }
</script>

<button
  class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4 relative cursor-pointer hover:bg-indigo-50 dark:hover:bg-gray-700 transition-colors text-left"
  onclick={() => copyToClipboard(item.text)}
>
  {#if item.label == item.text}
    <pre class="text-sm whitespace-pre-wrap text-gray-800">{item.text}</pre>
  {:else}
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-50 mb-2">
      {item.label}
    </h2>
    <pre class="text-xs whitespace-pre-wrap text-gray-500">{item.text}</pre>
  {/if}
  <div class="absolute top-2 right-2 z-10">
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
