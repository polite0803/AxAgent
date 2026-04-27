import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import ar from "./locales/ar.json";
import de from "./locales/de.json";
import enUS from "./locales/en-US.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import hi from "./locales/hi.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import ru from "./locales/ru.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

i18n
  .use(initReactI18next)
  .init({
    resources: {
      "zh-CN": { translation: zhCN },
      "zh-TW": { translation: zhTW },
      "en-US": { translation: enUS },
      ja: { translation: ja },
      ko: { translation: ko },
      fr: { translation: fr },
      de: { translation: de },
      es: { translation: es },
      ru: { translation: ru },
      hi: { translation: hi },
      ar: { translation: ar },
    },
    lng: "zh-CN",
    fallbackLng: "zh-CN",
    interpolation: { escapeValue: false },
  });

export default i18n;
