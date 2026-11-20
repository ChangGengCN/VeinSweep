# 把 VeinSweep 上架 Horizon 官方模组集

本文按 **2026-11-18 的新规范**（Horizon MOD API 合并后）整理：包格式、清单字段、
发布流程、本地安装，以及本仓库产出的可提交文件。

参考文档（Horizon `main`；0.3.7 已按 Nightly 2026-11-20 / `0a875da` 复核）：

- `docs/modding/src/reference/manifest.md`（`.hmod`、`horizon-mod.toml`）
- `docs/modding/src/how-to/native-mods.md`、`docs/modding/src/reference/sdk.md`
- `docs/modding/src/start/installing.md`（安装目录与启用方式）
- `docs/modding/src/reference/registry-index.md`、`docs/modding/src/how-to/publish.md`
- `crates/horizon-mod-vocab/src/hooks/break_block.rs`（钩子声明）

---

## 0. 结论

- 新版把旧的 native mixin 包（`.hnmod` + `horizon-native.json` +
  `mixins/<id>/horizon-mixin.toml`）换成了 **`.hmod`**：根目录
  `horizon-mod.toml` + 根 `Cargo.toml` + `src/`。启动器用**预编译游戏 + 签名 SDK**
  编译 mod，不再为每个 mod 重编整份游戏。
- 发布仍是 `kind = "native-mod"`，条目在 `horizon-rust/horizon-mods` 的
  `mods/veinsweep.toml`，文件按 sha256 固定；**上架和每个新版本都要人工审核**
  （原生代码以玩家权限运行）。
- 本仓库已按新规范产出 0.3.7 的可提交包与条目草稿（见第 2 节）。
- 仍需你本人完成：把源码放到公开仓库、用 GitHub 登录提交表单（或提 PR）；
  条目里的 `owners` / `source` / `license` 是占位符。

---

## 1. 新旧规范的实际差异

| 项目 | 旧（Nightly 2026-11-13，9f42064c） | 新（Nightly 2026-11-18 起，0b14f8a 引入） |
| --- | --- | --- |
| 分发文件 | `.hnmod` | `.hmod` |
| 清单 | `horizon-native.json`（JSON，schema 1） | `horizon-mod.toml`（TOML，含 `[native]`） |
| 注入方式 | `mixins/<id>/horizon-mixin.toml` + `native-hooks.json` 命名钩子 | 实现 `mod_api::Hooks` 的方法；`[[inject]]` 只用于函数槽监听 |
| 依赖 | 游戏源码中的 `horizon_client` 等 crate | `horizon-mod-api`（lib `mod_api`），路径 `.horizon/sdk/api/crates/horizon-mod-api` |
| 构建 | 把 mod 注入游戏源码后整份 `cargo build -p horizon-client` | 启动器下载固定构建的预编译游戏与签名 SDK，只编译 mod |
| 触发钩子 | `Game::native_destroy_block` | `Hooks::break_block`（服务端 `BreakBlockEvent`，`canceled`、`exp_to_drop`） |
| 连锁破坏 | `Game::native_player_break_block` | `PlayerRef::destroy_block(host, pos, into_inventory)` |
| 设置 | 自定义 `settings` / `settings_click` 按钮列表 + `native_setting` | `Registrar::register_setting(SettingSpec)`，自动出现在 Mods → Config；读取用 `Host::setting` |
| 安装目录 | `horizon-native-mods/` | **不变**：`horizon-native-mods/` |
| 启用 | `horizon-mods.txt` 的 `mod.<id>:true` | **不变** |
| 注册表 | `[[versions]]` 的 `version/published/game/file/sha256` | 同样的 `[[versions]]`，文件改为 `.hmod`；索引记录 `api`（native 为 `native-1/<horizon>/<mixins>`）与 `file` 对象 |
| 发布入口 | 表单可上传 `.hnmod` | 表单可上传 `.hmod`；也可把文件挂到自有仓库 release 后提 PR |

关键影响：

