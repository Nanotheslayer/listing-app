import { invoke } from "@tauri-apps/api/core";

// Типы данных
export interface SkinPrice {
  skin_name: string;
  price: string;
}

export interface SkinPriceRequest {
  skins: string[];
  server: string;
}

export interface SkinPriceResponse {
  prices: SkinPrice[];
  total_value: string;
  most_expensive: SkinPrice | null;
}

// Класс для управления ценами скинов
export class SkinPriceManager {
  // Проверить наличие конфигурации G2G
  async checkConfig(): Promise<boolean> {
    try {
      const status = await invoke<boolean>("get_g2g_config_status");
      return status;
    } catch (error) {
      console.error("G2G config check failed:", error);
      return false;
    }
  }

  // Получить цены для списка скинов
  async fetchPrices(skins: string[], server: string): Promise<SkinPriceResponse> {
    try {
      console.log("=== Запрос цен скинов ===");
      console.log("Количество скинов:", skins.length);
      console.log("Сервер:", server);

      // Проверяем конфигурацию перед запросом
      const configOk = await this.checkConfig();
      if (!configOk) {
        throw new Error(
          "G2G токены не настроены. Пожалуйста, создайте файл .env с токенами (см. .env.example)"
        );
      }

      const request: SkinPriceRequest = {
        skins,
        server,
      };

      const response = await invoke<SkinPriceResponse>("fetch_skin_prices", { request });

      console.log("=== Результат запроса цен ===");
      console.log("Получено цен:", response.prices.length);
      console.log("Общая стоимость:", response.total_value);
      console.log("Самый дорогой скин:", response.most_expensive);

      return response;
    } catch (error) {
      console.error("=== Ошибка получения цен ===", error);
      throw error;
    }
  }

  // Форматирование результатов для отображения
  formatPriceInfo(response: SkinPriceResponse): string {
    const lines: string[] = [];

    lines.push("📊 Анализ цен скинов");
    lines.push("");
    lines.push(`Общая стоимость: ${response.total_value}`);
    lines.push(`Проверено скинов: ${response.prices.length}`);
    lines.push("");

    if (response.most_expensive) {
      lines.push(`💎 Самый дорогой скин:`);
      lines.push(`   ${response.most_expensive.skin_name}: ${response.most_expensive.price}`);
      lines.push("");
    }

    lines.push("📋 Детальный список:");
    lines.push("");

    // Сортируем скины по цене (от дорогих к дешевым)
    const sortedPrices = [...response.prices].sort((a, b) => {
      const priceA = parsePrice(a.price);
      const priceB = parsePrice(b.price);
      return priceB - priceA;
    });

    for (const item of sortedPrices) {
      const icon = getIcon(item.price);
      lines.push(`${icon} ${item.skin_name}: ${item.price}`);
    }

    return lines.join("\n");
  }
}

// Вспомогательные функции
function parsePrice(priceStr: string): number {
  if (priceStr.startsWith("$")) {
    return parseFloat(priceStr.substring(1)) || 0;
  }
  return 0;
}

function getIcon(priceStr: string): string {
  const price = parsePrice(priceStr);
  
  if (priceStr === "No offers" || priceStr === "Error") {
    return "❌";
  }
  
  if (price === 0) {
    return "⚠️";
  }
  
  if (price >= 100) {
    return "💎";
  } else if (price >= 50) {
    return "⭐";
  } else if (price >= 20) {
    return "✨";
  } else {
    return "🔹";
  }
}

// Экспорт singleton instance
export const skinPriceManager = new SkinPriceManager();
