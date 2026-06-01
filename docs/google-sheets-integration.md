# Автозаполнение Google-таблицы при выставлении

После успешного выставления аккаунта приложение может автоматически записывать
в твою Google-таблицу четыре поля: **Username**, **Offer ID**, **Listed Date** и **Status**.

Запись работает через веб-хук **Google Apps Script** — это самый простой способ
для десктопного приложения: не нужно хранить ключи Google внутри приложения и
расшаривать таблицу посторонним.

## Как это работает

1. После создания оффера на G2G приложение отправляет `POST`-запрос с JSON на
   твой URL веб-хука Apps Script:
   ```json
   {
     "username": "логин аккаунта (поле Login: из файла)",
     "offer_id": "G17...",
     "listed_date": "2026-06-01 14:32:05",
     "status": "Live"
   }
   ```
2. Скрипт Apps Script находит нужные колонки по их заголовкам и:
   - если в таблице уже есть строка с таким **Username** и пустым **Offer ID** — заполняет её;
   - иначе добавляет новую строку в конец.

> **Username** берётся из строки `Login:` в текстовом файле аккаунта. Если её нет,
> используется имя папки аккаунта.

## Настройка (один раз)

### Шаг 1. Открой редактор скриптов

1. Открой свою Google-таблицу.
2. Меню **Extensions → Apps Script** (Расширения → Apps Script).
3. Удали содержимое файла `Code.gs` и вставь скрипт ниже.

### Шаг 2. Вставь скрипт

```javascript
// Допустимые названия колонок (как в строке заголовков). Регистр не важен.
// Можно указать несколько вариантов на колонку — подойдёт любой из них.
const COL_USERNAME = ['Username'];
const COL_OFFER_ID = ['Offer ID'];
const COL_LISTED_DATE = ['Listed Data', 'Listed Date'];
const COL_STATUS = ['Status'];

// Сколько верхних строк просматривать в поисках строки заголовков.
const HEADER_SCAN_ROWS = 5;

// Возвращает номер столбца (1-based) для первого подходящего имени, иначе undefined.
function pickCol_(colIndex, names) {
  for (const n of names) {
    const c = colIndex[String(n).trim().toLowerCase()];
    if (c) return c;
  }
  return undefined;
}

// Находит лист и строку заголовков, где есть все 4 нужные колонки.
// Перебирает ВСЕ вкладки, чтобы не зависеть от их порядка/имени.
function findTarget_() {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  const sheets = ss.getSheets();
  for (let s = 0; s < sheets.length; s++) {
    const sheet = sheets[s];
    const lastCol = sheet.getLastColumn();
    if (lastCol === 0) continue;
    const scanRows = Math.min(HEADER_SCAN_ROWS, Math.max(1, sheet.getLastRow()));
    const grid = sheet.getRange(1, 1, scanRows, lastCol).getValues();

    for (let r = 0; r < grid.length; r++) {
      const colIndex = {};
      grid[r].forEach((h, i) => {
        const key = String(h).trim().toLowerCase();
        if (key) colIndex[key] = i + 1; // 1-based
      });
      const cUser = pickCol_(colIndex, COL_USERNAME);
      const cOffer = pickCol_(colIndex, COL_OFFER_ID);
      const cDate = pickCol_(colIndex, COL_LISTED_DATE);
      const cStatus = pickCol_(colIndex, COL_STATUS);
      if (cUser && cOffer && cDate && cStatus) {
        return { sheet: sheet, headerRow: r + 1, cUser, cOffer, cDate, cStatus };
      }
    }
  }
  return null;
}

function writeRow_(data) {
  const t = findTarget_();
  if (!t) {
    return {
      ok: false,
      error: 'Не найдена вкладка со всеми колонками: ' +
        [COL_USERNAME[0], COL_OFFER_ID[0], COL_LISTED_DATE[0], COL_STATUS[0]].join(', ') +
        '. Проверь, что заголовки написаны точно так же (в первых ' +
        HEADER_SCAN_ROWS + ' строках любой вкладки).'
    };
  }

  const sheet = t.sheet;
  const lastRow = sheet.getLastRow();

  // Ищем существующую строку с таким Username и пустым Offer ID — заполним её.
  let targetRow = -1;
  if (lastRow > t.headerRow) {
    const n = lastRow - t.headerRow;
    const usernames = sheet.getRange(t.headerRow + 1, t.cUser, n, 1).getValues();
    const offers = sheet.getRange(t.headerRow + 1, t.cOffer, n, 1).getValues();
    const wantUser = String(data.username || '').trim().toLowerCase();
    for (let i = 0; i < n; i++) {
      const u = String(usernames[i][0]).trim().toLowerCase();
      const o = String(offers[i][0]).trim();
      if (u && u === wantUser && o === '') {
        targetRow = t.headerRow + 1 + i;
        break;
      }
    }
  }

  if (targetRow === -1) {
    targetRow = lastRow + 1; // добавляем новую строку в конец
    sheet.getRange(targetRow, t.cUser).setValue(data.username || '');
  }

  sheet.getRange(targetRow, t.cOffer).setValue(data.offer_id || '');
  sheet.getRange(targetRow, t.cDate).setValue(data.listed_date || '');
  sheet.getRange(targetRow, t.cStatus).setValue(data.status || '');

  return { ok: true, sheet: sheet.getName(), headerRow: t.headerRow, row: targetRow };
}

function doPost(e) {
  try {
    const data = JSON.parse(e.postData.contents);
    const result = writeRow_(data);
    Logger.log(JSON.stringify(result));
    return json_(result);
  } catch (err) {
    Logger.log('ERROR: ' + err);
    return json_({ ok: false, error: String(err) });
  }
}

// Тест прямо из браузера: открой URL веб-приложения с ?test=1 в адресной строке.
// В таблицу запишется строка с username "TEST".
function doGet(e) {
  if (e && e.parameter && e.parameter.test) {
    const result = writeRow_({
      username: 'TEST',
      offer_id: 'TEST-OFFER',
      listed_date: new Date().toISOString(),
      status: 'TEST'
    });
    return json_(result);
  }
  return json_({ ok: true, message: 'Webhook is alive. Add ?test=1 to write a test row.' });
}

function json_(obj) {
  return ContentService
    .createTextOutput(JSON.stringify(obj))
    .setMimeType(ContentService.MimeType.JSON);
}
```

