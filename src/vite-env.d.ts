/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

interface ImportMetaEnv {
  readonly VITE_APP_VARIANT?: "local" | "personal" | "enterprise";
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
