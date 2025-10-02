<script lang="ts">
  import { goto } from "$app/navigation";
  import { settingsManager, type G2GSettings } from "../../lib/settings";
  import { onMount } from "svelte";

  let loading = $state(false);
  let statusMessage = $state("");
  let messageType = $state<"success" | "error" | "info">("info");

  // Поля формы
  let userId = $state("");
  let refreshToken = $state("");
  let longLivedToken = $state("");
  let activeDeviceToken = $state("");

  // Флаги видимости токенов
  let showRefreshToken = $state(false);
  let showLongLivedToken = $state(false);
  let showActiveDeviceToken = $state(false);

  // Загружаем настройки при монтировании
  onMount(async () => {
    await loadCurrentSettings();
  });

  async function loadCurrentSettings() {
    try {
      const settings = await settingsManager.loadSettings();
      if (settings && settings.g2g) {
        userId = settings.g2g.user_id || "";
        refreshToken = settings.g2g.refresh_token || "";
        longLivedToken = settings.g2g.long_lived_token || "";
        activeDeviceToken = settings.g2g.active_device_token || "";

        console.log("✅ Settings loaded");
      }
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  }

  async function saveSettings() {
    loading = true;
    statusMessage = "";

    const g2gSettings: G2GSettings = {
      user_id: userId.trim(),
      refresh_token: refreshToken.trim(),
      long_lived_token: longLivedToken.trim(),
      active_device_token: activeDeviceToken.trim(),
    };

    // Валидация
    const errors = settingsManager.validateG2GSettings(g2gSettings);
    if (errors.length > 0) {
      statusMessage = errors.join(", ");
      messageType = "error";
      loading = false;
      setTimeout(() => { statusMessage = ""; }, 10000);
      return;
    }

    try {
      await settingsManager.saveSettings({
        g2g: g2gSettings,
      });

      statusMessage = "✅ Настройки успешно сохранены!";
      messageType = "success";

      setTimeout(() => {
        goto("/");
      }, 2000);
    } catch (error) {
      console.error("Failed to save settings:", error);
      statusMessage = `❌ Ошибка сохранения: ${error}`;
      messageType = "error";
      setTimeout(() => { statusMessage = ""; }, 5000);
    } finally {
      loading = false;
    }
  }

  async function clearAllSettings() {
    if (!confirm("Вы уверены, что хотите удалить все настройки?")) {
      return;
    }

    loading = true;
    try {
      await settingsManager.clearSettings();

      userId = "";
      refreshToken = "";
      longLivedToken = "";
      activeDeviceToken = "";

      statusMessage = "✅ Настройки успешно удалены";
      messageType = "success";

      setTimeout(() => { statusMessage = ""; }, 3000);
    } catch (error) {
      statusMessage = `❌ Ошибка удаления: ${error}`;
      messageType = "error";
      setTimeout(() => { statusMessage = ""; }, 3000);
    } finally {
      loading = false;
    }
  }

  function goBack() {
    goto("/");
  }
</script>

<main class="min-h-screen bg-gradient-to-br from-gray-900 via-slate-900 to-gray-800">
  <div class="container mx-auto px-6 py-12">
    <!-- Шапка -->
    <div class="max-w-3xl mx-auto mb-8">
      <div class="flex items-center gap-4 mb-6">
        <button
          onclick={goBack}
          class="p-3 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-lg transition-colors"
          title="Вернуться назад"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
        </button>
        <div>
          <h1 class="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-600">
            ⚙️ Настройки
          </h1>
          <p class="text-gray-400 mt-2">Настройки для работы с G2G API</p>
        </div>
      </div>

      {#if statusMessage}
        <div class="mb-6 animate-fade-out">
          <div class="rounded-xl p-4 border {messageType === 'success' ? 'bg-green-500/10 border-green-500/30' : messageType === 'error' ? 'bg-red-500/10 border-red-500/30' : 'bg-blue-500/10 border-blue-500/30'}">
            <p class="{messageType === 'success' ? 'text-green-400' : messageType === 'error' ? 'text-red-400' : 'text-blue-400'} font-medium">
              {statusMessage}
            </p>
          </div>
        </div>
      {/if}
    </div>

    <!-- Форма настроек -->
    <div class="max-w-3xl mx-auto">
      <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl border border-gray-700 shadow-xl overflow-hidden">
        <div class="p-8 space-y-6">
          <!-- User ID -->
          <div>
            <label class="block text-sm font-semibold text-gray-300 mb-2">
              🆔 User ID
            </label>
            <input
              type="text"
              bind:value={userId}
              placeholder="Введите User ID..."
              class="w-full px-4 py-3 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
            />
            <p class="mt-2 text-xs text-gray-400">
              Ваш уникальный идентификатор пользователя G2G
            </p>
          </div>

          <!-- Refresh Token -->
          <div>
            <label class="block text-sm font-semibold text-gray-300 mb-2">
              🔄 Refresh Token
            </label>
            <div class="relative">
              <input
                type={showRefreshToken ? "text" : "password"}
                bind:value={refreshToken}
                placeholder="Введите Refresh Token..."
                class="w-full px-4 py-3 pr-12 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
              />
              <button
                type="button"
                onclick={() => showRefreshToken = !showRefreshToken}
                class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-300 transition"
                title={showRefreshToken ? "Скрыть" : "Показать"}
              >
                {showRefreshToken ? "👁️" : "🔒"}
              </button>
            </div>
            <p class="mt-2 text-xs text-gray-400">
              Токен для обновления сессии
            </p>
          </div>

          <!-- Long Lived Token -->
          <div>
            <label class="block text-sm font-semibold text-gray-300 mb-2">
              🔑 Long Lived Token
            </label>
            <div class="relative">
              <input
                type={showLongLivedToken ? "text" : "password"}
                bind:value={longLivedToken}
                placeholder="Введите Long Lived Token..."
                class="w-full px-4 py-3 pr-12 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
              />
              <button
                type="button"
                onclick={() => showLongLivedToken = !showLongLivedToken}
                class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-300 transition"
                title={showLongLivedToken ? "Скрыть" : "Показать"}
              >
                {showLongLivedToken ? "👁️" : "🔒"}
              </button>
            </div>
            <p class="mt-2 text-xs text-gray-400">
              Долгосрочный токен доступа
            </p>
          </div>

          <!-- Active Device Token -->
          <div>
            <label class="block text-sm font-semibold text-gray-300 mb-2">
              📱 Active Device Token
            </label>
            <div class="relative">
              <input
                type={showActiveDeviceToken ? "text" : "password"}
                bind:value={activeDeviceToken}
                placeholder="Введите Active Device Token..."
                class="w-full px-4 py-3 pr-12 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
              />
              <button
                type="button"
                onclick={() => showActiveDeviceToken = !showActiveDeviceToken}
                class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-300 transition"
                title={showActiveDeviceToken ? "Скрыть" : "Показать"}
              >
                {showActiveDeviceToken ? "👁️" : "🔒"}
              </button>
            </div>
            <p class="mt-2 text-xs text-gray-400">
              Токен активного устройства
            </p>
          </div>

          <!-- Информация о получении токенов -->
          <div class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <span class="text-2xl">💡</span>
              <div class="flex-1">
                <h3 class="text-blue-400 font-semibold mb-2">Как получить токены?</h3>
                <ol class="text-sm text-gray-300 space-y-1 list-decimal list-inside">
                  <li>Откройте сайт G2G в браузере</li>
                  <li>Войдите в свой аккаунт</li>
                  <li>Откройте DevTools (F12) → Network → Fetch/XHR</li>
                  <li>Найдите любой запрос к sls.g2g.com</li>
                  <li>В Headers найдите нужные токены</li>
                </ol>
              </div>
            </div>
          </div>

          <!-- Кнопки действий -->
          <div class="flex gap-4 pt-4">
            <button
              onclick={saveSettings}
              disabled={loading}
              class="flex-1 py-4 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 text-white font-bold rounded-xl transition-all duration-200 shadow-lg hover:shadow-purple-500/50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {#if loading}
                <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Сохранение...</span>
              {:else}
                <span>💾</span>
                <span>Сохранить настройки</span>
              {/if}
            </button>

            <button
              onclick={clearAllSettings}
              disabled={loading}
              class="px-6 py-4 bg-red-600/20 hover:bg-red-600/30 text-red-400 font-semibold rounded-xl transition-all duration-200 border border-red-500/30 hover:border-red-500/50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <span>🗑️</span>
              <span>Очистить</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</main>

<style>
  @keyframes fade-out {
    0% { opacity: 1; }
    70% { opacity: 1; }
    100% { opacity: 0; }
  }

  .animate-fade-out {
    animation: fade-out 10s ease-out forwards;
  }
</style>
