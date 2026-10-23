# ProxyBot 完成情况统计（2026-10-22 基线）

> 一次性状态快照，用于回答「这个项目现在完成到什么程度、还欠什么」。
> 基线提交：`ada81db`（`main`，2026-08-20），产品版本 `1.3.0`（未发布）。
> 统计口径与证据均可在本仓库复现；结论以代码、测试、CI、release 事实为准，不采信文档自述。

## 0. 一句话结论

**后端能力完成度不低，工程验证也是健康的；欠账集中在「产品收敛、UI 可用性、轻量化、发布证据」四件事，而不是缺功能。**

- 核心 MITM 抓包链路（显式代理 → CA → 解密 → 持久化 → 历史/筛选/重放/HAR）已经可运行，并有真实 Tauri 组合根的 packaged acceptance 证据。
- 但 131 个 IPC 命令里只有 36 个进入类型化契约，30 个命令前端零引用；4 个 Labs 页面在应用内没有任何入口却仍默认打包；核心「查看 → 修改/重放」动作在最关键处断开。
- 打包侧：当前 `main` 的 release 路径引入 95,137,320 B（90.7 MiB）的 Apktool + 3 个 Frida Gadget 资源（解压后 14.8 MB + 25.19/27.92/26.55 MB），并静态链接 Frida Core（约 20–40 MB），构建时还需联网下载 37.8 MB 资源与 43 MB devkit。按 Tauri bundle 估算，未收敛前 `.app` 约 130–165 MB、DMG 约 40–60 MB；收敛后预计 `.app` 15–30 MB、DMG 8–15 MB（需一次真实 release 构建实测）。注意 `v1.2.15` 的 6.0 MB ZIP 是旧的**手工组装 `.app`**（非 Tauri bundle），不能作为当前产物体积基线。
- 发布侧：最新 GitHub Release 仍是 `v1.2.15`（2026-05-15，两个 ZIP、release notes 为空）；新的签名/公证 DMG 流水线从未产出过成功制品，阻塞在 issue #27（签名凭据）与 #28（真机验证）。

按能力层加权估算（方法见 §2.4）：

| 维度 | 完成度 | 判断依据 |
| --- | ---: | --- |
| Core 产品闭环（后端） | ~80% | 9/12 项可运行；breakpoint、HAR import、WS 深度检查为部分 |
| Core 产品闭环（UI/交互） | ~45% | 详情页缺少修改/重放动作；失败被吞成空态；布局/主题/可访问性缺陷 |
| Advanced 能力 | ~60% | DNS/rules/anomaly 可运行；pf 无测试；scripting/network conditions/workspace 无产品入口 |
| Labs 能力 | ~35% | 后端模块多数存在，但产品入口缺失或为占位实现 |
| 发布与分发 | ~30% | 有完整 release workflow 设计，但从未产出签名制品；版本 1.3.0 未发布 |
| 开源治理 | ~55% | License/CoC/Security/模板齐全；缺 CHANGELOG/SUPPORT/ADR/CODEOWNERS；81% 已跟踪文档是内部过程文档 |
| **综合（加权）** | **~60%** | 工程实现强、产品化与分发弱 |

---

## 1. 基线健康度（可复现证据）

