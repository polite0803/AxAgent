<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useData } from 'vitepress';
import {
  AppleOutlined,
  WindowsOutlined,
  ClockCircleOutlined,
  LinkOutlined,
} from '@ant-design/icons-vue';

declare const __APP_VERSION__: string;

const VERSION = __APP_VERSION__;
const BASE = `https://github.com/polite0803/AxAgent/releases/download/v${VERSION}`;

type OS = 'macos' | 'windows' | 'linux';

interface DownloadItem {
  labelZh: string;
  labelEn: string;
  file: string;
  arch: string;
  os: OS;
  primary?: boolean;
}

const downloads: DownloadItem[] = [
  { os: 'macos', arch: 'Apple Silicon', labelEn: 'Apple Silicon (M1/M2/M3/M4)', labelZh: 'Apple Silicon（M 系列芯片）', file: `AxAgent_${VERSION}_aarch64.dmg`, primary: true },
  { os: 'macos', arch: 'Intel', labelEn: 'Intel', labelZh: 'Intel（英特尔芯片）', file: `AxAgent_${VERSION}_x64.dmg`, primary: true },
  { os: 'windows', arch: 'x64', labelEn: 'Windows x64', labelZh: 'Windows x64', file: `AxAgent_${VERSION}_x64-setup.exe`, primary: true },
  { os: 'windows', arch: 'x64 Portable', labelEn: 'Windows x64 Portable', labelZh: 'Windows x64 绿色版', file: `AxAgent_v${VERSION}_windows-x64-portable.zip` },
  { os: 'windows', arch: 'ARM64', labelEn: 'Windows ARM64', labelZh: 'Windows ARM64', file: `AxAgent_${VERSION}_arm64-setup.exe` },
  { os: 'windows', arch: 'ARM64 Portable', labelEn: 'Windows ARM64 Portable', labelZh: 'Windows ARM64 绿色版', file: `AxAgent_v${VERSION}_windows-arm64-portable.zip` },
  { os: 'linux', arch: 'x64 deb', labelEn: 'x64 .deb (Debian/Ubuntu)', labelZh: 'x64 .deb（Debian/Ubuntu）', file: `AxAgent_${VERSION}_amd64.deb`, primary: true },
  { os: 'linux', arch: 'x64 AppImage', labelEn: 'x64 AppImage', labelZh: 'x64 AppImage', file: `AxAgent_${VERSION}_amd64.AppImage` },
  { os: 'linux', arch: 'ARM64 deb', labelEn: 'ARM64 .deb', labelZh: 'ARM64 .deb', file: `AxAgent_${VERSION}_arm64.deb` },
  { os: 'linux', arch: 'x64 rpm', labelEn: 'x64 .rpm (Fedora/RHEL)', labelZh: 'x64 .rpm（Fedora/RHEL）', file: `AxAgent-${VERSION}-1.x86_64.rpm` },
  { os: 'linux', arch: 'ARM64 rpm', labelEn: 'ARM64 .rpm', labelZh: 'ARM64 .rpm', file: `AxAgent-${VERSION}-1.aarch64.rpm` },
];

const osTabs: { id: OS; label: string }[] = [
  { id: 'macos', label: 'macOS' },
  { id: 'windows', label: 'Windows' },
  { id: 'linux', label: 'Linux' },
];

const { lang } = useData();
const isZh = computed(() => lang.value === 'zh-CN');

const activeOS = ref<OS>('macos');

onMounted(() => {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('win')) {
    activeOS.value = 'windows';
  } else if (ua.includes('linux')) {
    activeOS.value = 'linux';
  } else {
    activeOS.value = 'macos';
  }
});

const currentDownloads = computed(() =>
  downloads.filter(d => d.os === activeOS.value)
);

function itemLabel(item: DownloadItem) {
  return isZh.value ? item.labelZh : item.labelEn;
}

function downloadUrl(item: DownloadItem) {
  return `${BASE}/${item.file}`;
}

interface InstallStep {
  titleZh: string;
  titleEn: string;
  stepsZh: string[];
  stepsEn: string[];
}

