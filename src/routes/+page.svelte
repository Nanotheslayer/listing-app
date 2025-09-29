<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let loading = $state(false);
  let statusMessage = $state("");
  let messageType = $state<"success" | "error" | "info">("info");

  let accounts = $state([
    { id: 1, email: "player123@gmail.com", level: 45, server: "EU", status: "Готов" },
    { id: 2, email: "gamer456@gmail.com", level: 62, server: "NA", status: "Готов" },
    { id: 3, email: "pro789@gmail.com", level: 38, server: "Asia", status: "На продаже" },
  ]);

  async function loadAccounts() {
    loading = true;
    statusMessage = "";
    try {
      await new Promise(resolve => setTimeout(resolve, 1000));
      statusMessage = "Аккаунты успешно загружены!";
      messageType = "success";
    } catch (error) {
      statusMessage = `Ошибка загрузки: ${error}`;
      messageType = "error";
    } finally {
      loading = false;
    }
  }

  async function listAccount() {
    loading = true;
    statusMessage = "";
    try {
      await new Promise(resolve => setTimeout(resolve, 1000));
      statusMessage = "Аккаунт выставлен на продажу!";
      messageType = "success";
    } catch (error) {
      statusMessage = `Ошибка выставления: ${error}`;
      messageType = "error";
    } finally {
      loading = false;
    }
  }
</script>

