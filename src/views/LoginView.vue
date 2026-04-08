<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

const router = useRouter()
const repoStore = useRepoStore()

type Step = 'idle' | 'loading' | 'verify' | 'polling' | 'done' | 'setup'
const step = ref<Step>('idle')
const userCode = ref('')
const verifyUrl = ref('')
const error = ref('')
const copiedCode = ref(false)

// First-run setup state
const clientId = ref('')
const setupStep = ref(1) // 1 = instructions, 2 = paste id

let unlistenSuccess: UnlistenFn | null = null
let unlistenError: UnlistenFn | null = null

onMounted(async () => {
  unlistenSuccess = await listen('oauth-success', async () => {
    step.value = 'done'
    await repoStore.loadCurrentUser()
    router.replace('/repos')
  })
  unlistenError = await listen<string>('oauth-error', (e) => {
    error.value = e.payload
    step.value = 'idle'
  })

  // Check if Client ID is configured
  const configured = await invoke<boolean>('is_oauth_configured')
  if (!configured) {
    step.value = 'setup'
  }
})

onUnmounted(() => {
  unlistenSuccess?.()
  unlistenError?.()
})

const openDevSettings = () =>
  openUrl('https://github.com/settings/developers')

const saveClientId = async () => {
  if (!clientId.value.trim() || clientId.value.length < 10) {
    error.value = 'Client ID 格式不对，请检查'
    return
  }
  error.value = ''
  await invoke('set_github_client_id', { clientId: clientId.value.trim() })
  step.value = 'idle'
}

const startLogin = async () => {
  step.value = 'loading'
  error.value = ''
  try {
    const info = await invoke<{ userCode: string; verificationUri: string; expiresIn: number }>(
      'start_github_oauth'
    )
    userCode.value = info.userCode
    verifyUrl.value = info.verificationUri
    step.value = 'verify'
  } catch (e) {
    error.value = String(e)
    step.value = 'idle'
  }
}

const copyCode = async () => {
  await navigator.clipboard.writeText(userCode.value)
  copiedCode.value = true
  setTimeout(() => { copiedCode.value = false }, 2000)
}

const openAuthBrowser = async () => {
  await openUrl(verifyUrl.value)
  step.value = 'polling'
}
</script>

