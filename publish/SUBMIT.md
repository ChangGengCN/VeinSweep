# 把 VeinSweep 上架 Horizon 官方注册表

本文按 **Nightly 2026-11-20（`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`）对应的文档**核对：
`docs/modding/src/how-to/publish.md`、`docs/modding/src/reference/registry-index.md`、
`horizon-rust/horizon-mods` 的 `README.md` 与 `MOD-POLICY.md`。

> 注意：网站上的 “Publish a mod” 页仍描述旧的 `.hnmod` / `native.py` 流程，**不适用于**
> 当前 SDK 的 `.hmod`；以仓库文档和 `horizon-mods` 的 README 为准。

---

## 0. 结论

- 当前 SDK 的原生 mod 是 **`.hmod`**（根 `horizon-mod.toml` + 根 `Cargo.toml` + `src/`），
  注册表 `kind = "native-mod"`；启动器用**固定签名的 SDK 与预编译游戏**在本地编译，
  注册表**不编译、不执行**提交的代码。
- **网页表单不能用**：官方说明表单尚不能接收新 SDK 的 `.hmod`（而且当前未开放）。
  唯一可行方式是把源码放到公开仓库、把包挂到该仓库的 GitHub Release，然后向
  `horizon-rust/horizon-mods` **提 PR**。
- 每个新项目、每个 mod 版本都要**人工审核**（原生代码以玩家权限运行，还会做源码安全审查）。
- 文件以 SHA-256 固定；已发布的文件永不替换，升级只能发新版本。
- 签名**可选但推荐**；一旦条目里填了公钥，之后每个版本都必须用同一把私钥签名。
- 本项目 0.6.1 的包与条目草稿已就绪；公开源码仓库（含 MIT LICENSE）与 `owner_ids`
  已就绪；仍需你本人完成：GitHub Release 上传与向注册表的 PR。

---

## 1. 提交入口与两种方式

| 方式 | 现状 | 能否用于本项目 |
| --- | --- | --- |
| 网页表单 <https://playhorizon.dev/mods/submit> | 目前未开放；官方明确说“不能接收新 SDK 生成的 `.hmod`” | **不能** |
| 向 `horizon-rust/horizon-mods` 提 PR | 当前唯一支持的入口；文件挂到你自己的 GitHub Release | **用它** |

### PR 方式的完整链路

1. 你把包挂到自己公开仓库的 GitHub Release，文件名与条目一致。
2. 你 fork `horizon-rust/horizon-mods`，在 PR 里新增/更新：
   - `mods/veinsweep.toml`（项目页 + 新 `[[versions]]` 记录）
   - `icons/veinsweep.png`（首次提交时）
3. PR 描述里给出文件的 release 下载地址：
   `https://github.com/ChangGengCN/VeinSweep/releases/download/v0.6.1/veinsweep-0.6.1.hmod`
4. `stage` 工作流下载该文件，核对文件名与 SHA-256 是新记录，运行检查器，
   把文件放进 `staging` 预发布（每次最多 3 个文件、每个 ≤64 MiB）。
5. `check` 工作流对比条目与 main、校验包结构、哈希与签名，并构建候选索引。
6. 审核者（CODEOWNERS）阅读条目、源码与检查报告；原生 mod 另做源码安全审查，
   可能要求修改。
7. 合并即发布：文件成为正式 release，项目几分钟内出现在网站与启动器里。

---

## 2. 材料清单（逐项对照本项目）

