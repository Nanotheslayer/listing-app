import { invoke } from "@tauri-apps/api/core";
import { sortChampionsByUsage } from "./championTracking";

// Типы данных
export interface AccountData {
  server: string;
  level: number;
  honorLevel: number;
  championsCount: number;
  championsList: string[];
  skinsCount: number;
  skinsList: string[];
  riotPoints: number;
  blueEssence: number;
  orangeEssence: number;
  lastPlayDate: string;
  opggLink?: string;
}

export interface ParsedForm {
  title: string;
  description: string;
  usedChampions: string[];
}

// Чтение файла из папки аккаунта
async function readAccountFile(accountPath: string, fileName: string): Promise<string> {
  try {
    const content = await invoke<string>("read_account_file", {
      accountPath: accountPath,
      fileName: fileName
    });
    return content;
  } catch (error) {
    console.error(`Ошибка чтения файла ${fileName}:`, error);
    return "";
  }
}

// Извлечение числа из строки
function extractNumber(text: string, pattern: RegExp): number {
  const match = text.match(pattern);
  return match ? parseInt(match[1], 10) : 0;
}

// Извлечение списка элементов
function extractList(text: string, startMarker: string): string[] {
  const startIndex = text.indexOf(startMarker);
  if (startIndex === -1) return [];

  // Берем текст после маркера
  let listText = text.substring(startIndex + startMarker.length);

  // Находим конец списка - добавляем "\nList of" чтобы остановиться перед следующим списком
  const endMarkers = [
    "\n\n",
    "\n[",
    "\n─",
    "\nLink:",
    "\nRegion:",
    "\nList of"  // ВАЖНО: остановиться перед следующим списком
  ];
  let endIndex = listText.length;

  for (const marker of endMarkers) {
    const idx = listText.indexOf(marker);
    if (idx !== -1 && idx < endIndex) {
      endIndex = idx;
    }
  }

  listText = listText.substring(0, endIndex).trim();

  // Если список разделен запятыми
  if (listText.includes(",")) {
    return listText
      .split(",")
      .map(item => item.trim())
      .map(item => item.replace(/\.$/, "")) // Убираем точку в конце
      .filter(item => item.length > 0);
  }

  // Если список построчно
  return listText
    .split("\n")
    .map(item => item.replace(/^[•\-\*]\s*/, "").trim())
    .map(item => item.replace(/\.$/, "")) // Убираем точку в конце
    .filter(item => item.length > 0 && !item.startsWith("─") && !item.startsWith("["));
}

// Нормализация имени сервера - ОБНОВЛЕНО для поддержки полных названий
function normalizeServer(server: string): string {
  const serverMap: Record<string, string> = {
    "brazil": "BR",
    "br": "BR",
    "br1": "BR",
    "euw": "EUW",
    "euw1": "EUW",
    "europe west": "EUW",
    "eune": "EUNE",
    "eune1": "EUNE",
    "europe nordic & east": "EUNE",
    "europe nordic east": "EUNE",
    "na": "NA",
    "na1": "NA",
    "north america": "NA",
    "oce": "OCE",
    "oce1": "OCE",
    "oceania": "OCE",
    "las": "LAS",
    "las1": "LAS",
    "latin america south": "LAS",
    "lan": "LAN",
    "lan1": "LAN",
    "latin america north": "LAN",
    "tr": "TR",
    "tr1": "TR",
    "turkey": "TR",
    "ru": "RU",
    "ru1": "RU",
    "russia": "RU",
    "jp": "JP",
    "jp1": "JP",
    "japan": "JP",
    "kr": "KR",
    "korea": "KR",
  };

  const normalized = server.toLowerCase().trim();
  return serverMap[normalized] || server.toUpperCase();
}

// Извлечение сервера из текста - ИЗМЕНЕН ПРИОРИТЕТ
function extractServer(text: string): string {
  // 🎯 ПРИОРИТЕТ 1: Ищем в явной строке "Region: Brazil" (из файла {account}.txt)
  let match = text.match(/Region:\s*([^\n\r]+)/i);
  if (match) {
    const region = match[1].trim();
    console.log("✅ Найден регион в строке 'Region:':", region);
    return normalizeServer(region);
  }

  // 🎯 ПРИОРИТЕТ 2: Ищем в строке типа "Account(Server - Brazil)"
  match = text.match(/Account\s*\(\s*Server\s*-\s*([^)\n\r]+)\s*\)/i);
  if (match) {
    const region = match[1].trim();
    console.log("✅ Найден регион в 'Account(Server - ...)':", region);
    return normalizeServer(region);
  }

  // 🎯 ПРИОРИТЕТ 3: Ищем в ссылке OP.GG (может отсутствовать)
  match = text.match(/op\.gg\/summoners\/([a-z0-9]+)\//i);
  if (match) {
    console.log("✅ Найден регион в OP.GG:", match[1]);
    return normalizeServer(match[1]);
  }

  // 🎯 ПРИОРИТЕТ 4: Ищем в строке с общим паттерном "Server:"
  match = text.match(/Server\s*[-:]\s*([^\n\r]+)/i);
  if (match) {
    const region = match[1].trim();
    console.log("✅ Найден регион в 'Server:':", region);
    return normalizeServer(region);
  }

  // 🎯 ПРИОРИТЕТ 5: Ищем в имени файла (например, "uyep_br1_info.txt")
  match = text.match(/_([a-z]+\d?)_/i);
  if (match) {
    console.log("✅ Найден регион в имени файла:", match[1]);
    return normalizeServer(match[1]);
  }

  console.warn("⚠️ Регион не найден ни одним методом, используем Unknown");
  return "Unknown";
}

