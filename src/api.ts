import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppSettings,
  CertPick,
  ConnStatusEvent,
  ConnectionInput,
  ConnectionProfile,
  DownloadProgress,
  MessageBatchEvent,
  MessageRecord,
  PublishRecord,
  Subscription,
  UpdateInfo,
} from './types'

export const api = {
  connectionList: () => invoke<ConnectionProfile[]>('connection_list'),
  connectionSave: (input: ConnectionInput) => invoke<ConnectionProfile>('connection_save', { input }),
  connectionDelete: (id: string) => invoke<void>('connection_delete', { id }),

  subscriptionList: (connectionId: string) => invoke<Subscription[]>('subscription_list', { connectionId }),
  subscriptionAdd: (connectionId: string, topic: string, qos: number, color: string) =>
    invoke<Subscription>('subscription_add', { connectionId, topic, qos, color }),
  subscriptionDelete: (id: string) => invoke<void>('subscription_delete', { id }),

  connect: (id: string) => invoke<void>('connect', { id }),
  disconnect: (id: string) => invoke<void>('disconnect', { id }),
  subscribeLive: (connectionId: string, topic: string, qos: number) =>
    invoke<void>('subscribe_live', { connectionId, topic, qos }),
  unsubscribeLive: (connectionId: string, topic: string) =>
    invoke<void>('unsubscribe_live', { connectionId, topic }),
  publish: (input: { connectionId: string; topic: string; qos: number; retain: boolean; payload: string }) =>
    invoke<void>('publish', { input }),

  messageList: (connectionId: string, before?: number, limit?: number) =>
    invoke<MessageRecord[]>('message_list', { connectionId, before: before ?? null, limit: limit ?? 200 }),
  messageClear: (connectionId: string) => invoke<void>('message_clear', { connectionId }),

  pickCertFile: () => invoke<CertPick | null>('pick_cert_file'),

  publishHistoryList: (connectionId: string, limit?: number) =>
    invoke<PublishRecord[]>('publish_history_list', { connectionId, limit: limit ?? 200 }),
  publishHistorySetPinned: (id: number, pinned: boolean) =>
    invoke<void>('publish_history_set_pinned', { id, pinned }),
  publishHistoryClear: (connectionId: string) => invoke<void>('publish_history_clear', { connectionId }),

  settingsGet: () => invoke<AppSettings>('settings_get'),
  settingsSet: (patch: Partial<AppSettings>) => invoke<AppSettings>('settings_set', { patch }),

  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  downloadUpdate: (info: UpdateInfo) => invoke<string>('download_update', { info }),
  installUpdate: (filePath: string) => invoke<void>('install_update', { filePath }),
}

export const updateEvents = {
  onProgress: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>('update-progress', (e) => cb(e.payload)),
}

export const events = {
  onConnStatus: (handler: (ev: ConnStatusEvent) => void): Promise<UnlistenFn> =>
    listen<ConnStatusEvent>('conn-status', (e) => handler(e.payload)),
  onMessages: (handler: (ev: MessageBatchEvent) => void): Promise<UnlistenFn> =>
    listen<MessageBatchEvent>('messages', (e) => handler(e.payload)),
}
