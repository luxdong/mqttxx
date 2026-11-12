<script setup lang="ts">
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useRouter } from 'vue-router'

const { t } = useI18n()
const appStore = useAppStore()
const router = useRouter()

function setTheme(theme: 'light' | 'dark') {
  appStore.updateSettings({ theme })
}
function setLang(lang: 'zh' | 'en') {
  appStore.updateSettings({ lang })
}
function setMemory(v: number) {
  appStore.updateSettings({ maxMessagesInMemory: v })
}
function setAutoResub(v: boolean) {
  appStore.updateSettings({ autoResub: v })
}
</script>

<template>
  <div class="settings">
    <div class="s-head">
      <button class="btn btn-ghost btn-sm" @click="router.back()">←</button>
      <h2>{{ t('settings.title') }}</h2>
    </div>

    <div class="card section">
      <div class="s-row">
        <div class="s-label">{{ t('settings.theme') }}</div>
        <div class="seg">
          <button class="seg-btn" :class="{ active: appStore.settings.theme === 'light' }" @click="setTheme('light')">
            {{ t('settings.theme.light') }}
          </button>
          <button class="seg-btn" :class="{ active: appStore.settings.theme === 'dark' }" @click="setTheme('dark')">
            {{ t('settings.theme.dark') }}
          </button>
        </div>
      </div>
      <div class="s-row">
        <div class="s-label">{{ t('settings.lang') }}</div>
        <div class="seg">
          <button class="seg-btn" :class="{ active: appStore.settings.lang === 'zh' }" @click="setLang('zh')">中文</button>
          <button class="seg-btn" :class="{ active: appStore.settings.lang === 'en' }" @click="setLang('en')">English</button>
        </div>
      </div>
      <div class="s-row">
        <div class="s-label">{{ t('settings.memory') }}</div>
        <input
          class="field-input num"
          type="number"
          min="100"
          max="100000"
          :value="appStore.settings.maxMessagesInMemory"
          @change="setMemory(Number(($event.target as HTMLInputElement).value) || 1000)"
        />
      </div>
      <div class="s-row">
        <div class="s-label">{{ t('settings.autoResub') }}</div>
        <button
          class="btn btn-sm"
          :class="appStore.settings.autoResub ? 'btn-primary' : ''"
          @click="setAutoResub(!appStore.settings.autoResub)"
        >
          {{ appStore.settings.autoResub ? 'ON' : 'OFF' }}
        </button>
      </div>
    </div>

    <div class="card section about">
      <div class="s-label">{{ t('settings.about') }}</div>
      <p class="about-text">{{ t('settings.aboutDesc') }}</p>
    </div>
  </div>
</template>

<style scoped>
.settings {
  flex: 1;
  overflow-y: auto;
  padding: 18px 24px;
}
.s-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.s-head h2 {
  font-size: 17px;
}
.section {
  padding: 6px 18px;
  margin-bottom: 14px;
  max-width: 640px;
}
.s-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 13px 0;
  border-bottom: 1px solid var(--border);
}
.s-row:last-child {
  border-bottom: none;
}
.s-label {
  font-size: 13px;
  color: var(--text);
  font-weight: 500;
}
.seg {
  display: flex;
  border: 1px solid var(--border);
  border-radius: 7px;
  overflow: hidden;
}
.seg-btn {
  border: none;
  background: transparent;
  padding: 6px 16px;
  cursor: pointer;
  color: var(--text-dim);
}
.seg-btn.active {
  background: var(--primary);
  color: #fff;
}
.num {
  width: 120px;
}
.about-text {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-dim);
}
</style>
