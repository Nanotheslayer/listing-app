import { invoke } from "@tauri-apps/api/core";

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
}

// Чтение файла из папки аккаунта
async function readAccountFile(accountPath: string, fileName: string): Promise<string> {
  try {
    const filePath = `${accountPath}/${fileName}`;
    const content = await invoke<string>("read_text_file", { path: filePath });
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
  
  // Находим конец списка (до следующего заголовка или пустой строки)
  const endMarkers = ["\n\n[", "\n\n◉", "\n\nList of", "\n\n─"];
  let endIndex = listText.length;
  
  for (const marker of endMarkers) {
    const idx = listText.indexOf(marker);
    if (idx !== -1 && idx < endIndex) {
      endIndex = idx;
    }
  }
  
  listText = listText.substring(0, endIndex).trim();
  
  // Удаляем маркеры списка
  listText = listText.replace(/^[•\-\*]\s*/gm, "");
  
  // Разбиваем по запятым или переносам строк
  let items: string[];
  if (listText.includes(",")) {
    items = listText.split(",");
  } else {
    items = listText.split("\n");
  }
  
  // Очищаем каждый элемент
  return items
    .map((item) => item.trim())
    .filter((item) => item.length > 0 && !item.startsWith("─") && !item.startsWith("["));
}

// Нормализация имени сервера
function normalizeServer(server: string): string {
  const serverMap: Record<string, string> = {
    "brazil": "BR1",
    "br": "BR1",
    "br1": "BR1",
    "euw": "EUW",
    "euw1": "EUW",
    "eune": "EUNE",
    "eune1": "EUNE",
    "na": "NA",
    "na1": "NA",
    "oce": "OCE",
    "oce1": "OCE",
    "las": "LAS",
    "las1": "LAS",
    "lan": "LAN",
    "lan1": "LAN",
    "tr": "TR",
    "tr1": "TR",
    "ru": "RU",
    "ru1": "RU",
    "jp": "JP",
    "jp1": "JP",
    "kr": "KR",
  };

  const normalized = server.toLowerCase().trim();
  return serverMap[normalized] || server.toUpperCase();
}

// Извлечение сервера из текста
function extractServer(text: string): string {
  // Ищем в строке типа "Account(Server - Brazil)"
  let match = text.match(/Server\s*[-:]\s*([A-Za-z0-9]+)/i);
  if (match) return normalizeServer(match[1]);

  // Ищем в ссылке OP.GG
  match = text.match(/op\.gg\/summoners\/([a-z0-9]+)\//i);
  if (match) return normalizeServer(match[1]);

  // Ищем в URL типа "br1" или "euw1"
  match = text.match(/[\/\-]([a-z]+\d?)[\/_]/i);
  if (match) return normalizeServer(match[1]);

  return "Unknown";
}

// Главная функция парсинга
export async function parseAccountData(accountPath: string, files: string[]): Promise<AccountData> {
  console.log("Начало парсинга аккаунта:", accountPath);
  console.log("Файлы:", files);

  let allContent = "";
  
  // Читаем все .txt файлы
  for (const file of files) {
    if (file.toLowerCase().endsWith(".txt")) {
      const content = await readAccountFile(accountPath, file);
      allContent += content + "\n\n";
    }
  }

  console.log("Прочитано символов:", allContent.length);

  // Парсим данные
  const data: AccountData = {
    server: extractServer(allContent),
    level: extractNumber(allContent, /Level\s*[-:]\s*(\d+)/i),
    honorLevel: extractNumber(allContent, /Honor\s+level\s+is\s+(\d+)/i) || 3,
    championsCount: extractNumber(allContent, /Champions\s*[-:]\s*(\d+)/i),
    championsList: extractList(allContent, "List of Champions:"),
    skinsCount: extractNumber(allContent, /Skins\s*[-:]\s*(\d+)/i),
    skinsList: extractList(allContent, "List of Skins:"),
    riotPoints: extractNumber(allContent, /Riot\s+Points\s*[-:]\s*(\d+)/i),
    blueEssence: extractNumber(allContent, /Blue\s+Essence\s*[-:]\s*(\d+)/i),
    orangeEssence: extractNumber(allContent, /Orange\s+Essence\s*[-:]\s*(\d+)/i),
    lastPlayDate: "Unknown",
  };

  // Пытаемся найти дату последней игры
  const dateMatch = allContent.match(/Last\s+Play[^0-9]*([0-9\-\/\.]+)/i);
  if (dateMatch) {
    data.lastPlayDate = dateMatch[1];
  }

  // Пытаемся найти ссылку OP.GG
  const opggMatch = allContent.match(/(https?:\/\/[^\s]+op\.gg[^\s]+)/i);
  if (opggMatch) {
    data.opggLink = opggMatch[1];
  }

  console.log("Распарсенные данные:", data);
  return data;
}

// Генерация заголовка
export function generateTitle(data: AccountData): string {
  const MAX_LENGTH = 128;
  
  // Базовая часть
  const baseTitle = `[${data.server} ⍜] - [${data.level} LVL | ${data.championsCount} Champions`;
  const endTitle = " | Handleveled | Full Access ⍜]";
  
  // Доступное пространство для скинов/чемпионов
  const availableSpace = MAX_LENGTH - baseTitle.length - endTitle.length;
  
  console.log(`Доступно символов для заголовка: ${availableSpace}`);
  
  // Если есть скины, добавляем их в приоритете
  const items: string[] = [];
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
  if (currentSpace > 0 && data.championsList.length > 0) {
    for (const champion of data.championsList) {
      const itemLength = champion.length + 3; // +3 для " | "
      if (itemLength <= currentSpace) {
        items.push(champion);
        currentSpace -= itemLength;
      } else {
        break;
      }
      
      // Не добавляем слишком много чемпионов
      if (items.length >= 10) break;
    }
  }
  
  // Формируем итоговый заголовок
  if (items.length > 0) {
    const itemsPart = " | " + items.join(" | ");
    return baseTitle + itemsPart + endTitle;
  }
  
  return baseTitle + endTitle;
}

// Генерация описания
export function generateDescription(data: AccountData): string {
  const lines: string[] = [
    "⮸Full info into the media⮸\n",
    "▸ Instant Auto-Delivery 24/7",
    "⤱ You must play 10 Quickplay or Draft games to unlock Ranked.",
    "⤱ Last Rank: The Account has never been ranked, but MMR is random.",
    "⤱ Current Rank: Unranked",
    `⤱ Last Play / Inactive From - ${data.lastPlayDate}\n`,
    `◉ Level - ${data.level}`,
    `◉ Honor level is ${data.honorLevel}`,
    `◉ Champions - ${data.championsCount}`,
    `◉ Skins - ${data.skinsCount}`,
    `◉ Riot Points - ${data.riotPoints}`,
    `◉ Blue Essence - ${data.blueEssence}`,
    `◉ Orange Essence - ${data.orangeEssence}\n`,
    "✓ Full Access [You can change the email, password, etc.]",
    "⍜ Completely Safe with 0% Banrate",
    "⮸ Hand-Leveled",
    "✫ Positive Reviews\n",
  ];

  // Добавляем список чемпионов
  if (data.championsList.length > 0) {
    lines.push("◉ List of Champions:\n");
    lines.push(data.championsList.join(", ") + ".\n");
  }

  // Добавляем список скинов
  if (data.skinsList.length > 0) {
    lines.push("◉ List of Skins:\n");
    lines.push(data.skinsList.join(", ") + ".");
  }

  return lines.join("\n");
}

// Главная функция автозаполнения
export async function autofillListing(accountPath: string, files: string[]): Promise<ParsedForm> {
  try {
    console.log("Запуск автозаполнения для аккаунта:", accountPath);
    
    // Парсим данные из файлов
    const data = await parseAccountData(accountPath, files);
    
    // Генерируем заголовок и описание
    const title = generateTitle(data);
    const description = generateDescription(data);
    
    console.log("Автозаполнение завершено");
    console.log("Длина заголовка:", title.length);
    console.log("Длина описания:", description.length);
    
    return { title, description };
  } catch (error) {
    console.error("Ошибка автозаполнения:", error);
    throw error;
  }
}