// Главная функция парсинга
export async function parseAccountData(accountPath: string, files: string[]): Promise<AccountData> {
  console.log("=== Начало парсинга аккаунта ===");
  console.log("Путь:", accountPath);
  console.log("Файлы:", files);

  let allContent = "";

  // Читаем только файлы с реальной информацией
  // Игнорируем Info.txt (пустой шаблон)
  for (const file of files) {
    const lowerFile = file.toLowerCase();

    // Пропускаем файл Info.txt (это просто шаблон)
    if (lowerFile === "info.txt") {
      console.log(`Пропускаем шаблон: ${file}`);
      continue;
    }

    // Читаем только .txt файлы
    if (lowerFile.endsWith(".txt")) {
      console.log(`Чтение файла: ${file}`);
      const content = await readAccountFile(accountPath, file);
      console.log(`Прочитано символов из ${file}:`, content.length);

      if (content.length > 0) {
        console.log(`Первые 500 символов из ${file}:`, content.substring(0, 500));
      }

      allContent += content + "\n\n";
    }
  }

  console.log("=== Общая длина контента:", allContent.length);

  if (allContent.length === 0) {
    console.error("ОШИБКА: Не удалось прочитать содержимое файлов!");
    throw new Error("Не удалось прочитать содержимое файлов");
  }

  console.log("Первые 1000 символов всего контента:", allContent.substring(0, 1000));

  // Парсим данные
  const server = extractServer(allContent);
  console.log("Извлеченный сервер:", server);

  const level = extractNumber(allContent, /Level\s*[-:]\s*(\d+)/i);
  console.log("Извлеченный уровень:", level);

  const championsCount = extractNumber(allContent, /Champions\s*[-:]\s*(\d+)/i);
  console.log("Количество чемпионов:", championsCount);

  const championsList = extractList(allContent, "List of Champions:");
  console.log("Список чемпионов:", championsList);

  const skinsList = extractList(allContent, "List of Skins:");
  console.log("Список скинов:", skinsList);

  const data: AccountData = {
    server,
    level,
    honorLevel: extractNumber(allContent, /Honor\s+level\s+is\s+(\d+)/i) || 3,
    championsCount,
    championsList,
    skinsCount: extractNumber(allContent, /Skins\s*[-:]\s*(\d+)/i),
    skinsList,
    riotPoints: extractNumber(allContent, /Riot\s+Points\s*[-:]\s*(\d+)/i),
    blueEssence: extractNumber(allContent, /Blue\s+Essence\s*[-:]\s*(\d+)/i),
    orangeEssence: extractNumber(allContent, /Orange\s+Essence\s*[-:]\s*(\d+)/i),
    lastPlayDate: (() => {
      const match = allContent.match(/⤱\s*Last Play \/ Inactive From\s*-\s*(.+)/i);
      return match ? match[1].trim() : "Unknown";
    })(),
  };

  // Пытаемся найти ссылку OP.GG
  const opggMatch = allContent.match(/(https?:\/\/[^\s]+op\.gg[^\s]+)/i);
  if (opggMatch) {
    data.opggLink = opggMatch[1];
  }

  console.log("=== Распарсенные данные:", JSON.stringify(data, null, 2));
  return data;
}

