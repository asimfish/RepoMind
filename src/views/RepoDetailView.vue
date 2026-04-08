<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { searchApi, indexApi } from '@/services/api'
import type { SearchResult, ImpactResult } from '@/types'
import IndexBadge from '@/components/IndexBadge.vue'

const route = useRoute()
const repoStore = useRepoStore()
const repoId = computed(() => route.params.id as string)
const repo = computed(() => repoStore.indexedRepos.find(r => r.id === repoId.value))

const query = ref('')
const results = ref<SearchResult[]>([])
const isSearching = ref(false)
const activeResult = ref<SearchResult | null>(null)
const impact = ref<ImpactResult | null>(null)
const isLoadingImpact = ref(false)
const activeTab = ref<'search' | 'impact'>('search')

let timer: ReturnType<typeof setTimeout>
const onInput = () => {
  clearTimeout(timer)
  timer = setTimeout(doSearch, 300)
}

const doSearch = async () => {
  if (!query.value.trim()) { results.value = []; return }
  isSearching.value = true
  try {
    results.value = await searchApi.search(repoId.value, query.value)
    activeResult.value = results.value[0] ?? null
  } finally {
    isSearching.value = false
  }
}

const loadImpact = async (symbol: string) => {
  isLoadingImpact.value = true
  try {
    impact.value = await searchApi.getImpact(repoId.value, symbol)
    activeTab.value = 'impact'
  } finally {
    isLoadingImpact.value = false
  }
}

