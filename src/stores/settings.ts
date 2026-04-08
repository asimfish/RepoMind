import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AppSettings } from '@/types'
import { settingsApi } from '@/services/api'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    indexStoragePath: '',
    mcpEnabled: true,
    autoIndexOnCommit: true,
    searchLanguage: 'zh',
  })
  const isLoaded = ref(false)

  const load = async () => {
    settings.value = await settingsApi.getSettings()
    isLoaded.value = true
  }

  const save = async (updates: Partial<AppSettings>) => {
    Object.assign(settings.value, updates)
    await settingsApi.updateSettings(updates)
  }

  return { settings, isLoaded, load, save }
})
