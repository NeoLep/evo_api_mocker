import { createApp } from "vue";
import App from "./App.vue";
import i18n from "./i18n";
import { loader } from "@guolao/vue-monaco-editor";

loader.config({
  paths: {
    vs: import.meta.env.DEV 
      ? '/node_modules/monaco-editor/min/vs' 
      : '/monaco-editor/vs',
  },
});

createApp(App).use(i18n).mount("#app");
