<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { useRepoStore } from '@/stores/repo'

const router = useRouter()
const route = useRoute()
const repoStore = useRepoStore()

const navItems = [
  { path: '/repos', icon: 'M3 7h18M3 12h18M3 17h18', label: '仓库' },
  { path: '/search', icon: 'M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z', label: '搜索' },
  { path: '/settings', icon: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.94 11a7.94 7.94 0 0 0-.46-2.24l1.98-1.16-2-3.46-1.98 1.16A7.96 7.96 0 0 0 13 3.06V1h-4v2.06a7.96 7.96 0 0 0-3.48 2.04L3.54 3.94l-2 3.46 1.98 1.16A7.94 7.94 0 0 0 3.06 11H1v4h2.06c.12.79.35 1.54.68 2.24L1.76 18.4l2 3.46 1.98-1.16A7.96 7.96 0 0 0 11 22.94V25h4v-2.06a7.96 7.96 0 0 0 3.48-2.04l1.98 1.16 2-3.46-1.98-1.16c.33-.7.56-1.45.68-2.24H25v-4h-2.06z', label: '设置' },
]

const isActive = (path: string) => route.path.startsWith(path)
</script>

<template>
  <div class="flex h-screen bg-[#0d1117] text-[#e6edf3]">
    <!-- Sidebar -->
    <aside class="flex w-14 flex-col items-center border-r border-[#30363d] bg-[#010409] py-4 pt-10">
      <!-- Logo -->
      <div class="mb-6 flex h-8 w-8 items-center justify-center rounded-lg bg-[#388bfd]">
        <span class="text-xs font-bold text-white">RM</span>
      </div>

      <!-- Nav items -->
      <nav class="flex flex-1 flex-col items-center gap-1">
        <button
          v-for="item in navItems"
          :key="item.path"
          class="group relative flex h-10 w-10 items-center justify-center rounded-lg transition-colors"
          :class="isActive(item.path) ? 'bg-[#161b22] text-[#388bfd]' : 'text-[#8b949e] hover:bg-[#161b22] hover:text-[#e6edf3]'"
          @click="router.push(item.path)"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
            <path :d="item.icon" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <!-- Tooltip -->
          <span class="absolute left-12 z-50 hidden whitespace-nowrap rounded bg-[#161b22] px-2 py-1 text-xs text-[#e6edf3] shadow-lg group-hover:block">
            {{ item.label }}
          </span>
        </button>
      </nav>

      <!-- User avatar -->
      <div v-if="repoStore.currentUser" class="mt-auto">
        <img
          :src="repoStore.currentUser.avatarUrl"
          :alt="repoStore.currentUser.login"
          class="h-8 w-8 rounded-full border-2 border-[#30363d] cursor-pointer hover:border-[#388bfd] transition-colors"
          @click="router.push('/settings')"
        />
      </div>
    </aside>

    <!-- Main content -->
    <main class="flex flex-1 flex-col overflow-hidden">
      <!-- macOS titlebar drag region -->
      <div class="h-8 w-full" data-tauri-drag-region />

      <div class="flex-1 overflow-auto">
        <router-view />
      </div>
    </main>
  </div>
</template>
