<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

const router = useRouter()
const repoStore = useRepoStore()
const isLoading = ref(false)
const error = ref('')

let unlisten: UnlistenFn | null = null

onMounted(async () => {
  // Listen for OAuth success event from Rust backend
  unlisten = await listen('oauth-success', async () => {
    await repoStore.loadCurrentUser()
    if (repoStore.isAuthenticated) {
      router.replace('/repos')
    }
  })
})

onUnmounted(() => unlisten?.())

const startLogin = async () => {
  isLoading.value = true
  error.value = ''
  try {
    await repoStore.startOAuth()
    // The Rust backend opens the browser and starts the local callback server
    // When OAuth completes, it emits 'oauth-success' event
  } catch (e) {
    error.value = '启动登录失败，请重试'
    isLoading.value = false
    console.error(e)
  }
  // Keep loading state until oauth-success arrives (or user closes window)
  setTimeout(() => { isLoading.value = false }, 120_000)
}
</script>

<template>
  <div class="flex h-screen flex-col items-center justify-center bg-[#0d1117]">
    <div class="mb-8 flex flex-col items-center gap-4">
      <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-[#388bfd] shadow-lg shadow-[#388bfd]/30">
        <span class="text-2xl font-bold text-white">RM</span>
      </div>
      <div class="text-center">
        <h1 class="text-2xl font-semibold text-[#e6edf3]">RepoMind</h1>
        <p class="mt-1 text-sm text-[#8b949e]">代码仓库 AI 知识管家</p>
      </div>
    </div>

    <div class="w-80 rounded-xl border border-[#30363d] bg-[#161b22] p-8">
      <h2 class="mb-2 text-center text-lg font-medium text-[#e6edf3]">连接你的代码仓库</h2>
      <p class="mb-6 text-center text-sm text-[#8b949e]">
        通过 GitHub OAuth 授权，管理你的所有代码仓库
      </p>

      <button
        class="flex w-full items-center justify-center gap-3 rounded-lg border border-[#30363d] bg-[#21262d] px-4 py-3 text-sm font-medium text-[#e6edf3] transition-colors hover:bg-[#30363d] disabled:opacity-50"
        :disabled="isLoading"
        @click="startLogin"
      >
        <svg class="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0 1 12 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
        </svg>
        <span v-if="!isLoading">使用 GitHub 登录</span>
        <span v-else class="flex items-center gap-2">
          <span class="h-4 w-4 animate-spin rounded-full border-2 border-[#e6edf3] border-t-transparent" />
          等待授权中...
        </span>
      </button>

      <p v-if="error" class="mt-3 text-center text-sm text-[#f85149]">{{ error }}</p>

      <p class="mt-4 text-center text-xs text-[#484f58]">
        授权后，代码仅在本地处理，不上传任何服务器
      </p>
    </div>

    <div class="mt-8 flex gap-6">
      <div v-for="feature in ['知识图谱', '增量索引', 'AI 搜索', 'MCP 集成']" :key="feature"
        class="text-xs text-[#484f58]">
        ✦ {{ feature }}
      </div>
    </div>
  </div>
</template>