| 指标 | 数值 | 证据 |
| --- | --- | --- |
| 产品版本 | `1.3.0`（package.json 单一来源，Rust/Tauri 镜像一致） | `node scripts/version.mjs --check` → consistent |
| 最新发布 | `v1.2.15`（2026-05-15），两个手工组装的 6.0 MB ZIP（非 Tauri bundle），release notes 为空 | `gh release list` / `gh release view v1.2.15` |
| 未发布版本 | `1.3.0` 无 tag、无 Release | `git tag`；最新 release 仍为 v1.2.15 |
| Rust 测试 | **817 通过 / 0 失败 / 1 ignored** | `CARGO_HOME=$PWD/.cache-cargo cargo test --workspace --locked --no-default-features` |
| 前端测试 | **29 文件 / 102 用例通过** | `./node_modules/.bin/vitest run` |
| TypeScript | `tsc --noEmit` 通过 | 本地执行 |
| CI（main） | 最近一次全绿（2026-08-20，5m51s） | `gh run list --workflow=ci.yml --branch main` |
| CI 分 job 耗时 | Rust 3m25s / Frontend 1m22s / E2E 2m06s / Bundle Smoke 5m46s | run `32317012154` |
| 代码规模 | Rust 44,117 行（desktop 35,622 / core 8,495，132 文件）；前端 13,037 行（131 文件） | `wc -l` 全量统计 |
| 仓库规模 | 485 个已跟踪文件；647 次提交（mbpz 631 / ttbb 10 / bot 6） | `git ls-files`、`git log` |
| 文档规模 | 118 个已跟踪 markdown；其中 106 个在 `docs/`；约 81% 为内部/历史文档 | `git ls-files '*.md'`、docs 审计 |
| 桌面 IPC 面 | **131** 个命令；类型化契约覆盖 **36**（27.5%）；**30** 个命令前端零引用；15 个前端文件绕过契约 | `.audit/architecture-inventory.md` §1、§3 |
| 前端 bundle | dist 4.3 MB / 50 个 JS chunk；初始 index chunk **1.94 MB（45%）**；无路由级代码分割 | `du -sh dist`；`ls dist/assets/*.js` |
| 打包重项 | Apktool 14.8 MB + 3 个 Frida Gadget（解压后 25.19/27.92/26.55 MB，资源合计 90.7 MiB）+ Frida Core 静态链接（约 20–40 MB）；仅 release 路径，构建需联网下载 37.8 MB 资源 + 43 MB devkit | `resources.lock`、`release.yml`、`.audit/packaging-inventory.md` |
| CI 成本 | 每次 main CI 约 126.5 billed macOS 分钟（4 job=205s/82s/126s/346s）；tag 不触发 CI，release 不跑测试 | `.audit/packaging-inventory.md` |
| 依赖更新 | 3 个 dependabot PR 开着，#40（Rust 41 项）与 #39（Actions 8 项）CI 失败 | `gh pr list`、`gh run list` |

---

## 2. 能力完成度明细

判定口径沿用仓库自身定义（`docs/roadmap.md`）：

- **可运行**：从组合根可达、有实现、前端或 acceptance 有真实路径、有测试；
- **部分**：后端已接，但缺 UI/命令/IPC，或依赖外部条件（管理员权限、API key、release feature）；
- **空壳**：不可达、占位实现或只存在于文档。

### 2.1 Core（默认导航，必须可用）

| 能力 | 状态 | 证据 / 缺口 |
| --- | --- | --- |
| 显式代理 MITM（HTTP/HTTPS） | 可运行 | `proxybot-core/src/proxy_engine.rs`；`src-tauri/src/acceptance.rs` + `scripts/test_desktop_acceptance.mjs` |
| CA 生命周期（生成/导出/重生成） | 可运行 | `cert_manager.rs`、`cert/`；CI Bundle Smoke 跑真实组合根 |
| 设备接入/引导 | 可运行（未真机验证） | `commands/device_setup.rs`、`src/features/device-onboarding/`；issue #28 待真机验证 |
| Routing Rules | 可运行 | `rules_engine.rs`、`rules.rs`；前端 Rules 页 |
| Breakpoint | 部分 | 后端可暂停；edit-and-forward 自认未接线（`BreakpointPanel.tsx:206-209`） |
| Captured Request 历史/详情/筛选 | 部分 | 查询与详情可用；详情页无重放/修改/加规则动作；查询失败仅 console |
| Replay / Composer | 部分 | 后端命令可用；Replay 只按 host 批量，缺少单请求上下文重放 |
| HAR / 代码导出 | 部分 | HAR 导出可用；HAR import 未实现；脱敏策略未落地 |
| WebSocket 抓包/帧查看 | 部分 | 帧存储与查看器可用；二进制/协议级检查未完成 |
| SQLite 持久化 | 可运行 | `db.rs` + `db/captured_requests.rs`，29 个测试 |
| Alerts / Anomaly | 可运行 | `alerts.rs`、`anomaly.rs`；前端 Alerts 页 |
| 移动端 dashboard | 可运行（未验证真机） | `dashboard/`；不在 Core 发布承诺内 |

### 2.2 Advanced（保留但应从默认导航降级）

| 能力 | 状态 | 证据 / 缺口 |
| --- | --- | --- |
| macOS `pf` 透明代理 | 部分 | 实现存在；需要管理员权限；无自动化测试覆盖（审计 §7） |
| 内置 DNS 服务器 + DNS 观测 | 可运行 | `dns.rs`（1,955 行，54 测试） |
| MCP stdio Adapter | 可运行 | `mcp/`，6 个只读为主 tools；无鉴权策略 |
| Rhai scripting / plugins | 部分 | 引擎可执行；插件 loader/WASM 为占位；无 UI |
| Network Conditions | 部分 | IPC 可调用；前端零引用 |
| Workspace | 部分 | 序列化/管理命令存在；前端零引用 |
| Graph / Topology / DAG | 部分 | 后端与页面存在；Topology 深链 `?host=` 无人解析；两套分析视图未合并 |
| TLS 指纹 / gRPC / GraphQL / protobuf 解码 | 部分 | 解码器可用；无完整 inspector 工作流 |

