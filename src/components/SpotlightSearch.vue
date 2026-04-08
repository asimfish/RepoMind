<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { searchApi } from '@/services/api'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { SearchResult } from '@/types'

const router = useRouter()
const repoStore = useRepoStore()
const isOpen = ref(false)
const query = ref('')
const results = ref<SearchResult[]>([])
const isSearching = ref(false)
const activeIndex = ref(0)

let unlistenSpotlight: UnlistenFn | null = null
let searchTimer: ReturnType<typeof setTimeout>

onMounted(async () => {
  // Listen for tray "quick search" menu click
  unlistenSpotlight = await listen('open-spotlight', () => { isOpen.value = true })

  // Global keyboard shortcut: Cmd+Shift+R
  window.addEventListener('keydown', handleGlobalKey)
})

onUnmounted(() => {
  unlistenSpotlight?.()
  window.removeEventListener('keydown', handleGlobalKey)
})

const handleGlobalKey = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'r') {
    e.preventDefault()
    isOpen.value = !isOpen.value
  }
  if (e.key === 'Escape') isOpen.value = false
}

watch(query, () => {
  clearTimeout(searchTimer)
  if (!query.value.trim()) { results.value = []; return }
  searchTimer = setTimeout(doSearch, 250)
})

const doSearch = async () => {
  const indexedRepos = repoStore.indexedRepos.filter(r => r.indexStatus === 'indexed')
  if (!indexedRepos.length || !query.value.trim()) return

  isSearching.value = true
  activeIndex.value = 0
  try {
    // Search across all indexed repos, take first repo for now
    // TODO: parallel search across all repos
    const r = await searchApi.search(indexedRepos[0].id, query.value)
    results.value = r.slice(0, 8)
  } finally {
    isSearching.value = false
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'ArrowDown') { activeIndex.value = Math.min(activeIndex.value + 1, results.value.length - 1); e.preventDefault() }
  if (e.key === 'ArrowUp') { activeIndex.value = Math.max(activeIndex.value - 1, 0); e.preventDefault() }
  if (e.key === 'Enter' && results.value[activeIndex.value]) selectResult(results.value[activeIndex.value])
}

const selectResult = (r: SearchResult) => {
  const repo = repoStore.indexedRepos.find(repo => repo.name === r.repoName)
  if (repo) {
    router.push(`/repos/${repo.id}`)
  }
  isOpen.value = false
  query.value = ''
  results.value = []
}

const typeColors: Record<string, string> = {
  function: 'text-[#388bfd]',
  class: 'text-[#3fb950]',
  method: 'text-[#d29922]',
  variable: 'text-[#8b949e]',
  interface: 'text-[#a371f7]',
  enum: 'text-[#f85149]',
}
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-all duration-150"
      enter-from-class="opacity-0 scale-95"
      enter-to-class="opacity-100 scale-100"
      leave-active-class="transition-all duration-100"
      leave-from-class="opacity-100 scale-100"
      leave-to-class="opacity-0 scale-95"
    >
      <div v-if="isOpen" class="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]">
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="isOpen = false" />

        <!-- Panel -->
        <div class="relative w-[560px] overflow-hidden rounded-xl border border-[#30363d] bg-[#161b22] shadow-2xl shadow-black/50">
          <!-- Search input -->
          <div class="flex items-center gap-3 border-b border-[#30363d] px-4 py-3">
            <svg class="h-4 w-4 flex-shrink-0 text-[#484f58]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
            </svg>
            <input
              v-model="query"
              ref="inputRef"
              class="flex-1 bg-transparent text-sm text-[#e6edf3] placeholder-[#484f58] outline-none"
              placeholder="搜索符号、函数、类..."
              autofocus
              @keydown="handleKeydown"
            />
            <div v-if="isSearching" class="h-4 w-4 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent flex-shrink-0" />
            <kbd class="flex-shrink-0 rounded bg-[#21262d] px-1.5 py-0.5 text-xs text-[#484f58]">ESC</kbd>
          </div>

          <!-- Results -->
          <div v-if="results.length" class="max-h-72 overflow-auto py-1">
            <button
              v-for="(r, i) in results"
              :key="`${r.file}:${r.line}`"
              class="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors"
              :class="i === activeIndex ? 'bg-[#388bfd]/15' : 'hover:bg-[#21262d]'"
              @click="selectResult(r)"
              @mouseover="activeIndex = i"
            >
              <span class="w-16 flex-shrink-0 text-right font-mono text-xs" :class="typeColors[r.type]">{{ r.type }}</span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium text-[#e6edf3]">{{ r.symbol }}</p>
                <p class="truncate text-xs text-[#8b949e]">{{ r.file }}:{{ r.line }}</p>
              </div>
              <span class="flex-shrink-0 text-xs text-[#484f58]">{{ r.repoName }}</span>
            </button>
          </div>

          <!-- Empty state -->
          <div v-else-if="query && !isSearching" class="py-6 text-center text-sm text-[#484f58]">
            未找到 "{{ query }}"
          </div>

          <!-- Hint -->
          <div v-if="!query" class="px-4 py-3 flex items-center gap-4 text-xs text-[#484f58]">
            <span>↑↓ 导航</span>
            <span>↵ 跳转</span>
            <span class="ml-auto">⌘⇧R 呼出</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
