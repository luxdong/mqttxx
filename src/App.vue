<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useAppStore } from './stores/app'
import { useConnStore } from './stores/connections'
import ConnectionSidebar from './components/ConnectionSidebar.vue'
import ConnectionForm from './components/ConnectionForm.vue'
import TitleBar from './components/TitleBar.vue'

const appStore = useAppStore()
const connStore = useConnStore()

// 连接表单可见性直接由 store 驱动（打开时左侧栏隐藏）
const formVisible = computed({
  get: () => appStore.connForm.visible,
  set: (v: boolean) => (v ? appStore.openConnForm(appStore.connForm.editing) : appStore.closeConnForm()),
})

onMounted(async () => {
  await appStore.init()
  await connStore.init()
})
</script>

<template>
  <div class="app-root">
    <TitleBar />
    <div class="app-shell">
      <ConnectionSidebar v-show="!appStore.connForm.visible" />
      <main class="app-main">
        <router-view />
      </main>
    </div>
    <ConnectionForm v-model="formVisible" :profile="appStore.connForm.editing" />
    <div class="toast-wrap">
      <div v-for="toast in appStore.toasts" :key="toast.id" class="toast" :class="toast.type">
        {{ toast.msg }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-root {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.app-shell {
  flex: 1;
  min-height: 0;
  display: flex;
}
.app-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  overflow: hidden;
}
</style>