### 2.3 Labs（默认不应发布）

| 能力 | 状态 | 证据 / 缺口 |
| --- | --- | --- |
| AI 分析 / vision / 推断 | 部分 | 依赖外部 API；`ai_pipeline` 为占位（本次已作为死代码删除） |
| OpenAPI/AsyncAPI spec 生成 | 可运行（无产品入口） | core `specgen/` 2,224 行 + 测试；前端 `/gen` 在应用内无入口 |
| Mock / scaffold / deploy 生成 | 部分 | 模块存在；无产品入口；依赖外部条件 |
| Android SSL bypass（Frida/ADB/Apktool） | 部分（release-only） | 默认构建是返回 capability error 的 stub；release 才启用 Frida；**本次已移除** |
| 插件/WASM 沙箱 | 空壳 | loader 从不调用、WASM 占位（本次已作为死代码删除） |
| Prometheus metrics server | 死代码 | 无调用方（本次已删除） |

### 2.4 完成度评分方法

- 每层按下表折算：可运行 = 1，部分 = 0.5，空壳/缺失 = 0；层内按能力条目等权。
- 综合分权重：Core 后端 25%、Core UI 25%、Advanced 20%、Labs 10%、发布 10%、治理 10%。
- 这是**工程交付视角**的估计，不替代产品验收；发布与真机证据未达成前，任何「已支持」声明都不成立。

---

## 3. 最严重的完成度缺口（按影响排序）

1. **IPC 契约漂移**：131 个命令只有 36 个有生成类型；15 个前端文件直接 `invoke` 绕过契约；30 个注册命令前端零引用。结果是「后端有、产品无入口」与「前端调用无类型保护」同时存在，是架构混乱最硬的证据。
2. **Labs 默认发布**：roadmap 要求 Labs 必须显式 opt-in 且不进入默认导航/发布声明，但 AI/Gen/Deploy/SSL-bypass/Graph/Topology 全部是顶层路由；release 还固定 `--features frida-runtime` 并打包 Frida/Apktool 资源。
3. **核心闭环断点**：请求详情没有「重放 / 修改 / 加规则」；删除规则/停止捕获等破坏性操作缺确认；失败被 `safeInvoke` 吞成 null 或旧数据（Certs 显示「无证书」但没有生成按钮、Topology 失败显示「无数据」、Breakpoint 面板直接消失）。
4. **UI 系统性缺陷**：默认 5 个导航目的地内部仍有 16 条路由；4 个 Labs 页面零入口；Layout 与页面 `h-screen` 混用导致双滚动/裁切；浅色系统主题下出现白底白字；核心表格/弹窗缺键盘可达性；无路由级代码分割（index chunk 1.94 MB）。
5. **发布不成熟**：新签名/公证 DMG 流水线从未产出制品；release notes 为空；最新 release 仍是旧 ZIP；issue #27（签名凭据）、#28（真机验证）未关闭。仓库对「已验证发布」的声明因此不能成立。
6. **轻量化欠账**：Apktool + Frida Gadget + Frida Core 静态链接把打包复杂度与体积推高，构建依赖联网下载；前端 mermaid/vis-network 图表链约 2.2 MB 常驻主包。
7. **文档/治理失真**：`CLAUDE.md` 整份过时；`docs/roadmap.md` 的 "Why convergence is required" 仍有已修复问题；`SECURITY.md` 版本策略与 1.3.0 矛盾；`docs/agents` 引用 7 个不存在文件；81% 已跟踪文档是内部过程文档且被公开构建；缺 CHANGELOG/SUPPORT/ADR/CODEOWNERS；仓库 description 仍是 "proxybot"、topics 为空。
8. **单点维护**：647 次提交中 631 次来自同一人；发布、真机验证、签名凭据都依赖单个维护者。

---

## 4. 本次轻量化收敛（2026-10-22）

用户决策：

