<script lang="ts">
  import { goto } from "$app/navigation";
  import { settingsManager } from "../settings";
  import { onMount } from "svelte";

  let hasSettings = $state(false);
  let checking = $state(true);

  onMount(async () => {
    hasSettings = await settingsManager.hasG2GSettings();
    checking = false;
  });

  function goToSettings() {
    goto("/settings");
  }
</script>

{#if !checking && !hasSettings}
  <div class="bg-yellow-500/10 border border-yellow-500/30 rounded-xl p-6 mb-6">
    <div class="flex items-start gap-4">
      <div class="flex-shrink-0 w-12 h-12 rounded-full bg-yellow-500/20 flex items-center justify-center">
        <span class="text-3xl">⚠️</span>
      </div>
      <div class="flex-1">
        <h3 class="text-yellow-400 font-bold text-lg mb-2">
          Настройки G2G не найдены
        </h3>
        <p class="text-gray-300 mb-4">
          Для работы с G2G API необходимо указать токены в настройках приложения.
          Без них функции выставления аккаунтов и получения цен скинов работать не будут.
        </p>
        <button
          onclick={goToSettings}
          class="px-4 py-2 bg-yellow-600 hover:bg-yellow-500 text-white font-semibold rounded-lg transition-colors flex items-center gap-2"
        >
          <span>⚙️</span>
          <span>Перейти в настройки</span>
        </button>
      </div>
    </div>
  </div>
{/if}
