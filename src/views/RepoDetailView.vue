<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { searchApi } from '@/services/api'
import type { SearchResult } from '@/types'

const route = useRoute()
const repoId = computed(() => route.params.id as string)
const query = ref('')
const results = ref<SearchResult[]>([])
const isSearching = ref(false)
const activeResult = ref<SearchResult | null>(null)

let searchTimer: ReturnType<typeof setTimeout>

const onInput = () => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(doSearch, 300)
}

const doSearch = async () => {
  if (!query.value.trim()) {
    results.value = []
    return
  }
  isSearching.value = true
  try {
    results.value = await searchApi.search(repoId.value, query.value)
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
  <div class="flex h-full flex-col px-6 py-4">
    <div class="mb-4">
      <h1 class="text-xl font-semibold text-[#e6edf3]">代码探索</h1>
    </div>

    <!-- Search input -->
    <div class="relative mb-4">
      <svg class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#484f58]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
      </svg>
      <input
        v-model="query"
        class="input pl-10"
        placeholder="搜索函数、类、变量..."
        @input="onInput"
        @keyup.enter="doSearch"
      />
      <div v-if="isSearching" class="absolute right-3 top-1/2 -translate-y-1/2">
        <div class="h-4 w-4 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>
    </div>

    <div class="flex flex-1 gap-4 overflow-hidden">
      <!-- Results list -->
      <div class="w-80 flex-shrink-0 overflow-auto space-y-1">
        <div
          v-for="result in results"
          :key="`${result.file}:${result.line}`"
          class="rounded-lg border border-[#30363d] p-3 cursor-pointer hover:border-[#388bfd] transition-colors"
          :class="activeResult === result ? 'border-[#388bfd] bg-[#161b22]' : 'bg-[#0d1117]'"
          @click="activeResult = result"
        >
          <div class="flex items-center gap-2 mb-1">
            <span class="tag" :class="typeColors[result.type]">{{ result.type }}</span>
            <span class="text-sm font-medium text-[#e6edf3] truncate">{{ result.symbol }}</span>
          </div>
          <p class="text-xs text-[#8b949e] truncate">{{ result.file }}:{{ result.line }}</p>
        </div>

        <div v-if="!isSearching && results.length === 0 && query" class="text-center py-8 text-[#484f58] text-sm">
          未找到 "{{ query }}" 相关符号
        </div>
        <div v-if="!query" class="text-center py-8 text-[#484f58] text-sm">
          输入关键词开始搜索
        </div>
      </div>

      <!-- Result detail -->
      <div class="flex-1 overflow-auto rounded-lg border border-[#30363d] bg-[#161b22]">
        <div v-if="activeResult" class="p-4">
          <div class="flex items-center gap-3 mb-4">
            <span class="tag text-sm py-1 px-3" :class="typeColors[activeResult.type]">{{ activeResult.type }}</span>
            <h2 class="text-lg font-semibold text-[#e6edf3]">{{ activeResult.symbol }}</h2>
          </div>
          <p class="text-sm text-[#8b949e] mb-4">{{ activeResult.file }}:{{ activeResult.line }}</p>
          <pre class="rounded-lg bg-[#0d1117] p-4 text-sm text-[#e6edf3] overflow-auto">{{ activeResult.snippet }}</pre>
        </div>
        <div v-else class="flex h-full items-center justify-center text-[#484f58] text-sm">
          选择一个搜索结果查看详情
        </div>
      </div>
    </div>
  </div>
</template>
