import { createRouter, createWebHashHistory } from 'vue-router'
import WorkspaceView from './views/WorkspaceView.vue'
import SettingsView from './views/SettingsView.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/workspace' },
    { path: '/workspace', name: 'workspace', component: WorkspaceView },
    { path: '/workspace/:id', name: 'workspace-conn', component: WorkspaceView },
    { path: '/settings', name: 'settings', component: SettingsView },
  ],
})
