import type { Composer } from 'vue-i18n'

export interface I18nMessages {
  app: {
    title: string
    newConnection: string
    settings: string
    selectConnectionToStart: string
    connections: string
    tunnels: string
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
    tunnels: string
  }
  tunnels: {
    title: string
    new: string
    name: string
    type: string
    local: string
    remote: string
    dynamic: string
    localHost: string
    localPort: string
    remoteHost: string
    remotePort: string
    remoteBindHost: string
    proxyJump: string
    proxyCommand: string
    agentForwarding: string
    start: string
    stop: string
    save: string
    cancel: string
    refresh: string
    connection: string
    allConnections: string
    manage: string
    none: string
    selectConnection: string
    deleteConfirm: string
    connectionMissing: string
    delete: string
    inactive: string
    active: string
    mapping: string
    required: string
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
    providerType: string
    saveChanges: string
    cancel: string
  }
  aiProviders: {
    openai: string
    anthropic: string
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