- 只移除 Android SSL bypass（Frida/ADB/Apktool 与资源下载管线）；AI/Gen/Deploy/Graph/Topology 保留，但必须退到 Labs 开关后面。
- UI 收敛为 3 个默认目的地：**Capture / Setup / Settings**；Rules、Replay、Composer、DNS、Alerts、Analysis 收进 Capture 上下文；保留全部深链。
- 直接删除，不保留 feature flag；删除前用 tag `archive/pre-lightweight-2026-10-22` 留档。

### 4.1 收敛后状态（实测）

| 指标 | 收敛前 | 收敛后 |
| --- | --- | --- |
| Rust 源码 | 132 文件 / 44,117 行 | `src-tauri` 90 文件 / 32,302 行；core 8,499 行 |
| 桌面命令 | 131 | **123** |
| 类型化契约 | 36 / 131（27.5%） | 36 / 123（29.3%） |
| Rust 直接依赖 / Cargo.lock | 42 / 717 包 | **34 / 667 包** |
| Rust 测试 | 817 通过 | **763 通过 / 0 失败**（删除死代码/SSL bypass 测试 54 个） |
| 前端默认目的地 | 5（另有 16 条路由） | **3**（Capture / Setup / Settings） |
| 前端测试 | 29 文件 / 102 用例 | **30 文件 / 104 用例** |
| 首屏 index chunk | 1.94 MB | **466.6 kB**（gzip 144.6 kB，懒加载 Labs 页） |
| 打包资源 | 90.7 MiB（Apktool + 3 个 Frida Gadget）+ Frida Core 静态链接 | **0**（不再联网下载、不再打包原生资源） |
| 文档 | 118 个 md，81% 为内部过程文档，站点 174 URL | 当前文档 + ADR + history 归档；站点排除 history/agents/status |
| 开源治理文件 | 无 CHANGELOG/SUPPORT/ADR/CODEOWNERS | 全部补齐（`docs/adr/` 0001–0008、`CHANGELOG.md`、`SUPPORT.md`、`.github/CODEOWNERS`） |

### 4.2 执行内容

1. **Rust（−21 文件 / −3,290 行）**：端到端删除 Frida/SSL bypass/ADB（1,833 行）与零产品影响死代码 CDP/旧 replay 命令/ai_pipeline placeholder/Prometheus server/plugin loader+WASM 占位（1,457 行）；`DESKTOP_COMMANDS` 131→123；删除 8 个直接依赖与 4 个 dev 依赖；`build.rs` 去掉 Frida devkit 静态链接；修复 clippy 1.99 暴露的两处既有问题（`async_trait` 误报、`fetch_update` 弃用）。
2. **前端**：删除 SSL bypass 页面/store 与 `safeInvoke`（7 个调用点全部改为可重试显式错误）；侧栏 3 目的地；Capture 上下文 6 项 + Replay 上下文；Labs 默认关闭（Settings 开关 + ⌘K 命令面板 + "Labs are off"页）；请求详情新增 Replay / Add rule / Copy as cURL / Export HAR；共享 Loading/Empty/Error/InlineNotice/Dialog/Confirm；修复双滚动、浅色主题白底白字、表格/对话框键盘可达性；Labs 页 `React.lazy` 代码分割。
3. **打包与 CI**：删除 `scripts/fetch-bundle-resources.sh`、`src-tauri/resources/**`、`src-tauri/tauri.bundle.conf.json`；`build:tauri` 变为普通 `tauri build`；release 去掉资源下载与 Frida feature；CI 删除 Frida check、frontend/e2e 迁移 ubuntu、新增 docs-build 与 parity/workflow-config 门禁；release validate 增加 version/typecheck/UI/parity 发布门禁。
4. **仓库与文档**：历史计划移入 `docs/history/` 并退出站点构建；新增 ADR/CHANGELOG/SUPPORT/CODEOWNERS；重写 CLAUDE.md 与 CONTRIBUTING.md；设计源文件迁入 `design/`；删除 `.mcp.json`、Homebrew 占位 cask 与 tracked-but-ignored 状态；pnpm lockfile 移除未使用依赖（−243 行）。

### 4.3 验证结果（收敛后，全部实跑）

