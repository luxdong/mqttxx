<script setup lang="ts" generic="T">
import { computed, nextTick, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    items: T[]
    itemHeight: number
    itemKey: (item: T, index: number) => string | number
    autoScroll?: boolean
    buffer?: number
  }>(),
  { autoScroll: true, buffer: 8 },
)

const emit = defineEmits<{ (e: 'top', value: boolean): void }>()

const container = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewHeight = ref(0)
let stickBottom = true

onMounted(() => {
  viewHeight.value = container.value?.clientHeight ?? 0
})

function onScroll() {
  const el = container.value
  if (!el) return
  scrollTop.value = el.scrollTop
  stickBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - props.itemHeight - 4
  emit('top', el.scrollTop <= 2)
}

const start = computed(() => Math.max(0, Math.floor(scrollTop.value / props.itemHeight) - props.buffer))
const end = computed(
  () => Math.min(props.items.length, start.value + Math.ceil(viewHeight.value / props.itemHeight) + props.buffer * 2),
)
const visible = computed(() => props.items.slice(start.value, end.value))
const totalHeight = computed(() => props.items.length * props.itemHeight)
const padTop = computed(() => start.value * props.itemHeight)

watch(
  () => props.items.length,
  async () => {
    if (props.autoScroll && stickBottom) {
      await nextTick()
      const el = container.value
      if (el) el.scrollTop = el.scrollHeight
    }
  },
)

function scrollToBottom() {
  nextTick(() => {
    const el = container.value
    if (el) {
      el.scrollTop = el.scrollHeight
      stickBottom = true
    }
  })
}

/** prepend 场景：内容整体下移 delta 像素后保持视口位置 */
function scrollBy(delta: number) {
  const el = container.value
  if (el) el.scrollTop += delta
}

defineExpose({ scrollToBottom, scrollBy })
</script>

<template>
  <div ref="container" class="virtual-list" @scroll.passive="onScroll">
    <div class="virtual-phantom" :style="{ height: totalHeight + 'px' }">
      <div class="virtual-window" :style="{ transform: `translateY(${padTop}px)` }">
        <div v-for="(item, i) in visible" :key="itemKey(item, start + i)" :style="{ height: itemHeight + 'px' }">
          <slot name="item" :item="item" :index="start + i" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}
.virtual-phantom {
  position: relative;
}
.virtual-window {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  will-change: transform;
}
</style>
