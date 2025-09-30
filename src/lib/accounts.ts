import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";

// Типы данных
export interface AccountFolder {
  name: string;
  path: string;
}

export interface AccountsData {
  accounts: AccountFolder[];
  base_path: string;
}

export interface Account {
  id: number;
  name: string;
  path: string;
  status: "loaded" | "processing" | "listed" | "error";
  files?: string[];
}

export interface LoadAccountsResult {
  success: boolean;
  message: string;
  accounts: Account[];
}

// Класс для управления аккаунтами
export class AccountManager {
  private accounts: Account[] = [];
  private basePath: string = "";
  private nextId: number = 1;
  private lastSelectedPath: string = "";
  private readonly STORAGE_KEY = "g2g_last_folder_path";
  private initialized: boolean = false;

  constructor() {
    // Синхронный конструктор - инициализацию делаем отдельно
    this.loadLastPathSync();
  }

  // Синхронная загрузка из localStorage
  private loadLastPathSync(): void {
    try {
      const saved = localStorage.getItem(this.STORAGE_KEY);
      if (saved) {
        this.lastSelectedPath = saved;
        console.log("Загружен сохраненный путь:", saved);
        this.initialized = true;
      }
    } catch (error) {
      console.warn("Не удалось загрузить последний путь:", error);
    }
  }

  // Асинхронная инициализация с домашней директорией
  private async ensureInitialized(): Promise<void> {
    if (!this.initialized || !this.lastSelectedPath) {
      try {
        this.lastSelectedPath = await homeDir();
        this.initialized = true;
        console.log("Используем домашнюю директорию:", this.lastSelectedPath);
      } catch (e) {
        console.error("Не удалось получить домашнюю директорию:", e);
      }
    }
  }

  // Сохранить последний путь в localStorage
  private saveLastPath(path: string): void {
    try {
      this.lastSelectedPath = path;
      localStorage.setItem(this.STORAGE_KEY, path);
      console.log("Путь сохранен в localStorage:", path);
    } catch (error) {
      console.warn("Не удалось сохранить последний путь:", error);
    }
  }

  // Открыть диалог выбора папки и загрузить аккаунты
  async selectAndLoadAccounts(): Promise<LoadAccountsResult> {
    try {
      // Убедимся что путь инициализирован
      await this.ensureInitialized();

      console.log("Открываем диалог с начальным путем:", this.lastSelectedPath);

      // Формируем опции для диалога
      const dialogOptions: Parameters<typeof open>[0] = {
        directory: true,
        multiple: false,
        title: "Выберите папку с аккаунтами",
      };

      // Добавляем defaultPath только если он есть и не пустой
      if (this.lastSelectedPath) {
        dialogOptions.defaultPath = this.lastSelectedPath;
      }

      // Открываем диалог выбора папки
      const selectedPath = await open(dialogOptions);

      console.log("Пользователь выбрал:", selectedPath);

      if (!selectedPath || selectedPath === null) {
        return {
          success: false,
          message: "Папка не выбрана",
          accounts: [],
        };
      }

      // Сохраняем выбранный путь
      const pathStr = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;
      this.saveLastPath(pathStr);

      // Загружаем папки аккаунтов из выбранной директории
      const result = await invoke<AccountsData>("load_account_folders", {
        folderPath: pathStr,
      });

      this.basePath = result.base_path;

      // Преобразуем в формат Account
      this.accounts = result.accounts.map((folder) => ({
        id: this.nextId++,
        name: folder.name,
        path: folder.path,
        status: "loaded" as const,
      }));

      return {
        success: true,
        message: `Загружено ${this.accounts.length} аккаунтов из папки "${this.getBaseFolderName()}"`,
        accounts: this.getAccounts(),
      };
    } catch (error) {
      console.error("Ошибка загрузки аккаунтов:", error);
      return {
        success: false,
        message: `Ошибка: ${error}`,
        accounts: [],
      };
    }
  }

  // Получить список файлов для конкретного аккаунта
  async getAccountFiles(accountId: number): Promise<string[]> {
    const account = this.accounts.find((acc) => acc.id === accountId);

    if (!account) {
      throw new Error("Аккаунт не найден");
    }

    // Если файлы уже загружены, возвращаем их
    if (account.files) {
      return account.files;
    }

    // Иначе загружаем с бэкенда
    try {
      const files = await invoke<string[]>("get_account_files", {
        accountPath: account.path,
      });

      // Сохраняем файлы в объекте аккаунта
      account.files = files;
      return files;
    } catch (error) {
      console.error(`Ошибка получения файлов для аккаунта ${account.name}:`, error);
      throw error;
    }
  }

  // Получить все аккаунты
  getAccounts(): Account[] {
    return [...this.accounts];
  }

  // Получить аккаунт по ID
  getAccount(id: number): Account | undefined {
    return this.accounts.find((acc) => acc.id === id);
  }

  // Обновить статус аккаунта
  updateAccountStatus(id: number, status: Account["status"]): void {
    const account = this.accounts.find((acc) => acc.id === id);
    if (account) {
      account.status = status;
    }
  }

  // Удалить аккаунт из списка
  removeAccount(id: number): boolean {
    const index = this.accounts.findIndex((acc) => acc.id === id);
    if (index !== -1) {
      this.accounts.splice(index, 1);
      return true;
    }
    return false;
  }

  // Очистить все аккаунты
  clearAccounts(): void {
    this.accounts = [];
    this.basePath = "";
    this.nextId = 1;
  }

  // Получить имя базовой папки
  getBaseFolderName(): string {
    if (!this.basePath) return "";
    const parts = this.basePath.split(/[/\\]/);
    return parts[parts.length - 1] || this.basePath;
  }

  // Получить количество аккаунтов
  getCount(): number {
    return this.accounts.length;
  }

  // Получить последний выбранный путь
  getLastSelectedPath(): string {
    return this.lastSelectedPath;
  }

  // Очистить последний путь
  clearLastPath(): void {
    this.lastSelectedPath = "";
    try {
      localStorage.removeItem(this.STORAGE_KEY);
    } catch (error) {
      console.warn("Не удалось очистить последний путь:", error);
    }
  }
}

// Экспортируем singleton instance
export const accountManager = new AccountManager();