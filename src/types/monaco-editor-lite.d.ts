declare module "monaco-editor/esm/vs/editor/standalone/browser/standaloneEditor.js" {
  export const create: typeof import("monaco-editor")["editor"]["create"];
  export const createModel: typeof import("monaco-editor")["editor"]["createModel"];
  export const setModelLanguage: typeof import("monaco-editor")["editor"]["setModelLanguage"];
}

declare module "monaco-editor/esm/vs/editor/standalone/browser/standaloneLanguages.js" {
  export const register: typeof import("monaco-editor")["languages"]["register"];
  export const setLanguageConfiguration: typeof import("monaco-editor")["languages"]["setLanguageConfiguration"];
  export const setMonarchTokensProvider: typeof import("monaco-editor")["languages"]["setMonarchTokensProvider"];
  export const setTokensProvider: typeof import("monaco-editor")["languages"]["setTokensProvider"];
}