| 项目 | 规范要求 | 我们的现状 |
| --- | --- | --- |
| `id` | 清单 `id`，`[a-z0-9_]{1,64}`，全局唯一；已归档 id 被占用 | `veinsweep`；已查 `mods/`、`archive/native-v1/`、`archive/api-0.1/` 均无此 id |
| `kind` | 带 `[native]` 的 `.hmod` 用 `native-mod` | `native-mod` ✔ |
| `name` / `summary` | 名称；摘要 ≤160 字符，列表用 | `VeinSweep`；摘要 96 字符 ✔ |
| `description` | Markdown，去空白后 50–10,000 字符 | 852 字符 ✔ |
| `authors` / `owners` | 显示用作者；可发版的 GitHub 登录名 | `ChangGengCN` ✔ |
| `owner_ids` | 与 `owners` 对应的 GitHub 数字 ID（owner 更新与归档 id 确认要用） | `41566656` ✔ |
| `license` | SPDX 表达式；必须与源码仓库一致 | `MIT`，仓库已放 LICENSE ✔ |
| `source` | 源码的 `https://` 地址，公开可读 | `https://github.com/ChangGengCN/VeinSweep` ✔ |
| `icon` | `icons/<id>.png`：正方形 PNG，64–1024 px，≤256 KiB | `publish/registry/icons/veinsweep.png`：512×512，39,918 字节 ✔（PR 时放入 `icons/`） |
| `categories` | 1–3 个，取自该 kind 的类别表 | `game-mechanics`、`utility`（与已上架 Veinminer 的 “Game mechanics / Utility” 一致）✔ |
| `tags` | ≤10 个、小写 | 4 个，均小写 ✔ |
| `key` / `signature` | 可选；填了 `key` 则每个版本必须带签名 | 未签名（允许）；若要签名见第 6 节 |
| `screenshots` | 可选，≤8 张，PNG/JPEG | 无（可选，不阻塞） |
| `[[versions]]` | `version`、`published`、`game`、`file`、`sha256`、`changelog`；新记录在最上面 | 0.6.1 ✔ |
| 文件 | 扩展名匹配 kind；≤64 MiB；名字与条目一致 | `veinsweep-0.6.1.hmod`，36,714 字节 ✔ |
| 源码公开仓库 | 必须有，许可证允许分发；reviewer 要读 | 已推送，公开可读 ✔ |
| GitHub Release 附件 | 文件名必须与条目给出的名字一致 | **缺**（需你上传） |
| `cargo horizon-mod check` | 提交前自行跑一遍（CI 也会跑） | 已用 `checker@0a875da…` 官方预编译检查器运行，通过（exit 0） |
| 构建证明 | 原生 mod 固定到确切构建；reviewer 会看 | `native-build` 已产出 `native-0a875da-5d2421133c57`，可复现 |

---

## 3. 当前产物、已回填字段与必须你亲自做的事

`publish/` 下已有：

| 文件 | 用途 |
| --- | --- |
| `publish/veinsweep-0.6.1.hmod` | 要发布的分发包 |
| `publish/registry/mods/veinsweep.toml` | 注册表条目草稿 |
| `publish/registry/icons/veinsweep.png` | 正方形图标（512×512） |
| `publish/pack.py` | 重新打包与结构校验（`python publish/pack.py`） |

条目里的身份与来源字段已回填：

- `authors = ["ChangGengCN"]`、`owners = ["ChangGengCN"]`
- `owner_ids = [41566656]`（`https://api.github.com/users/ChangGengCN` 取到的数字 ID）
- `source = "https://github.com/ChangGengCN/VeinSweep"`
- `license = "MIT"`，仓库根目录已有配套的 LICENSE 文件

注意：本地提交身份是 `changgeng <18355166959@163.com>`（GitHub 数字 ID 21967815），
与 `ChangGengCN`（41566656）不是同一个账号；注册表以 `owners` / `owner_ids` 为准。

只能由你完成的事项：

- **GitHub Release**：把 `veinsweep-0.6.1.hmod` 作为附件上传到公开仓库，文件名不要改。
- **向 `horizon-rust/horizon-mods` 提 PR**：fork、提交条目与图标、写 PR 描述。
- **（可选）签名**：决定是否生成 minisign 密钥并固定公钥。

已完成、无需再处理：公开源码仓库、许可证、`authors` / `owners` / `owner_ids`、`source`。

---

## 4. 可直接照做的步骤

### 4.1 准备源码仓库（已完成）

1. 公开仓库已建立并推送：`https://github.com/ChangGengCN/VeinSweep`，默认分支 `main`。
2. 仓库已包含源码与 MIT LICENSE：
   - `mod/horizon-mod.toml`、`mod/Cargo.toml`、`mod/src/`、`mod/assets/`
   - `LICENSE`（MIT，2026，署名 ChangGengCN）
   - `README.md`、`README.zh-CN.md`、`AGENTS.md`、`publish/`（便于 reviewer 对照）