При необходимости поменяй названия колонок в константах `COL_*` вверху, если у тебя
они называются иначе. Имя вкладки указывать **не нужно** — скрипт сам найдёт вкладку,
в которой есть все 4 заголовка.

### Шаг 3. Опубликуй как веб-приложение

1. Нажми **Deploy → New deployment** (Развернуть → Новое развёртывание).
2. Тип: **Web app**.
3. *Execute as*: **Me** (от твоего имени).
4. *Who has access*: **Anyone** (нужно, чтобы приложение могло отправлять запрос
   без авторизации). Скрипт принимает только запись в твою таблицу, секретов он не раскрывает.
5. Нажми **Deploy**, разреши доступ, скопируй **Web app URL**
   (вида `https://script.google.com/macros/s/AKfy.../exec`).

### Шаг 4. Вставь URL в приложение

1. Открой приложение → **Настройки**.
2. Вставь скопированный URL в поле **Google Sheets Webhook URL**.
3. Сохрани настройки.

Готово. Теперь при каждом успешном выставлении строка в таблице заполняется автоматически.

## Диагностика: «выполнение завершено, но строки нет»

Если в логах Apps Script видно `doPost` и «Выполнение завершено», но в таблице
ничего не появилось — значит скрипт отработал, но не записал. Проверь по порядку:

1. **Обнови развёртывание после замены кода.** Это самое частое. После любой правки
   кода нужно: **Deploy → Manage deployments → ✎ (Edit) → Version: New version → Deploy**.
   Без этого работает старая версия скрипта. URL при этом остаётся прежним.

2. **Проверь через браузер.** Открой URL веб-приложения, дописав в конец `?test=1`:
   `https://script.google.com/macros/s/AKfy.../exec?test=1`
   - Ответ `{"ok":true,"sheet":"...","row":N}` — запись прошла, в таблице появилась
     тестовая строка `TEST` на вкладке `sheet`. Значит механика рабочая.
   - Ответ `{"ok":false,"error":"Не найдена вкладка со всеми колонками..."}` — скрипт
     не нашёл заголовки. Убедись, что в одной из вкладок в первых строках есть ровно
     колонки `Username`, `Offer ID`, `Listed Date`, `Status` (проверь точное написание
     и лишние пробелы). При других названиях поправь константы `COL_*` вверху скрипта.

3. **Смотри ответ в логах.** Новый скрипт логирует результат: **Executions →** открой
   запись `doPost` → в деталях будет JSON с `ok`, именем вкладки и номером строки.
   Если `ok:false` — там же причина.

4. **Куда именно записалось.** Скрипт сам выбирает вкладку, где есть все 4 заголовка
   (перебирает все вкладки, первые `HEADER_SCAN_ROWS` строк). Проверь все вкладки —
   возможно, данные ушли в другую, чем ты смотришь.

5. **Логика заполнения.** Если в таблице уже есть строка с таким же `Username` и
   **пустым** `Offer ID`, скрипт заполнит именно её, а не добавит новую. Если хочешь
   всегда добавлять новую строку — скажи, уберу этот поиск.

## Замечания

- Запись в таблицу — best-effort: если веб-хук недоступен, оффер всё равно создаётся,
  а ошибка просто пишется в консоль приложения.
- Если поле **Google Sheets Webhook URL** пустое — запись в таблицу отключена.
- При изменении кода скрипта нужно сделать **Deploy → Manage deployments → Edit →
  New version**, иначе изменения не применятся.
