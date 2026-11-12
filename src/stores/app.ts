import { defineStore } from 'pinia'
import { api } from '../api'
import { useI18n } from '../i18n'
import type { AppSettings, ConnectionProfile, UpdateInfo } from '../types'

interface Toast {
  id: number
  msg: string
  type: 'info' | 'error' | 'success'
}

let toastSeq = 0

export const useAppStore = defineStore('app', {
  state: () => ({
    settings: {
      theme: 'light',
      lang: 'zh',
      maxMessagesInMemory: 1000,
      autoResub: true,
      giteeToken: '',
    } as AppSettings,
    inited: false,
    toasts: [] as Toast[],
    /** 连接表单（全屏模式：打开时左侧栏隐藏） */
    connForm: {
      visible: false,
      editing: null as ConnectionProfile | null,
    },
    /** 连接面板（侧栏）手动收起 */
    sidebarCollapsed: localStorage.getItem('mqttxx.sidebarCollapsed') === '1',
    /** 可用的新版本（null = 无更新） */
    updateInfo: null as UpdateInfo | null,
  }),
  actions: {
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
      localStorage.setItem('mqttxx.sidebarCollapsed', this.sidebarCollapsed ? '1' : '0')
    },
    /** 检查更新；silent=false 时弹结果提示（手动检查用） */
    async checkForUpdate(silent = true): Promise<UpdateInfo | null> {
      const { t } = useI18n()
      try {
        const info = await api.checkUpdate()
        this.updateInfo = info
        if (info && !silent) this.toast(t('update.newFound', { v: info.latestVersion }), 'success')
        if (!info && !silent) this.toast(t('update.upToDate'), 'success')
        return info
      } catch (e) {
        if (!silent) this.toast(`${t('update.failed')}: ${e}`, 'error')
        return null
      }
    },
    openConnForm(editing: ConnectionProfile | null = null) {
      this.connForm.editing = editing
      this.connForm.visible = true
    },
    closeConnForm() {
      this.connForm.visible = false
      this.connForm.editing = null
    },
    async init() {
      if (this.inited) return
      this.inited = true
      try {
        this.settings = await api.settingsGet()
      } catch (e) {
        console.error('failed to load settings', e)
      }
      this.applyTheme()
      // 启动自动检查更新（后台静默）
      if (this.settings.autoCheckUpdate) {
        const { t } = useI18n()
        this.checkForUpdate(true).then((info) => {
          if (info) this.toast(t('update.newFound', { v: info.latestVersion }), 'info', 6000)
        })
      }
    },
    applyTheme() {
      document.documentElement.dataset.theme = this.settings.theme
    },
    async updateSettings(patch: Partial<AppSettings>) {
      try {
        this.settings = await api.settingsSet(patch)
        this.applyTheme()
      } catch (e) {
        this.toast(String(e), 'error')
      }
    },
    toast(msg: string, type: Toast['type'] = 'info', duration = 3000) {
      const id = ++toastSeq
      this.toasts.push({ id, msg, type })
      setTimeout(() => {
        this.toasts = this.toasts.filter((t) => t.id !== id)
      }, duration)
    },
  },
})
