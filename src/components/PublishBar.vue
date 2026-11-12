<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'

const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()

const topic = ref('')
const payload = ref('')
const qos = ref(0)
const retain = ref(false)
const sending = ref(false)

const connected = () => connStore.activeStatus === 'connected'

// 发布记录面板点击条目 → 回填到表单
watch(
  () => connStore.draftToApply,
  (d) => {
    if (!d) return
    topic.value = d.topic
    payload.value = d.payload
    qos.value = d.qos
    retain.value = d.retain
  },
)

// 切换连接：清空表单
watch(
  () => connStore.activeId,
  () => {
    topic.value = ''
    payload.value = ''
    qos.value = 0
    retain.value = false
  },
)

function formatJson() {
  try {
    payload.value = JSON.stringify(JSON.parse(payload.value), null, 2)
  } catch {
    appStore.toast('invalid JSON', 'error')
  }
}

async function send() {
  if (!connected() || !connStore.activeId || !topic.value.trim()) return
  sending.value = true
  try {
    await connStore.publish({
      connectionId: connStore.activeId,
      topic: topic.value.trim(),
      qos: qos.value,
      retain: retain.value,
      payload: payload.value,
    })
    // 发布记录由 Rust 侧落库，刷新常驻面板
    await connStore.loadPublishHistory(connStore.activeId)
  } catch (e) {
    appStore.toast(String(e), 'error')
  } finally {
    sending.value = false
  }
}
</script>

<template>
  <div class="pub card">
    <div class="pub-row">
      <input
        v-model="topic"
        class="field-input topic"
        :placeholder="t('pub.topicPlaceholder')"
        :disabled="!connected()"
        @keydown.enter="send"
      />
      <select v-model.number="qos" class="field-select qos" :disabled="!connected()">
        <option :value="0">QoS 0</option>
        <option :value="1">QoS 1</option>
        <option :value="2">QoS 2</option>
      </select>
      <label class="retain" :class="{ disabled: !connected() }">
        <input v-model="retain" type="checkbox" :disabled="!connected()" /> {{ t('pub.retain') }}
      </label>
      <button class="btn btn-sm" :disabled="!connected()" @click="formatJson">{{ t('pub.format') }}</button>
      <button class="btn btn-primary" :disabled="!connected() || !topic.trim() || sending" @click="send">
        {{ t('pub.send') }} ⇧
      </button>
    </div>
    <textarea
      v-model="payload"
      class="field-textarea payload"
      rows="4"
      spellcheck="false"
      :placeholder="t('pub.payloadPlaceholder')"
      :disabled="!connected()"
      @keydown.ctrl.enter.prevent="send"
      @keydown.meta.enter.prevent="send"
    />
  </div>
</template>

<style scoped>
.pub {
  flex-shrink: 0;
  margin: 10px 14px 14px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.pub-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.topic {
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
}
.qos {
  width: 86px;
}
.retain {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  cursor: pointer;
}
.retain.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.payload {
  margin: 0;
}
</style>