const reindex = async () => {
  if (!repo.value) return
  await indexApi.startIndex(repoId.value)
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
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <h1 class="text-xl font-semibold text-[#e6edf3]">{{ repo?.name ?? repoId }}</h1>
        <IndexBadge v-if="repo" :status="repo.indexStatus" />
      </div>

      <!-- Stale warning + re-index button -->
      <div v-if="repo?.indexStatus === 'stale'" class="flex items-center gap-2">
        <span class="text-xs text-[#d29922]">检测到代码变更</span>
        <button class="btn-primary py-1.5 text-xs" @click="reindex">重新索引</button>
      </div>
    </div>

    <!-- Search bar -->
    <div class="relative mb-4">
      <svg class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#484f58]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
      </svg>
      <input v-model="query" class="input pl-10" placeholder="搜索函数、类、变量..." @input="onInput" @keyup.enter="doSearch" />
      <div v-if="isSearching" class="absolute right-3 top-1/2 -translate-y-1/2">
        <div class="h-4 w-4 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>
    </div>

    <div class="flex flex-1 gap-4 overflow-hidden">
      <!-- Results list -->
      <div class="w-72 flex-shrink-0 overflow-auto space-y-1 pr-1">
        <div v-if="results.length === 0 && query && !isSearching" class="py-8 text-center text-sm text-[#484f58]">
          未找到相关符号
        </div>
        <div v-else-if="!query" class="py-8 text-center text-sm text-[#484f58]">
          输入关键词搜索代码
        </div>

        <button
          v-for="r in results"
          :key="`${r.file}:${r.line}`"
          class="w-full rounded-lg border p-3 text-left transition-colors"
          :class="activeResult === r
            ? 'border-[#388bfd] bg-[#161b22]'
            : 'border-[#30363d] bg-[#0d1117] hover:border-[#388bfd]/50'"
          @click="activeResult = r; activeTab = 'search'"
        >
          <div class="flex items-center gap-2 mb-1">
            <span class="tag text-xs" :class="typeColors[r.type]">{{ r.type }}</span>
            <span class="text-sm font-medium text-[#e6edf3] truncate">{{ r.symbol }}</span>
          </div>
          <p class="text-xs text-[#8b949e] truncate">{{ r.file }}:{{ r.line }}</p>
        </button>
      </div>

      <!-- Detail panel -->
      <div class="flex-1 overflow-hidden rounded-lg border border-[#30363d] bg-[#161b22] flex flex-col">
        <!-- Tabs -->
        <div v-if="activeResult" class="flex items-center gap-1 border-b border-[#30363d] px-4 pt-3">
          <button
            class="rounded-t px-3 py-1.5 text-sm font-medium transition-colors"
            :class="activeTab === 'search' ? 'border-b-2 border-[#388bfd] text-[#388bfd]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
            @click="activeTab = 'search'"
          >代码</button>
          <button
            class="rounded-t px-3 py-1.5 text-sm font-medium transition-colors"
            :class="activeTab === 'impact' ? 'border-b-2 border-[#388bfd] text-[#388bfd]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
            @click="activeResult && loadImpact(activeResult.symbol)"
          >
            影响分析
            <span v-if="isLoadingImpact" class="ml-1 h-3 w-3 inline-block animate-spin rounded-full border border-[#8b949e] border-t-transparent" />
          </button>
        </div>

        <!-- Code view -->
        <div v-if="activeResult && activeTab === 'search'" class="flex-1 overflow-auto p-4">
          <div class="flex items-center gap-3 mb-3">
            <span class="tag py-1 px-3" :class="typeColors[activeResult.type]">{{ activeResult.type }}</span>
            <h2 class="text-base font-semibold text-[#e6edf3]">{{ activeResult.symbol }}</h2>
            <button
              class="ml-auto text-xs text-[#8b949e] hover:text-[#388bfd] transition-colors"
              @click="loadImpact(activeResult!.symbol)"
            >分析影响范围 →</button>
          </div>
          <p class="text-xs text-[#8b949e] mb-3">{{ activeResult.file }}:{{ activeResult.line }}</p>
          <pre class="rounded-lg bg-[#0d1117] p-4 text-sm text-[#e6edf3] overflow-auto leading-relaxed">{{ activeResult.snippet }}</pre>
        </div>

        <!-- Impact analysis view -->
        <div v-else-if="activeTab === 'impact' && impact" class="flex-1 overflow-auto p-4">
          <h3 class="mb-4 text-base font-semibold text-[#e6edf3]">
            {{ impact.symbol }} 的影响范围
          </h3>

          <div class="space-y-4">
            <div v-if="impact.directlyAffected.length">
              <p class="mb-2 text-xs font-medium text-[#f85149]">直接影响（必须更新）</p>
              <div class="space-y-1">
                <div v-for="n in impact.directlyAffected" :key="n.symbol"
                  class="flex items-center gap-2 rounded bg-[#f85149]/10 px-3 py-2 text-sm">
                  <span class="font-mono text-[#e6edf3]">{{ n.symbol }}</span>
                  <span class="ml-auto text-xs text-[#8b949e] truncate">{{ n.file }}</span>
                  <span class="text-xs text-[#f85149]">{{ Math.round(n.confidence * 100) }}%</span>
                </div>
              </div>
            </div>

            <div v-if="impact.indirectlyAffected.length">
              <p class="mb-2 text-xs font-medium text-[#d29922]">间接影响（建议测试）</p>
              <div class="space-y-1">
                <div v-for="n in impact.indirectlyAffected" :key="n.symbol"
                  class="flex items-center gap-2 rounded bg-[#d29922]/10 px-3 py-2 text-sm">
                  <span class="font-mono text-[#e6edf3]">{{ n.symbol }}</span>
                  <span class="ml-auto text-xs text-[#8b949e] truncate">{{ n.file }}</span>
                  <span class="text-xs text-[#d29922]">{{ Math.round(n.confidence * 100) }}%</span>
                </div>
              </div>
            </div>

            <div v-if="impact.processes.length">
              <p class="mb-2 text-xs font-medium text-[#388bfd]">相关执行流程</p>
              <div class="flex flex-wrap gap-2">
                <span v-for="p in impact.processes" :key="p"
                  class="tag bg-[#388bfd]/10 text-[#388bfd]">{{ p }}</span>
              </div>
            </div>

            <div v-if="!impact.directlyAffected.length && !impact.indirectlyAffected.length"
              class="py-8 text-center text-sm text-[#484f58]">
              该符号无已知依赖，修改风险较低 ✓
            </div>
          </div>
        </div>

        <div v-else class="flex flex-1 items-center justify-center text-sm text-[#484f58]">
          选择一个搜索结果查看详情
        </div>
      </div>
    </div>
  </div>
</template>
