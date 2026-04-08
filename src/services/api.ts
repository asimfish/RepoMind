import { invoke } from '@tauri-apps/api/core'
import type { GitHubUser, Repo, AppSettings, IndexStatus, SearchResult, GraphNode, ImpactResult } from '@/types'

export const githubAuthApi = {
  startOAuth: () => invoke<void>('start_github_oauth'),
  getToken: () => invoke<string | null>('get_github_token'),
  clearToken: () => invoke<void>('clear_github_token'),
  getCurrentUser: () => invoke<GitHubUser>('get_github_user'),
}

export const repoApi = {
  listGitHubRepos: (page = 1) => invoke<Repo[]>('list_github_repos', { page }),
  listIndexedRepos: () => invoke<Repo[]>('list_indexed_repos'),
  addRepo: (repoFullName: string) => invoke<Repo>('add_repo', { repoFullName }),
  removeRepo: (repoId: string) => invoke<void>('remove_repo', { repoId }),
  getRepo: (repoId: string) => invoke<Repo>('get_repo', { repoId }),
}

export const indexApi = {
  startIndex: (repoId: string) => invoke<void>('start_index', { repoId }),
  cancelIndex: (repoId: string) => invoke<void>('cancel_index', { repoId }),
  getIndexStatus: (repoId: string) => invoke<IndexStatus>('get_index_status', { repoId }),
}

export const searchApi = {
  search: (repoId: string, query: string) =>
    invoke<SearchResult[]>('search', { repoId, query }),
  getContext: (repoId: string, symbol: string) =>
    invoke<GraphNode[]>('get_context', { repoId, symbol }),
  getImpact: (repoId: string, symbol: string) =>
    invoke<ImpactResult>('get_impact', { repoId, symbol }),
}

export const settingsApi = {
  getSettings: () => invoke<AppSettings>('get_settings'),
  updateSettings: (settings: Partial<AppSettings>) =>
    invoke<void>('update_settings', { settings }),
}
