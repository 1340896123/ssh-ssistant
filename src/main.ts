import { createApp } from "vue";
import { createPinia } from "pinia";
import "./style.css";
import App from "./App.vue";
import { createAppI18n } from "./i18n";

// 初始化应用
const initApp = async () => {
  // 获取保存的语言设置，默认为中文
  const savedLanguage = (localStorage.getItem('preferred-locale') as 'zh' | 'en') || 'zh';

  // 创建 i18n 实例
  const i18n = await createAppI18n(savedLanguage);

  const app = createApp(App);
  app.use(createPinia());
  app.use(i18n);
  app.mount("#app");
};

// 启动应用
initApp().catch(error => {
  console.error('Failed to initialize app:', error);
});