3. 确认仓库里的源码与 `publish/veinsweep-0.6.1.hmod` 内的源码一致。

### 4.2 建 Release

1. 在源码仓库新建 release，tag 任意（如 `v0.6.1`）。
2. 上传 `publish/veinsweep-0.6.1.hmod`，文件名保持 `veinsweep-0.6.1.hmod`。
3. 记录下载地址：
   `https://github.com/ChangGengCN/VeinSweep/releases/download/v0.6.1/veinsweep-0.6.1.hmod`。

### 4.3 填条目

1. 核对 `publish/registry/mods/veinsweep.toml` 里的 `authors` / `owners` / `owner_ids` /
   `source` 已是真实值（见第 3 节）。
2. 核对 `sha256` 与 `publish/veinsweep-0.6.1.hmod` 一致：
   `sha256sum publish/veinsweep-0.6.1.hmod`（Windows：`Get-FileHash -Algorithm SHA256 ...`）。
3. 确认 `versions[0]` 是最新版本（新记录在最上面）。

### 4.4 提 PR

1. fork `horizon-rust/horizon-mods`（可直接在 fork 的默认分支 `main` 上提交）。
2. 加入两个文件：
   - `mods/veinsweep.toml`（来自 `publish/registry/mods/veinsweep.toml`）
   - `icons/veinsweep.png`（来自 `publish/registry/icons/veinsweep.png`）
3. PR 标题如 `Add VeinSweep`，并按仓库的 `.github/PULL_REQUEST_TEMPLATE.md` 勾选声明；
   描述里必须放第 4.2 步的 release 下载地址（`stage` 只从 PR 描述里读链接），并说明：
   - 这是原生 mod，源码仓库地址；
   - 固定的 Nightly 2026-11-20（`0a875da`）与 mixins `e59da90`，仅在该构建验证；
   - 已在本地用 `native-build` 构建并实测；
   - 连锁由客户端按键触发，专用服务器上不生效；
   - 与已有 Veinminer / TreeFall 的差异（不限手持物、只在同种方块内扩散、黑名单命令、
     创造模式开关）。
4. 等待 `stage` / `check`；按 reviewer 意见修改。

### 4.5 后续版本

1. 改版本号（清单、Cargo.toml、pack.py、条目、SUBMIT/AGENTS），重新打包与构建。
2. 新版本上传到**新的** GitHub Release（不要覆盖旧文件）。
3. 在条目顶部新增 `[[versions]]`，提新 PR；只有 `owners` / `owner_ids` 中的账号可以更新。
4. 每个 mod 版本仍需人工审核。

---

## 5. 审核、CI、常见失败与下架

- **首次审核**：reviewer 阅读条目、源码和检查报告；原生 mod 的源码另做安全审查，
  重点看是否有隐藏网络访问、越权、破坏性行为，以及描述是否与实际一致。
- **CI 检查**：
  - `stage`：下载 PR 描述里的文件（≤3 个/次，64 MiB/个），核对文件名与 SHA-256 是新记录，
    运行检查器，放入 `staging`。
  - `check`：条目与 main 对比、包结构、哈希、签名、候选索引；原生包会核对清单里的
    `[native]`（game、horizon、mixins）与固定构建。
- **常见失败原因**：
  - SHA-256 或文件名与条目不一致；release 链接不是 GitHub release 下载地址；
  - 条目出现未知键（未知键会被拒绝）；
  - 描述不足 50 字符、摘要超过 160 字符、类别或标签不合法；
  - 图标不是正方形、超出 64–1024 px 或超过 256 KiB；
  - 包内清单的 `id`/`version`/`api`/`[native]` 与条目或固定构建不一致；
  - 版本号不是新的（已发布版本不能替换）；
  - 归档 id 需要 `owner_ids` 确认，而 PR 账号不在其中。
- **下架**：被下架的项目写入 `revoked.toml`；启动器拒绝其文件，并把已安装的副本停用、
  告知原因。作者可在对应的 issue 里申诉。

