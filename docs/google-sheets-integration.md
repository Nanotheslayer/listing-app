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
// Название листа (вкладки), в который писать. Поставь имя своей вкладки.
// Если оставить пустым, скрипт возьмёт первый лист таблицы.
const SHEET_NAME = '';

// Названия колонок в таблице (как в строке заголовков). Регистр не важен.
const COL_USERNAME = 'Username';
const COL_OFFER_ID = 'Offer ID';
const COL_LISTED_DATE = 'Listed Date';
const COL_STATUS = 'Status';

function doPost(e) {
  try {
    const data = JSON.parse(e.postData.contents);

    const ss = SpreadsheetApp.getActiveSpreadsheet();
    const sheet = SHEET_NAME ? ss.getSheetByName(SHEET_NAME) : ss.getSheets()[0];
    if (!sheet) {
      return json_({ ok: false, error: 'Sheet not found: ' + SHEET_NAME });
    }

    // Читаем заголовки и строим карту "имя колонки -> номер столбца".
    const lastCol = sheet.getLastColumn();
    const headers = sheet.getRange(1, 1, 1, lastCol).getValues()[0];
    const colIndex = {};
    headers.forEach((h, i) => {
      colIndex[String(h).trim().toLowerCase()] = i + 1; // 1-based
    });

    const cUser = colIndex[COL_USERNAME.toLowerCase()];
    const cOffer = colIndex[COL_OFFER_ID.toLowerCase()];
    const cDate = colIndex[COL_LISTED_DATE.toLowerCase()];
    const cStatus = colIndex[COL_STATUS.toLowerCase()];

    if (!cUser || !cOffer || !cDate || !cStatus) {
      return json_({
        ok: false,
        error: 'Не найдены колонки. Проверь заголовки: ' +
               [COL_USERNAME, COL_OFFER_ID, COL_LISTED_DATE, COL_STATUS].join(', ')
      });
    }

    // Ищем существующую строку с таким Username и пустым Offer ID.
    let targetRow = -1;
    const lastRow = sheet.getLastRow();
    if (lastRow >= 2) {
      const usernames = sheet.getRange(2, cUser, lastRow - 1, 1).getValues();
      const offers = sheet.getRange(2, cOffer, lastRow - 1, 1).getValues();
      for (let i = 0; i < usernames.length; i++) {
        const u = String(usernames[i][0]).trim().toLowerCase();
        const o = String(offers[i][0]).trim();
        if (u && u === String(data.username).trim().toLowerCase() && o === '') {
          targetRow = i + 2; // +2: строка 1 — заголовки, индекс с 0
          break;
        }
      }
    }

    if (targetRow === -1) {
      // Новой строки нет — добавляем в конец.
      targetRow = lastRow + 1;
      sheet.getRange(targetRow, cUser).setValue(data.username || '');
    }

    sheet.getRange(targetRow, cOffer).setValue(data.offer_id || '');
    sheet.getRange(targetRow, cDate).setValue(data.listed_date || '');
    sheet.getRange(targetRow, cStatus).setValue(data.status || '');

    return json_({ ok: true, row: targetRow });
  } catch (err) {
    return json_({ ok: false, error: String(err) });
  }
}

function json_(obj) {
  return ContentService
    .createTextOutput(JSON.stringify(obj))
    .setMimeType(ContentService.MimeType.JSON);
}
```

При необходимости поменяй константы вверху:
- `SHEET_NAME` — имя вкладки (если в файле несколько листов);
- `COL_*` — если твои колонки называются иначе.

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

## Замечания

- Запись в таблицу — best-effort: если веб-хук недоступен, оффер всё равно создаётся,
  а ошибка просто пишется в консоль приложения.
- Если поле **Google Sheets Webhook URL** пустое — запись в таблицу отключена.
- При изменении кода скрипта нужно сделать **Deploy → Manage deployments → Edit →
  New version**, иначе изменения не применятся.
