<script setup lang="ts">
import { computed } from 'vue';
import { useData } from 'vitepress';
import {
  RobotOutlined,
  ThunderboltOutlined,
  ApiOutlined,
  CloudServerOutlined,
  BookOutlined,
  SearchOutlined,
  EditOutlined,
  DesktopOutlined,
  LockOutlined,
} from '@ant-design/icons-vue';

const iconMap: Record<string, any> = {
  robot: RobotOutlined,
  thunderbolt: ThunderboltOutlined,
  api: ApiOutlined,
  'cloud-server': CloudServerOutlined,
  book: BookOutlined,
  search: SearchOutlined,
  edit: EditOutlined,
  desktop: DesktopOutlined,
  lock: LockOutlined,
};

const { frontmatter } = useData();
const features = computed(() => frontmatter.value.features || []);
</script>

<template>
  <div class="features-grid" v-if="features.length">
    <div class="features-container">
      <div
        v-for="(feature, index) in features"
        :key="index"
        class="feature-card"
      >
        <div class="card-inner">
          <div class="icon-wrapper">
            <component
              :is="iconMap[feature.icon] || RobotOutlined"
              class="feature-icon"
            />
          </div>
          <h3 class="feature-title">{{ feature.title }}</h3>
          <p class="feature-details">{{ feature.details }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.features-grid {
  max-width: 1152px;
  margin: 0 auto;
  padding: 0 24px 48px;
}

.features-container {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.feature-card {
  position: relative;
  border-radius: 12px;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.feature-card::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 12px;
  padding: 1px;
  background: linear-gradient(
    135deg,
    transparent 0%,
    transparent 100%
  );
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  opacity: 0;
  transition: opacity 0.35s ease;
  pointer-events: none;
}

.feature-card:hover {
  border-color: var(--vp-c-brand-1);
  box-shadow:
    0 0 0 1px var(--vp-c-brand-1),
    0 4px 24px rgba(48, 151, 49, 0.15),
    0 0 48px rgba(48, 151, 49, 0.08);
  transform: translateY(-2px);
}

.dark .feature-card:hover {
  box-shadow:
    0 0 0 1px var(--vp-c-brand-1),
    0 4px 24px rgba(63, 186, 64, 0.2),
    0 0 48px rgba(63, 186, 64, 0.1);
}

.card-inner {
  padding: 24px;
}

.icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 10px;
  background: var(--vp-c-default-soft);
  margin-bottom: 16px;
  transition: all 0.35s ease;
}

.feature-card:hover .icon-wrapper {
  background: var(--vp-c-brand-soft);
}

.feature-icon {
  font-size: 24px;
  color: var(--vp-c-brand-1);
}

.feature-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--vp-c-text-1);
  margin: 0 0 8px;
  line-height: 1.4;
}

.feature-details {
  font-size: 14px;
  color: var(--vp-c-text-2);
  margin: 0;
  line-height: 1.6;
}

@media (max-width: 960px) {
  .features-container {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .features-container {
    grid-template-columns: 1fr;
  }
  .features-grid {
    padding: 0 16px 32px;
  }
}
</style>