---

## 6. 本 mod 的特定风险与说明义务

1. **固定到单个 Nightly**：0.6.1 只对 `Nightly 2026-11-20（0a875da）` +
   `Horizon Mixins e59da90` 构建与验证。换 Nightly 需要重新构建并发新版本；
   条目 changelog 与描述里已写明，避免玩家在其他构建上误装。
2. **专用服务器上不生效**：连锁由客户端按住触发键发起；专用服务器没有客户端按键输入，
   服务端能加载但不会连锁。条目描述已写明“只在单人游戏或自己主持的局域网世界生效”，
   不要宣称服务端可用。
3. **功能重叠**：注册表里已有 Veinminer（旧 native-v1 条目）和 TreeFall 等类似项目。
   重叠本身不是拒绝理由，但建议在 PR/描述里说明差异：不要求手持工具、同种方块内扩散、
   黑名单命令、创造模式开关；并确认代码独立、未复制他人代码或素材。
4. **原生代码审查**：源码必须可读、可复现；不要有隐藏网络访问或与描述不符的行为。
5. **签名状态**：默认未签名（允许，页面会标注 unsigned）。若希望防止他人冒名发版，
   生成 minisign 密钥：把公钥填进条目的 `key`，每个版本的 `.minisig` 文本填进该版本的
   `signature`；此后所有版本都必须用同一把私钥签名，私钥丢失只能请维护者手工更换。

---

## 7. 本地安装与构建（速查）

1. 准备构建：`horizon-launcher prepare --version nightly-2026-11-20-0a875da`。
2. 构建：`horizon-launcher native-build "<本仓库>\mod" --version nightly-2026-11-20-0a875da --trust-native-code`
   （`native-build` 接收 **mod 文件夹**，不是实例目录）。
3. 启动测试时不要显式传 `--version`，否则会覆盖实例已选的原生构建。
4. 检查：游戏内 Mods 列表应出现 VeinSweep 0.6.1。

---

## 8. 提交前检查清单

- [x] 公开源码仓库已就绪，`source` 指向它，许可证与 `license` 一致，仓库内有 LICENSE
- [x] `authors`、`owners`、`owner_ids` 已替换为真实值，且与提 PR 的账号对应
- [ ] `publish/veinsweep-0.6.1.hmod` 的 sha256 与条目一致
      （`af5b5d6bfd2ee7a9cc800246083c46e3d8db3b91ff1542496afc8642170ae70c`）
- [ ] 文件已上传到 GitHub Release，文件名未改
- [ ] 图标为正方形 PNG、64–1024 px、≤256 KiB，随 PR 放入 `icons/veinsweep.png`
- [ ] 摘要 ≤160 字符、描述 50–10,000 字符、类别 1–3 个、标签 ≤10 个且小写
- [ ] `[native]` 的 `horizon` / `mixins` 与实际构建一致（0a875da… / e59da90…）
- [ ] 0.6.1 的 `[[versions]]` 在条目顶部，`game = "26.3"`
- [ ] 如有公钥：本版本已签名，`signature` 已填写
- [x] 已跑 `cargo horizon-mod check publish/veinsweep-0.6.1.hmod`（官方 checker@0a875da，exit 0）

## 9. 本地预检结果（2026-11-21）

使用 registry 仓库 `checker@0a875da91dbb0fdcb7ba8c42c11341aa229af51a` 的预编译检查器
（ELF，经 WSL 运行），在 fork 的临时提交上执行：

| 命令 | 结果 |
| --- | --- |
| `cargo horizon-mod check publish/veinsweep-0.6.1.hmod --kind native-mod` | 通过，exit 0；识别为 `native-1/0a875da…/e59da90…`，36,714 字节，sha256 `af5b5d6b…`，unsigned |
| `cargo horizon-mod registry review <base> <head> --author ChangGengCN` | 通过，exit 0；仅提示 “a new native-mod by ChangGengCN” |
| `cargo horizon-mod registry check <base> <head> --files <dir>` | 通过，exit 0 |

这三项与 PR 的 `stage` / `check` 使用同一份固定检查器，条目字段、图标、哈希与包结构均已验证。
