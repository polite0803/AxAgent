import { inBrowser, useRouter } from "vitepress";
import DefaultTheme from "vitepress/theme";
import { onMounted } from "vue";
import DownloadHero from "./DownloadHero.vue";
import HomeLayout from "./HomeLayout.vue";
import "./custom.css";

export default {
  extends: DefaultTheme,
  Layout: HomeLayout,
  enhanceApp({ app }) {
    app.component("DownloadHero", DownloadHero);
  },
  setup() {
    const router = useRouter();

    onMounted(() => {
      if (!inBrowser) { return; }
      const path = window.location.pathname;
      if (path !== "/") { return; }
      const lang = navigator.language || navigator.languages?.[0] || "";
      if (/^zh/i.test(lang)) {
        router.go("/zh/");
      }
    });
  },
};
