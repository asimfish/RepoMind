// Repository types
export interface Repo {
  id: string
  name: string
  fullName: string        // owner/repo
  description: string | null
  language: string | null
  stars: number
  isPrivate: boolean
  cloneUrl: string
  htmlUrl: string
  updatedAt: string
  localPath?: string      // 本地存储路径（已克隆时）
  indexStatus: IndexStatus
  lastIndexedAt?: string
  lastCommit?: string     // 最后索引时的 commit hash
}

export type IndexStatus = 'not_indexed' | 'indexing' | 'indexed' | 'stale' | 'error'

// GitHub user
export interface GitHubUser {
  login: string
  name: string | null
  avatarUrl: string
  email: string | null
}

// Index progress
export interface IndexProgress {
  repoId: string
  phase: string
  percent: number
  message: string
}

// Search result
export interface SearchResult {
  symbol: string
  file: string
  line: number
  snippet: string
  type: 'function' | 'class' | 'method' | 'variable' | 'interface' | 'enum'
  score: number
  repoName: string
}

// Knowledge graph node
export interface GraphNode {
  id: string
  label: string
  type: 'file' | 'function' | 'class' | 'community' | 'process'
  file?: string
  line?: number
  community?: string
}

// Knowledge graph edge
export interface GraphEdge {
  id: string
  source: string
  target: string
  type: 'calls' | 'imports' | 'extends' | 'implements' | 'member_of'
  confidence: number
}

// Impact analysis
export interface ImpactResult {
  symbol: string
  directlyAffected: ImpactNode[]
  indirectlyAffected: ImpactNode[]
  processes: string[]
}

export interface ImpactNode {
  symbol: string
  file: string
  confidence: number
  depth: number
}

// App settings
export interface AppSettings {
  githubToken?: string
  indexStoragePath: string
  claudeApiKey?: string
  mcpEnabled: boolean
  autoIndexOnCommit: boolean
  searchLanguage: 'en' | 'zh'
}

// Skills / workflows (Tauri camelCase JSON)
export type SkillPlatform = 'cursor' | 'claude' | 'codex'

export interface Skill {
  id: string
  name: string
  description: string
  sourcePath: string
  sourcePlatform: SkillPlatform | string
  author?: string | null
  version?: string | null
  tags: string[]
  category?: string | null
  triggerPatterns: string[]
  dependsOn: string[]
  contentHash: string
  parsedAt: string
  rawContent?: string
}

export interface SkillInvokeCount {
  name: string
  count: number
}

export interface SkillStats {
  totalSkills: number
  totalChains: number
  totalWorkflows: number
  byPlatform: Record<string, number>
  byCategory: Record<string, number>
  topInvoked: SkillInvokeCount[]
  recentWorkflows: WorkflowTemplate[]
}

export interface SkillScanResult {
  totalScanned: number
  newSkills: number
  updatedSkills: number
  byPlatform: Record<string, number>
}

export type WorkflowStatus = 'discovered' | 'confirmed' | 'exported' | 'dismissed'

export interface WorkflowStep {
  order: number
  skillName: string
  skillId?: string | null
  isOptional: boolean
  avgPosition: number
  coOccurrenceRatio: number
}

export interface WorkflowTemplate {
  id: string
  name: string
  description: string
  steps: WorkflowStep[]
  frequency: number
  confidence: number
  sourceSessions: string[]
  category?: string | null
  createdAt: string
  status: WorkflowStatus | string
}

export interface SkillGraphNode {
  id: string
  label: string
  platform: string
  category?: string | null
  invokeCount: number
}

export interface SkillGraphEdge {
  source: string
  target: string
  weight: number
}

export interface SkillGraphData {
  nodes: SkillGraphNode[]
  edges: SkillGraphEdge[]
}
