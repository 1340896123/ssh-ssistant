import "monaco-editor/esm/vs/editor/standalone/browser/standalone-tokens.css";
import {
  StandaloneEditor,
  createTextModel,
} from "monaco-editor/esm/vs/editor/standalone/browser/standaloneCodeEditor.js";
import { StandaloneServices } from "monaco-editor/esm/vs/editor/standalone/browser/standaloneServices.js";
import { ILanguageService } from "monaco-editor/esm/vs/editor/common/languages/language.js";
import { PLAINTEXT_LANGUAGE_ID } from "monaco-editor/esm/vs/editor/common/languages/modesRegistry.js";
import { IModelService } from "monaco-editor/esm/vs/editor/common/services/model.js";
import { IStandaloneThemeService } from "monaco-editor/esm/vs/editor/standalone/common/standaloneTheme.js";

export function defineTheme(themeName, themeData) {
  StandaloneServices.get(IStandaloneThemeService).defineTheme(themeName, themeData);
}

export function create(domElement, options, override) {
  const instantiationService = StandaloneServices.initialize(override || {});
  return instantiationService.createInstance(StandaloneEditor, domElement, options);
}

export function createModel(value, language, uri) {
  const languageService = StandaloneServices.get(ILanguageService);
  const languageId = languageService.getLanguageIdByMimeType(language) || language;
  return createTextModel(
    StandaloneServices.get(IModelService),
    languageService,
    value,
    languageId,
    uri,
  );
}

export function setModelLanguage(model, mimeTypeOrLanguageId) {
  const languageService = StandaloneServices.get(ILanguageService);
  const languageId =
    languageService.getLanguageIdByMimeType(mimeTypeOrLanguageId) ||
    mimeTypeOrLanguageId ||
    PLAINTEXT_LANGUAGE_ID;
  model.setLanguage(languageService.createById(languageId));
}
