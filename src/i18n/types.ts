import type { Composer } from 'vue-i18n'

export interface I18nMessages {
  app: {
    title: string
    newConnection: string
    settings: string
    selectConnectionToStart: string
  }
  connections: {
    edit: string
    delete: string
    new: string
    editTitle: string
    name: string
    host: string
    port: string
    username: string
    password: string
    proxyJump: string
    proxyJumpOptional: string
    jumpHost: string
    jumpUsername: string
    jumpPassword: string
    cancel: string
    save: string
  }
  settings: {
    title: string
    appearance: string
    language: string
    theme: string
    aiAssistant: string
    apiUrl: string
    apiKey: string
    modelName: string
    saveChanges: string
    cancel: string
  }
  fileManager: {
    toolbar: {
      upLevel: string
      pathPlaceholder: string
      refresh: string
      newFile: string
      newFolder: string
      uploadFile: string
      uploadDirectory: string
    }
    headers: {
      name: string
      size: string
      dateModified: string
      owner: string
    }
    emptyDirectory: string
    contextMenu: {
      download: string
      rename: string
      delete: string
      copyPath: string
      changePermissions: string
    }
    deleteConfirm: {
      title: string
      message: string
    }
  }
  transfers: {
    title: string
    clearCompleted: string
    pause: string
    resumeRetry: string
    cancel: string
    remove: string
    status: {
      running: string
      failed: string
      completed: string
      cancelled: string
      error: string
      paused: string
    }
  }
  aiAssistant: {
    welcome: string
  }
  themes: {
    light: string
    dark: string
  }
  languages: {
    en: string
    zh: string
  }
}

export type SupportedLocale = 'en' | 'zh'

export interface EnhancedI18n extends Composer {
  t: (key: keyof I18nMessages | string, ...args: any[]) => string
  te: (key: keyof I18nMessages | string) => boolean
}

export interface LocaleSettings {
  locale: SupportedLocale
  fallbackLocale: SupportedLocale
}