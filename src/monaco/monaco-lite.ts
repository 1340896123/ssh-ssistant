import {
  create,
  createModel,
  setModelLanguage,
} from "../shims/monaco-standalone-editor-lite.js";

export const editor = {
  create,
  createModel,
  setModelLanguage,
};

type MonacoLanguagesModule = typeof import("monaco-editor/esm/vs/editor/standalone/browser/standaloneLanguages.js");

let monacoLanguagesModule: MonacoLanguagesModule | null = null;

export async function loadMonacoLanguages() {
  if (!monacoLanguagesModule) {
    monacoLanguagesModule = await import(
      "monaco-editor/esm/vs/editor/standalone/browser/standaloneLanguages.js"
    );
  }
  return monacoLanguagesModule;
}