const installInstructions = computed<InstallStep[]>(() => {
  const os = activeOS.value;
  if (os === 'macos') {
    return [{
      titleZh: '安装步骤',
      titleEn: 'Installation',
      stepsZh: [
        '打开下载的 .dmg 文件',
        '将 AxAgent 拖入「应用程序」文件夹',
        '首次运行时，在「系统设置 → 隐私与安全性」中允许运行',
      ],
      stepsEn: [
        'Open the downloaded .dmg file',
        'Drag AxAgent to the Applications folder',
        'On first launch, allow it in System Settings → Privacy & Security',
      ],
    }];
  }
  if (os === 'windows') {
    return [
      {
        titleZh: '安装版',
        titleEn: 'Installer',
        stepsZh: [
          '运行下载的安装程序',
          '按向导完成安装',
          '从开始菜单或桌面快捷方式启动',
        ],
        stepsEn: [
          'Run the downloaded installer',
          'Follow the wizard to complete installation',
          'Launch from Start Menu or desktop shortcut',
        ],
      },
      {
        titleZh: '绿色免安装版 (Portable)',
        titleEn: 'Portable',
        stepsZh: [
          '解压 .zip 文件到任意目录',
          '双击 AxAgent.exe 即可运行',
        ],
        stepsEn: [
          'Extract the .zip file to any directory',
          'Double-click AxAgent.exe to run',
        ],
      },
    ];
  }
  return [{
    titleZh: '安装步骤',
    titleEn: 'Installation',
    stepsZh: [
      'Debian/Ubuntu: sudo dpkg -i AxAgent_x.x.x_amd64.deb',
      'AppImage: chmod +x AxAgent_x.x.x_amd64.AppImage && ./AxAgent_x.x.x_amd64.AppImage',
      'RPM: sudo rpm -i AxAgent-x.x.x-1.x86_64.rpm',
    ],
    stepsEn: [
      'Debian/Ubuntu: sudo dpkg -i AxAgent_x.x.x_amd64.deb',
      'AppImage: chmod +x AxAgent_x.x.x_amd64.AppImage && ./AxAgent_x.x.x_amd64.AppImage',
      'RPM: sudo rpm -i AxAgent-x.x.x-1.x86_64.rpm',
    ],
  }];
});

const sysReq = computed(() => {
  const os = activeOS.value;
  if (os === 'macos') return isZh.value ? 'macOS 11.0 (Big Sur) 及以上' : 'macOS 11.0 (Big Sur) or later';
  if (os === 'windows') return isZh.value ? 'Windows 10 (1803) 及以上' : 'Windows 10 (1803) or later';
  return isZh.value ? '各主流 Linux 发行版' : 'Major Linux distributions';
});
</script>

