<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { api, updateEvents } from '../api'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import type { DownloadProgress } from '../types'

const { t } = useI18n()
const appStore = useAppStore()
const router = useRouter()
const win = getCurrentWindow()

const bubbleVisible = ref(false)
const checking = ref(false)
const error = ref('')
const downloading = ref(false)
const progress = ref<DownloadProgress>({ received: 0, total: 0 })
const downloadedPath = ref('')
const wrapRef = ref<HTMLElement | null>(null)

const percent = computed(() =>
  progress.value.total > 0
    ? Math.min(100, Math.round((progress.value.received / progress.value.total) * 100))
    : 0,
)

function fmtSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB'
  if (bytes >= 1024) return (bytes / 1024).toFixed(0) + ' KB'
  return bytes + ' B'
}

let closeTimer: number | null = null

/** 点击云朵：打开气泡并检查（已有更新卡片/下载中时不重复检查） */
async function onCloud(e: MouseEvent) {
  e.stopPropagation()
  if (closeTimer) {
    clearTimeout(closeTimer)
    closeTimer = null
  }
  bubbleVisible.value = true
  if (checking.value || downloading.value || appStore.updateInfo) return
  checking.value = true
  error.value = ''
  try {
    const info = await api.checkUpdate()
    appStore.updateInfo = info
    if (!info) {
      // 无更新时自动收起
      closeTimer = window.setTimeout(() => (bubbleVisible.value = false), 4000)
    }
  } catch (err) {
    error.value = String(err)
    closeTimer = window.setTimeout(() => (bubbleVisible.value = false), 8000)
  } finally {
    checking.value = false
  }
}

/** 点外部关闭 */
function onDocClick() {
  if (bubbleVisible.value) bubbleVisible.value = false
}
onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  if (closeTimer) clearTimeout(closeTimer)
})

const unlisten = updateEvents.onProgress((p) => (progress.value = p))
onUnmounted(() => unlisten.then((fn) => fn()))

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

function setAutoCheck(v: boolean) {
  appStore.updateSettings({ autoCheckUpdate: v })
}

function goSettings() {
  bubbleVisible.value = false
  router.push('/settings')
}

const minimize = () => win.minimize()
const toggleMaximize = () => win.toggleMaximize()
const close = () => win.close()
</script>

<template>
  <div class="titlebar">
    <div class="tb-brand">MQTTXX</div>
    <div class="tb-drag" data-tauri-drag-region />

    <!-- 更新中心入口 -->
    <div ref="wrapRef" class="tb-cloud-wrap">
      <button
        class="tb-btn tb-cloud"
        :class="{ 'has-update': appStore.updateInfo }"
        :title="t('update.title')"
        @click.stop="onCloud"
      >
        <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" />
        </svg>
        <span v-if="appStore.updateInfo" class="tb-dot" />
      </button>

      <!-- 更新气泡 -->
      <div v-if="bubbleVisible" class="tb-bubble" @click.stop>
        <div v-if="checking" class="tb-bubble-text">{{ t('update.checking') }}</div>

        <div v-else-if="error" class="tb-bubble-text error">{{ error }}</div>

        <template v-else-if="appStore.updateInfo">
          <div class="uc-line">
            <span class="uc-version">v{{ appStore.updateInfo.latestVersion }}</span>
            <span class="uc-size">{{ fmtSize(appStore.updateInfo.assetSize) }}</span>
          </div>
          <pre v-if="appStore.updateInfo.notes" class="uc-notes">{{ appStore.updateInfo.notes }}</pre>

          <div v-if="downloading || downloadedPath" class="uc-progress">
            <div class="uc-bar">
              <div class="uc-bar-fill" :style="{ width: (downloadedPath ? 100 : percent) + '%' }" />
            </div>
            <span class="uc-pct">
              {{ downloadedPath ? t('update.downloadDone') : `${t('update.downloading')} ${percent}%` }}
            </span>
          </div>

          <div class="uc-actions">
            <button v-if="!downloadedPath" class="btn btn-primary btn-sm" :disabled="downloading" @click="download">
              {{ downloading ? t('update.downloading') : t('update.download') }}
            </button>
            <button v-else class="btn btn-primary btn-sm" @click="install">{{ t('update.install') }}</button>
            <button class="btn btn-sm" @click="goSettings">{{ t('update.title') }} →</button>
          </div>
        </template>

        <div v-else class="tb-bubble-text">{{ t('update.upToDate') }}</div>

        <label class="uc-auto" @click.prevent="setAutoCheck(!appStore.settings.autoCheckUpdate)">
          <input type="checkbox" :checked="appStore.settings.autoCheckUpdate" />
          {{ t('update.autoCheck') }}
        </label>
      </div>
    </div>

    <!-- 窗口控制 -->
    <button class="tb-btn" title="—" @click="minimize">
      <svg viewBox="0 0 12 12" width="12" height="12"><path d="M1 6h10" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="tb-btn" title="□" @click="toggleMaximize">
      <svg viewBox="0 0 12 12" width="12" height="12"><rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="tb-btn tb-close" title="✕" @click="close">
      <svg viewBox="0 0 12 12" width="12" height="12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
  </div>
</template>

<style scoped>
.titlebar {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: stretch;
  justify-content: flex-end;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  user-select: none;
}
.tb-brand {
  display: flex;
  align-items: center;
  padding-left: 12px;
  font-size: 12px;
  font-weight: 700;
  color: var(--text-faint);
  letter-spacing: 0.5px;
}
.tb-drag {
  flex: 1;
}
.tb-cloud-wrap {
  position: relative;
  display: flex;
  align-items: stretch;
}
.tb-btn {
  width: 44px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}
.tb-btn:hover {
  background: var(--panel-2);
  color: var(--text);
}
.tb-cloud.has-update {
  color: var(--primary);
}
.tb-dot {
  position: absolute;
  top: 7px;
  right: 9px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--danger);
  border: 1px solid var(--panel);
}
.tb-bubble {
  position: absolute;
  top: calc(100% + 6px);
  right: 4px;
  z-index: 1200;
  width: 340px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: stretch;
}
.tb-bubble-text {
  font-size: 12px;
  color: var(--text-dim);
}
.tb-bubble-text.error {
  color: var(--danger);
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
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-dim);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 110px;
  overflow-y: auto;
  background: var(--panel-2);
  border-radius: 6px;
  padding: 6px 8px;
}
.uc-progress {
  margin-top: 2px;
}
.uc-bar {
  height: 5px;
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
  margin-top: 3px;
  font-size: 10px;
  color: var(--text-dim);
  font-family: var(--mono);
}
.uc-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.uc-auto {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: var(--text-faint);
  cursor: pointer;
  border-top: 1px solid var(--border);
  padding-top: 8px;
}
.tb-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
