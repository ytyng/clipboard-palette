<div
  class="bg-blue-50 dark:bg-blue-900/30 rounded-lg p-4 border border-blue-200 dark:border-blue-800"
>
  <h2 class="text-xl font-semibold text-blue-800 dark:text-blue-200 mb-4">
    使用方法
  </h2>
  <div class="text-sm text-blue-700 dark:text-blue-300 space-y-3">
    <p class="font-medium">標準入力からデータを渡してアプリを起動できます：</p>

    <div class="space-y-2">
      <div>
        <span
          class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs"
          >通常モード:</span
        >
        <code
          class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded"
          >echo 'コピーしたいテキスト' | clipboard-palette</code
        >
      </div>

      <div>
        <span
          class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs"
          >複数行モード:</span
        >
        <code
          class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded"
          >echo -e 'テキスト1\nテキスト2\nテキスト3' | clipboard-palette
          --multiline</code
        >
      </div>

      <div>
        <span
          class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs"
          >空行区切りモード:</span
        >
        <code
          class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded"
          >cat file.txt | clipboard-palette --split-empty-line</code
        >
      </div>

      <div>
        <span
          class="font-mono bg-blue-100 dark:bg-blue-800/50 px-2 py-1 rounded text-xs"
          >JSONモード:</span
        >
        <code
          class="block ml-4 mt-1 font-mono text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded"
        >
          echo '[{"{"}label": "ラベル", "text": "テキスト"{"}"}]' |
          clipboard-palette --json
        </code>
      </div>
    </div>

    <p class="text-xs mt-4 italic">
      各ボタンをクリックするとテキストがクリップボードにコピーされます。
    </p>
  </div>
</div>