<template>
  <div class="download-page">
    <!-- Version badge -->
    <div class="version-header">
      <a class="version-badge" href="https://github.com/polite0803/AxAgent/releases" target="_blank" rel="noopener">
        <ClockCircleOutlined />
        {{ isZh ? '最新发行版本' : 'Latest Release' }}：v{{ VERSION }}
      </a>
    </div>

    <!-- OS Tabs -->
    <div class="os-tabs">
      <button
        v-for="tab in osTabs"
        :key="tab.id"
        :class="['os-tab', { active: activeOS === tab.id }]"
        @click="activeOS = tab.id"
      >
        <AppleOutlined v-if="tab.id === 'macos'" class="tab-icon" />
        <WindowsOutlined v-else-if="tab.id === 'windows'" class="tab-icon" />
        <svg v-else class="tab-icon" xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 448 512" fill="currentColor"><path d="M220.8 123.3c1 .5 1.8 1.7 3 1.7 1.1 0 2.8-.4 2.9-1.5.2-1.4-1.9-2.3-3.2-2.9-1.7-.7-3.9-1-5.5-.1-.4.2-.8.7-.6 1.1.3 1.3 2.3 1.1 3.4 1.7zm-21.9 1.7c1.2 0 2-1.2 3-1.7 1.1-.6 3.1-.4 3.5-1.6.2-.4-.2-.9-.6-1.1-1.6-.9-3.8-.6-5.5.1-1.3.6-3.4 1.5-3.2 2.9.1 1 1.8 1.5 2.8 1.4zM420 403.8c-3.6-4-5.3-11.6-7.2-19.7-1.8-8.1-3.9-16.8-10.5-22.4-1.3-1.1-2.6-2.1-4-2.9-1.3-.8-2.7-1.5-4.1-2 9.2-27.3 5.6-54.5-3.7-79.1-11.4-30.1-31.3-56.4-46.5-74.4-17.1-21.5-33.7-41.9-33.4-72C311.1 85.4 315.7.1 234.8 0 132.4-.2 158 85.4 157.7 130.9c.3 30.4-16.3 51-33.4 72-15.2 18-35.1 44.3-46.5 74.4-9.3 24.6-12.9 51.8-3.7 79.1-1.4.5-2.8 1.2-4.1 2-1.4.8-2.7 1.8-4 2.9-6.6 5.6-8.7 14.3-10.5 22.4-1.9 8.1-3.6 15.7-7.2 19.7-6.3 6.7-7.1 16.5-3.5 22.9 3.6 6.4 11.5 9.5 18.5 6.9 3.6-1.3 6.5-4 9.1-6.5 2.5-2.5 4.9-5 7.9-6.5 5.4-2.8 12.4-2.3 19-1.2 6.8 1.2 13.2 3.1 18.8 1.9 12.2-2.6 16.5-11.1 23.7-16.9.5-.4.4-1.2-.1-1.6-.4-.3-1-.3-1.4.1-3.6 4-10.1 12.6-19.4 14.5-5.1 1-11.1-1-17.5-2.1-6.4-1.2-13.5-1.6-19.4 1.4-3.5 1.8-6.1 4.6-8.7 7.1-2.6 2.6-5.1 4.9-8.2 6-4.3 1.6-9.4-.3-11.7-4.4-2.3-4.1-1.7-10.6 2.6-15.1 4.5-4.9 6.3-13.5 8.2-21.5 1.9-8.1 3.9-16.3 9.6-21.1 2.4-2 5.3-3.2 8.3-3.5 2 12.3 9.2 23.4 19.5 31.1 1.4 1 1 3.6-.5 4.1-5.1 1.6-7.4 6.5-4.8 10.4 2.5 3.7 7.9 4.7 11.6 2.2 3.8-2.5 5.1-7.8 2.6-11.6-.5-.8-1.2-1.4-1.9-1.9 1.4-.5 2.4-1.8 2.1-3.2-.3-1.3-1.7-2.2-3-2.2h-.7c3.4-3.7 6.2-8.1 7.9-13 4.3.3 8.4 2 11.3 5.1 4.4 4.8 5.3 11.7 7.1 18 1.8 6.2 4.5 12.4 10.2 15.7 2.5 1.4 5.6 2 8.5 1 2.5-.9 4.3-3.1 5-5.7.4-1.5.2-3.1-.4-4.5-1.4-3.2-4.6-5-7.6-6.6-3-1.5-6.2-3-8-5.8-1.6-2.5-2-5.6-1.2-8.5.6-2.2 2.1-4.2 4.2-5 3.2-1.2 6.7.3 8.5 3.1 1 1.5 1.4 3.4 1.2 5.2.2.5.4 1 .5 1.5.3 1 .5 2.1.5 3.2 0 3.5-1.3 6.8-3.5 9.3 2.4 1.7 5.2 2.7 8.2 2.7 4.4 0 8.5-2.1 11.1-5.5 2.8-3.7 3.5-8.6 1.8-12.9z"/></svg>
        {{ tab.label }}
      </button>
    </div>

    <!-- Download buttons -->
    <div class="download-section">
      <div class="download-grid">
        <a
          v-for="item in currentDownloads"
          :key="item.file"
          :href="downloadUrl(item)"
          :class="['dl-btn', item.primary ? 'dl-primary' : 'dl-secondary']"
        >
          <AppleOutlined v-if="activeOS === 'macos'" class="btn-icon" />
          <WindowsOutlined v-else-if="activeOS === 'windows'" class="btn-icon" />
          <svg v-else class="btn-icon" xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 448 512" fill="currentColor"><path d="M220.8 123.3c1 .5 1.8 1.7 3 1.7 1.1 0 2.8-.4 2.9-1.5.2-1.4-1.9-2.3-3.2-2.9-1.7-.7-3.9-1-5.5-.1-.4.2-.8.7-.6 1.1.3 1.3 2.3 1.1 3.4 1.7zm-21.9 1.7c1.2 0 2-1.2 3-1.7 1.1-.6 3.1-.4 3.5-1.6.2-.4-.2-.9-.6-1.1-1.6-.9-3.8-.6-5.5.1-1.3.6-3.4 1.5-3.2 2.9.1 1 1.8 1.5 2.8 1.4zM420 403.8c-3.6-4-5.3-11.6-7.2-19.7-1.8-8.1-3.9-16.8-10.5-22.4-1.3-1.1-2.6-2.1-4-2.9-1.3-.8-2.7-1.5-4.1-2 9.2-27.3 5.6-54.5-3.7-79.1-11.4-30.1-31.3-56.4-46.5-74.4-17.1-21.5-33.7-41.9-33.4-72C311.1 85.4 315.7.1 234.8 0 132.4-.2 158 85.4 157.7 130.9c.3 30.4-16.3 51-33.4 72-15.2 18-35.1 44.3-46.5 74.4-9.3 24.6-12.9 51.8-3.7 79.1-1.4.5-2.8 1.2-4.1 2-1.4.8-2.7 1.8-4 2.9-6.6 5.6-8.7 14.3-10.5 22.4-1.9 8.1-3.6 15.7-7.2 19.7-6.3 6.7-7.1 16.5-3.5 22.9 3.6 6.4 11.5 9.5 18.5 6.9 3.6-1.3 6.5-4 9.1-6.5 2.5-2.5 4.9-5 7.9-6.5 5.4-2.8 12.4-2.3 19-1.2 6.8 1.2 13.2 3.1 18.8 1.9 12.2-2.6 16.5-11.1 23.7-16.9.5-.4.4-1.2-.1-1.6-.4-.3-1-.3-1.4.1-3.6 4-10.1 12.6-19.4 14.5-5.1 1-11.1-1-17.5-2.1-6.4-1.2-13.5-1.6-19.4 1.4-3.5 1.8-6.1 4.6-8.7 7.1-2.6 2.6-5.1 4.9-8.2 6-4.3 1.6-9.4-.3-11.7-4.4-2.3-4.1-1.7-10.6 2.6-15.1 4.5-4.9 6.3-13.5 8.2-21.5 1.9-8.1 3.9-16.3 9.6-21.1 2.4-2 5.3-3.2 8.3-3.5 2 12.3 9.2 23.4 19.5 31.1 1.4 1 1 3.6-.5 4.1-5.1 1.6-7.4 6.5-4.8 10.4 2.5 3.7 7.9 4.7 11.6 2.2 3.8-2.5 5.1-7.8 2.6-11.6-.5-.8-1.2-1.4-1.9-1.9 1.4-.5 2.4-1.8 2.1-3.2-.3-1.3-1.7-2.2-3-2.2h-.7c3.4-3.7 6.2-8.1 7.9-13 4.3.3 8.4 2 11.3 5.1 4.4 4.8 5.3 11.7 7.1 18 1.8 6.2 4.5 12.4 10.2 15.7 2.5 1.4 5.6 2 8.5 1 2.5-.9 4.3-3.1 5-5.7.4-1.5.2-3.1-.4-4.5-1.4-3.2-4.6-5-7.6-6.6-3-1.5-6.2-3-8-5.8-1.6-2.5-2-5.6-1.2-8.5.6-2.2 2.1-4.2 4.2-5 3.2-1.2 6.7.3 8.5 3.1 1 1.5 1.4 3.4 1.2 5.2.2.5.4 1 .5 1.5.3 1 .5 2.1.5 3.2 0 3.5-1.3 6.8-3.5 9.3 2.4 1.7 5.2 2.7 8.2 2.7 4.4 0 8.5-2.1 11.1-5.5 2.8-3.7 3.5-8.6 1.8-12.9z"/></svg>
          <span>{{ itemLabel(item) }}</span>
        </a>
      </div>

      <!-- System requirement -->
      <div class="sys-req">
        <span class="sys-req-label">{{ isZh ? '系统要求' : 'System Requirements' }}:</span>
        {{ sysReq }}
      </div>

      <!-- GitHub Releases link -->
      <a
        class="releases-link"
        href="https://github.com/polite0803/AxAgent/releases"
        target="_blank"
        rel="noopener"
      >
        {{ isZh ? '前往 GitHub Releases 下载更多版本' : 'View all versions on GitHub Releases' }}
        <LinkOutlined />
      </a>
    </div>

    <!-- Installation instructions -->
    <div class="install-section">
      <h3 class="section-title">{{ isZh ? '安装说明' : 'Installation' }}</h3>
      <div v-for="(inst, idx) in installInstructions" :key="idx" class="install-block">
        <h4 v-if="installInstructions.length > 1" class="install-subtitle">
          {{ isZh ? inst.titleZh : inst.titleEn }}
        </h4>
        <ol class="install-steps">
          <li v-for="(step, si) in (isZh ? inst.stepsZh : inst.stepsEn)" :key="si">
            {{ step }}
          </li>
        </ol>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-page {
  max-width: 720px;
  margin: 0 auto;
  padding: 48px 24px 64px;
}