| 检查 | 结果 |
| --- | --- |
| `cargo test --workspace --locked --no-default-features` | 763 通过 / 0 失败 |
| `cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings` | 0 error / 0 warning |
| `cargo fmt --all -- --check` | 通过 |
| `cargo run -p proxybot --example export_desktop_contract -- --check` | contract 一致 |
| `tsc --noEmit` / `vitest run` | 通过 / 105 用例通过 |
| `vite build` | 成功，index ~467 kB |
| `playwright test`（浏览器 mock） | 110 通过；CI 配置 `retries: 2` 以吸收共享 dev server 的偶发抖动 |
| `node scripts/check_parity_matrix.mjs` | 47 capabilities PASS |
| `node --test scripts/*.test.mjs` | 25 通过 |
| `node scripts/version.mjs --check` | 1.3.0 一致 |
| `mkdocs build` | 0 warning（en+zh） |
| 全仓 markdown 相对链接 | 0 broken（排除 history 归档） |

### 4.4 收敛后仍开放

1. 请求详情「真正内联重放」需要把 `compose_request` 纳入生成的 Desktop Contract（Rust 契约扩展），当前为「跳转 Replay 并预选 host」。
2. Breakpoint 的 edit-and-forward 仍未接线（UI 已如实说明）；部分 Labs AI tab 仍有原生 `alert/prompt`；Analysis 导航项已按 Labs 开关显隐，但 Graph/Topology 两个分析视图仍是分开页面。
3. 打包体积的最终数字仍需一次真实 release 构建实测（预计 `.app` 15–30 MB / DMG 8–15 MB）。
4. 发布仍被 issue #27（签名凭据）与 #28（真机验证）阻塞；`csp:null` 与 `withGlobalTauri:true` 仍是已记录的技术债。
5. dependabot 三个 PR 未合并（jsonschema 0.49 API、tauri crate/npm 版本匹配、jsdom 30 与 Node 20）；建议拆分 major 升级并固定工具链后再处理。
6. Git 历史中仍保留约 46 MB 的旧 Labs 资源 blob；如需进一步瘦身仓库需单独做 `git filter-repo`（会改写历史，需维护者授权）。

---

## 5. 剩余工作建议（收敛后）

### P0（决定「可信发布」）

1. 关闭 issue #27：配置 Developer ID + 公证凭据，跑通一次手动 Release（tag + workflow_dispatch，见 [发布 runbook](../../docs/releasing.md)），产出 arm64/x64 两个已签名 DMG、sha256、SPDX SBOM、provenance attestation；预检脚本：`scripts/release-preflight.sh --dispatch`。
2. 关闭 issue #28：在物理 iOS/Android 设备上按 `docs/getting-started.md` 走通显式代理 + CA 信任 + 首个 HTTPS 请求，用 [device validation template](device-validation-template.md) 记录并留作 release evidence。
3. 把「已验证发布」与「源码构建」的声明继续分开：在 release 产出证据前，README 不承诺签名安装体验。

### P1（决定「产品可用」）

1. 完成剩余 IPC 命令的契约迁移，删除绕过 `Desktop Contract` 的直接 `invoke`（当前 36/131）。
2. 清理 30 个前端零引用命令：要么补产品入口，要么删除命令与模块。
3. 把 breakpoint edit-and-forward、HAR import/脱敏导出、WebSocket 二进制检查补成完整闭环，或从 Core 声明中降级。
4. 为 pf 与 MITM listener/runtime adapter 补自动化测试（当前 0 覆盖）。

### P2（决定「成熟开源」）

1. Graph/Topology 合并为一个分析工作流；Labs 能力逐个「毕业」（owner + 真实旅程 + 失败模型 + 文档 + 真实 Adapter 测试）。
2. 增加 CODEOWNERS / 第二位 maintainer / 发布轮值，降低单点风险。
3. 把 `cargo test`、契约检查、parity 检查、文档构建、链接检查全部纳入 CI 门禁（本次已补齐一部分）。
4. 配置仓库 description/topics/Release notes 模板与依赖更新策略（dependabot 失败 PR 需要处理）。

---

## 6. 证据附录

- 架构与体积明细、UI/交互明细、文档真伪与治理缺口、打包与构建成本：本次审计工作笔记保存在本地 `.audit/`（已 gitignore，不随仓库分发）；本报告已内联关键结论与可复现命令。
- 复现命令：
  ```bash
  CARGO_HOME=$PWD/.cache-cargo cargo test --workspace --locked --no-default-features
  ./node_modules/.bin/tsc --noEmit
  ./node_modules/.bin/vitest run
  node scripts/version.mjs --check
  node scripts/check_parity_matrix.mjs
  XDG_CACHE_HOME=$PWD/.cache-gh gh run list --workflow=ci.yml --branch main --limit 5
  ```
