<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'
import type { ConnectionProfile } from '../types'
import Dropdown from './ui/Dropdown.vue'

const router = useRouter()
const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

const search = ref('')
const menuRef = ref<InstanceType<typeof Dropdown> | null>(null)
const menuTarget = ref<ConnectionProfile | null>(null)

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return connStore.profiles
  return connStore.profiles.filter(
    (p) => p.name.toLowerCase().includes(q) || `${p.host}:${p.port}`.toLowerCase().includes(q),
  )
})

const menuItems = computed(() => [
  { label: t('common.edit'), key: 'edit' },
  { label: t('conn.duplicate'), key: 'duplicate' },
  { label: t('common.delete'), key: 'delete', danger: true },
])

function openMenu(e: MouseEvent, p: ConnectionProfile) {
  menuTarget.value = p
  menuRef.value?.open(e)
}

function onMenuSelect(key: string) {
  const p = menuTarget.value
  if (!p) return
  if (key === 'edit') {
    appStore.openConnForm(p)
  } else if (key === 'duplicate') {
    connStore
      .duplicateConnection(p.id)
      .then(() => appStore.toast(t('settings.saved'), 'success'))
      .catch((e) => appStore.toast(String(e), 'error'))
  } else if (key === 'delete') {
    if (confirm(t('conn.deleteConfirm'))) {
      connStore.removeConnection(p.id).catch((e) => console.error(e))
      if (connStore.activeId === null || connStore.activeId === p.id) router.push('/workspace')
    }
  }
}

function select(p: ConnectionProfile) {
  connStore.select(p.id)
  router.push(`/workspace/${p.id}`)
}

function openNew() {
  appStore.openConnForm(null)
}

function statusColor(p: ConnectionProfile): string {
  const s = connStore.statuses[p.id] ?? 'idle'
  return (
    {
      connected: '#00b17e',
      connecting: '#e6a23c',
      reconnecting: '#e6a23c',
      error: '#e02020',
    } as Record<string, string>
  )[s] ?? '#9aa4ad'
}
</script>

<template>
  <!-- 收起态：迷你竖条 -->
  <div v-if="appStore.sidebarCollapsed" class="mini-bar">
    <button class="mini-btn" :title="t('sidebar.expand')" @click="appStore.toggleSidebar()">»</button>
    <button class="mini-btn" :title="t('sidebar.new')" @click="openNew()">＋</button>
    <div class="mini-spacer" />
    <button class="mini-btn has-dot" :class="{ dot: appStore.updateInfo }" :title="t('sidebar.settings')" @click="router.push('/settings')">⚙</button>
  </div>

  <aside v-else class="sidebar">
    <div class="brand">
      <div class="logo">M</div>
      <div class="brand-name">{{ t('app.name') }}</div>
      <button class="btn btn-ghost btn-sm" :title="t('sidebar.collapse')" @click="appStore.toggleSidebar()">«</button>
      <button class="btn btn-ghost btn-sm settings-btn" :title="t('sidebar.settings')" @click="router.push('/settings')">
        ⚙<span v-if="appStore.updateInfo" class="red-dot" />
      </button>
    </div>

    <div class="toolbar">
      <input v-model="search" class="field-input search" :placeholder="t('sidebar.searchPlaceholder')" />
      <button class="btn btn-primary new-btn" @click="openNew">＋ {{ t('sidebar.new') }}</button>
    </div>

    <div class="conn-list">
      <div
        v-for="p in filtered"
        :key="p.id"
        class="conn-item"
        :class="{ active: p.id === connStore.activeId }"
        @click="select(p)"
        @contextmenu.prevent="openMenu($event, p)"
      >
        <span class="status-dot" :style="{ background: statusColor(p) }" />
        <div class="conn-meta">
          <div class="conn-name">{{ p.name }}</div>
          <div class="conn-addr">{{ p.protocol }}://{{ p.host }}:{{ p.port }}</div>
        </div>
        <button class="btn btn-ghost btn-sm more" @click.stop="openMenu($event, p)">⋯</button>
      </div>
      <div v-if="filtered.length === 0" class="list-empty">{{ t('sidebar.empty') }}</div>
    </div>

    <Dropdown ref="menuRef" :items="menuItems" @select="onMenuSelect" />
  </aside>
</template>

<style scoped>
.mini-bar {
  width: 44px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 0;
  background: var(--panel);
  border-right: 1px solid var(--border);
}
.mini-btn {
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  border-radius: 7px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 14px;
}
.mini-btn:hover {
  background: var(--panel-2);
  color: var(--primary);
}
.mini-btn.dot {
  color: var(--warning);
}
.settings-btn {
  position: relative;
}
.red-dot {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--danger);
}
.mini-spacer {
  flex: 1;
}
.sidebar {
  width: 264px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border-right: 1px solid var(--border);
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 14px 10px;
}
.logo {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--primary);
  color: #fff;
  font-weight: 700;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.brand-name {
  font-weight: 700;
  font-size: 15px;
  flex: 1;
}
.toolbar {
  padding: 4px 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.search {
  font-size: 12px;
}
.new-btn {
  font-size: 12px;
}
.conn-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 10px;
}
.conn-item {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 8px 9px;
  border-radius: 8px;
  cursor: pointer;
  margin-bottom: 2px;
}
.conn-item:hover {
  background: var(--panel-2);
}
.conn-item.active {
  background: var(--panel-2);
  box-shadow: inset 3px 0 0 var(--primary);
}
.conn-meta {
  flex: 1;
  min-width: 0;
}
.conn-name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.conn-addr {
  font-size: 11px;
  color: var(--text-faint);
  font-family: var(--mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.more {
  opacity: 0;
}
.conn-item:hover .more {
  opacity: 1;
}
.list-empty {
  text-align: center;
  color: var(--text-faint);
  padding: 32px 0;
  font-size: 12px;
}
</style>
