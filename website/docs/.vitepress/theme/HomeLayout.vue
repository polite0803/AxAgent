<script setup lang="ts">
import DefaultTheme from 'vitepress/theme';
import { useData } from 'vitepress';
import DownloadBar from './DownloadBar.vue';
import ImageCarousel from './ImageCarousel.vue';
import FeaturesGrid from './FeaturesGrid.vue';

const { Layout } = DefaultTheme;
const { frontmatter } = useData();
</script>

<template>
  <Layout>
    <template #home-hero-info-after>
      <div v-if="frontmatter?.layout === 'home'" class="hero-actions-custom">
        <DownloadBar />
      </div>
    </template>
    <template #home-features-before>
      <div v-if="frontmatter?.layout === 'home'">
        <ImageCarousel />
      </div>
    </template>
    <template #home-features-after>
      <div v-if="frontmatter?.layout === 'home'">
        <FeaturesGrid />
      </div>
    </template>
    <template #layout-bottom>
      <footer class="site-footer">
        <p>© 2026 AxAgent</p>
      </footer>
    </template>
  </Layout>
</template>

<style scoped>
.hero-actions-custom {
  width: 100%;
}
.site-footer {
  text-align: center;
  padding: 24px 0;
  color: var(--vp-c-text-3);
  font-size: 14px;
  border-top: 1px solid var(--vp-c-divider);
}
.site-footer p {
  margin: 0;
}
</style>