<main class="min-h-screen bg-gradient-to-br from-gray-900 via-slate-900 to-gray-800">
  <div class="container mx-auto px-6 py-12">
    <div class="text-center mb-16">
      <div class="inline-flex items-center justify-center w-20 h-20 bg-gradient-to-br from-purple-600 to-blue-600 rounded-2xl shadow-2xl mb-6">
        <span class="text-4xl">🎮</span>
      </div>
      <h1 class="text-6xl font-bold bg-gradient-to-r from-purple-400 via-pink-400 to-blue-400 bg-clip-text text-transparent">
        G2G Manager
      </h1>
    </div>

    <div class="grid md:grid-cols-2 gap-8 max-w-4xl mx-auto mb-12">
      <div class="group relative bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl p-8 border border-gray-700 hover:border-purple-500 transition-all duration-300 shadow-xl hover:shadow-purple-500/20 hover:-translate-y-1">
        <div class="absolute inset-0 bg-gradient-to-br from-purple-600/10 to-transparent opacity-0 group-hover:opacity-100 rounded-2xl transition-opacity duration-300"></div>
        <div class="relative">
          <div class="flex items-center gap-4 mb-6">
            <div class="w-14 h-14 bg-purple-500/20 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
              <span class="text-3xl">📥</span>
            </div>
            <h2 class="text-2xl font-bold text-white">Загрузить аккаунты</h2>
          </div>
          <button
            onclick={loadAccounts}
            disabled={loading}
            class="w-full px-6 py-4 bg-gradient-to-r from-purple-600 to-purple-700 text-white font-semibold rounded-xl hover:from-purple-500 hover:to-purple-600 active:scale-95 transition-all duration-200 shadow-lg hover:shadow-purple-500/50 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex items-center justify-center gap-3"
          >
            {#if loading}
              <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>Загрузка...</span>
            {:else}
              <span>Загрузить аккаунты</span>
              <span>→</span>
            {/if}
          </button>
        </div>
      </div>

      <div class="group relative bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl p-8 border border-gray-700 hover:border-blue-500 transition-all duration-300 shadow-xl hover:shadow-blue-500/20 hover:-translate-y-1">
        <div class="absolute inset-0 bg-gradient-to-br from-blue-600/10 to-transparent opacity-0 group-hover:opacity-100 rounded-2xl transition-opacity duration-300"></div>
        <div class="relative">
          <div class="flex items-center gap-4 mb-6">
            <div class="w-14 h-14 bg-blue-500/20 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
              <span class="text-3xl">🏷️</span>
            </div>
            <h2 class="text-2xl font-bold text-white">Выставить аккаунт</h2>
          </div>
          <button
            onclick={listAccount}
            disabled={loading}
            class="w-full px-6 py-4 bg-gradient-to-r from-blue-600 to-blue-700 text-white font-semibold rounded-xl hover:from-blue-500 hover:to-blue-600 active:scale-95 transition-all duration-200 shadow-lg hover:shadow-blue-500/50 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex items-center justify-center gap-3"
          >
            {#if loading}
              <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>Обработка...</span>
            {:else}
              <span>Выставить аккаунт</span>
              <span>→</span>
            {/if}
          </button>
        </div>
      </div>
    </div>

    {#if statusMessage}
      <div class="max-w-4xl mx-auto mb-12">
        <div class="rounded-xl p-6 border {messageType === 'success' ? 'bg-green-500/10 border-green-500/30' : messageType === 'error' ? 'bg-red-500/10 border-red-500/30' : 'bg-blue-500/10 border-blue-500/30'}">
          <div class="flex items-center gap-4">
            <div class="w-10 h-10 rounded-full flex items-center justify-center {messageType === 'success' ? 'bg-green-500/20' : messageType === 'error' ? 'bg-red-500/20' : 'bg-blue-500/20'}">
              <span class="text-2xl">
                {messageType === 'success' ? '✓' : messageType === 'error' ? '✗' : 'ℹ'}
              </span>
            </div>
            <p class="flex-1 {messageType === 'success' ? 'text-green-400' : messageType === 'error' ? 'text-red-400' : 'text-blue-400'} font-medium">
              {statusMessage}
            </p>
          </div>
        </div>
      </div>
    {/if}

    <div class="max-w-6xl mx-auto">
      <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl border border-gray-700 shadow-xl overflow-hidden">
        <div class="p-6 border-b border-gray-700">
          <div class="flex items-center gap-3">
            <span class="text-2xl">📋</span>
            <h2 class="text-2xl font-bold text-white">Загруженные аккаунты</h2>
            <span class="ml-auto bg-purple-500/20 text-purple-400 px-3 py-1 rounded-lg text-sm font-semibold">
              {accounts.length} {accounts.length === 1 ? 'аккаунт' : accounts.length < 5 ? 'аккаунта' : 'аккаунтов'}
            </span>
          </div>
        </div>

        {#if accounts.length === 0}
          <div class="p-12 text-center">
            <div class="text-6xl mb-4">📭</div>
            <p class="text-gray-400 text-lg">Аккаунты не загружены</p>
            <p class="text-gray-500 text-sm mt-2">Нажмите "Загрузить аккаунты" для импорта</p>
          </div>
        {:else}
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-800/50">
                <tr>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">ID</th>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Email</th>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Уровень</th>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Сервер</th>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Статус</th>
                  <th class="px-6 py-4 text-left text-xs font-semibold text-gray-400 uppercase tracking-wider">Действия</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-700/50">
                {#each accounts as account}
                  <tr class="hover:bg-gray-800/30 transition-colors">
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-gray-300 font-medium">#{account.id}</span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-gray-300">{account.email}</span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-purple-400 font-semibold">{account.level}</span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-gray-300">{account.server}</span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="inline-flex px-3 py-1 rounded-full text-xs font-semibold {account.status === 'Готов' ? 'bg-green-500/20 text-green-400' : 'bg-yellow-500/20 text-yellow-400'}">
                        {account.status}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <div class="flex gap-2">
                        <button class="p-2 bg-blue-500/20 text-blue-400 rounded-lg hover:bg-blue-500/30 transition-colors" title="Выставить">
                          🏷️
                        </button>
                        <button class="p-2 bg-red-500/20 text-red-400 rounded-lg hover:bg-red-500/30 transition-colors" title="Удалить">
                          🗑️
                        </button>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  </div>
</main>