<template>
  <div class="flex h-screen flex-col items-center justify-center bg-[#0d1117]">
    <!-- Logo -->
    <div class="mb-8 flex flex-col items-center gap-4">
      <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-[#388bfd] shadow-lg shadow-[#388bfd]/30">
        <span class="text-2xl font-bold text-white">RM</span>
      </div>
      <div class="text-center">
        <h1 class="text-2xl font-semibold text-[#e6edf3]">RepoMind</h1>
        <p class="mt-1 text-sm text-[#8b949e]">代码仓库 AI 知识管家</p>
      </div>
    </div>

    <!-- ── 首次配置引导 ── -->
    <div v-if="step === 'setup'" class="w-[420px] rounded-xl border border-[#d29922]/40 bg-[#161b22] p-8">
      <div class="mb-5 flex items-center gap-2">
        <span class="text-lg">🔧</span>
        <h2 class="font-semibold text-[#e6edf3]">首次配置（仅需一次）</h2>
      </div>

      <!-- Sub-step 1: instructions -->
      <template v-if="setupStep === 1">
        <p class="mb-4 text-sm text-[#8b949e] leading-relaxed">
          RepoMind 需要一个 GitHub OAuth App 来读取你的仓库列表。
          只需注册一次，之后登录只需点一下按钮。
        </p>

        <ol class="mb-6 space-y-3 text-sm">
          <li class="flex gap-3">
            <span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#388bfd] text-xs font-bold text-white">1</span>
            <span class="text-[#e6edf3]">点击下方按钮，打开 GitHub 开发者设置</span>
          </li>
          <li class="flex gap-3">
            <span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#388bfd] text-xs font-bold text-white">2</span>
            <div class="text-[#e6edf3]">
              点 <strong>New OAuth App</strong>，填写：
              <div class="mt-1.5 rounded bg-[#0d1117] p-2 font-mono text-xs text-[#8b949e] space-y-0.5">
                <div>Name: <span class="text-[#e6edf3]">RepoMind</span></div>
                <div>Homepage URL: <span class="text-[#e6edf3]">http://localhost</span></div>
                <div>Callback URL: <span class="text-[#e6edf3]">http://localhost</span></div>
                <div class="text-[#3fb950]">✓ Enable Device Flow（勾选）</div>
              </div>
            </div>
          </li>
          <li class="flex gap-3">
            <span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#388bfd] text-xs font-bold text-white">3</span>
            <span class="text-[#e6edf3]">注册后复制 <strong>Client ID</strong>，粘贴到下一步</span>
          </li>
        </ol>

        <div class="flex gap-3">
          <button
            class="flex flex-1 items-center justify-center gap-2 rounded-lg bg-[#238636] px-4 py-2.5 text-sm font-medium text-white hover:bg-[#2ea043]"
            @click="openDevSettings"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
            </svg>
            打开 GitHub 开发者设置
          </button>
          <button
            class="rounded-lg border border-[#388bfd] px-4 py-2.5 text-sm font-medium text-[#388bfd] hover:bg-[#388bfd]/10"
            @click="setupStep = 2"
          >
            已注册 →
          </button>
        </div>
      </template>

      <!-- Sub-step 2: paste Client ID -->
      <template v-else>
        <p class="mb-4 text-sm text-[#8b949e]">
          将 GitHub OAuth App 的 <strong class="text-[#e6edf3]">Client ID</strong> 粘贴到下方：
        </p>
        <input
          v-model="clientId"
          class="input mb-2 font-mono"
          placeholder="Ov23li..."
          autofocus
          @keyup.enter="saveClientId"
        />
        <p v-if="error" class="mb-2 text-xs text-[#f85149]">{{ error }}</p>
        <div class="flex gap-3">
          <button class="btn-secondary text-sm flex-shrink-0" @click="setupStep = 1">← 返回</button>
          <button class="btn-primary flex-1 text-sm" @click="saveClientId">
            确认，开始使用
          </button>
        </div>
        <p class="mt-3 text-xs text-center text-[#484f58]">
          Client ID 是公开信息，安全存储在本机，不会上传
        </p>
      </template>
    </div>

    <!-- ── 正常登录流程 ── -->
    <div v-else class="w-[340px] rounded-xl border border-[#30363d] bg-[#161b22] p-8">

      <!-- idle -->
      <template v-if="step === 'idle'">
        <h2 class="mb-2 text-center text-lg font-medium text-[#e6edf3]">连接 GitHub</h2>
        <p class="mb-6 text-center text-sm text-[#8b949e]">点击后会在浏览器中完成授权</p>
        <button
          class="flex w-full items-center justify-center gap-3 rounded-lg border border-[#30363d] bg-[#21262d] px-4 py-3 text-sm font-medium text-[#e6edf3] hover:bg-[#30363d] transition-colors"
          @click="startLogin"
        >
          <svg class="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0 1 12 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
          </svg>
          使用 GitHub 登录
        </button>
        <p v-if="error" class="mt-3 text-center text-sm text-[#f85149]">{{ error }}</p>
        <p class="mt-4 text-center text-xs text-[#484f58]">代码本地处理，不上传任何服务器</p>
      </template>

      <!-- loading -->
      <template v-else-if="step === 'loading'">
        <div class="flex flex-col items-center gap-3 py-4">
          <div class="h-8 w-8 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
          <p class="text-sm text-[#8b949e]">正在获取授权码...</p>
        </div>
      </template>

      <!-- verify / polling -->
      <template v-else-if="step === 'verify' || step === 'polling'">
        <div class="space-y-5">
          <p class="text-center text-sm text-[#8b949e]">
            {{ step === 'polling' ? '等待浏览器中完成授权...' : '在 GitHub 页面输入以下验证码：' }}
          </p>
          <div
            class="group relative mx-auto flex cursor-pointer items-center justify-center gap-3 rounded-lg border border-[#388bfd]/50 bg-[#388bfd]/10 px-6 py-4"
            @click="copyCode"
          >
            <span class="font-mono text-2xl font-bold tracking-widest text-[#388bfd]">{{ userCode }}</span>
            <span class="text-xs text-[#484f58] group-hover:text-[#388bfd]">{{ copiedCode ? '已复制 ✓' : '点击复制' }}</span>
          </div>
          <button
            v-if="step === 'verify'"
            class="flex w-full items-center justify-center gap-2 rounded-lg bg-[#238636] px-4 py-3 text-sm font-medium text-white hover:bg-[#2ea043]"
            @click="openAuthBrowser"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
            </svg>
            打开 GitHub 授权页面
          </button>
          <div v-else class="flex items-center justify-center gap-2 py-1">
            <div class="h-4 w-4 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
            <span class="text-sm text-[#8b949e]">等待授权确认...</span>
          </div>
          <button class="w-full text-center text-xs text-[#484f58] hover:text-[#8b949e]" @click="step = 'idle'">取消</button>
        </div>
      </template>

      <!-- done -->
      <template v-else-if="step === 'done'">
        <div class="flex flex-col items-center gap-3 py-4">
          <div class="flex h-12 w-12 items-center justify-center rounded-full bg-[#3fb950]/20 text-2xl">✓</div>
          <p class="text-sm text-[#3fb950]">授权成功，正在跳转...</p>
        </div>
      </template>
    </div>

    <div class="mt-8 flex gap-6">
      <div v-for="f in ['知识图谱', '增量索引', 'AI 搜索', 'MCP 集成']" :key="f" class="text-xs text-[#484f58]">✦ {{ f }}</div>
    </div>
  </div>
</template>
