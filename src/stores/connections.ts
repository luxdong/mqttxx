import { defineStore } from 'pinia'
import { api, events } from '../api'
import { useI18n } from '../i18n'
import type {
  ConnStatus,
  ConnectionInput,
  ConnectionProfile,
  MessageRecord,
  PublishRecord,
  Subscription,
} from '../types'
import { useAppStore } from './app'

export const SUB_COLORS = ['#e6772e', '#7265e6', '#00b17e', '#00a2ae', '#d4586a', '#8d6cac']

function pickColor(topic: string): string {
  let h = 0
  for (let i = 0; i < topic.length; i++) h = (h * 31 + topic.charCodeAt(i)) | 0
  return SUB_COLORS[Math.abs(h) % SUB_COLORS.length]
}

export const useConnStore = defineStore('connections', {
  state: () => ({
    profiles: [] as ConnectionProfile[],
    activeId: null as string | null,
    statuses: {} as Record<string, ConnStatus>,
    statusErrors: {} as Record<string, string>,
    messages: {} as Record<string, MessageRecord[]>,
    subs: {} as Record<string, Subscription[]>,
    hasEarlier: {} as Record<string, boolean>,
    /** 每连接的主题筛选（点击订阅项设置，空串 = 不过滤） */
    topicFilters: {} as Record<string, string>,
    /** 每连接的发布历史（常驻面板数据源） */
    publishHistory: {} as Record<string, PublishRecord[]>,
    /** 待回填到发布栏的草稿（PublishHistoryPanel → PublishBar） */
    draftToApply: null as (PublishRecord & { seq: number }) | null,
    draftSeq: 0,
    loadingHistory: false,
    eventsInited: false,
  }),
  getters: {
    activeProfile(state): ConnectionProfile | null {
      return state.profiles.find((p) => p.id === state.activeId) ?? null
    },
    activeStatus(state): ConnStatus {
      return this.activeId ? state.statuses[this.activeId] ?? 'idle' : 'idle'
    },
    activeMessages(state): MessageRecord[] {
      return this.activeId ? state.messages[this.activeId] ?? [] : []
    },
    activeSubs(state): Subscription[] {
      return this.activeId ? state.subs[this.activeId] ?? [] : []
    },
    activeTopicFilter(state): string {
      return this.activeId ? state.topicFilters[this.activeId] ?? '' : ''
    },
  },
  actions: {
    async init() {
      if (this.eventsInited) return
      this.eventsInited = true
      await this.loadProfiles()
      events.onConnStatus((ev) => {
        this.statuses[ev.id] = ev.status
        this.statusErrors[ev.id] = ev.error ?? ''
        if (ev.status !== 'connected' && ev.error) {
          useAppStore().toast(ev.error, 'error')
        }
      })
      events.onMessages((ev) => {
        const arr = this.messages[ev.connectionId] ?? (this.messages[ev.connectionId] = [])
        arr.push(...ev.items)
        this.trimMemory(ev.connectionId)
      })
    },
    trimMemory(connectionId: string) {
      const max = useAppStore().settings.maxMessagesInMemory
      const arr = this.messages[connectionId]
      if (arr && arr.length > max) {
        this.messages[connectionId] = arr.slice(arr.length - max)
      }
    },
    async loadProfiles() {
      try {
        this.profiles = await api.connectionList()
      } catch (e) {
        useAppStore().toast(String(e), 'error')
      }
    },
    async select(id: string | null) {
      this.activeId = id
      if (!id) return
      if (this.messages[id] === undefined) await this.loadHistory(id)
      if (this.subs[id] === undefined) {
        try {
          this.subs[id] = await api.subscriptionList(id)
        } catch (e) {
          useAppStore().toast(String(e), 'error')
        }
      }
      if (this.publishHistory[id] === undefined) await this.loadPublishHistory(id)
    },
    async loadHistory(id: string, before?: number) {
      if (this.loadingHistory) return
      this.loadingHistory = true
      try {
        const items = await api.messageList(id, before, 200)
        this.hasEarlier[id] = items.length === 200
        if (before === undefined) {
          // 最新一页，正序放置
          this.messages[id] = items.slice().reverse()
        } else {
          const arr = this.messages[id] ?? (this.messages[id] = [])
          arr.unshift(...items.slice().reverse())
        }
        this.trimMemory(id)
      } catch (e) {
        useAppStore().toast(String(e), 'error')
      } finally {
        this.loadingHistory = false
      }
    },
    async saveConnection(input: ConnectionInput) {
      // 编辑运行中的连接：先断开，让新配置在下次连接时生效
      if (input.id) {
        const st = this.statuses[input.id]
        if (st === 'connected' || st === 'connecting' || st === 'reconnecting') {
          await api.disconnect(input.id).catch(() => undefined)
        }
      }
      const saved = await api.connectionSave(input)
      const idx = this.profiles.findIndex((p) => p.id === saved.id)
      if (idx >= 0) this.profiles[idx] = saved
      else this.profiles.unshift(saved)
      return saved
    },
    /** 复制连接配置生成新连接（clientId 重新生成，避免与原连接冲突） */
    async duplicateConnection(id: string) {
      const src = this.profiles.find((p) => p.id === id)
      if (!src) return null
      const { t } = useI18n()
      const saved = await api.connectionSave({
        name: `${src.name}-${t('conn.copySuffix')}`,
        clientId: `mqttxx-${Math.random().toString(16).slice(2, 10)}`,
        host: src.host,
        port: src.port,
        username: src.username,
        password: src.password,
        protocol: src.protocol,
        wsPath: src.wsPath,
        protocolVersion: src.protocolVersion,
        sslInsecure: src.sslInsecure,
        ca: src.ca,
        clientCert: src.clientCert,
        clientKey: src.clientKey,
        willTopic: src.willTopic,
        willPayload: src.willPayload,
        willQos: src.willQos,
        willRetain: src.willRetain,
        keepalive: src.keepalive,
        cleanSession: src.cleanSession,
        connectTimeout: src.connectTimeout,
        autoResub: src.autoResub,
        description: src.description,
      })
      this.profiles.unshift(saved)
      return saved
    },
    async removeConnection(id: string) {
      await api.connectionDelete(id)
      this.profiles = this.profiles.filter((p) => p.id !== id)
      delete this.messages[id]
      delete this.subs[id]
      delete this.statuses[id]
      delete this.hasEarlier[id]
      delete this.topicFilters[id]
      delete this.publishHistory[id]
      if (this.activeId === id) this.activeId = null
    },
    async connect(id: string) {
      try {
        this.statuses[id] = 'connecting'
        await api.connect(id)
      } catch (e) {
        this.statuses[id] = 'error'
        this.statusErrors[id] = String(e)
        useAppStore().toast(String(e), 'error')
      }
    },
    async disconnect(id: string) {
      try {
        await api.disconnect(id)
      } catch (e) {
        useAppStore().toast(String(e), 'error')
      }
    },
    async addSubscription(connectionId: string, topic: string, qos: number) {
      const t = topic.trim()
      if (!t) return
      const existing = this.subs[connectionId] ?? (this.subs[connectionId] = [])
      if (existing.some((s) => s.topic === t)) {
        useAppStore().toast('topic already subscribed', 'info')
        return
      }
      const sub = await api.subscriptionAdd(connectionId, t, qos, pickColor(t))
      existing.push(sub)
      // 已连接则立即在线订阅（重连时 Rust 侧会按订阅列表自动恢复）
      if (this.statuses[connectionId] === 'connected') {
        try {
          await api.subscribeLive(connectionId, t, qos)
        } catch (e) {
          useAppStore().toast(String(e), 'error')
        }
      }
    },
    async removeSubscription(connectionId: string, sub: Subscription) {
      await api.subscriptionDelete(sub.id)
      const arr = this.subs[connectionId]
      if (arr) this.subs[connectionId] = arr.filter((s) => s.id !== sub.id)
      if (this.statuses[connectionId] === 'connected') {
        try {
          await api.unsubscribeLive(connectionId, sub.topic)
        } catch (e) {
          useAppStore().toast(String(e), 'error')
        }
      }
    },
    async publish(input: { connectionId: string; topic: string; qos: number; retain: boolean; payload: string }) {
      await api.publish(input)
    },
    /** 点击订阅项切换主题筛选；传入当前筛选则取消 */
    toggleTopicFilter(id: string, topic: string) {
      const cur = this.topicFilters[id] ?? ''
      this.topicFilters[id] = cur === topic ? '' : topic
    },
    clearTopicFilter(id: string) {
      this.topicFilters[id] = ''
    },
    /** 把一条记录回填到发布栏 */
    applyDraft(record: PublishRecord) {
      this.draftSeq += 1
      this.draftToApply = { ...record, seq: this.draftSeq }
    },
    async loadPublishHistory(id: string) {
      try {
        this.publishHistory[id] = await api.publishHistoryList(id)
      } catch (e) {
        useAppStore().toast(String(e), 'error')
      }
    },
    async togglePublishPinned(id: string, record: PublishRecord) {
      const next = !record.pinned
      await api.publishHistorySetPinned(record.id, next)
      record.pinned = next
      void id
    },
    async clearPublishHistory(id: string) {
      await api.publishHistoryClear(id)
      const arr = this.publishHistory[id]
      if (arr) this.publishHistory[id] = arr.filter((r) => r.pinned)
    },
    async clearMessages(id: string) {
      await api.messageClear(id)
      this.messages[id] = []
      this.hasEarlier[id] = false
    },
  },
})
