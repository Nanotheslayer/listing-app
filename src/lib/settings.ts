import { invoke } from "@tauri-apps/api/core";

// Типы для настроек
export interface G2GSettings {
  user_id: string;
  refresh_token: string;
  long_lived_token: string;
  active_device_token: string;
}

export interface AppSettings {
  g2g: G2GSettings;
  theme?: "dark" | "light";
}

// Класс для управления настройками
export class SettingsManager {
  private static instance: SettingsManager;
  
  private constructor() {}
  
  static getInstance(): SettingsManager {
    if (!SettingsManager.instance) {
      SettingsManager.instance = new SettingsManager();
    }
    return SettingsManager.instance;
  }

  // Загрузить настройки
  async loadSettings(): Promise<AppSettings | null> {
    try {
      const settings = await invoke<AppSettings>("load_settings");
      return settings;
    } catch (error) {
      console.error("Failed to load settings:", error);
      return null;
    }
  }

  // Сохранить настройки
  async saveSettings(settings: AppSettings): Promise<void> {
    try {
      await invoke("save_settings", { settings });
      console.log("✅ Settings saved successfully");
    } catch (error) {
      console.error("❌ Failed to save settings:", error);
      throw error;
    }
  }

  // Проверить наличие настроек G2G
  async hasG2GSettings(): Promise<boolean> {
    try {
      const settings = await this.loadSettings();
      if (!settings || !settings.g2g) {
        return false;
      }
      
      const g2g = settings.g2g;
      return !!(
        g2g.user_id &&
        g2g.refresh_token &&
        g2g.long_lived_token &&
        g2g.active_device_token
      );
    } catch {
      return false;
    }
  }

  // Валидация настроек G2G
  validateG2GSettings(g2g: G2GSettings): string[] {
    const errors: string[] = [];
    
    if (!g2g.user_id || g2g.user_id.trim().length === 0) {
      errors.push("User ID не может быть пустым");
    }
    
    if (!g2g.refresh_token || g2g.refresh_token.trim().length === 0) {
      errors.push("Refresh Token не может быть пустым");
    }
    
    if (!g2g.long_lived_token || g2g.long_lived_token.trim().length === 0) {
      errors.push("Long Lived Token не может быть пустым");
    }
    
    if (!g2g.active_device_token || g2g.active_device_token.trim().length === 0) {
      errors.push("Active Device Token не может быть пустым");
    }
    
    return errors;
  }

  // Очистить настройки
  async clearSettings(): Promise<void> {
    try {
      await invoke("clear_settings");
      console.log("✅ Settings cleared");
    } catch (error) {
      console.error("❌ Failed to clear settings:", error);
      throw error;
    }
  }
}

// Экспортируем singleton
export const settingsManager = SettingsManager.getInstance();

// Хелпер для проверки настроек перед использованием API
export async function ensureG2GSettings(): Promise<void> {
  const hasSettings = await settingsManager.hasG2GSettings();
  if (!hasSettings) {
    throw new Error(
      "G2G токены не настроены. Пожалуйста, перейдите в настройки и укажите токены."
    );
  }
}
