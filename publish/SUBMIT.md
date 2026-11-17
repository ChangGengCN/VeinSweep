# 把 VeinSweep 上架 Horizon 官方模组集

本文分两部分：**Horizon 官方对第三方 mod 的发布规范**（查自本机安装的 Horizon 源码与文档），
以及 **VeinSweep 按该规范产出的可提交产物、还差什么**。

相关官方文档（本机 Horizon 源码 `docs/` 下）：
`docs/api/mod-index.md`、`docs/api/mod-files.md`、`docs/design/registry-v2.md`、
`docs/native-mods.md`、`docs/mod-browser.md`。

---

## 0. 结论

- **可以上架，路径是明确的**：Horizon 官方注册表接受第三方 `native-mod`，
  分发文件是 `.hnmod`，提交入口是 <https://playhorizon.dev/mods/submit>（或直接向
  [horizon-rust/horizon-mods](https://github.com/horizon-rust/horizon-mods) 提 PR）。
- **不需要申请开发者账号，也不需要额外密钥**：用 GitHub 账号登录提交表单即可。
  签名（minisign）是**可选**的；不签名也能被收录，启动器会标注 “unsigned”。
- **本仓库已产出可提交的包和条目草稿**（见第 2 节），包结构按官方读取器
  `horizon_mods::index::native::Bundle` 的规则逐条校验通过。
- **有两个客观限制，无法在本地绕过**：
  1. 真正“上架”必须由**你的 GitHub 账号**完成（表单 OAuth 登录、`owners`/`owner_ids`、
     公开源码仓库地址），我无法代你登录或向你不知名的仓库提 PR。
  2. 本版本使用的 `player_break`、`can_build` 两个接口在 Horizon 里**还没有 API 编号**
     （仍是 `tools/native-hook-fragments/` 里的 pending 片段），所以不能声明 `hooks`，
     只能**固定到 Nightly 2026-11-13 的提交 `9f42064c`**。这会影响新 Nightly 上的可用性与自动更新，
     详见第 3 节。

---

## 1. 官方发布规范

### 1.1 分发包的格式

Horizon 的代码模组分为两类：数据内容模组 `.hmod`（不含代码，不编译）和 native mod `.hnmod`
（Rust 源码，安装后由启动器编译进游戏）。连锁挖掘要改游戏行为，必须走 **native mod**。

`.hnmod` 就是一个 zip，顶层只能有：

| 条目 | 含义 |
| --- | --- |
| `horizon-native.json` | native 包清单（必需） |
| `mixins/<id>/horizon-mixin.toml` | 注入清单（必需） |
| `mixins/<id>/…` | 只允许 `.rs`、`.toml`、`data/` 下的文件、以及 crate 的 `README*`/`LICENSE*`/`COPYING*`/`NOTICE*`/`.gitignore` |

限制：最多 512 个文件、解压后总共 64 MiB；路径不能含 `\`、`:`、`..`，不能是符号链接；
一个包里不能再带 `mod.wasm`（WebAssembly 模组已停止支持）。完整规则见 `docs/api/mod-files.md`。

### 1.2 包内清单的字段

`horizon-native.json`（未知字段会被拒绝）：

| 字段 | 规则 |
| --- | --- |
| `schema` | 固定 `1` |
| `id` | `[a-z0-9_]{1,64}`，必须与 `mixins/<id>/` 和 `horizon-mixin.toml` 的 `[mod] id` 一致 |
| `version` | 非空，必须与 `horizon-mixin.toml` 的 `[mod] version` 一致 |
| `game` | 非空，Minecraft 版本，例如 `"26.3"` |
| `horizon` | 40 位小写十六进制：开发/实测所用的 Horizon 提交，必须与 `[compatibility] horizon_revision` 一致 |
| `mixins` | 40 位小写十六进制：horizon-mixins 的提交 |
| `hooks` | 可选，整数 ≥1：只使用“命名钩子 + `Host` 接口”的模组可以写它，代替固定 Horizon 提交 |

`horizon-mixin.toml` 里写注入点（`hook` + `hook_version`，或 `target`）、绑定、helper 与模块。
VeinSweep 的四个注入点（`destroy_block`、`mods`、`settings`、`settings_click`）都是
`native-hooks.json` 里的命名钩子 v1。

### 1.3 注册表条目（`mods/<id>.toml`）的字段

项目由 `horizon-rust/horizon-mods` 里的一个 TOML 条目描述。必填/常用字段：

| 字段 | 说明 |
| --- | --- |
| `id`、`kind` | `kind = "native-mod"` |
| `owners` | GitHub 登录名，至少一个；有权限发更新的人 |
| `owner_ids` | 可选：这些人的**数字 GitHub ID**。写了它之后，只按 ID 认人（登录名可能被回收）；不写则按登录名匹配，更新总要人工审核 |
| `name`、`summary` | 名称；一句话摘要（≤160 字符） |
| `description` | Markdown，**至少 50 字符**，最多 10000 |
| `authors`、`license` | 作者列表；SPDX 许可证表达式，**必填** |
| `source` | 源码的 `https://` 地址，**必填** |
| `icon` | **必填**，且必须是注册表仓库里的 `icons/<id>.png`：正方形 PNG，64–1024 px，≤256 KiB |
| `categories` | **必填**，1–3 个，取值来自官方类别表；mod 与 native-mod 可用 `game-mechanics`、`utility` 等 |
| `tags` | 可选，≤10 个，小写字母/数字/`-`，≤24 字符，不能涉及赌博 |
| `screenshots` | 可选，≤8 张，放在注册表仓库 `screenshots/<id>/`，PNG/JPEG，320×180 到 3840×2160，每张 ≤1 MiB |
| `key` | 可选：作者的 minisign 公钥。一旦填写，每个版本都必须用对应私钥签名 |
| `[[versions]]` | 每个版本一条：`version`、`published`（YYYY-MM-DD）、`game`、`file`（下载文件名）、`sha256`、可选 `signature`（`.minisig` 文本）与 `changelog` |

条目里的 `api`、`side`、`capabilities`、包大小等由 CI 从文件里读取，不能手写。
文件大小为 64 MiB 上限；文件名必须与扩展名匹配（native mod 用 `.hnmod`）。

### 1.4 签名与身份

- **签名可选**。不填 `key` 就是“未签名项目”，启动器在安装前会明确标注。
  填了 `key` 后，所有版本必须用同一把 minisign 私钥签名，换钥匙的更新会被拒绝。
  官方建议签名：这样即使 GitHub 仓库或账号被盗，也无法发布冒充的更新。
- **身份**：不需要 Horizon 开发者注册。提交表单用 GitHub OAuth 登录，条目记录 `owners` 与 `owner_ids`；
  后续版本只有 owner 的更新可能自动合并（且仅在 `owner_ids` 匹配、检查通过时）。

### 1.5 提交入口、审核与更新

- **入口**：<https://playhorizon.dev/mods/submit>，或直接向
  `horizon-rust/horizon-mods` 提 PR（加 `mods/<id>.toml` 与 `icons/<id>.png`，
  版本文件上传到该仓库的 `staging` prerelease，命名为 `<sha256>.hnmod`）。
- **审核**：**上架时人工审核**（PR 由 CODEOWNER 审，native mod 的源码会被阅读；
  `cargo horizon-mod check` 只做结构/清单校验，不执行代码）。**之后的每个新版本由 CI 自动检查**，
  不再逐个人工审核；`revoked.toml` 用来下架有问题的文件。
- **native mod 的额外机制**：注册表每天用最新 Nightly 试装每个 native mod 的新版本
  （`native-compat.yml`），失败会记录在索引的 `broken_on` 里；启动器在安装前会告知
  “这个版本在你要玩的 Horizon 上构建不了”。
- **更新**：新条目不允许再写 `repo`（早期条目可以）。没有 `repo` 的项目通过提交表单上传新文件；
  owner 的更新在没有其他改动、检查通过时可以自动合并。
- **固定版本 vs `hooks`**：声明 `hooks` 的 native mod 能在任何 `NATIVE_HOOKS_API` 不低于它的 Horizon 上构建；
  固定 `horizon` 的 mod 只能在其提交上构建。固定 Nightly 的实例不会自动更新 native mod。

---

## 2. 已产出的可提交产物

| 文件 | 用途 |
| --- | --- |
| `publish/veinsweep-0.2.1.hnmod` | 真正要上传的分发包 |
| `publish/registry/mods/veinsweep.toml` | 注册表条目草稿（复制进 horizon-mods 的 `mods/`） |
| `publish/registry/icons/veinsweep.png` | 512×512 PNG 图标（复制进 `icons/`） |
| `publish/pack.py` | 重新打包 + 结构校验脚本（改了源码或 README 后运行它） |

包信息：

```
id       veinsweep
version  0.2.1
game     26.3
horizon  9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0   (Nightly 2026-11-13)
mixins   5a64af938ec86b00d53d16a408aabc0038fb3e57
size     12787 bytes
sha256   8dd769761698a156e416fcf6e4a83332ffc505640763129494b93f6dcf39426f
```

包内文件：`horizon-native.json`、`mixins/veinsweep/horizon-mixin.toml`、
`veinsweep.rs`、`chain.rs`、`list.rs`、`page.rs`、`click.rs`、`README.md`。

修改源码或 README 后，运行 `python publish/pack.py` 重新生成包；它是确定性的
（固定时间戳与权限），同样的文件得到同样的 sha256。把新 sha256 同步到条目与本文即可。

已做的验证：

- `horizon-mixins plan` / `apply`（官方工具，提交 `5a64af938ec8`，对应 Horizon `9f42064c`）：
  4 个注入点、1 个模块全部解析成功，注入到 `native_destroy_block`、`native_mods`、
  `native_settings`、`native_settings_click`。
- 按 `horizon_mods::index::native::Bundle` 的规则逐条检查 zip：路径、允许的文件、清单字段、
  `id`/`version`/`horizon_revision` 一致性、数量与大小限制，全部通过。
- 0.2.1 源码用官方 `horizon-mixins apply` 注入后，用 launcher 自带的 Rust 1.99.1 工具链
  （`x86_64-pc-windows-gnu`，`fast` profile）完整 `cargo build -p horizon-client` 通过（7 分 40 秒）；
  产物中确认是 `VeinSweep 0.2.1`，并包含“基础方块”设置项，旧的三个分类按钮已不存在。
  （launcher 的 `native-build --server-output` 因本机 `native/build.log` 权限被拒绝而未能复跑。）
- 0.2.0 的泥土连锁已在游戏内复测：纯泥土 9×9 平面、上限 64 时，从中心按六向连锁了 64 块，
  邻居枚举和 BFS 正常；“沿斜线单向扩展”来自当时要求邻居与起点方块名完全相同，
  草方块在自然斜坡上恰好连成一条线。0.2.1 已改为基础方块按类别匹配。
- 0.2.1 的按类别连通效果尚未在游戏内复测（安装新 native 构建需要 launcher 的 Build 步骤）。

**没有运行** `cargo horizon-mod check`：官方工具需要从 Horizon 源码构建（本机只有游戏运行时，
没有预编译的 `cargo-horizon-mod`）。提交 PR 后 CI 会自动运行它；建议在提交前于 Horizon 源码目录执行一次：

```
cargo run -p cargo-horizon-mod -- check <路径>\veinsweep-0.2.1.hnmod
```

---

## 3. 卡住的客观条件

### 3.1 两个接口还没编号，只能固定 Nightly（最主要）

VeinSweep 调用：

- `Game::native_player_break_block`（`tools/native-hook-fragments/player_break.json`）
- `Game::native_may_build`（`tools/native-hook-fragments/can_build.json`）

这两个都还是 **pending 片段**，没有分配 `NATIVE_HOOKS_API`，也没有进入 `native-hooks.json` 的 helpers。
因此本包**不能**写 `"hooks"`，只能固定 `horizon = 9f42064c…`。后果：

- 包只有在 **Nightly 2026-11-13 这个提交**上构建；玩家安装时启动器可能把该实例的构建切到这个提交，
  或（与固定到别的提交的 mod 同时启用时）要求玩家点 “Build anyway”。
- 注册表每天会用**最新 Nightly** 试装一次；在新提交上它很可能被记为 `broken_on`，
  启动器会提示“在这个 Horizon 上构建不了”。
- 固定 Nightly 的实例**不会自动更新** native mod；Horizon 更新后需要重新构建并提交新版本。

**要彻底解决**：等 Horizon 上游把这两个片段 stamp 成正式 API（每个片段分配 API 号，
加入 `native-hooks.json` 的 helpers，模组改用 `Host` 接口），然后本模组可以声明 `"hooks": <版本>`，
一次发版长期可用。这需要向 `horizon-rust/horizon` 提交 API 片段 PR——不是本仓库能单方面完成的。

### 3.2 上架必须由你的 GitHub 账号完成

`owners`、`owner_ids`、`source` 都是账号相关且无法猜测的：

- `owners` 必须是你的 GitHub 登录名；
- `owner_ids` 需要你的数字 ID：`gh api users/<login> --jq .id`（推荐填，否则更新总要人工审核）；
- `source` 必须是**公开**源码仓库的 `https://` 地址（注册表上架时审的就是这份源码）；
- 表单提交需要 GitHub OAuth 登录。

当前条目里这三处是占位符（`your-github-login`），**直接提交会被 CI 拒绝**，必须先替换。

### 3.3 需要你确认的页面信息

- `license = "MIT"` 是草稿值。请确认许可证，或改成你实际使用的 SPDX 表达式。
- 图标是我生成的 512×512 PNG（深色背景 + 3×3 矿石 + 碎裂的一格），可以随时替换为你的版本，
  只要仍是正方形 PNG、64–1024 px、≤256 KiB。

---

## 4. 提交步骤

### 方式 A：提交表单（推荐）

1. 把源码推到公开 GitHub 仓库，记下地址（例如 `https://github.com/<你>/veinsweep`）。
2. 用 `<你>` 和实际许可证替换 `publish/registry/mods/veinsweep.toml` 里的占位符，
  并补上 `owner_ids = [<你的数字 ID>]`。
3. 打开 <https://playhorizon.dev/mods/submit>，GitHub 登录，按表单填写：
   id `veinsweep`、kind `native-mod`、名称、摘要、描述、作者、许可证、源码地址、类别与标签。
4. 上传 `publish/veinsweep-0.2.1.hnmod` 和 `publish/registry/icons/veinsweep.png`。
5. 提交。服务会把文件上传到注册表的 `staging` prerelease（命名为 `<sha256>.hnmod`），
   并自动开一个 PR；PR 里就是 `mods/veinsweep.toml` 和 `icons/veinsweep.png`。
6. 等 CI（`cargo horizon-mod registry check` + 索引构建）和 CODEOWNER 审核；通过并合并后，
   下一个小时的 publish 任务会把项目写进 `index.json`，启动器的模组页就能搜到。

### 方式 B：手动 PR

1. Fork `horizon-rust/horizon-mods`。
2. 把 `publish/registry/mods/veinsweep.toml` 放到 `mods/veinsweep.toml`，
   `publish/registry/icons/veinsweep.png` 放到 `icons/veinsweep.png`。
3. 把 `veinsweep-0.2.1.hnmod` 上传到该仓库的 `staging` prerelease，**文件名必须是 `<sha256>.hnmod`**
   （即 `8dd769761698a156….hnmod`，CI 按 sha256 找文件）。
4. 开 PR，写清这是新项目、固定了哪个 Horizon 提交、以及为什么（两个 pending API）。

### 可选：签名

如果决定签名，用 minisign 生成一对密钥：

```
minisign -G -p veinsweep.pub -s veinsweep.key
minisign -S -s veinsweep.key -m veinsweep-0.2.1.hnmod
```

然后把 `veinsweep.pub`（去掉注释行后的公钥文本）填进条目的 `key`，
把 `veinsweep-0.2.1.hnmod.minisig` 的内容填进 `[[versions]]` 的 `signature`。
私钥务必离线保存；以后每个版本都必须用同一把私钥签名。

---

## 5. 上架之后

- 索引每小时发布一次；项目页、图标、下载量、源码仓库 star 数会自动更新。
- 后续版本：没有 `repo` 的新条目通过表单上传新文件，表单会开 PR；
  `owner_ids` 匹配且检查通过时，owner 的纯更新可以自动合并，否则等人工审核。
- 出现问题时，官方通过 `revoked.toml` 按 `sha256` 或整个项目下架；启动器会停用已安装的副本并说明原因。
- 每次 Horizon 更新后，注册表会在最新 Nightly 上试装；如果本包因固定提交而失败，
  会出现在 `broken_on` 里，提醒你发新版本。

---

## 6. 提交前检查清单

- [ ] `publish/veinsweep-0.2.1.hnmod` 的 sha256 与条目里的 `sha256` 一致
- [ ] 条目里的 `owners`、`author`、`source`、`license` 已替换为真实值
- [ ] `owner_ids` 已填写（与 `owners` 顺序一致），或明确接受“更新需人工审核”
- [ ] 图标仍是正方形 PNG、64–1024 px、≤256 KiB，且路径为 `icons/veinsweep.png`
- [ ] 描述至少 50 字符、类别 1–3 个、标签合法（小写、≤24 字符、无赌博词）
- [ ] 版本 `game` 与包里的 `game` 一致（`26.3`）
- [ ] 已在 `cargo horizon-mod check` 下通过（或在 PR 的 CI 里确认通过）