- 旧 `.hnmod` 不能再用于新构建；必须换成 `.hmod` 并重新编译。
- 不再需要 `horizon` / `mixins` 之外的 API 片段兼容声明；但 mod 仍固定到一个
  确切构建（`[native] horizon`），新 Nightly 上要用就重新构建/发版。
- 设置页不再是自绘按钮列表，而是 6 个注册设置项。

---

## 2. 已产出的可提交产物

| 文件 | 用途 |
| --- | --- |
| `publish/veinsweep-0.3.7.hmod` | 真正要上传的分发包 |
| `publish/registry/mods/veinsweep.toml` | 注册表条目草稿 |
| `publish/registry/icons/veinsweep.png` | 512×512 PNG 图标 |
| `publish/pack.py` | 重新打包 + 结构校验（改了源码后运行） |

包信息：

```
id       veinsweep
version  0.3.7
api      0.2
game     26.3
horizon  0a875da91dbb0fdcb7ba8c42c11341aa229af51a   (Nightly 2026-11-20)
mixins   e59da9094f0c2fdda6c627fe9eff22dec6a906f0
size     31525 bytes
sha256   a121149296549349853500b3d5f6752168b3efd0e816798d820fb736ef06ff82
```

包内文件（6 个）：`horizon-mod.toml`、`Cargo.toml`、`src/lib.rs`、`icon.png`、
`assets/veinsweep/lang/zh_cn.json`、`assets/veinsweep/lang/en_us.json`。

已做的验证：

- `publish/pack.py`：路径、允许的文件、重复项、清单 id/version/api/`[native]`
  与固定构建一致、`Cargo.toml` 只按 SDK 路径依赖、条目数/总大小，全部通过
  （`bundle check: OK`）。
- `horizon-launcher native-build` 针对 Nightly 2026-11-20（`0a875da`）完整编译通过，
  0.3.7 产出 `native-0a875da-22e33cf1f346`；随后在同一构建、默认设置
  （仅潜行开、上限 32、含对角连通开）下进游戏实测：
  - **砍树不带泥土**：3×3 原木平台（含 `axis=x` 与 `axis=y` 混排）下方紧贴 3×3 泥土，
    潜行破坏一角原木：9 块原木全部连锁（不同朝向算同种），下方 9 块泥土一块未动；
  - **挖泥土不带草方块**：3×3 泥土平台里混入 `grass_block`、`coarse_dirt`，下方为石头：
    只连锁 7 块泥土，草方块、砂土与下方石头全部保留；
  - **挖沙子不带沙砾或石头**：3×3 沙子平台混入 `gravel`、`stone`，下方为石头：
    只连锁 7 块沙子，沙砾、石头全部保留；
  - **黑名单仍生效**：`/veinsweep blacklist add minecraft:sand` 后同一沙平台只破手动一块
    （其余 8 块保留），`remove` 后再挖恢复整片连锁。
- 0.3.1 的创造模式门控实测（创造世界、3×3 白色羊毛、空手、上限 32）：
  - 设置页当时显示 7 项，“创造模式连锁”默认“关”；此时按住潜行挖掉一块，只有 1 块消失；
  - 在 Config 里打开“创造模式连锁”后，按住潜行挖同一位置，3×3 剩余 8 块全部连锁；
  - 打开该开关但不按潜行，单击破坏仍然只破 1 块（潜行条件独立生效）。
- 0.3.2 实测（新建实例、编译产出 `native-0b14f8a-92fe2f73dad2`）：
  - 设置页只剩 6 项，已无“空手触发”；顺序为 仅潜行时触发、最大方块数、
    掉落直接入包、含对角连通、保留工具耐久、创造模式连锁；
  - 生存模式、空手、按住潜行破坏 3×3 白色羊毛：一次连锁挖掉 8 块
    （连带周围泥土/树叶，受上限 32 约束）。
- 未运行 `cargo horizon-mod check`：本机没有该工具（需 `cargo install --git
  https://github.com/horizon-rust/horizon cargo-horizon-mod`）。提交 PR 后 CI 会跑；
  建议提交前在 Horizon 源码目录执行一次：
  `cargo run -p cargo-horizon-mod -- check <路径>\veinsweep-0.3.7.hmod`。

