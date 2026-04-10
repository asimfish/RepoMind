# 参与贡献

感谢你对 RepoMind 的关注。以下为开发环境、常用命令与提交流程说明。

## 开发环境要求

- **Node.js**：建议 20 LTS（与 CI 一致）
- **包管理**：[pnpm](https://pnpm.io/) 9.x
- **Rust**：stable（通过 [rustup](https://rustup.rs/) 安装）
- **系统依赖（Linux 桌面构建 / CI）**：`libwebkit2gtk-4.1-dev`、`libappindicator3-dev`、`librsvg2-dev`、`patchelf`（Ubuntu/Debian 系可参考 CI 中的 `apt-get` 命令）

macOS / Windows 开发请参考 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/) 安装对应依赖。

## 开始开发

```bash
# 安装前端依赖
pnpm install

# 启动 Vite + Tauri 开发模式
pnpm tauri dev
```

仅调试前端（不启动桌面壳）时：

```bash
pnpm dev
```

构建生产桌面应用：

```bash
pnpm tauri build
```

## 代码规范

- **TypeScript / Vue**：提交前建议执行 `pnpm exec vue-tsc --noEmit`；构建脚本已包含类型检查。
- **Rust**：在 `src-tauri` 目录下使用 `cargo fmt`、`cargo clippy`；本仓库提供 `src-tauri/rustfmt.toml` 统一格式。
- **风格**：与现有代码保持一致；避免无关重构与大范围格式化混在功能提交中。

## 提交 Pull Request 流程

1. 从 `main`（或默认分支）创建功能分支。
2. 本地完成改动后运行与改动相关的检查（前端构建、Rust 测试 / 静态检查等）。
3. 推送到你的 fork 或本仓库分支，发起 PR。
4. 在 PR 描述中说明**动机**、**主要变更**与**验证方式**；若修复 Issue，请写上 `Closes #编号`。
5. 根据 Review 意见更新分支；通过 CI 后再合并。

若新增用户可见行为或配置变更，请在 PR 中简要说明，便于发布说明与文档更新。

## 问题与讨论

- **Bug**：请使用 Issue 表单「Bug 报告」，尽量给出复现步骤与环境。
- **功能建议**：请使用「功能请求」模板，说明场景与期望行为。

再次感谢你的贡献。
