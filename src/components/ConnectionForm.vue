<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '../api'
import { useI18n } from '../i18n'
import { useAppStore } from '../stores/app'
import { useConnStore } from '../stores/connections'
import type { ConnectionProfile, Protocol } from '../types'

const props = defineProps<{ profile: ConnectionProfile | null }>()
const visible = defineModel<boolean>({ default: false })

const { t } = useI18n()
const appStore = useAppStore()
const connStore = useConnStore()
const router = useRouter()

const PROTOCOL_PORT: Record<Protocol, number> = {
  mqtt: 1883,
  mqtts: 8883,
  ws: 8083,
  wss: 8084,
}

const form = reactive({
  name: '',
  clientId: '',
  protocol: 'mqtt' as Protocol,
  wsPath: '/mqtt',
  host: 'broker.emqx.io',
  port: 1883,
  username: '',
  password: '',
  sslInsecure: false,
  ca: '',
  clientCert: '',
  clientKey: '',
  caName: '',
  certName: '',
  keyName: '',
  protocolVersion: 4,
  willTopic: '',
  willPayload: '',
  willQos: 0,
  willRetain: false,
  keepalive: 60,
  cleanSession: true,
  connectTimeout: 30,
  autoResub: true,
  description: '',
})
const advancedOpen = ref(false)
const willOpen = ref(false)
const portTouched = ref(false)
const saving = ref(false)
const picking = ref<'ca' | 'cert' | 'key' | null>(null)

const isTls = computed(() => form.protocol === 'mqtts' || form.protocol === 'wss')
const isWs = computed(() => form.protocol === 'ws' || form.protocol === 'wss')

function randomClientId() {
  return `mqttxx-${Math.random().toString(16).slice(2, 10)}`
}

watch(visible, (v) => {
  if (!v) return
  portTouched.value = false
  const p = props.profile
  if (p) {
    Object.assign(form, {
      name: p.name,
      clientId: p.clientId,
      protocol: p.protocol,
      wsPath: p.wsPath,
      host: p.host,
      port: p.port,
      username: p.username ?? '',
      password: p.password ?? '',
      sslInsecure: p.sslInsecure,
      ca: p.ca,
      clientCert: p.clientCert,
      clientKey: p.clientKey,
      caName: p.ca ? 'certificate.pem' : '',
      certName: p.clientCert ? 'certificate.pem' : '',
      keyName: p.clientKey ? 'private.key' : '',
      protocolVersion: p.protocolVersion,
      willTopic: p.willTopic,
      willPayload: p.willPayload,
      willQos: p.willQos,
      willRetain: p.willRetain,
      keepalive: p.keepalive,
      cleanSession: p.cleanSession,
      connectTimeout: p.connectTimeout,
      autoResub: p.autoResub,
      description: p.description,
    })
  } else {
    Object.assign(form, {
      name: '',
      clientId: randomClientId(),
      protocol: 'mqtt',
      wsPath: '/mqtt',
      host: 'broker.emqx.io',
      port: 1883,
      username: '',
      password: '',
      sslInsecure: false,
      ca: '',
      clientCert: '',
      clientKey: '',
      caName: '',
      certName: '',
      keyName: '',
      protocolVersion: 4,
      willTopic: '',
      willPayload: '',
      willQos: 0,
      willRetain: false,
      keepalive: 60,
      cleanSession: true,
      connectTimeout: 30,
      autoResub: true,
      description: '',
    })
  }
})

watch(
  () => form.protocol,
  (p) => {
    if (!portTouched.value) form.port = PROTOCOL_PORT[p]
    if (!portTouched.value) portTouched.value = false
  },
)

async function pickFile(which: 'ca' | 'cert' | 'key') {
  picking.value = which
  try {
    const picked = await api.pickCertFile()
    if (!picked) return
    if (which === 'ca') {
      form.ca = picked.content
      form.caName = picked.name
    } else if (which === 'cert') {
      form.clientCert = picked.content
      form.certName = picked.name
    } else {
      form.clientKey = picked.content
      form.keyName = picked.name
    }
  } catch (e) {
    appStore.toast(String(e), 'error')
  } finally {
    picking.value = null
  }
}

function clearFile(which: 'ca' | 'cert' | 'key') {
  if (which === 'ca') {
    form.ca = ''
    form.caName = ''
  } else if (which === 'cert') {
    form.clientCert = ''
    form.certName = ''
  } else {
    form.clientKey = ''
    form.keyName = ''
  }
}

