<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRepoStore } from '@/stores/repo'
import type { SearchResult } from '@/types'
import { searchApi } from '@/services/api'

const repoStore = useRepoStore()
const query = ref('')
const results = ref<SearchResult[]>([])
const isSearching = ref(false)
const selectedRepoId = ref<string | null>(null)

const indexedRepos = computed(() => repoStore.indexedRepos.filter(r => r.indexStatus === 'indexed'))

onMounted(() => {
  if (indexedRepos.value.length > 0) {
    selectedRepoId.value = indexedRepos.value[0].id
  }
})

let timer: ReturnType<typeof setTimeout>
const onInput = () => {
  clearTimeout(timer)
  timer = setTimeout(doSearch, 300)
}

const doSearch = async () => {
  if (!query.value.trim() || !selectedRepoId.value) return
  isSearching.value = true
  try {
    results.value = await searchApi.search(selectedRepoId.value, query.value)
  } finally {
    isSearching.value = false
  }
}

const typeColors: Record<string, string> = {
  function: 'bg-[#388bfd]/20 text-[#388bfd]',
  class: 'bg-[#3fb950]/20 text-[#3fb950]',
  method: 'bg-[#d29922]/20 text-[#d29922]',
  variable: 'bg-[#8b949e]/20 text-[#8b949e]',
  interface: 'bg-[#a371f7]/20 text-[#a371f7]',
  enum: 'bg-[#f85149]/20 text-[#f85149]',
}
</script>

<template>
  <div class="flex flex-col h-full px-6 py-4">
    <div class="mb-4">
      <h1 class="text-xl font-semibold text-[#e6edf3]">全局搜索</h1>
      <p class="text-sm text-[#8b949e]">跨仓库搜索代码符号</p>
    </div>

    <div class="mb-4 flex gap-3">
      <select
        v-model="selectedRepoId"
        class="input w-48"
      >
        <option v-for="repo in indexedRepos" :key="repo.id" :value="repo.id">
          {{ repo.name }}
        </option>
      </select>

      <div class="relative flex-1">
        <svg class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#484f58]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
        </svg>
        <input v-model="query" class="input pl-10" placeholder="搜索函数名、类名、变量..." @input="onInput" @keyup.enter="doSearch" />
      </div>
    </div>

    <!-- Results -->
    <div class="flex-1 overflow-auto">
      <div v-if="isSearching" class="flex h-32 items-center justify-center">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>

      <div v-else-if="results.length === 0 && query" class="flex h-32 items-center justify-center text-sm text-[#484f58]">
        未找到 "{{ query }}"
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="r in results"
          :key="`${r.file}:${r.line}`"
          class="card hover:border-[#388bfd] transition-colors"
        >
          <div class="flex items-center gap-3">
            <span class="tag flex-shrink-0" :class="typeColors[r.type]">{{ r.type }}</span>
            <span class="font-mono text-sm font-medium text-[#e6edf3]">{{ r.symbol }}</span>
            <span class="ml-auto text-xs text-[#484f58]">{{ r.repoName }}</span>
          </div>
          <p class="mt-1 text-xs text-[#8b949e]">{{ r.file }}:{{ r.line }}</p>
          <pre v-if="r.snippet" class="mt-2 overflow-x-auto rounded bg-[#0d1117] p-2 text-xs text-[#e6edf3]">{{ r.snippet }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>
