import { createI18n } from 'vue-i18n'
import en from './locales/en.json'
import zh from './locales/zh.json'
import ja from './locales/ja.json'

const i18n = createI18n({
  legacy: false, // Use Composition API
  locale: 'zh', // Default locale
  fallbackLocale: 'en',
  messages: {
    en,
    zh,
    ja
  }
})

export default i18n