/* Version header */
.version-header {
  text-align: center;
  margin-bottom: 32px;
}

.version-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-soft);
  padding: 6px 16px;
  border-radius: 20px;
  letter-spacing: 0.02em;
  text-decoration: none;
  transition: opacity 0.2s;
}

.version-badge:hover {
  opacity: 0.8;
}

/* OS Tabs */
.os-tabs {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-bottom: 32px;
  border-bottom: 1px solid var(--vp-c-divider);
  padding-bottom: 0;
}

.os-tab {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 24px;
  font-size: 15px;
  font-weight: 500;
  color: var(--vp-c-text-2);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all 0.2s ease;
  margin-bottom: -1px;
}

.os-tab:hover {
  color: var(--vp-c-text-1);
}

.os-tab.active {
  color: var(--vp-c-brand-1);
  border-bottom-color: var(--vp-c-brand-1);
}

.tab-icon {
  flex-shrink: 0;
  opacity: 0.7;
}

.os-tab.active .tab-icon {
  opacity: 1;
}

/* Download section */
.download-section {
  margin-bottom: 40px;
}

.download-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.dl-btn {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 24px;
  height: 52px;
  border-radius: 12px;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  color: var(--vp-c-text-2);
  font-size: 15px;
}

.dl-btn:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-text-1);
  background: var(--vp-c-brand-soft);
  transform: translateY(-1px);
}

