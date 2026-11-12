<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'

const props = defineProps<{
  items: { label: string; key: string; danger?: boolean }[]
}>()

const emit = defineEmits<{ (e: 'select', key: string): void }>()

const visible = ref(false)
const pos = ref({ x: 0, y: 0 })

function open(e: MouseEvent) {
  pos.value = { x: e.clientX, y: e.clientY }
  visible.value = true
}

function select(key: string) {
  visible.value = false
  emit('select', key)
}

function onDocClick() {
  visible.value = false
}

onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))

defineExpose({ open })
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="dd-menu" :style="{ left: `${pos.x}px`, top: `${pos.y}px` }">
      <div v-for="it in props.items" :key="it.key" class="dd-item" :class="{ danger: it.danger }" @click.stop="select(it.key)">
        {{ it.label }}
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dd-menu {
  position: fixed;
  z-index: 1500;
  min-width: 132px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
  padding: 4px;
}
.dd-item {
  padding: 7px 12px;
  border-radius: 5px;
  cursor: pointer;
  white-space: nowrap;
}
.dd-item:hover {
  background: var(--panel-2);
}
.dd-item.danger {
  color: var(--danger);
}
</style>
