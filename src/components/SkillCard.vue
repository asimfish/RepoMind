<script setup lang="ts">
import { ref } from 'vue'
import type { Skill } from '@/types'

defineProps<{
  skill: Skill
}>()

const expanded = ref(false)

const platformClass: Record<string, string> = {
  cursor: 'bg-[#1f6feb]/20 text-[#388bfd]',
  claude: 'bg-[#d29922]/15 text-[#d29922]',
  codex: 'bg-[#a371f7]/15 text-[#a371f7]',
}

const platformLabel = (p: string) => {
  const m: Record<string, string> = {
    cursor: 'Cursor',
    claude: 'Claude',
    codex: 'Codex',
  }
  return m[p.toLowerCase()] ?? p
}
</script>

<template>
  <div
    class="card cursor-pointer transition-colors"
    :class="expanded ? 'border-[#388bfd]' : 'hover:border-[#388bfd]/60'"
    @click="expanded = !expanded"
  >
    <div class="flex items-start justify-between gap-2">
      <div class="min-w-0 flex-1">
        <h3 class="text-sm font-medium text-[#e6edf3]">{{ skill.name }}</h3>
        <p
          class="mt-1 text-xs leading-relaxed text-[#8b949e]"
          :class="expanded ? '' : 'line-clamp-2'"
        >
          {{ skill.description || '暂无描述' }}
        </p>
      </div>
      <svg
        class="h-4 w-4 flex-shrink-0 text-[#484f58] transition-transform"
        :class="expanded ? 'rotate-180' : ''"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </div>

    <div class="mt-3 flex flex-wrap items-center gap-2">
      <span
        class="tag capitalize"
        :class="platformClass[String(skill.sourcePlatform).toLowerCase()] ?? 'bg-[#21262d] text-[#8b949e]'"
      >
        {{ platformLabel(String(skill.sourcePlatform)) }}
      </span>
      <span v-if="skill.category" class="tag bg-[#21262d] text-[#8b949e]">
        {{ skill.category }}
      </span>
      <span v-if="skill.version" class="tag bg-[#161b22] text-[#484f58]"> v{{ skill.version }} </span>
    </div>

    <div v-if="expanded" class="mt-4 space-y-3 border-t border-[#30363d] pt-4">
      <p v-if="skill.description" class="text-xs leading-relaxed text-[#c9d1d9]">
        {{ skill.description }}
      </p>
      <div v-if="skill.tags?.length" class="flex flex-wrap gap-1.5">
        <span
          v-for="t in skill.tags"
          :key="t"
          class="rounded border border-[#30363d] bg-[#0d1117] px-2 py-0.5 font-mono text-[10px] text-[#8b949e]"
        >
          {{ t }}
        </span>
      </div>
      <p v-if="skill.sourcePath" class="truncate font-mono text-[10px] text-[#484f58]">
        {{ skill.sourcePath }}
      </p>
    </div>
  </div>
</template>
