<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'
import type { PublishRecord } from '../types'

const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

const COLLAPSE_KEY = 'mqttxx.phCollapsed'
const collapsed = ref(localStorage.getItem(COLLAPSE_KEY) === '1')
const search = ref('')
/** 收起的 topic 分组 */
const folded = ref(new Set<string>())

watch(
  () => connStore.activeId,
  () => {
    folded.value = new Set()
    search.value = ''
  },
)

function toggleCollapsed() {
  collapsed.value = !collapsed.value
  localStorage.setItem(COLLAPSE_KEY, collapsed.value ? '1' : '0')
}

const list = computed(() => (connStore.activeId ? connStore.publishHistory[connStore.activeId] ?? [] : []))

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  const all = list.value
  if (!q) return all
  return all.filter(
    (r) => r.topic.toLowerCase().includes(q) || r.payload.toLowerCase().includes(q),
  )
})

/** 收藏置顶段 + 按 topic 分组的历史段（组内时间倒序，组按最新消息时间排序） */
const sections = computed(() => {
  const pinnedItems = filtered.value.filter((r) => r.pinned)
  const byTopic = new Map<string, PublishRecord[]>()
  for (const r of filtered.value) {
    if (r.pinned) continue
    const arr = byTopic.get(r.topic)
    if (arr) arr.push(r)
    else byTopic.set(r.topic, [r])
  }
  const groups = [...byTopic.entries()]
    .map(([topic, items]) => ({ topic, items }))
    .sort((a, b) => (b.items[0]?.ts ?? 0) - (a.items[0]?.ts ?? 0))
  return { pinnedItems, groups, total: filtered.value.length }
})

const isEmpty = computed(() => list.value.length === 0)

function toggleTopic(topic: string) {
  const next = new Set(folded.value)
  if (next.has(topic)) next.delete(topic)
  else next.add(topic)
  folded.value = next
}

function isFolded(topic: string): boolean {
  return folded.value.has(topic)
}

function apply(r: PublishRecord) {
  connStore.applyDraft(r)
}

async function togglePinned(r: PublishRecord) {
  const id = connStore.activeId
  if (!id) return
  try {
    await connStore.togglePublishPinned(id, r)
  } catch (e) {
    appStore.toast(String(e), 'error')
  }
}

async function clearAll() {
  const id = connStore.activeId
  if (!id) return
  if (!confirm(t('pub.clearHistoryConfirm'))) return
  try {
    await connStore.clearPublishHistory(id)
  } catch (e) {
    appStore.toast(String(e), 'error')
  }
}

function fmtTime(ts: number) {
  return new Date(ts).toLocaleTimeString('zh-CN', { hour12: false })
}
</script>

<template>
  <div class="ph card">
    <div class="ph-head" :title="t('ph.collapse')" @click="toggleCollapsed">
      <span class="arrow">{{ collapsed ? '▸' : '▾' }}</span>
      <span class="ph-title">{{ t('ph.title') }}</span>
      <span class="badge">{{ list.length }}</span>
      <template v-if="!collapsed">
        <button class="btn btn-ghost btn-sm del" :title="t('pub.clearHistory')" @click.stop="clearAll">🗑</button>
      </template>
    </div>

    <div v-if="!collapsed" class="ph-body" @click.stop>
      <input v-model="search" class="field-input ph-search" :placeholder="t('ph.search')" />

      <div class="ph-scroll">
        <template v-if="!isEmpty">
          <!-- 收藏置顶 -->
          <div v-if="sections.pinnedItems.length" class="ph-section">
            <div class="ph-section-title">⭐ {{ t('ph.pinned') }}</div>
            <div v-for="r in sections.pinnedItems" :key="'p' + r.id" class="ph-item pinned" @click="apply(r)">
              <div class="pi-line">
                <span class="pi-topic" :title="r.topic">{{ r.topic }}</span>
                <span class="badge">Q{{ r.qos }}</span>
                <span v-if="r.retain" class="badge retain">R</span>
              </div>
              <div class="pi-payload">{{ r.payload }}</div>
              <div class="pi-meta">
                <span class="pi-time">{{ fmtTime(r.ts) }}</span>
                <button class="star on" title="★" @click.stop="togglePinned(r)">★</button>
              </div>
            </div>
          </div>

          <!-- 按 topic 分组 -->
          <div v-for="g in sections.groups" :key="g.topic" class="ph-section">
            <div class="ph-group-head" @click="toggleTopic(g.topic)">
              <span class="arrow-sm">{{ isFolded(g.topic) ? '▸' : '▾' }}</span>
              <span class="g-topic" :title="g.topic">{{ g.topic }}</span>
              <span class="badge">{{ g.items.length }}</span>
            </div>
            <div v-if="!isFolded(g.topic)">
              <div v-for="r in g.items" :key="r.id" class="ph-item" @click="apply(r)">
                <div class="pi-payload">{{ r.payload }}</div>
                <div class="pi-meta">
                  <span class="pi-time">{{ fmtTime(r.ts) }}</span>
                  <button class="star" :class="{ on: r.pinned }" title="★" @click.stop="togglePinned(r)">★</button>
                </div>
              </div>
            </div>
          </div>

          <div v-if="sections.total === 0" class="ph-empty">{{ t('ph.empty') }}</div>
        </template>
        <div v-else class="ph-empty">{{ t('ph.empty') }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ph {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin: 0;
  position: relative;
}
.ph-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 12px 8px;
  cursor: pointer;
  user-select: none;
}
.arrow {
  font-size: 11px;
  color: var(--text-faint);
}
.ph-title {
  font-weight: 600;
  font-size: 12px;
}
.del {
  margin-left: auto;
  color: var(--danger);
}
.ph-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 0 8px 8px;
}
.ph-search {
  font-size: 12px;
  margin-bottom: 6px;
  flex-shrink: 0;
}
/* 面板唯一滚动容器：搜索框固定，列表整体滚动 */
.ph-scroll {
  flex: 1 1 0;
  min-height: 0;
  overflow-y: auto;
}
.ph-section {
  margin-bottom: 4px;
}
.ph-section-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--primary);
  padding: 6px 4px 4px;
}
.ph-group-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 4px 4px;
  cursor: pointer;
  border-radius: 5px;
}
.ph-group-head:hover {
  background: var(--panel-2);
}
.arrow-sm {
  font-size: 10px;
  color: var(--text-faint);
}
.g-topic {
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ph-item {
  padding: 5px 8px;
  margin: 2px 0 2px 14px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--panel);
}
.ph-item:hover {
  background: var(--panel-2);
}
.ph-item.pinned {
  border-color: color-mix(in srgb, var(--warning) 45%, var(--border));
}
.pi-line {
  display: flex;
  align-items: center;
  gap: 6px;
}
.pi-topic {
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pi-payload {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
.pi-meta {
  display: flex;
  align-items: center;
  margin-top: 2px;
}
.pi-time {
  font-size: 10px;
  color: var(--text-faint);
  font-family: var(--mono);
  flex: 1;
}
.star {
  border: none;
  background: transparent;
  cursor: pointer;
  color: var(--text-faint);
  font-size: 13px;
  padding: 0 2px;
  line-height: 1;
}
.star:hover {
  color: var(--warning);
}
.star.on {
  color: var(--warning);
}
.ph-empty {
  text-align: center;
  color: var(--text-faint);
  font-size: 12px;
  padding: 20px 0;
}
.badge.retain {
  color: var(--warning);
}
</style>
