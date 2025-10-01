// src/lib/championTracking.ts
// Система отслеживания использования чемпионов в заголовках

const STORAGE_KEY = 'g2g_champion_usage_count';

export interface ChampionUsage {
  [championName: string]: number;
}

// Загрузить статистику использования
export function loadChampionUsage(): ChampionUsage {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (error) {
    console.warn('Не удалось загрузить статистику чемпионов:', error);
  }
  return {};
}

// Сохранить статистику использования
function saveChampionUsage(usage: ChampionUsage): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(usage));
  } catch (error) {
    console.warn('Не удалось сохранить статистику чемпионов:', error);
  }
}

// Увеличить счетчик использования для списка чемпионов
export function trackChampionUsage(championNames: string[]): void {
  const usage = loadChampionUsage();
  
  for (const name of championNames) {
    usage[name] = (usage[name] || 0) + 1;
  }
  
  saveChampionUsage(usage);
  console.log('📊 Обновлена статистика использования чемпионов:', championNames);
}

// Отсортировать список чемпионов по частоте использования (от меньшего к большему)
export function sortChampionsByUsage(champions: string[]): string[] {
  const usage = loadChampionUsage();
  
  return [...champions].sort((a, b) => {
    const usageA = usage[a] || 0;
    const usageB = usage[b] || 0;
    
    // Сначала те, что использовались реже
    if (usageA !== usageB) {
      return usageA - usageB;
    }
    
    // Если одинаково - сохраняем исходный порядок (алфавитный)
    return a.localeCompare(b);
  });
}

// Получить статистику для отладки
export function getChampionUsageStats(): { champion: string; count: number }[] {
  const usage = loadChampionUsage();
  return Object.entries(usage)
    .map(([champion, count]) => ({ champion, count }))
    .sort((a, b) => a.count - b.count);
}

// Очистить всю статистику (для отладки)
export function clearChampionUsage(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
    console.log('🗑️ Статистика чемпионов очищена');
  } catch (error) {
    console.warn('Не удалось очистить статистику:', error);
  }
}
