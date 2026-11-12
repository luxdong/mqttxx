<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import type { MessageRecord } from '../types'
import Dialog from './ui/Dialog.vue'

const props = defineProps<{ message: MessageRecord | null }>()
const visible = defineModel<boolean>({ default: false })
const { t } = useI18n()
const appStore = useAppStore()
const pretty = ref(true)

const isJson = computed(() => {
  if (!props.message || props.message.payloadBinary) return false
  try {
    JSON.parse(props.message.payload)
    return true
  } catch {
    return false
  }
})

const displayPayload = computed(() => {
  const m = props.message
  if (!m) return ''
  if (isJson.value && pretty.value) {
    try {
      return JSON.stringify(JSON.parse(m.payload), null, 2)
    } catch {
      return m.payload
    }
  }
  return m.payload
})

const payloadSize = computed(() => {
  const m = props.message
  if (!m) return 0
  return m.payloadBinary ? Math.floor((m.payload.length * 3) / 4) : m.payload.length
})

async function copy() {
  if (!props.message) return
  await navigator.clipboard.writeText(displayPayload.value)
  appStore.toast(t('common.copied'), 'success')
}

function fmtTime(ts: number) {
  return new Date(ts).toLocaleString('zh-CN', { hour12: false })
}
</script>

<template>
  <Dialog v-model="visible" :title="t('msg.detail')" :width="640">
    <div v-if="message" class="detail">
      <div class="meta">
        <span class="dir" :class="message.direction === 0 ? 'in' : 'out'">
          {{ message.direction === 0 ? '↓ RECEIVE' : '↑ PUBLISH' }}
        </span>
        <span class="badge">{{ fmtTime(message.ts) }}</span>
        <span class="badge">QoS {{ message.qos }}</span>
        <span v-if="message.retain" class="badge retain-badge">Retain</span>
        <span class="badge">{{ payloadSize }} {{ t('msg.bytes') }}</span>
      </div>
      <div class="topic-row">
        <span class="topic-label">Topic</span>
        <code class="topic-val">{{ message.topic }}</code>
      </div>
      <div class="payload-toolbar">
        <span v-if="message.payloadBinary" class="bin-hint">{{ t('msg.binary') }}</span>
        <template v-else-if="isJson">
          <button class="btn btn-sm" :class="{ 'btn-primary': pretty }" @click="pretty = true">JSON</button>
          <button class="btn btn-sm" :class="{ 'btn-primary': !pretty }" @click="pretty = false">RAW</button>
        </template>
        <button class="btn btn-sm copy" @click="copy">{{ t('common.copy') }}</button>
      </div>
      <pre class="payload-view">{{ displayPayload }}</pre>
    </div>
  </Dialog>
</template>

<style scoped>
.meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.dir {
  font-weight: 700;
  font-size: 12px;
}
.dir.in {
  color: var(--primary);
}
.dir.out {
  color: var(--warning);
}
.retain-badge {
  color: var(--warning);
}
.topic-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.topic-label {
  font-size: 12px;
  color: var(--text-dim);
}
.topic-val {
  font-family: var(--mono);
  background: var(--panel-2);
  padding: 4px 8px;
  border-radius: 5px;
  word-break: break-all;
}
.payload-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}
.bin-hint {
  font-size: 12px;
  color: var(--warning);
}
.copy {
  margin-left: auto;
}
.payload-view {
  margin-top: 8px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 320px;
  overflow: auto;
}
</style>
