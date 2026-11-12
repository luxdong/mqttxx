<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'

const { t } = useI18n()
const appStore = useAppStore()
const router = useRouter()
const win = getCurrentWindow()

const bubbleVisible = ref(false)
const checking = ref(false)

async function onCloud() {
  bubbleVisible.value = !bubbleVisible.value
  if (!bubbleVisible.value) return
  checking.value = true
  await appStore.checkForUpdate(false)
  checking.value = false
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
    <div class="tb-cloud-wrap">
      <button class="tb-btn tb-cloud" :title="t('update.title')" @click.stop="onCloud">
        <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" />
        </svg>
        <span v-if="appStore.updateInfo" class="tb-dot" />
      </button>

      <!-- 检查结果气泡 -->
      <div v-if="bubbleVisible" class="tb-bubble" @click.stop>
        <div v-if="checking" class="tb-bubble-text">{{ t('update.checking') }}</div>
        <template v-else-if="appStore.updateInfo">
          <div class="tb-bubble-text strong">
            {{ t('update.newFound', { v: appStore.updateInfo.latestVersion }) }}
          </div>
          <button class="btn btn-primary btn-sm" @click="goSettings">{{ t('update.title') }} →</button>
        </template>
        <div v-else class="tb-bubble-text">{{ t('update.upToDate') }}</div>
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
.tb-cloud {
  color: var(--text-dim);
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
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: flex-start;
  min-width: 220px;
}
.tb-bubble-text {
  font-size: 12px;
  color: var(--text-dim);
}
.tb-bubble-text.strong {
  color: var(--text);
  font-weight: 600;
}
.tb-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
