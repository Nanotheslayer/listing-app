<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { accountManager, type Account } from "../../../lib/accounts";
  import { autofillListing, parseAccountData } from "../../../lib/parser";
  import { onMount, onDestroy } from "svelte";

  // Интерфейсы для API ответов
  interface SkinPrice {
    skin_name: string;
    price: string;
  }

  interface SkinPriceResponse {
    prices: SkinPrice[];
    total_value: string;
    most_expensive: SkinPrice | null;
  }

  interface PriceProgress {
    current: number;
    total: number;
    skin_name: string;
    status: string;
  }

  // Получаем ID аккаунта из URL
  const accountId = parseInt($page.params.id);

  let account = $state<Account | undefined>(undefined);
  let loading = $state(false);
  let statusMessage = $state("");
  let messageType = $state<"success" | "error" | "info">("info");

  // Поля формы
  let title = $state("");
  let description = $state("");
  let skinsPriceInfo = $state("Информация о ценах скинов появится здесь...");

  // Прогресс расчета цен
  let priceProgress = $state<PriceProgress | null>(null);
  let isCalculatingPrices = $state(false);

  // Счетчики символов
  const MAX_TITLE_LENGTH = 128;
  const MAX_DESCRIPTION_LENGTH = 5000;

  let titleLength = $derived(title.length);
  let descriptionLength = $derived(description.length);

  // Unsubscribe функция для событий
  let unsubscribeProgress: (() => void) | null = null;

  // Загружаем данные аккаунта ОДИН РАЗ при монтировании компонента
  onMount(async () => {
    account = accountManager.getAccount(accountId);
    if (!account) {
      statusMessage = "Аккаунт не найден";
      messageType = "error";
      setTimeout(() => goBack(), 2000);
      return;
    }

    // Подписываемся на события прогресса
    unsubscribeProgress = await listen<PriceProgress>("price-progress", (event) => {
      priceProgress = event.payload;
      console.log(`Progress: ${event.payload.current}/${event.payload.total} - ${event.payload.skin_name} (${event.payload.status})`);
    });
  });

  // Отписываемся при размонтировании
  onDestroy(() => {
    if (unsubscribeProgress) {
      unsubscribeProgress();
    }
  });

  function goBack() {
    goto("/");
  }

  async function autoFillForm() {
    if (!account) {
      console.error("Account is undefined!");
      return;
    }

    loading = true;
    statusMessage = "Автозаполнение формы...";
    messageType = "info";

    console.log("=== АВТОЗАПОЛНЕНИЕ НАЧАТО ===");
    console.log("ID аккаунта:", accountId);
    console.log("Путь аккаунта:", account.path);
    console.log("Имя аккаунта:", account.name);

    try {
      // Получаем список файлов аккаунта
      console.log("Шаг 1: Получаем файлы...");
      const files = await accountManager.getAccountFiles(accountId);
      console.log("Файлы аккаунта:", files);

      if (!files || files.length === 0) {
        throw new Error("Файлы не найдены в папке аккаунта");
      }

      // Парсим и заполняем форму
      console.log("Шаг 2: Вызываем autofillListing...");
      const result = await autofillListing(account.path, files);

      console.log("Шаг 3: Получен результат");
      console.log("Результат автозаполнения:", result);
      console.log("Заголовок:", result.title);
      console.log("Длина заголовка:", result.title.length);
      console.log("Описание (первые 200 символов):", result.description.substring(0, 200));
      console.log("Длина описания:", result.description.length);

      // Присваиваем значения
      console.log("Шаг 4: Присваиваем значения полям...");
      title = result.title;
      description = result.description;

      console.log("Шаг 5: Значения присвоены");
      console.log("title переменная:", title);
      console.log("description переменная (первые 100 символов):", description.substring(0, 100));

      console.log("=== АВТОЗАПОЛНЕНИЕ ЗАВЕРШЕНО УСПЕШНО ===");

      statusMessage = "Форма успешно заполнена!";
      messageType = "success";

      setTimeout(() => {
        statusMessage = "";
      }, 3000);
    } catch (error) {
      console.error("=== ОШИБКА АВТОЗАПОЛНЕНИЯ ===");
      console.error("Тип ошибки:", typeof error);
      console.error("Ошибка:", error);

      if (error instanceof Error) {
        console.error("Сообщение:", error.message);
        console.error("Stack trace:", error.stack);
      } else {
        console.error("Не-Error объект:", String(error));
      }

      statusMessage = `Ошибка: ${error instanceof Error ? error.message : String(error)}`;
      messageType = "error";

      setTimeout(() => {
        statusMessage = "";
      }, 3000);
    } finally {
      console.log("Шаг 6: Finally блок, loading = false");
      loading = false;
    }
  }

  async function calculatePrices() {
    if (!account) return;

    loading = true;
    isCalculatingPrices = true; // ⬅️ ДОБАВЬТЕ ЭТУ СТРОКУ
    statusMessage = "Подсчет цен скинов...";
    messageType = "info";

    try {
      // Шаг 1: Получаем файлы аккаунта
      console.log("Получение файлов аккаунта...");
      const files = await accountManager.getAccountFiles(accountId);

      if (!files || files.length === 0) {
        throw new Error("Файлы не найдены в папке аккаунта");
      }

      // Шаг 2: Парсим данные аккаунта для получения списка скинов и сервера
      console.log("Парсинг данных аккаунта...");
      const accountData = await parseAccountData(account.path, files);

      console.log("Найдено скинов:", accountData.skinsList.length);
      console.log("Сервер:", accountData.server);

      if (accountData.skinsList.length === 0) {
        skinsPriceInfo = "❌ Скины не найдены в данных аккаунта";
        statusMessage = "Скины не найдены";
        messageType = "error";
        setTimeout(() => { statusMessage = ""; }, 3000);
        return;
      }

      // Шаг 3: Вызываем Rust команду для получения цен
      statusMessage = `Получение цен для ${accountData.skinsList.length} скинов...`;
      console.log("Вызов fetch_skin_prices...");

      const response = await invoke<SkinPriceResponse>("fetch_skin_prices", {
        request: {
          skins: accountData.skinsList,
          server: accountData.server
        }
      });

      console.log("Получен ответ от API:", response);

      // Шаг 4: Форматируем результат для отображения
      const priceLines = response.prices.map(p =>
        `  • ${p.skin_name}: ${p.price}`
      ).join('\n');

      skinsPriceInfo = `
📊 Анализ завершен для аккаунта: ${account.name}
🌍 Сервер: ${accountData.server}

Найдено скинов: ${response.prices.length}
💰 Общая стоимость: ${response.total_value}
${response.most_expensive ? `⭐ Самый дорогой: ${response.most_expensive.skin_name} (${response.most_expensive.price})` : ''}

📋 Список цен:
${priceLines}
      `.trim();

      statusMessage = "Цены успешно рассчитаны!";
      messageType = "success";

      setTimeout(() => {
        statusMessage = "";
      }, 3000);

    } catch (error) {
      console.error("Ошибка расчета цен:", error);

      skinsPriceInfo = `❌ Ошибка получения цен:\n${error instanceof Error ? error.message : String(error)}`;

      statusMessage = `Ошибка расчета: ${error instanceof Error ? error.message : String(error)}`;
      messageType = "error";

      setTimeout(() => {
        statusMessage = "";
      }, 3000);
    } finally {
      loading = false;
      isCalculatingPrices = false;
    }
  }

  async function listAccount() {
    if (!account) return;

    // Валидация
    if (!title.trim()) {
      statusMessage = "Заголовок не может быть пустым";
      messageType = "error";
      setTimeout(() => { statusMessage = ""; }, 3000);
      return;
    }

    if (!description.trim()) {
      statusMessage = "Описание не может быть пустым";
      messageType = "error";
      setTimeout(() => { statusMessage = ""; }, 3000);
      return;
    }

    loading = true;
    statusMessage = "Выставление аккаунта на продажу...";
    messageType = "info";

    try {
      // TODO: Здесь будет реальная логика выставления аккаунта
      console.log("Данные для выставления:", {
        accountId: account.id,
        accountPath: account.path,
        title,
        description,
        skinsPriceInfo
      });

      await new Promise(resolve => setTimeout(resolve, 2000));

      // Обновляем статус в менеджере
      accountManager.updateAccountStatus(accountId, "listed");

      statusMessage = "Аккаунт успешно выставлен на продажу!";
      messageType = "success";

      setTimeout(() => {
        goBack();
      }, 2000);
    } catch (error) {
      accountManager.updateAccountStatus(accountId, "error");
      statusMessage = `Ошибка выставления: ${error}`;
      messageType = "error";

      setTimeout(() => {
        statusMessage = "";
      }, 3000);
    } finally {
      loading = false;
    }
  }

  function handleTitleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    if (target.value.length <= MAX_TITLE_LENGTH) {
      title = target.value;
    } else {
      target.value = title;
    }
  }

  function handleDescriptionInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    if (target.value.length <= MAX_DESCRIPTION_LENGTH) {
      description = target.value;
    } else {
      target.value = description;
    }
  }
