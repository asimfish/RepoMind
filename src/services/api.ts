import { invoke } from '@tauri-apps/api/core'
import type { GitHubUser, Repo } from '@/types'

// GitHub OAuth via Tauri backend
export const githubAuthApi = {
  startOAuth: () => invoke<string>('start_github_oauth'),
  getToken: () => invoke<string | null>('get_github_token'),
  clearToken: () => invoke<void>('clear_github_token'),
  getCurrentUser: () => invoke<GitHubUser>('get_github_user'),
}

// Repository management
export const repoApi = {
  listGitHubRepos: (page = 1) => invoke<Repo[]>('list_github_repos', { page }),
  listIndexedRepos: () => invoke<Repo[]>('list_indexed_repos'),
  addRepo: (repoFullName: string) => invoke<Repo>('add_repo', { repoFullName }),
  removeRepo: (repoId: string) => invoke<void>('remove_repo', { repoId }),
  getRepo: (repoId: string) => invoke<Repo>('get_repo', { repoId }),
}

// Indexing
export const indexApi = {
  startIndex: (repoId: string) => invoke<void>('start_index', { repoId }),
  cancelIndex: (repoId: string) => invoke<void>('cancel_index', { repoId }),
  getIndexStatus: (repoId: string) => invoke<import('@/types').IndexStatus>('get_index_status', { repoId }),
}

// Search & Query
export const searchApi = {
  search: (repoId: string, query: string) =>
    invoke<import('@/types').SearchResult[]>('search', { repoId, query }),
  getContext: (repoId: string, symbol: string) =>
    invoke<import('@/types').GraphNode[]>('get_context', { repoId, symbol }),
  getImpact: (repoId: string, symbol: string) =>
    invoke<import('@/types').ImpactResult>('get_impact', { repoId, symbol }),
}

// App settings
export const settingsApi = {
  getSettings: () => invoke<import('@/types').AppSettings>('get_settings'),
  updateSettings: (settings: Partial<import('@/types').AppSettings>) =>
    invoke<void>('update_settings', { settings }),
}
