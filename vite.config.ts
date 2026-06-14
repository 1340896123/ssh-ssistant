import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { readFileSync } from "node:fs";
import path from "node:path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
const emptyMonacoModule = path.resolve(
  __dirname,
  "src",
  "shims",
  "empty-monaco-module.js",
);
const monacoEditorLanguageShimModules = new Set([
  "javascript",
  "typescript",
  "html",
  "css",
  "python",
  "rust",
  "markdown",
  "shell",
  "yaml",
  "xml",
  "sql",
  "go",
  "java",
  "cpp",
  "csharp",
  "php",
  "dockerfile",
  "ini",
  "bat",
  "powershell",
]);
const strippedMonacoStandaloneModules = [
  "editor/contrib/codeAction/browser/codeActionContributions.js",
  "editor/contrib/codelens/browser/codelensController.js",
  "editor/contrib/colorPicker/browser/colorPickerContribution.js",
  "editor/contrib/documentSymbols/browser/documentSymbols.js",
  "editor/contrib/floatingMenu/browser/floatingMenu.contribution.js",
  "editor/contrib/gotoError/browser/gotoError.js",
  "editor/contrib/gotoSymbol/browser/goToCommands.js",
  "editor/contrib/gotoSymbol/browser/link/goToDefinitionAtPosition.js",
  "editor/contrib/hover/browser/hoverContribution.js",
  "editor/contrib/inlayHints/browser/inlayHintsContribution.js",
  "editor/contrib/inlineCompletions/browser/inlineCompletions.contribution.js",
  "editor/contrib/inlineProgress/browser/inlineProgress.js",
  "editor/contrib/linkedEditing/browser/linkedEditing.js",
  "editor/contrib/links/browser/links.js",
  "editor/contrib/parameterHints/browser/parameterHints.js",
  "editor/contrib/placeholderText/browser/placeholderText.contribution.js",
  "editor/contrib/rename/browser/rename.js",
  "editor/contrib/sectionHeaders/browser/sectionHeaders.js",
  "editor/contrib/stickyScroll/browser/stickyScrollContribution.js",
  "editor/contrib/suggest/browser/suggestController.js",
  "editor/contrib/suggest/browser/suggestInlineCompletions.js",
  "editor/contrib/unicodeHighlighter/browser/unicodeHighlighter.js",
  "editor/contrib/wordHighlighter/browser/wordHighlighter.js",
  "editor/standalone/browser/iPadShowKeyboard/iPadShowKeyboard.js",
  "editor/standalone/browser/inspectTokens/inspectTokens.js",
  "editor/standalone/browser/quickAccess/standaloneHelpQuickAccess.js",
  "editor/standalone/browser/quickAccess/standaloneGotoLineQuickAccess.js",
  "editor/standalone/browser/quickAccess/standaloneGotoSymbolQuickAccess.js",
  "editor/standalone/browser/quickAccess/standaloneCommandsQuickAccess.js",
  "editor/standalone/browser/referenceSearch/standaloneReferenceSearch.js",
  "editor/standalone/browser/toggleHighContrast/toggleHighContrast.js",
];

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    {
      name: "strip-monaco-standalone-extras",
      load(id) {
        const normalizedId = id.replace(/\\/g, "/");
        const match = normalizedId.match(
          /\/monaco-editor\/esm\/vs\/basic-languages\/([^/]+)\/\1\.js$/,
        );
        if (!match) {
          return null;
        }

        const languageId = match[1];
        if (!monacoEditorLanguageShimModules.has(languageId)) {
          return null;
        }

        const source = readFileSync(id, "utf8");
        const exportIndex = source.lastIndexOf("export { conf, language };");
        if (exportIndex === -1) {
          return null;
        }

        const confIndex = source.indexOf("const conf =");
        if (confIndex === -1) {
          return null;
        }

        return source.slice(confIndex);
      },
      resolveId(source, importer) {
        if (!importer) {
          return null;
        }
        const importerPath = importer.replace(/\\/g, "/");
        const sourcePath = source.replace(/\\/g, "/");
        if (!importerPath.includes("/monaco-editor/esm/")) {
          return null;
        }
        if (
          strippedMonacoStandaloneModules.some((fragment) =>
            sourcePath.includes(fragment),
          )
        ) {
          return emptyMonacoModule;
        }
        return null;
      },
    },
    vue(),
  ],
  resolve: {
    alias: [
      {
        find: "vue-i18n",
        replacement: "vue-i18n/dist/vue-i18n.runtime.esm-bundler.js",
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]contrib[\\/]floatingMenu[\\/]browser[\\/]floatingMenu\.contribution\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]iPadShowKeyboard[\\/]iPadShowKeyboard\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]inspectTokens[\\/]inspectTokens\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]quickAccess[\\/]standaloneHelpQuickAccess\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]quickAccess[\\/]standaloneGotoLineQuickAccess\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]quickAccess[\\/]standaloneGotoSymbolQuickAccess\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]quickAccess[\\/]standaloneCommandsQuickAccess\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]referenceSearch[\\/]standaloneReferenceSearch\.js$/,
        replacement: emptyMonacoModule,
      },
      {
        find: /(?:^|.*[\\/])editor[\\/]standalone[\\/]browser[\\/]toggleHighContrast[\\/]toggleHighContrast\.js$/,
        replacement: emptyMonacoModule,
      },
    ],
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/toggleHighContrast")) {
            return "monaco-toggle-high-contrast";
          }
          if (id.includes("src/monaco/monaco-lite.ts")) {
            return "monaco-lite";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/standaloneLanguages")) {
            return "monaco-languages";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/standaloneThemeService")) {
            return "monaco-editor";
          }
          if (id.includes("monaco-editor/esm/vs/editor/editor.api2")) {
            return "monaco-editor";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/standaloneEditor")) {
            return "monaco-editor";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/quickAccess")) {
            return "monaco-editor-quick-access";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/referenceSearch")) {
            return "monaco-editor-reference-search";
          }
          if (id.includes("monaco-editor/esm/vs/editor/standalone/browser/inspectTokens")) {
            return "monaco-editor-inspect-tokens";
          }
          return undefined;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1422,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri` and Rust build outputs
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
}));