---

## 3. 本地安装步骤（新版）

1. **准备版本**：启动器里选/装 Nightly 2026-11-20（`0a875da`）。命令行：
   `horizon-launcher prepare --version nightly-2026-11-20-0a875da`。
2. **放 mod**：把 `veinsweep-0.3.7.hmod` 放进实例的 `horizon-native-mods/`
   （选项 → 模组 → Open Mod Folder）。
3. **启用**：启动器 Mods 页打开 VeinSweep，或在实例目录的 `horizon-mods.txt`
   里写 `mod.veinsweep:true`。
4. **构建**：
   `horizon-launcher native-build <实例>\horizon-native-mods --trust-native-code`
   或 `native-build <本仓库>\mod --trust-native-code`。
   构建会下载该构建的预编译游戏与 SDK；首次较慢，之后增量。
5. **进游戏**：模组列表出现 VeinSweep 0.3.7；选项 → 模组 → Config 可调 6 个设置项。

与旧版的差异：目录与启用方式没变，但**必须重新 Build**；旧 `.hnmod` 不再可用。
如果实例里还有其他旧 native mod（例如 jade/jei 的 `.hnmod`），新版启动器无法编译它们，
需要换成各自的 `.hmod` 版本或先停用。

---

## 4. 提交步骤

### 方式 A：提交表单

1. 把 `mod/`、`publish/`、README 推到公开 GitHub 仓库，记下地址。
2. 替换 `publish/registry/mods/veinsweep.toml` 里的占位符：
   `owners`/`authors` 的 `your-github-login`、`source`、`license`；
   推荐补 `owner_ids = [<你的数字 ID>]`（`gh api users/<login> --jq .id`）。
3. 打开 <https://playhorizon.dev/mods/submit>，GitHub 登录，上传
   `publish/veinsweep-0.3.7.hmod`，填写页面信息（id、名称、摘要、描述、作者、
   许可证、源码、1–3 个类别、≤10 个标签）。
4. 表单会校验文件、把文件放到 staging、开 PR；审核通过并合并后发布。

### 方式 B：手动 PR

1. Fork `horizon-rust/horizon-mods`。
2. 把 `publish/registry/icons/veinsweep.png` 放到 `icons/veinsweep.png`。
3. 把 `veinsweep-0.3.7.hmod` 挂到你自己的公开仓库 release，文件名保持
   `veinsweep-0.3.7.hmod`。
4. 把 `publish/registry/mods/veinsweep.toml` 放到 `mods/veinsweep.toml`，
   在 PR 描述里链接 release 文件；审核通过后由维护者 staging。

### 可选：签名

用 minisign 生成密钥，把公钥填进条目的 `key`，把
`veinsweep-0.3.7.hmod.minisig` 的内容填进 `[[versions]]` 的 `signature`。
之后每个版本都必须用同一把私钥签名。

---

## 5. 客观限制

- **每个版本都要人工审核**：原生 mod 以玩家权限运行，注册表不会自动合并它的新版本。
- **固定构建**：`[native]` 固定 `horizon` 与 `mixins` 的完整提交；Horizon 更新后
  需要重新构建并发新版本，否则新 Nightly 上会构建失败（索引里记为不兼容）。
- **账号相关字段无法代填**：`owners`、`owner_ids`、`source` 必须由你的 GitHub
  账号与公开仓库决定。

---

## 6. 提交前检查清单

- [ ] `publish/veinsweep-0.3.7.hmod` 的 sha256 与条目一致
      （`a121149296549349853500b3d5f6752168b3efd0e816798d820fb736ef06ff82`）
- [ ] 条目里的 `owners`、`authors`、`source`、`license` 已替换为真实值
- [ ] 图标是正方形 PNG、64–1024 px、≤256 KiB
- [ ] 描述 ≥50 字符，类别 1–3 个，标签合法
- [ ] `[native]` 的 `horizon` / `mixins` 与实际构建一致
- [ ] 已在 `cargo horizon-mod check` 下通过（或确认 PR 的 CI 通过）
