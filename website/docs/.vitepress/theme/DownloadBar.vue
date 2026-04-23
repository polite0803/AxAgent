<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useData } from 'vitepress';
import {
  AppleOutlined,
  WindowsOutlined,
  ReadOutlined,
} from '@ant-design/icons-vue';

declare const __APP_VERSION__: string;
const VERSION = __APP_VERSION__;

const { lang } = useData();
const isZh = computed(() => lang.value === 'zh-CN');

const detectedOS = ref<'macos' | 'windows' | 'linux'>('macos');

onMounted(() => {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('win')) {
    detectedOS.value = 'windows';
  } else if (ua.includes('linux')) {
    detectedOS.value = 'linux';
  } else {
    detectedOS.value = 'macos';
  }
});
</script>

<template>
  <div class="download-bar">
    <a href="/download" class="dl-btn dl-primary">
      <AppleOutlined v-if="detectedOS === 'macos'" class="os-icon" />
      <WindowsOutlined v-else-if="detectedOS === 'windows'" class="os-icon" />
      <svg v-else class="os-icon" xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 448 512" fill="currentColor"><path d="M220.8 123.3c1 .5 1.8 1.7 3 1.7 1.1 0 2.8-.4 2.9-1.5.2-1.4-1.9-2.3-3.2-2.9-1.7-.7-3.9-1-5.5-.1-.4.2-.8.7-.6 1.1.3 1.3 2.3 1.1 3.4 1.7zm-21.9 1.7c1.2 0 2-1.2 3-1.7 1.1-.6 3.1-.4 3.5-1.6.2-.4-.2-.9-.6-1.1-1.6-.9-3.8-.6-5.5.1-1.3.6-3.4 1.5-3.2 2.9.1 1 1.8 1.5 2.8 1.4zM420 403.8c-3.6-4-5.3-11.6-7.2-19.7-1.8-8.1-3.9-16.8-10.5-22.4-1.3-1.1-2.6-2.1-4-2.9-1.3-.8-2.7-1.5-4.1-2 9.2-27.3 5.6-54.5-3.7-79.1-11.4-30.1-31.3-56.4-46.5-74.4-17.1-21.5-33.7-41.9-33.4-72C311.1 85.4 315.7.1 234.8 0 132.4-.2 158 85.4 157.7 130.9c.3 30.4-16.3 51-33.4 72-15.2 18-35.1 44.3-46.5 74.4-9.3 24.6-12.9 51.8-3.7 79.1-1.4.5-2.8 1.2-4.1 2-1.4.8-2.7 1.8-4 2.9-6.6 5.6-8.7 14.3-10.5 22.4-1.9 8.1-3.6 15.7-7.2 19.7-6.3 6.7-7.1 16.5-3.5 22.9 3.6 6.4 11.5 9.5 18.5 6.9 3.6-1.3 6.5-4 9.1-6.5 2.5-2.5 4.9-5 7.9-6.5 5.4-2.8 12.4-2.3 19-1.2 6.8 1.2 13.2 3.1 18.8 1.9 12.2-2.6 16.5-11.1 23.7-16.9.5-.4.4-1.2-.1-1.6-.4-.3-1-.3-1.4.1-3.6 4-10.1 12.6-19.4 14.5-5.1 1-11.1-1-17.5-2.1-6.4-1.2-13.5-1.6-19.4 1.4-3.5 1.8-6.1 4.6-8.7 7.1-2.6 2.6-5.1 4.9-8.2 6-4.3 1.6-9.4-.3-11.7-4.4-2.3-4.1-1.7-10.6 2.6-15.1 4.5-4.9 6.3-13.5 8.2-21.5 1.9-8.1 3.9-16.3 9.6-21.1 2.4-2 5.3-3.2 8.3-3.5 2 12.3 9.2 23.4 19.5 31.1 1.4 1 1 3.6-.5 4.1-5.1 1.6-7.4 6.5-4.8 10.4 2.5 3.7 7.9 4.7 11.6 2.2 3.8-2.5 5.1-7.8 2.6-11.6-.5-.8-1.2-1.4-1.9-1.9 1.4-.5 2.4-1.8 2.1-3.2-.3-1.3-1.7-2.2-3-2.2h-.7c3.4-3.7 6.2-8.1 7.9-13 4.3.3 8.4 2 11.3 5.1 4.4 4.8 5.3 11.7 7.1 18 1.8 6.2 4.5 12.4 10.2 15.7 2.5 1.4 5.6 2 8.5 1 2.5-.9 4.3-3.1 5-5.7.4-1.5.2-3.1-.4-4.5-1.4-3.2-4.6-5-7.6-6.6-3-1.5-6.2-3-8-5.8-1.6-2.5-2-5.6-1.2-8.5.6-2.2 2.1-4.2 4.2-5 3.2-1.2 6.7.3 8.5 3.1 1 1.5 1.4 3.4 1.2 5.2.2.5.4 1 .5 1.5.3 1 .5 2.1.5 3.2 0 3.5-1.3 6.8-3.5 9.3 2.4 1.7 5.2 2.7 8.2 2.7 4.4 0 8.5-2.1 11.1-5.5 2.8-3.7 3.5-8.6 1.8-12.9z"/></svg>
      <span>{{ isZh ? '下载' : 'Download' }} v{{ VERSION }}</span>
    </a>
    <a href="/features" class="dl-btn dl-docs">
      <ReadOutlined class="os-icon" />
      <span>{{ isZh ? '文档' : 'Docs' }}</span>
    </a>
  </div>
</template>

<style scoped>
.download-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 20px;
  padding: 32px 0 8px;
  flex-wrap: wrap;
}

.dl-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 0 36px;
  height: 56px;
  border-radius: 28px;
  font-size: 17px;
  font-weight: 600;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.25s ease;
  white-space: nowrap;
}

.dl-primary {
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  border: 2px solid var(--vp-c-divider);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}
.dl-primary:hover {
  border-color: var(--vp-c-brand-1);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
  transform: translateY(-1px);
}

.dark .dl-primary {
  background: var(--vp-c-bg-soft);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.3);
}

.dl-docs {
  color: var(--vp-c-text-1);
  border: 2px solid var(--vp-c-divider);
  background: transparent;
}
.dl-docs:hover {
  border-color: var(--vp-c-brand-1);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
  transform: translateY(-1px);
}

.os-icon {
  flex-shrink: 0;
}

@media (max-width: 640px) {
  .dl-btn {
    padding: 0 28px;
    height: 50px;
    font-size: 15px;
    border-radius: 25px;
    gap: 8px;
  }
  .download-bar {
    gap: 14px;
    flex-direction: column;
    align-items: center;
  }
}

@media (max-width: 480px) {
  .dl-btn {
    padding: 0 24px;
    height: 48px;
    font-size: 15px;
    border-radius: 24px;
    gap: 8px;
    width: 100%;
  }
  .download-bar {
    gap: 12px;
    flex-direction: column;
    align-items: stretch;
    padding: 24px 16px 8px;
  }
}
</style>
