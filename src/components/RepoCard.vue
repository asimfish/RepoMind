<script setup lang="ts">
import type { Repo, IndexProgress } from '@/types'

const props = defineProps<{
  repo: Repo
  showAddButton?: boolean
  isAdding?: boolean
  isIndexing?: boolean
  progress?: IndexProgress
  clickable?: boolean
}>()

const emit = defineEmits<{
  add: []
  click: []
}>()

const languageColors: Record<string, string> = {
  TypeScript: '#3178c6',
  JavaScript: '#f1e05a',
  Python: '#3572A5',
  Rust: '#dea584',
  Go: '#00ADD8',
  Java: '#b07219',
  Vue: '#41b883',
  Swift: '#FA7343',
  Kotlin: '#A97BFF',
  'C++': '#f34b7d',
  C: '#555555',
}

const langColor = (lang: string | null) => languageColors[lang ?? ''] ?? '#8b949e'

const statusLabel: Record<string, string> = {
  not_indexed: '未索引',
  indexing: '索引中',
  indexed: '已索引',
  stale: '需更新',
  error: '出错',
}
const statusColor: Record<string, string> = {
  not_indexed: 'text-[#484f58]',
  indexing: 'text-[#d29922]',
  indexed: 'text-[#3fb950]',
  stale: 'text-[#d29922]',
  error: 'text-[#f85149]',
}
</script>

<template>
  <div
    class="card group flex flex-col gap-3 transition-colors"
    :class="clickable ? 'cursor-pointer hover:border-[#388bfd]' : 'cursor-default'"
    @click="clickable && emit('click')"
  >
    <!-- Header -->
    <div class="flex items-start justify-between">
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <span class="truncate text-sm font-medium text-[#e6edf3] group-hover:text-[#388bfd]">
            {{ repo.fullName }}
          </span>
          <span v-if="repo.isPrivate" class="tag bg-[#21262d] text-[#8b949e]">Private</span>
        </div>
        <p v-if="repo.description" class="mt-1 truncate text-xs text-[#8b949e]">
          {{ repo.description }}
        </p>
      </div>

      <!-- Add button -->
      <button
        v-if="showAddButton"
        class="ml-3 flex-shrink-0 btn-primary py-1.5 text-xs"
        :disabled="isAdding"
        @click.stop="emit('add')"
      >
        {{ isAdding ? '添加中...' : '+ 添加' }}
      </button>
    </div>

    <!-- Index progress bar -->
    <div v-if="isIndexing && progress" class="space-y-1">
      <div class="flex items-center justify-between text-xs">
        <span class="text-[#8b949e]">{{ progress.message }}</span>
        <span class="text-[#d29922]">{{ progress.percent }}%</span>
      </div>
      <div class="h-1 rounded-full bg-[#21262d]">
        <div
          class="h-1 rounded-full bg-[#d29922] transition-all"
          :style="{ width: `${progress.percent}%` }"
        />
      </div>
    </div>

    <!-- Footer -->
    <div class="flex items-center gap-4 text-xs text-[#8b949e]">
      <div v-if="repo.language" class="flex items-center gap-1">
        <span class="h-2.5 w-2.5 rounded-full" :style="{ background: langColor(repo.language) }" />
        {{ repo.language }}
      </div>
      <div class="flex items-center gap-1">
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
        </svg>
        {{ repo.stars.toLocaleString() }}
      </div>
      <div :class="statusColor[repo.indexStatus]" class="ml-auto flex items-center gap-1">
        <span class="h-1.5 w-1.5 rounded-full bg-current" />
        {{ statusLabel[repo.indexStatus] }}
      </div>
    </div>
  </div>
</template>