</script>

<!-- Остальной HTML остается таким же -->
<main class="min-h-screen bg-gradient-to-br from-gray-900 via-slate-900 to-gray-800">
  <div class="container mx-auto px-6 py-8">
    <!-- Шапка -->
    <div class="max-w-5xl mx-auto mb-6">
      <div class="flex items-center gap-4">
        <button
          onclick={goBack}
          disabled={loading}
          class="p-3 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          title="Вернуться назад"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
        </button>
        <div class="flex-1">
          <h1 class="text-3xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-600">
            Выставление аккаунта
          </h1>
          {#if account}
            <p class="text-gray-400 mt-1">Аккаунт: <span class="font-semibold text-gray-300">{account.name}</span></p>
          {/if}
        </div>
        <button
          onclick={autoFillForm}
          disabled={loading}
          class="px-4 py-3 bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 text-white font-semibold rounded-lg transition-all duration-200 shadow-lg hover:shadow-indigo-500/50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          title="Автоматически заполнить форму"
        >
          {#if loading}
            <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          {:else}
            <span>✨</span>
          {/if}
          <span>Автозаполнить</span>
        </button>
      </div>
    </div>

    <!-- Статус сообщение -->
    {#if statusMessage}
      <div class="max-w-5xl mx-auto mb-6 animate-fade-out">
        <div class="rounded-xl p-4 border {messageType === 'success' ? 'bg-green-500/10 border-green-500/30' : messageType === 'error' ? 'bg-red-500/10 border-red-500/30' : 'bg-blue-500/10 border-blue-500/30'}">
          <div class="flex items-center gap-3">
            <span class="text-xl">
              {messageType === 'success' ? '✓' : messageType === 'error' ? '✗' : 'ℹ'}
            </span>
            <p class="{messageType === 'success' ? 'text-green-400' : messageType === 'error' ? 'text-red-400' : 'text-blue-400'} font-medium">
              {statusMessage}
            </p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Контент -->
    {#if !account}
      <div class="max-w-5xl mx-auto">
        <div class="bg-gray-800 rounded-2xl p-12 text-center border border-gray-700">
          <div class="text-6xl mb-4">⚠️</div>
          <p class="text-gray-400 text-lg">Аккаунт не найден</p>
        </div>
      </div>
    {:else}
      <div class="max-w-5xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Левая колонка - Основная форма -->
        <div class="lg:col-span-2 space-y-6">
          <!-- Заголовок -->
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl border border-gray-700 p-6">
            <div class="flex items-center justify-between mb-4">
              <label class="text-lg font-semibold text-white flex items-center gap-2">
                <span>📝</span>
                <span>Заголовок объявления</span>
              </label>
              <span class="text-sm font-mono {titleLength > MAX_TITLE_LENGTH * 0.9 ? 'text-red-400' : 'text-gray-400'}">
                {titleLength}/{MAX_TITLE_LENGTH}
              </span>
            </div>
            <textarea
              bind:value={title}
              oninput={handleTitleInput}
              placeholder="Введите заголовок объявления (макс. 128 символов)..."
              class="w-full px-4 py-3 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition resize-none"
              rows="2"
              maxlength={MAX_TITLE_LENGTH}
            ></textarea>
          </div>

          <!-- Описание -->
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl border border-gray-700 p-6">
            <div class="flex items-center justify-between mb-4">
              <label class="text-lg font-semibold text-white flex items-center gap-2">
                <span>📄</span>
                <span>Описание</span>
              </label>
              <span class="text-sm font-mono {descriptionLength > MAX_DESCRIPTION_LENGTH * 0.9 ? 'text-red-400' : 'text-gray-400'}">
                {descriptionLength}/{MAX_DESCRIPTION_LENGTH}
              </span>
            </div>
            <textarea
              bind:value={description}
              oninput={handleDescriptionInput}
              placeholder="Введите подробное описание аккаунта (макс. 5000 символов)..."
              class="w-full px-4 py-3 bg-gray-900/50 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition resize-none"
              rows="12"
              maxlength={MAX_DESCRIPTION_LENGTH}
            ></textarea>
          </div>

          <!-- Кнопка выставления -->
          <button
            onclick={listAccount}
            disabled={loading || !title.trim() || !description.trim()}
            class="w-full py-4 bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-500 hover:to-emerald-500 text-white font-bold rounded-xl transition-all duration-200 shadow-lg hover:shadow-green-500/50 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-none flex items-center justify-center gap-3 text-lg"
          >
            {#if loading}
              <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>Выставление...</span>
            {:else}
              <span>🚀</span>
              <span>Выставить аккаунт</span>
            {/if}
          </button>
        </div>

        <!-- Правая колонка - Цены скинов -->
        <!-- Правая колонка - Цены скинов -->
        <div class="lg:col-span-1">
          <div class="bg-gradient-to-br from-gray-800 to-gray-900 rounded-2xl border border-gray-700 p-6 sticky top-6">
            <div class="flex items-center gap-2 mb-4">
              <span class="text-xl">💰</span>
              <h3 class="text-lg font-semibold text-white">Цены скинов</h3>
            </div>

            <!-- Единое поле для прогресса и результатов -->
            <div class="bg-gray-900/50 border border-gray-700 rounded-lg p-4 mb-4 min-h-[300px] max-h-[400px] overflow-y-auto">
              {#if isCalculatingPrices && priceProgress}
                <!-- Прогресс-бар внутри окна -->
                <div class="space-y-4">
                  <div class="text-center">
                    <div class="text-2xl mb-2">🔄</div>
                    <div class="text-white font-semibold mb-4">Расчет цен скинов</div>
                  </div>

                  <!-- Визуальный прогресс-бар -->
                  <div class="space-y-2">
                    <div class="flex items-center justify-between text-sm">
                      <span class="text-gray-400">
                        Обработка: {priceProgress.current} / {priceProgress.total}
                      </span>
                      <span class="text-gray-400">
                        {Math.round((priceProgress.current / priceProgress.total) * 100)}%
                      </span>
                    </div>

                    <div class="w-full bg-gray-700 rounded-full h-3 overflow-hidden">
                      <div
                        class="h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-300 ease-out"
                        style="width: {(priceProgress.current / priceProgress.total) * 100}%"
                      ></div>
                    </div>

                    <!-- Текущий обрабатываемый скин -->
                    <div class="text-sm text-gray-300 text-center mt-4">
                      {#if priceProgress.status === "processing"}
                        <div class="flex items-center justify-center gap-2">
                          <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          <span>Обработка: {priceProgress.skin_name}</span>
                        </div>
                      {:else if priceProgress.status === "completed"}
                        <div class="text-green-400">✅ Завершено: {priceProgress.skin_name}</div>
                      {:else if priceProgress.status === "error"}
                        <div class="text-red-400">❌ Ошибка: {priceProgress.skin_name}</div>
                      {/if}
                    </div>
                  </div>
                </div>
              {:else}
                <!-- Результаты -->
                <pre class="text-sm text-gray-300 whitespace-pre-wrap font-mono">{skinsPriceInfo}</pre>
              {/if}
            </div>

            <button
              onclick={calculatePrices}
              disabled={loading || isCalculatingPrices}
              class="w-full py-3 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white font-semibold rounded-lg transition-all duration-200 shadow-lg hover:shadow-blue-500/50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {#if isCalculatingPrices}
                <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Расчет... {priceProgress ? `${priceProgress.current}/${priceProgress.total}` : ''}</span>
              {:else}
                <span>🧮</span>
                <span>Посчитать цены</span>
              {/if}
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</main>

<style>
  @keyframes fade-out {
    0% {
      opacity: 1;
    }
    70% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }

  .animate-fade-out {
    animation: fade-out 3s ease-out forwards;
  }

  /* Стилизация скроллбара */
  :global(*::-webkit-scrollbar) {
    width: 8px;
  }

  :global(*::-webkit-scrollbar-track) {
    background: rgb(31, 41, 55);
    border-radius: 4px;
  }

  :global(*::-webkit-scrollbar-thumb) {
    background: rgb(75, 85, 99);
    border-radius: 4px;
  }

  :global(*::-webkit-scrollbar-thumb:hover) {
    background: rgb(107, 114, 128);
  }
</style>