.dl-primary {
  font-size: 16px;
  color: var(--vp-c-text-1);
}

.dl-secondary {
  font-size: 14px;
  color: var(--vp-c-text-1);
}

.btn-icon {
  flex-shrink: 0;
  opacity: 1;
}

/* System requirements */
.sys-req {
  margin-top: 16px;
  padding: 12px 16px;
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
  font-size: 13px;
  color: var(--vp-c-text-2);
}

.sys-req-label {
  font-weight: 600;
  color: var(--vp-c-text-1);
}

/* Releases link */
.releases-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 16px;
  font-size: 14px;
  color: var(--vp-c-brand-1);
  text-decoration: none;
}

.releases-link:hover {
  text-decoration: underline;
}

/* Installation section */
.install-section {
  border-top: 1px solid var(--vp-c-divider);
  padding-top: 32px;
}

.section-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--vp-c-text-1);
  margin-bottom: 20px;
}

.install-block {
  margin-bottom: 20px;
}

.install-subtitle {
  font-size: 14px;
  font-weight: 600;
  color: var(--vp-c-text-2);
  margin-bottom: 8px;
}

.install-steps {
  margin: 0;
  padding-left: 20px;
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 2;
}

.install-steps li {
  padding-left: 4px;
}

/* Mobile responsive */
@media (max-width: 640px) {
  .download-page {
    padding: 32px 16px 48px;
  }

  .os-tabs {
    gap: 4px;
  }

  .os-tab {
    padding: 10px 16px;
    font-size: 14px;
    gap: 6px;
  }

  .tab-icon {
    width: 18px;
    height: 18px;
  }

  .dl-btn {
    padding: 0 16px;
    height: 48px;
  }

  .dl-primary {
    font-size: 15px;
  }
}
</style>
