<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getVersion } from '@tauri-apps/api/app'
import { api, updateEvents } from '../api'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import type { DownloadProgress } from '../types'

const { t } = useI18n()
const appStore = useAppStore()
const router = useRouter()

// ---- 版本与更新 ----
const version = ref('')
const checking = ref(false)
const checkedResult = ref<'' | 'upToDate' | 'found'>('')
const downloading = ref(false)
const progress = ref<DownloadProgress>({ received: 0, total: 0 })
const downloadedPath = ref('')
const progressPercent = computed(() =>
  progress.value.total > 0 ? Math.min(100, Math.round((progress.value.received / progress.value.total) * 100)) : 0,
)

let unlisten: (() => void) | null = null
onMounted(async () => {
  version.value = await getVersion()
  unlisten = await updateEvents.onProgress((p) => (progress.value = p))
  if (appStore.updateInfo) checkedResult.value = 'found'
})
onUnmounted(() => unlisten?.())

import { computed } from 'vue'

async function checkNow() {
  checking.value = true
  const info = await appStore.checkForUpdate(false)
  checking.value = false
  checkedResult.value = info ? 'found' : 'upToDate'
}

function fmtSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB'
  if (bytes >= 1024) return (bytes / 1024).toFixed(0) + ' KB'
  return bytes + ' B'
}

async function download() {
  if (!appStore.updateInfo) return
  downloading.value = true
  progress.value = { received: 0, total: appStore.updateInfo.assetSize }
  try {
    downloadedPath.value = await api.downloadUpdate(appStore.updateInfo)
    appStore.toast(t('update.downloadDone'), 'success')
  } catch (e) {
    appStore.toast(String(e), 'error')
  } finally {
    downloading.value = false
  }
}

async function install() {
  if (!downloadedPath.value) return
  if (!confirm(t('update.installConfirm'))) return
  try {
    await api.installUpdate(downloadedPath.value)
  } catch (e) {
    appStore.toast(String(e), 'error')
  }
}

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
function setAutoCheckUpdate(v: boolean) {
  appStore.updateSettings({ autoCheckUpdate: v })
}
function setGiteeToken(v: string) {
  appStore.updateSettings({ giteeToken: v })
}
</script>

<template>
  <div class="settings">
    <div class="s-head">
      <button class="btn btn-ghost btn-sm" @click="router.back()">←</button>
      <h2>{{ t('settings.title') }}</h2>
    </div>

    <!-- 更新中心 -->
    <div class="card section">
      <div class="s-row">
        <div class="s-label">
          {{ t('update.title') }}
          <span class="ver">v{{ version }}</span>
          <span v-if="appStore.updateInfo" class="new-dot">{{ t('update.newVersion') }}</span>
        </div>
        <button class="btn btn-sm btn-primary" :disabled="checking" @click="checkNow">
          {{ checking ? t('update.checking') : t('update.check') }}
        </button>
      </div>

      <div v-if="checkedResult === 'upToDate'" class="up-state ok">{{ t('update.upToDate') }}</div>

      <div v-if="appStore.updateInfo" class="update-card">
        <div class="uc-line">
          <span class="uc-version">v{{ appStore.updateInfo.latestVersion }}</span>
          <span class="uc-size">{{ fmtSize(appStore.updateInfo.assetSize) }}</span>
        </div>
        <pre v-if="appStore.updateInfo.notes" class="uc-notes">{{ appStore.updateInfo.notes }}</pre>

        <div v-if="downloading" class="uc-progress">
          <div class="uc-bar"><div class="uc-bar-fill" :style="{ width: progressPercent + '%' }" /></div>
          <span class="uc-pct">
            {{ t('update.downloading') }} {{ progressPercent }}%
            （{{ fmtSize(progress.received) }} / {{ fmtSize(progress.total) }}）
          </span>
        </div>

        <div class="uc-actions">
          <button v-if="!downloadedPath" class="btn btn-primary" :disabled="downloading" @click="download">
            {{ downloading ? t('update.downloading') : t('update.download') }}
          </button>
          <template v-else>
            <span class="up-state ok">{{ t('update.downloadDone') }}</span>
            <button class="btn btn-primary" @click="install">{{ t('update.install') }}</button>
          </template>
        </div>
      </div>

      <div class="s-row">
        <div class="s-label">{{ t('update.autoCheck') }}</div>
        <button
          class="btn btn-sm"
          :class="appStore.settings.autoCheckUpdate ? 'btn-primary' : ''"
          @click="setAutoCheckUpdate(!appStore.settings.autoCheckUpdate)"
        >
          {{ appStore.settings.autoCheckUpdate ? 'ON' : 'OFF' }}
        </button>
      </div>
      <div class="s-row">
        <div class="s-label" :title="t('update.tokenHint')">{{ t('update.token') }}</div>
        <input
          class="field-input num"
          type="password"
          spellcheck="false"
          autocomplete="off"
          :value="appStore.settings.giteeToken"
          :placeholder="t('update.tokenHint')"
          @change="setGiteeToken(($event.target as HTMLInputElement).value.trim())"
        />
      </div>
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
  display: flex;
  align-items: center;
  gap: 8px;
}
.ver {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--text-faint);
}
.new-dot {
  font-size: 11px;
  color: var(--warning);
  background: color-mix(in srgb, var(--warning) 15%, transparent);
  border-radius: 4px;
  padding: 2px 6px;
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
.up-state {
  font-size: 12px;
  padding: 8px 0 0;
}
.up-state.ok {
  color: var(--primary);
}
.update-card {
  margin: 10px 0 12px;
  border: 1px solid color-mix(in srgb, var(--primary) 40%, var(--border));
  background: color-mix(in srgb, var(--primary) 6%, var(--panel));
  border-radius: 8px;
  padding: 12px 14px;
}
.uc-line {
  display: flex;
  align-items: center;
  gap: 10px;
}
.uc-version {
  font-family: var(--mono);
  font-weight: 700;
  font-size: 14px;
  color: var(--primary);
}
.uc-size {
  font-size: 11px;
  color: var(--text-faint);
  font-family: var(--mono);
}
.uc-notes {
  margin-top: 8px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-dim);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 180px;
  overflow-y: auto;
}
.uc-progress {
  margin-top: 10px;
}
.uc-bar {
  height: 6px;
  border-radius: 3px;
  background: var(--panel-2);
  overflow: hidden;
}
.uc-bar-fill {
  height: 100%;
  background: var(--primary);
  transition: width 0.2s;
}
.uc-pct {
  display: block;
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-dim);
  font-family: var(--mono);
}
.uc-actions {
  margin-top: 10px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.about-text {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-dim);
}
</style>