async function save() {
  if (!form.name.trim()) {
    appStore.toast(t('conn.form.namePlaceholder'), 'error')
    return
  }
  if (!form.host.trim()) {
    appStore.toast(t('conn.form.host'), 'error')
    return
  }
  if (form.clientCert && !form.clientKey) {
    appStore.toast(t('conn.form.certKeyPair'), 'error')
    return
  }
  saving.value = true
  try {
    const saved = await connStore.saveConnection({
      id: props.profile?.id,
      name: form.name.trim(),
      clientId: form.clientId.trim() || randomClientId(),
      protocol: form.protocol,
      wsPath: form.wsPath.trim() || '/mqtt',
      host: form.host.trim(),
      port: Number(form.port) || PROTOCOL_PORT[form.protocol],
      username: form.username || null,
      password: form.password || null,
      protocolVersion: form.protocolVersion,
      sslInsecure: form.sslInsecure,
      ca: form.ca,
      clientCert: form.clientCert,
      clientKey: form.clientKey,
      willTopic: form.willTopic.trim(),
      willPayload: form.willPayload,
      willQos: form.willQos,
      willRetain: form.willRetain,
      keepalive: Number(form.keepalive) || 60,
      cleanSession: form.cleanSession,
      connectTimeout: Number(form.connectTimeout) || 30,
      autoResub: form.autoResub,
      description: form.description,
    })
    visible.value = false
    // 新建后直接选中新连接
    if (!props.profile) {
      await connStore.select(saved.id)
      router.push(`/workspace/${saved.id}`)
    }
    appStore.toast(t('settings.saved'), 'success')
  } catch (e) {
    appStore.toast(String(e), 'error')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="form-fullscreen">
      <div class="form-card">
        <div class="form-head">
          <span>{{ profile ? t('conn.form.edit') : t('conn.form.new') }}</span>
          <button class="btn btn-ghost btn-sm" @click="visible = false">✕</button>
        </div>
        <div class="form-body">
    <div class="grid2">
      <div>
        <label class="field-label">{{ t('conn.form.name') }}</label>
        <input v-model="form.name" class="field-input" :placeholder="t('conn.form.namePlaceholder')" />
      </div>
      <div>
        <label class="field-label">{{ t('conn.form.clientId') }}</label>
        <div class="row-gap">
          <input v-model="form.clientId" class="field-input" />
          <button class="btn btn-sm" :title="t('conn.form.genClientId')" @click="form.clientId = randomClientId()">⟳</button>
        </div>
      </div>
    </div>

    <!-- 协议 + 主机 + 端口（对齐 MQTTX 服务器地址行） -->
    <label class="field-label">{{ t('conn.form.host') }}</label>
    <div class="addr-row">
      <select v-model="form.protocol" class="field-select proto-select">
        <option value="mqtt">mqtt://</option>
        <option value="mqtts">mqtts://</option>
        <option value="ws">ws://</option>
        <option value="wss">wss://</option>
      </select>
      <input v-model="form.host" class="field-input host-input" placeholder="broker.emqx.io" />
      <input v-model.number="form.port" class="field-input port-input" type="number" @input="portTouched = true" />
    </div>
    <div v-if="isWs" class="grid2">
      <div>
        <label class="field-label">{{ t('conn.form.wsPath') }}</label>
        <input v-model="form.wsPath" class="field-input" placeholder="/mqtt" />
      </div>
    </div>

    <div class="grid2">
      <div>
        <label class="field-label">{{ t('conn.form.username') }}</label>
        <input v-model="form.username" class="field-input" />
      </div>
      <div>
        <label class="field-label">{{ t('conn.form.password') }}</label>
        <input v-model="form.password" class="field-input" type="password" />
      </div>
    </div>

    <!-- 证书区（mqtts/wss）：CA / 客户端证书 / 客户端私钥，均以文件方式添加 -->
    <template v-if="isTls">
      <div class="cert-head">{{ t('conn.form.certSection') }}</div>
      <label class="check-item">
        <input v-model="form.sslInsecure" type="checkbox" /> {{ t('conn.form.sslInsecure') }}
      </label>
      <div class="cert-row">
        <span class="cert-label">CA</span>
        <span class="cert-name" :class="{ empty: !form.caName }">{{ form.caName || t('conn.form.noFile') }}</span>
        <button class="btn btn-sm" :disabled="picking !== null" @click="pickFile('ca')">
          {{ t('conn.form.pickFile') }}
        </button>
        <button v-if="form.caName" class="btn btn-sm btn-danger" @click="clearFile('ca')">{{ t('common.delete') }}</button>
      </div>
      <div class="cert-row">
        <span class="cert-label">{{ t('conn.form.clientCertFile') }}</span>
        <span class="cert-name" :class="{ empty: !form.certName }">{{ form.certName || t('conn.form.noFile') }}</span>
        <button class="btn btn-sm" :disabled="picking !== null" @click="pickFile('cert')">
          {{ t('conn.form.pickFile') }}
        </button>
        <button v-if="form.certName" class="btn btn-sm btn-danger" @click="clearFile('cert')">{{ t('common.delete') }}</button>
      </div>
      <div class="cert-row">
        <span class="cert-label">{{ t('conn.form.clientKeyFile') }}</span>
        <span class="cert-name" :class="{ empty: !form.keyName }">{{ form.keyName || t('conn.form.noFile') }}</span>
        <button class="btn btn-sm" :disabled="picking !== null" @click="pickFile('key')">
          {{ t('conn.form.pickFile') }}
        </button>
        <button v-if="form.keyName" class="btn btn-sm btn-danger" @click="clearFile('key')">{{ t('common.delete') }}</button>
      </div>
      <div class="cert-hint">{{ t('conn.form.certHint') }}</div>
    </template>

    <button class="btn btn-ghost btn-sm adv-toggle" @click="advancedOpen = !advancedOpen">
      {{ advancedOpen ? '▾' : '▸' }} {{ t('conn.form.advanced') }}
    </button>
    <div v-if="advancedOpen" class="adv">
      <div class="grid2">
        <div>
          <label class="field-label">{{ t('conn.form.protocol') }}</label>
          <select v-model.number="form.protocolVersion" class="field-select">
            <option :value="4">MQTT 3.1.1</option>
            <option :value="5">MQTT 5.0</option>
          </select>
        </div>
        <div>
          <label class="field-label">{{ t('conn.form.keepalive') }}</label>
          <input v-model.number="form.keepalive" class="field-input" type="number" min="1" />
        </div>
        <div>
          <label class="field-label">{{ t('conn.form.connectTimeout') }}</label>
          <input v-model.number="form.connectTimeout" class="field-input" type="number" min="1" />
        </div>
        <div class="check-col">
          <label class="check-item">
            <input v-model="form.cleanSession" type="checkbox" /> {{ t('conn.form.cleanSession') }}
          </label>
          <label class="check-item">
            <input v-model="form.autoResub" type="checkbox" /> {{ t('conn.form.autoResub') }}
          </label>
        </div>
      </div>

      <button class="btn btn-ghost btn-sm adv-toggle" @click="willOpen = !willOpen">
        {{ willOpen ? '▾' : '▸' }} {{ t('conn.form.will') }}
      </button>
      <div v-if="willOpen" class="grid2 will">
        <div class="span2">
          <label class="field-label">{{ t('conn.form.willTopic') }}</label>
          <input v-model="form.willTopic" class="field-input" placeholder="client/status" />
        </div>
        <div class="span2">
          <label class="field-label">{{ t('conn.form.willPayload') }}</label>
          <textarea v-model="form.willPayload" class="field-textarea" rows="2" spellcheck="false" />
        </div>
        <div>
          <label class="field-label">{{ t('conn.form.willQos') }}</label>
          <select v-model.number="form.willQos" class="field-select">
            <option :value="0">QoS 0</option>
            <option :value="1">QoS 1</option>
            <option :value="2">QoS 2</option>
          </select>
        </div>
        <label class="check-item">
          <input v-model="form.willRetain" type="checkbox" /> {{ t('conn.form.willRetain') }}
        </label>
      </div>

      <div>
        <label class="field-label">{{ t('conn.form.description') }}</label>
        <input v-model="form.description" class="field-input" />
      </div>
    </div>

        </div>
        <div class="form-foot">
          <button class="btn" @click="visible = false">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" :disabled="saving" @click="save">{{ t('common.save') }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.form-fullscreen {
  position: fixed;
  inset: 0;
  background: var(--bg);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  z-index: 900;
  overflow-y: auto;
  padding: 40px 20px;
}
.form-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
  width: min(680px, 100%);
  display: flex;
  flex-direction: column;
}
.form-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 10px;
  font-size: 16px;
  font-weight: 700;
}
.form-body {
  padding: 4px 20px 16px;
}
.form-foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 20px;
  border-top: 1px solid var(--border);
}
.grid2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 14px;
}
.span2 {
  grid-column: span 2;
}
.row-gap {
  display: flex;
  gap: 6px;
}
.addr-row {
  display: flex;
  gap: 8px;
}
.proto-select {
  width: 108px;
  flex-shrink: 0;
}
.host-input {
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
}
.port-input {
  width: 96px;
  flex-shrink: 0;
}
.cert-head {
  margin-top: 14px;
  font-weight: 600;
  font-size: 13px;
}
.cert-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.cert-label {
  width: 88px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-dim);
}
.cert-name {
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cert-name.empty {
  color: var(--text-faint);
}
.cert-hint {
  margin-top: 6px;
  font-size: 11px;
  color: var(--text-faint);
}
.check-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-dim);
  cursor: pointer;
  padding: 10px 0 4px;
}
.check-col {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}
.adv-toggle {
  margin-top: 12px;
}
.adv {
  margin-top: 4px;
}
.will {
  margin-bottom: 8px;
}
</style>
