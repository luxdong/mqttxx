<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'
import type { MessageRecord } from '../types'
import MessageDetail from './MessageDetail.vue'
import VirtualList from './VirtualList.vue'

const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

const search = ref('')
const selected = ref<MessageRecord | null>(null)
const detailVisible = ref(false)
const atTop = ref(false)
interface VirtualListHandle {
  scrollToBottom: () => void
  scrollBy: (delta: number) => void
}
const listRef = ref<VirtualListHandle | null>(null)

const ITEM_HEIGHT = 96

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  const filter = connStore.activeTopicFilter
  const all = connStore.activeMessages
  return all.filter((m) => {
    if (filter && !topicMatch(filter, m.topic)) return false
    if (!q) return true
    return m.topic.toLowerCase().includes(q) || m.payload.toLowerCase().includes(q)
  })
})

function clearFilter() {
  const id = connStore.activeId
  if (id) connStore.clearTopicFilter(id)
}

const hasEarlier = computed(() => connStore.activeId ? connStore.hasEarlier[connStore.activeId] ?? false : false)

function openDetail(m: MessageRecord) {
  selected.value = m
  detailVisible.value = true
}

function fmtTime(ts: number) {
  const d = new Date(ts)
  const ms = String(d.getMilliseconds()).padStart(3, '0')
  return d.toLocaleTimeString('zh-CN', { hour12: false }) + '.' + ms
}

function subColor(topic: string): string | null {
  for (const s of connStore.activeSubs) {
    if (topicMatch(s.topic, topic)) return s.color
  }
  return null
}

function topicMatch(filter: string, topic: string): boolean {
  if (filter === '#' || filter === '') return true
  const fs = filter.split('/')
  const ts = topic.split('/')
  for (let i = 0; i < fs.length; i++) {
    if (fs[i] === '#') return true
    if (fs[i] === '+') continue
    if (fs[i] !== ts[i]) return false
  }
  return fs.length === ts.length
}

async function loadEarlier() {
  const id = connStore.activeId
  if (!id) return
  const arr = connStore.messages[id] ?? []
  const first = arr[0]
  const beforeCount = arr.length
  await connStore.loadHistory(id, first?.ts)
  // prepend 后保持视口锚点
  const delta = ((connStore.messages[id]?.length ?? 0) - beforeCount) * ITEM_HEIGHT
  if (delta > 0) listRef.value?.scrollBy(delta)
}

async function clearAll() {
  const id = connStore.activeId
  if (!id) return
  if (!confirm(t('msg.clearConfirm'))) return
  try {
    await connStore.clearMessages(id)
    appStore.toast(t('msg.empty'), 'success')
  } catch (e) {
    appStore.toast(String(e), 'error')
  }
}
</script>

<template>
  <div class="msglist">
    <div class="msg-head">
      <span class="title">{{ t('msg.title') }}</span>
      <span class="badge">{{ t('msg.count', { n: filtered.length }) }}</span>
      <div v-if="connStore.activeTopicFilter" class="filter-chip" :title="connStore.activeTopicFilter">
        <span class="filter-topic">{{ connStore.activeTopicFilter }}</span>
        <button class="chip-x" @click="clearFilter">✕</button>
      </div>
      <input v-model="search" class="field-input search" :placeholder="t('common.search')" />
      <button class="btn btn-sm" @click="listRef?.scrollToBottom()">⤓</button>
      <button class="btn btn-sm btn-danger" @click="clearAll">{{ t('msg.clear') }}</button>
    </div>

    <div class="msg-body">
      <div v-if="atTop && hasEarlier && !search" class="load-earlier">
        <button class="btn btn-sm" :disabled="connStore.loadingHistory" @click="loadEarlier">
          {{ t('msg.loadEarlier') }}
        </button>
      </div>
      <VirtualList
        v-if="filtered.length > 0"
        ref="listRef"
        :items="filtered"
        :item-height="ITEM_HEIGHT"
        :item-key="(m: MessageRecord) => m.id"
        @top="(v) => (atTop = v)"
      >
        <template #item="{ item }">
          <div class="chat-item" :class="item.direction === 0 ? 'in' : 'out'">
            <div class="bubble" @click="openDetail(item)">
              <div class="bubble-head">
                <span class="dot" :style="{ background: subColor(item.topic) ?? '#9aa4ad' }" />
                <span class="b-topic" :title="item.topic">{{ item.topic }}</span>
                <span class="badge">Q{{ item.qos }}</span>
                <span v-if="item.retain" class="badge retain">R</span>
                <span class="b-dir">{{ item.direction === 0 ? '↓' : '↑' }}</span>
                <span class="b-time">{{ fmtTime(item.ts) }}</span>
              </div>
              <div class="b-payload">{{ item.payloadBinary ? `[binary] ${item.payload.slice(0, 80)}…` : item.payload }}</div>
            </div>
          </div>
        </template>
      </VirtualList>
      <div v-else class="empty">{{ t('msg.empty') }}</div>
    </div>

    <MessageDetail v-model="detailVisible" :message="selected" />
  </div>
</template>

<style scoped>
.msglist {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin: 14px 0 10px;
}
.msg-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 2px 8px;
}
.title {
  font-weight: 600;
}
.filter-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: color-mix(in srgb, var(--primary) 14%, transparent);
  color: var(--primary);
  border-radius: 5px;
  padding: 3px 6px 3px 9px;
  max-width: 240px;
  font-family: var(--mono);
  font-size: 12px;
}
.filter-topic {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chip-x {
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  padding: 0 2px;
  font-size: 11px;
}
.search {
  flex: 1;
  max-width: 260px;
  margin-left: auto;
  font-size: 12px;
}
.msg-body {
  flex: 1;
  min-height: 0;
  position: relative;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
}
.load-earlier {
  position: absolute;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 5;
  box-shadow: var(--shadow);
  border-radius: 6px;
}
.chat-item {
  height: 100%;
  display: flex;
  align-items: center;
  padding: 4px 16px;
}
.chat-item.in {
  justify-content: flex-start;
}
.chat-item.out {
  justify-content: flex-end;
}
.bubble {
  max-width: 76%;
  min-width: 0;
  border-radius: 10px;
  padding: 7px 11px;
  cursor: pointer;
  border: 1px solid var(--border);
  overflow: hidden;
}
.chat-item.in .bubble {
  background: var(--panel-2);
  border-top-left-radius: 2px;
}
.chat-item.in .bubble:hover {
  background: var(--border);
}
.chat-item.out .bubble {
  background: color-mix(in srgb, var(--primary) 13%, var(--panel));
  border-color: color-mix(in srgb, var(--primary) 35%, var(--border));
  border-top-right-radius: 2px;
}
.chat-item.out .bubble:hover {
  background: color-mix(in srgb, var(--primary) 20%, var(--panel));
}
.bubble-head {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 3px;
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.b-topic {
  font-family: var(--mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
  word-break: break-all;
}
.b-dir {
  font-size: 10px;
  color: var(--text-faint);
}
.chat-item.in .b-dir {
  color: var(--primary);
}
.chat-item.out .b-dir {
  color: var(--warning);
}
.b-time {
  font-size: 10px;
  color: var(--text-faint);
  font-family: var(--mono);
  margin-left: auto;
  flex-shrink: 0;
}
.b-payload {
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.45;
  color: var(--text);
  word-break: break-all;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.retain {
  color: var(--warning);
}
.empty {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-faint);
}
</style>