// Генерация заголовка
export function generateTitle(data: AccountData): { title: string; usedChampions: string[] } {
  const MAX_LENGTH = 128;

  // Базовая часть
  const baseTitle = `[${data.server} ⍜] - [${data.level} LVL | ${data.championsCount} Champions`;
  const endTitle = " | Handleveled | Full Access ⍜]";

  // Доступное пространство для скинов/чемпионов
  const availableSpace = MAX_LENGTH - baseTitle.length - endTitle.length;

  console.log(`Доступно символов для заголовка: ${availableSpace}`);

  // Если есть скины, добавляем их в приоритете
  const items: string[] = [];
  const usedChampions: string[] = []; // 👈 Отслеживаем использованные чемпионы
  let currentSpace = availableSpace;

  // Сначала добавляем скины
  if (data.skinsList.length > 0) {
    for (const skin of data.skinsList) {
      const itemLength = skin.length + 3; // +3 для " | "
      if (itemLength <= currentSpace) {
        items.push(skin);
        currentSpace -= itemLength;
      } else {
        break;
      }
    }
  }

  // Если осталось место, добавляем чемпионов
  // 👇 ИЗМЕНЕНИЕ: Сортируем чемпионов по частоте использования
  if (currentSpace > 0 && data.championsList.length > 0) {
    const sortedChampions = sortChampionsByUsage(data.championsList);
    console.log('📋 Чемпионы отсортированы по частоте использования');

    for (const champion of sortedChampions) {
      const itemLength = champion.length + 3; // +3 для " | "
      if (itemLength <= currentSpace) {
        items.push(champion);
        usedChampions.push(champion); // 👈 Запоминаем использованного чемпиона
        currentSpace -= itemLength;
      } else {
        break;
      }

      // Не добавляем слишком много чемпионов
      if (items.length >= 10) break;
    }
  }

  // Формируем итоговый заголовок
  const title = items.length > 0
    ? baseTitle + " | " + items.join(" | ") + endTitle
    : baseTitle + endTitle;

  console.log(`✅ Заголовок создан. Использовано чемпионов: ${usedChampions.length}`);

  return { title, usedChampions }; // 👈 Возвращаем и заголовок, и список чемпионов
}

// Генерация описания
export function generateDescription(data: AccountData): string {
  const lines: string[] = [
    "▸ Instant Auto-Delivery 24/7",
    "⤱ You must play 10 Quickplay or Draft games to unlock Ranked.",
    "⤱ Last Rank: The Account has never been ranked, but MMR is random.",
    "⤱ Current Rank: Unranked",
    `⤱ Last Play / Inactive From - ${data.lastPlayDate}`,
    "",
    `◉ Level - ${data.level}`,
    `◉ Honor level is ${data.honorLevel}`,
    `◉ Champions - ${data.championsCount}`,
    `◉ Skins - ${data.skinsCount}`,
    `◉ Riot Points - ${data.riotPoints}`,
    `◉ Blue Essence - ${data.blueEssence}`,
    `◉ Orange Essence - ${data.orangeEssence}`,
    "",
    "✓ Full Access [You can change the email, password, etc.]",
    "⍜ Completely Safe with 0% Banrate",
    "⮸ Hand-Leveled",
    "✫ Positive Reviews",
  ];

  // Добавляем список чемпионов
  if (data.championsList.length > 0) {
    lines.push("");
    lines.push("◉ List of Champions:");
    lines.push(data.championsList.join(", ") + ".");
  }

  // Добавляем список скинов
  if (data.skinsList.length > 0) {
    lines.push("");
    lines.push("◉ List of Skins:");
    lines.push(data.skinsList.join(", ") + ".");
  }

  return lines.join("\n");
}

// Главная функция автозаполнения
export async function autofillListing(accountPath: string, files: string[]): Promise<ParsedForm> {
  try {
    console.log("=== Запуск автозаполнения для аккаунта:", accountPath);

    // Парсим данные из файлов
    const data = await parseAccountData(accountPath, files);

    // 👇 ИЗМЕНЕНИЕ: Теперь получаем и заголовок, и список чемпионов
    const { title, usedChampions } = generateTitle(data);
    const description = generateDescription(data);

    console.log("=== Автозаполнение завершено ===");
    console.log("Длина заголовка:", title.length);
    console.log("Длина описания:", description.length);
    console.log("Использовано чемпионов:", usedChampions);

    // 👇 Возвращаем список использованных чемпионов
    return { title, description, usedChampions };
  } catch (error) {
    console.error("=== ОШИБКА автозаполнения ===", error);
    throw error;
  }
}

// Чтение личной информации из файла {accountName}.txt
export async function readPersonalInfo(accountPath: string, accountName: string): Promise<string> {
  try {
    const fileName = `${accountName}.txt`;
    console.log(`Reading personal info file: ${fileName}`);

    const content = await readAccountFile(accountPath, fileName);

    if (!content || content.trim().length === 0) {
      return "❌ Файл с личной информацией не найден или пуст";
    }

    return content;
  } catch (error) {
    console.error("Ошибка чтения личной информации:", error);
    return `❌ Не удалось прочитать файл: ${error instanceof Error ? error.message : String(error)}`;
  }
}