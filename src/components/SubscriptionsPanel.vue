<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'

const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

const topic = ref('')
const qos = ref(0)
const adding = ref(false)
const collapsed = ref(localStorage.getItem('mqttxx.subsCollapsed') === '1')

function toggleCollapsed() {
  collapsed.value = !collapsed.value
  localStorage.setItem('mqttxx.subsCollapsed', collapsed.value ? '1' : '0')
}

async function add() {
  if (!topic.value.trim() || !connStore.activeId) return
  adding.value = true
  try {
    await connStore.addSubscription(connStore.activeId, topic.value, qos.value)
    topic.value = ''
  } catch (e) {
    appStore.toast(String(e), 'error')
  } finally {
    adding.value = false
  }
}

function toggleFilter(topic: string) {
  if (!connStore.activeId) return
  connStore.toggleTopicFilter(connStore.activeId, topic)
}

async function remove(topicId: string, subTopic: string) {
  if (!connStore.activeId) return
  if (!confirm(t('sub.unsubscribeConfirm'))) return
  const sub = connStore.activeSubs.find((s) => s.id === topicId)
  if (!sub) return
  try {
    await connStore.removeSubscription(connStore.activeId, sub)
    void subTopic
  } catch (e) {
    appStore.toast(String(e), 'error')
  }
}
</script>

<template>
  <div class="subs card" :class="{ 'is-collapsed': collapsed }">
    <div class="subs-head" @click="toggleCollapsed">
      <span class="arrow">{{ collapsed ? '▸' : '▾' }}</span>
      <span>{{ t('sub.title') }}</span>
      <span class="badge">{{ connStore.activeSubs.length }}</span>
    </div>
    <div v-if="!collapsed" class="subs-add">
      <input
        v-model="topic"
        class="field-input topic-input"
        :placeholder="t('sub.addPlaceholder')"
        @keydown.enter="add"
      />
      <select v-model.number="qos" class="field-select qos-select">
        <option :value="0">QoS 0</option>
        <option :value="1">QoS 1</option>
        <option :value="2">QoS 2</option>
      </select>
      <button class="btn btn-primary btn-sm" :disabled="adding || !topic.trim()" @click="add">
        {{ t('sub.add') }}
      </button>
    </div>
    <div v-if="!collapsed" class="subs-list">
      <div
        v-for="s in connStore.activeSubs"
        :key="s.id"
        class="sub-item"
        :class="{ selected: s.topic === connStore.activeTopicFilter }"
        :title="t('sub.clickToFilter')"
        @click="toggleFilter(s.topic)"
      >
        <span class="status-dot" :style="{ background: s.color || '#06b17e' }" />
        <span class="sub-topic" :title="s.topic">{{ s.topic }}</span>
        <span class="badge">Q{{ s.qos }}</span>
        <button class="btn btn-ghost btn-sm del" @click="remove(s.id, s.topic)">✕</button>
      </div>
      <div v-if="connStore.activeSubs.length === 0" class="subs-empty">{{ t('sub.empty') }}</div>
    </div>
  </div>
</template>

<style scoped>
.subs {
  /* 高度由父级 --subs-h 控制（上下拖拽调整），折叠时收缩为标题条 */
  height: var(--subs-h, 260px);
  flex-shrink: 0;
  max-height: calc(100% - 44px);
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.subs.is-collapsed {
  height: auto;
  max-height: none;
}
.subs-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 12px 8px;
  font-weight: 600;
  font-size: 12px;
  cursor: pointer;
  user-select: none;
}
.arrow {
  font-size: 11px;
  color: var(--text-faint);
}
.subs-add {
  display: flex;
  gap: 6px;
  padding: 0 12px 10px;
}
.topic-input {
  flex: 1;
  min-width: 0;
  font-size: 12px;
}
.qos-select {
  width: 74px;
  flex-shrink: 0;
  font-size: 12px;
}
.subs-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 10px;
}
.sub-item {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 7px;
  border-radius: 6px;
  cursor: pointer;
}
.sub-item:hover {
  background: var(--panel-2);
}
.sub-item.selected {
  background: color-mix(in srgb, var(--primary) 14%, transparent);
  box-shadow: inset 2px 0 0 var(--primary);
}
.sub-item:hover .del {
  opacity: 1;
}
.sub-topic {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--mono);
  font-size: 12px;
}
.del {
  opacity: 0;
  color: var(--danger);
}
.subs-empty {
  text-align: center;
  color: var(--text-faint);
  font-size: 12px;
  padding: 20px 0;
}
</style>
