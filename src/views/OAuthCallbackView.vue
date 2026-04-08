<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { githubAuthApi } from '@/services/api'

const router = useRouter()
const repoStore = useRepoStore()

onMounted(async () => {
  // Extract code from URL and exchange for token via Tauri backend
  const params = new URLSearchParams(window.location.search)
  const code = params.get('code')

  if (code) {
    try {
      await (window as any).__TAURI_INTERNALS__?.invoke('handle_oauth_callback', { code })
      await repoStore.loadCurrentUser()
      router.replace('/repos')
    } catch (e) {
      console.error('OAuth callback failed', e)
      router.replace('/login')
    }
  } else {
    router.replace('/login')
  }
})
</script>

<template>
  <div class="flex h-screen items-center justify-center bg-[#0d1117]">
    <div class="flex flex-col items-center gap-3 text-[#8b949e]">
      <div class="h-8 w-8 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      <p class="text-sm">正在完成 GitHub 授权...</p>
    </div>
  </div>
</template>
