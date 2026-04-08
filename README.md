# RepoMind

**代码仓库 AI 知识管家** — 连接 GitHub，一键索引，让 AI 工具（Claude ​Code、Cursor、Codex）在你的代码库里像工作了三年的老员工。

---

## ✨ 核心功能

| 功能 | 说明 |
|------|------|
| **GitHub 直连** | OAuth 登录，一键 Clone + 索引，无需手动配置 |
| **知识图谱** | Tree-sitter 解析 + LadybugDB，11 种语言，调用链 / 依赖 / 社区聚类 |
| **双路搜索** | BM25 全文 + nomic-embed-text 语义向量 RRF 融合 |
| **Impact 分析** | 改动前查看影响范围（直接 / 间接 / 执行流程） |
| **AI 摘要** | Claude Haiku 自动生成中文代码符号摘要 |
| **图谱可视化** | Sigma.js WebGL 渲染，600+ 节点流畅交互 |
| **Spotlight 搜索** | `⌘⇧R` 全局唤起，跨仓库符号搜索 |
| **MCP 集成** | 5 个工具，兼容 Claude ​Code / Cursor / Codex |

---

## 🚀 快速开始

### 环境要求

- macOS 12+、Node.js 18+、Rust 1.70+
- [pnpm](https://pnpm.io) (包管理器)

### 安装运行

```bash
# 克隆仓库并安装依赖
git clone https://github.com/yourname/repomind && cd repomind
pnpm install

# 安装索引引擎
pnpm add -g gitnexus

# 启动（首次会自动编译 Rust）
source ~/.cargo/env && pnpm tauri dev
```

### 可选：语义向量搜索

```bash
brew install ollama
ollama pull nomic-embed-text   # ~17MB 嵌入模型
```

---

## 🔧 MCP Server（AI Agent 集成）

```bash
pnpm build:mcp    # 编译 repomind-mcp 二进制
pnpm setup:mcp    # 注册到 Claude ​Code / Cursor / Codex
```

可用工具：`list_repos` · `search` · `context` · `impact` · `cypher`

---

## 🏗️ 技术栈

- **前端**: Vue 3 + TypeScript + Tailwind CSS 4 + Sigma.js
- **后端**: Tauri 2.x + Rust (tokio, reqwest, rusqlite)
- **知识图谱**: gitnexus (Tree-sitter + LadybugDB)
- **向量搜索**: Ollama (nomic-embed-text) + SQLite

所有数据本地存储，代码不上传任何服务器。

---

## 📁 数据路径

```
~/Library/Application Support/com.liyufeng.repomind/
├── state.json      # 仓库、设置、token
├── repos/          # 克隆的代码仓库
└── vectors/        # 语义向量索引 (SQLite)
```

---

## 🗺️ 路线图

- [x] Phase 1: GitHub OAuth + Tauri App + MCP Server
- [x] Phase 2: 知识图谱可视化 + 语义搜索 + AI 摘要
- [ ] Phase 3: Git 演化时间轴
- [ ] Phase 4: 多仓库关联图
- [ ] Phase 5: 团队云同步

---

MIT License
