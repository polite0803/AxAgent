<script setup lang="ts">
import { Swiper, SwiperSlide } from 'swiper/vue';
import { Autoplay, Pagination, Navigation } from 'swiper/modules';
import { ChevronLeft, ChevronRight } from 'lucide-vue-next';

import 'swiper/css';
import 'swiper/css/pagination';
import 'swiper/css/navigation';

const modules = [Autoplay, Pagination, Navigation];

const screenshots = [
  '/screenshots/s1-0412.png',
  '/screenshots/s2-0412.png',
  '/screenshots/s3-0412.png',
  '/screenshots/s4-0412.png',
  '/screenshots/s5-0412.png',
  '/screenshots/s6-0412.png',
  '/screenshots/s7-0412.png',
  '/screenshots/s8-0412.png',
  '/screenshots/s9-0412.png',
  '/screenshots/s10-0412.png',
];
</script>

<template>
  <div class="carousel-section">
    <Swiper
      :modules="modules"
      :slides-per-view="1"
      :space-between="16"
      :loop="true"
      :autoplay="{ delay: 4000, disableOnInteraction: false, pauseOnMouseEnter: true }"
      :pagination="{ clickable: true }"
      :navigation="{ nextEl: '.carousel-next', prevEl: '.carousel-prev' }"
      class="screenshots-swiper"
    >
      <SwiperSlide v-for="(src, idx) in screenshots" :key="idx">
        <div class="slide-wrapper">
          <img :src="src" :alt="`AxAgent Screenshot ${idx + 1}`" loading="lazy" />
        </div>
      </SwiperSlide>
    </Swiper>
    <button class="carousel-nav carousel-prev" aria-label="Previous slide">
      <ChevronLeft :size="24" />
    </button>
    <button class="carousel-nav carousel-next" aria-label="Next slide">
      <ChevronRight :size="24" />
    </button>
  </div>
</template>

<style scoped>
.carousel-section {
  position: relative;
  max-width: 960px;
  margin: 40px auto 24px;
  padding: 0 24px;
  overflow: hidden;
}

@media (min-width: 640px) {
  .carousel-section {
    padding: 0 48px;
  }
}

@media (min-width: 960px) {
  .carousel-section {
    padding: 0 64px;
  }
}

.screenshots-swiper {
  border-radius: 12px;
  overflow: hidden;
  padding-bottom: 40px;
}

.slide-wrapper {
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
  background: var(--vp-c-bg-soft);
  transition: transform 0.3s ease, box-shadow 0.3s ease;
}

.slide-wrapper:hover {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

.dark .slide-wrapper {
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
}
.dark .slide-wrapper:hover {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.slide-wrapper img {
  width: 100%;
  height: auto;
  display: block;
}

/* Navigation arrows */
.carousel-nav {
  position: absolute;
  top: calc(50% - 20px);
  transform: translateY(-50%);
  z-index: 10;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  color: var(--vp-c-text-2);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}

.carousel-nav:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.dark .carousel-nav {
  background: var(--vp-c-bg-soft);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.carousel-prev {
  left: 4px;
}

.carousel-next {
  right: 4px;
}

@media (min-width: 640px) {
  .carousel-prev { left: 16px; }
  .carousel-next { right: 16px; }
}

@media (min-width: 960px) {
  .carousel-prev { left: 28px; }
  .carousel-next { right: 28px; }
}

@media (max-width: 640px) {
  .carousel-nav {
    display: none;
  }
}

/* Swiper pagination bullets */
:deep(.swiper-pagination-bullet) {
  background: var(--vp-c-text-3);
  opacity: 0.4;
  width: 8px;
  height: 8px;
  transition: all 0.25s ease;
}

:deep(.swiper-pagination-bullet-active) {
  background: var(--vp-c-brand-1);
  opacity: 1;
  width: 24px;
  border-radius: 4px;
}
</style>
