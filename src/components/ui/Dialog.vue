<script setup lang="ts">
defineProps<{ title: string; width?: number }>()
const open = defineModel<boolean>({ default: false })
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="dialog-overlay" @click.self="open = false">
      <div class="dialog-box" :style="width ? { width: `${width}px` } : undefined">
        <div class="dialog-head">
          <span>{{ title }}</span>
          <button class="btn btn-ghost btn-sm" @click="open = false">✕</button>
        </div>
        <div class="dialog-body">
          <slot />
        </div>
        <div v-if="$slots.footer" class="dialog-foot">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
