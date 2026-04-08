<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { useSettingsStore } from '@/stores/settings'
import { listen } from '@tauri-apps/api/event'
import type { IndexProgress } from '@/types'

const router = useRouter()
const repoStore = useRepoStore()
const settingsStore = useSettingsStore()

onMounted(async () => {
  await settingsStore.load()
  await repoStore.loadCurrentUser()

  if (!repoStore.isAuthenticated) {
    router.replace('/login')
  }

  // Listen for index progress events from Rust backend
  await listen<IndexProgress>('index-progress', (event) => {
    repoStore.updateIndexProgress(event.payload)
  })

  // Listen for file change events (incremental update prompt)
  await listen<string>('repo-changed', (event) => {
    repoStore.markStale(event.payload)
  })
})
</script>

<template>
  <router-view />
</template>
