<script setup lang="ts">
import type { IndexStatus } from '@/types'

const props = defineProps<{ status: IndexStatus }>()

const labels: Record<IndexStatus, string> = {
  not_indexed: '未索引',
  indexing: '索引中',
  indexed: '已就绪',
  stale: '需更新',
  error: '出错',
}

const classes: Record<IndexStatus, string> = {
  not_indexed: 'bg-[#21262d] text-[#484f58]',
  indexing: 'bg-[#d29922]/20 text-[#d29922]',
  indexed: 'bg-[#3fb950]/20 text-[#3fb950]',
  stale: 'bg-[#d29922]/20 text-[#d29922]',
  error: 'bg-[#f85149]/20 text-[#f85149]',
}
</script>

<template>
  <span class="tag" :class="classes[status]">
    <span class="mr-1 h-1.5 w-1.5 rounded-full bg-current" />
    {{ labels[status] }}
  </span>
</template>
