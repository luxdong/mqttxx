/** 传输协议（对齐 MQTTX 协议下拉） */
export type Protocol = 'mqtt' | 'mqtts' | 'ws' | 'wss'

export interface ConnectionProfile {
  id: string
  name: string
  clientId: string
  host: string
  port: number
  username: string | null
  password: string | null
  protocol: Protocol
  wsPath: string
  /** 4 = MQTT 3.1.1, 5 = MQTT 5.0 */
  protocolVersion: number
  /** 派生：protocol 为 mqtts/wss */
  ssl: boolean
  sslInsecure: boolean
  ca: string
  clientCert: string
  clientKey: string
  willTopic: string
  willPayload: string
  willQos: number
  willRetain: boolean
  keepalive: number
  cleanSession: boolean
  connectTimeout: number
  autoResub: boolean
  description: string
  createdAt: number
  updatedAt: number
}

export interface ConnectionInput {
  id?: string
  name: string
  clientId?: string
  host: string
  port: number
  username?: string | null
  password?: string | null
  protocol?: Protocol
  wsPath?: string
  protocolVersion?: number
  sslInsecure?: boolean
  ca?: string
  clientCert?: string
  clientKey?: string
  willTopic?: string
  willPayload?: string
  willQos?: number
  willRetain?: boolean
  keepalive?: number
  cleanSession?: boolean
  connectTimeout?: number
  autoResub?: boolean
  description?: string
}

export interface CertPick {
  name: string
  content: string
}

export interface Subscription {
  id: string
  connectionId: string
  topic: string
  qos: number
  color: string
  createdAt: number
}

export interface MessageRecord {
  id: number
  connectionId: string
  /** 0 = received, 1 = published */
  direction: 0 | 1
  topic: string
  qos: number
  retain: boolean
  payload: string
  payloadBinary: boolean
  ts: number
}

export interface PublishRecord {
  id: number
  topic: string
  qos: number
  retain: boolean
  payload: string
  ts: number
  pinned: boolean
}

export interface UpdateInfo {
  currentVersion: string
  latestVersion: string
  notes: string
  assetName: string
  downloadUrl: string
  assetSize: number
}

export interface DownloadProgress {
  received: number
  total: number
}

export interface AppSettings {
  theme: 'light' | 'dark'
  lang: 'zh' | 'en'
  maxMessagesInMemory: number
  autoResub: boolean
  autoCheckUpdate: boolean
  giteeToken: string
}

export type ConnStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'disconnected' | 'error'

export interface ConnStatusEvent {
  id: string
  status: ConnStatus
  error: string | null
}

export interface MessageBatchEvent {
  connectionId: string
  items: MessageRecord[]
}
