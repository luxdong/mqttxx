<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'
import MessageList from '../components/MessageList.vue'
import PublishBar from '../components/PublishBar.vue'
import PublishHistoryPanel from '../components/PublishHistoryPanel.vue'
import SubscriptionsPanel from '../components/SubscriptionsPanel.vue'

const route = useRoute()
const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

watch(
  () => route.params.id,
  (id) => {
    connStore.select(typeof id === 'string' && id ? id : null)
  },
  { immediate: true },
)

// ---- 左栏宽度（订阅 + 发布记录共用），持久化 ----
const WIDTH_KEY = 'mqttxx.subsWidth'
const MIN_W = 220
const MAX_W = 560
const leftWidth = ref(clampWidth(Number(localStorage.getItem(WIDTH_KEY)) || 280))

// ---- 左栏内订阅块高度（发布记录占剩余），持久化 ----
const HEIGHT_KEY = 'mqttxx.subsHeight'
const leftCol = ref<HTMLElement | null>(null)
const subsHeight = ref(clampHeight(Number(localStorage.getItem(HEIGHT_KEY)) || 260))

function clampWidth(w: number): number {
  return Math.min(MAX_W, Math.max(MIN_W, w))
}

function clampHeight(h: number): number {
  const max = Math.max(120, (leftCol.value?.clientHeight ?? 600) - 120)
  return Math.min(max, Math.max(100, h))
}

/** 左栏右缘：调宽 */
function onColResizeStart(e: MouseEvent) {
  e.preventDefault()
  const startX = e.clientX
  const startW = leftWidth.value
  const onMove = (ev: MouseEvent) => {
    leftWidth.value = clampWidth(startW + (ev.clientX - startX))
  }
  const onUp = () => {
    localStorage.setItem(WIDTH_KEY, String(leftWidth.value))
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.userSelect = ''
  }
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

/** 两块之间：调订阅块高度（发布记录占剩余） */
function onRowResizeStart(e: MouseEvent) {
  e.preventDefault()
  const startY = e.clientY
  const startH = subsHeight.value
  const onMove = (ev: MouseEvent) => {
    subsHeight.value = clampHeight(startH + (ev.clientY - startY))
  }
  const onUp = () => {
    localStorage.setItem(HEIGHT_KEY, String(subsHeight.value))
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.userSelect = ''
  }
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

const profile = computed(() => connStore.activeProfile)
const status = computed(() => connStore.activeStatus)
const statusText = computed(() => t(`status.${status.value}`))
const statusColor = computed(
  () =>
    ({
      connected: '#00b17e',
      connecting: '#e6a23c',
      reconnecting: '#e6a23c',
      error: '#e02020',
      idle: '#9aa4ad',
      disconnected: '#9aa4ad',
    })[status.value] ?? '#9aa4ad',
)

function toggleConnect() {
  if (!profile.value) return
  if (status.value === 'connected' || status.value === 'connecting' || status.value === 'reconnecting') {
    connStore.disconnect(profile.value.id)
  } else {
    connStore.connect(profile.value.id)
  }
}
</script>

<template>
  <div class="workspace">
    <template v-if="connStore.profiles.length === 0">
      <div class="hero">
        <div class="hero-logo">M</div>
        <h1>{{ t('workspace.empty.title') }}</h1>
        <p>{{ t('workspace.empty.desc') }}</p>
        <button class="btn btn-primary" @click="appStore.openConnForm(null)">＋ {{ t('sidebar.new') }}</button>
        <p class="hint">{{ t('workspace.empty.hint') }}</p>
      </div>
    </template>

    <div v-else-if="!profile" class="hero">
      <h1>{{ t('workspace.empty.title') }}</h1>
      <p>{{ t('workspace.empty.hint') }}</p>
    </div>

    <template v-else-if="profile">
      <div class="ws-head">
        <span class="status-dot big" :style="{ background: statusColor }" />
        <span class="ws-name">{{ profile.name }}</span>
        <span class="badge mono">{{ profile.protocol }}://{{ profile.host }}:{{ profile.port }}</span>
        <span class="badge">MQTT {{ profile.protocolVersion === 5 ? '5.0' : '3.1.1' }}</span>
        <span class="status-text" :style="{ color: statusColor }">{{ statusText }}</span>
        <span v-if="status === 'error' && connStore.statusErrors[profile.id]" class="err" :title="connStore.statusErrors[profile.id]">
          {{ connStore.statusErrors[profile.id] }}
        </span>
        <div class="spacer" />
        <button class="btn btn-sm" @click="appStore.openConnForm(profile)">{{ t('common.edit') }}</button>
        <button
          class="btn btn-sm"
          :class="status === 'connected' ? 'btn-danger' : 'btn-primary'"
          :disabled="status === 'connecting'"
          @click="toggleConnect"
        >
          {{ status === 'connected' || status === 'reconnecting' ? t('conn.disconnect') : t('conn.connect') }}
        </button>
      </div>

      <div class="ws-body">
        <div ref="leftCol" class="left-col" :style="{ width: leftWidth + 'px', '--subs-h': subsHeight + 'px' }">
          <SubscriptionsPanel />
          <div class="row-resizer" @mousedown="onRowResizeStart" />
          <PublishHistoryPanel />
          <div class="left-resizer" @mousedown="onColResizeStart" />
        </div>
        <MessageList />
      </div>
      <PublishBar />
    </template>
  </div>
</template>

<style scoped>
.workspace {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.hero {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
.hero-logo {
  width: 64px;
  height: 64px;
  border-radius: 18px;
  background: var(--primary);
  color: #fff;
  font-size: 34px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
}
.hero h1 {
  font-size: 20px;
}
.hero p {
  color: var(--text-dim);
}
.hero .hint {
  font-size: 12px;
  color: var(--text-faint);
}
.ws-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}
.status-dot.big {
  width: 11px;
  height: 11px;
}
.ws-name {
  font-size: 15px;
  font-weight: 700;
}
.mono {
  font-family: var(--mono);
}
.status-text {
  font-size: 12px;
  font-weight: 600;
}
.err {
  font-size: 11px;
  color: var(--danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 280px;
}
.spacer {
  flex: 1;
}
.ws-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 0;
  padding: 0 0 0 14px;
}
.left-col {
  flex-shrink: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
  margin-right: 14px;
  overflow: hidden;
}
.row-resizer {
  flex-shrink: 0;
  height: 8px;
  margin: -2px 0;
  cursor: row-resize;
  z-index: 10;
}
.row-resizer:hover {
  background: color-mix(in srgb, var(--primary) 25%, transparent);
}
.left-resizer {
  position: absolute;
  top: 0;
  right: -6px;
  width: 8px;
  height: 100%;
  cursor: col-resize;
  z-index: 10;
}
.left-resizer:hover {
  background: color-mix(in srgb, var(--primary) 25%, transparent);
}
</style>
