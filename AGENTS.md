# VeinSweep 开发与发布说明

本文件面向开发者与代理，收录构建、打包、版本维护、校验、发布等工程信息。
面向玩家的功能说明与使用步骤在 [README.md](README.md)，两处不重复。

## 1. 工程形态与目录

Horizon 在 2026-11-15 合并了新 mod API。本项目使用新格式：

- 分发包是 **`.hmod`**（zip，根目录 `horizon-mod.toml`）；
- 原生 mod 带根 `Cargo.toml`、`Cargo.lock`（可选）与 `src/`，依赖
  `.horizon/sdk/api/crates/horizon-mod-api`；
- 由启动器用固定工具链针对**预编译游戏**编译，不再整份重编游戏；
- 钩子实现为 `mod_api::Hooks` 的方法：`break_block`（服务端，对应
  `BreakBlockEvent`）、`client_tick_post`（客户端每 tick 读触发键）、
  `register`（设置项与按键映射）、`register_commands` / `run_mod_command`
  （黑名单命令）。

```
VeinSweep/
├── mod/                       # 构建与打包的 mod 工程
│   ├── horizon-mod.toml       # 清单：id、version、description、api、[native] 固定构建
│   ├── Cargo.toml
│   ├── src/lib.rs             # 钩子、设置、命令、黑名单
│   ├── assets/veinsweep/lang  # 设置项、提示与命令反馈的英/简中/繁中文本
│   └── .horizon/sdk/          # 固定构建的 SDK（不入库、不入包）
├── publish/
│   ├── pack.py                # 生成 .hmod 并做结构校验
│   ├── veinsweep-0.6.1.hmod   # 分发包（构建产物，不入库）
│   ├── SUBMIT.md              # 上架说明：包格式、注册表、提交流程
│   └── registry/              # 注册表条目与图标
├── README.md                  # 项目描述与玩家使用说明
└── AGENTS.md                  # 本文件
```

`.horizon/`、`target/`、`*.hmod` 都在 `.gitignore` 里。

## 2. 固定构建与版本维护

固定构建：Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`
（Nightly 2026-11-20），horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`。

发版时版本号与哈希分散在多处，必须同步：

| 位置 | 需要同步的内容 |
| --- | --- |
| `mod/horizon-mod.toml` | `version` |
| `mod/Cargo.toml` | `version` |
| `publish/pack.py` | `VERSION`、`OUT` 文件名 |
| `publish/registry/mods/veinsweep.toml` | `version`、`file`、`sha256`、`changelog` |
| `publish/SUBMIT.md` | 文件名、包信息、检查清单哈希、实测记录 |
| `AGENTS.md` | 目录树里的包名、当前包大小与 sha256 |

- 启动器每次构建会产出 `native-<commit>-<hash>` 目录；源码变化可能生成新 tag。
  tag 用于实测记录，不参与打包；提交与报告中写最终一次构建的 tag
  （0.6.1 为 `native-0a875da-5d2421133c57`）。
- 升级固定构建：同时改 `horizon-mod.toml` 的 `[native] horizon` / `mixins`、
  `pack.py` 的 `HORIZON` / `MIXINS`，重新 `native-build` 并跑完整实测，
  然后发新版本；旧版本在新 Nightly 上会构建失败，属预期。
- 旧 `.hmod` 包发布前删除，`publish/` 只保留当前版本的分发包。

## 3. 行为规范（实现约束）

- **触发方式**：只有一个可自定义的 mod 按键映射（`trigger`），默认
  `key.keyboard.left.shift`。API 的 `KeyRegistration` 只接受物理键名，不能引用
  原版 `key.sneak` 映射；玩家改原版潜行键时本绑定不会跟随。
- 按住触发键且破坏方块才连锁；松开只破坏手动那一块。按键状态由客户端
  `client_tick_post` 采到静态变量，服务端 `break_block` 读取，并用本机玩家
  UUID 限定，避免 LAN 下影响其他玩家。
- **同种判定**：按方块 ID 比较，忽略朝向等方块状态。
- **两类扩散**：矿脉/原木固定 26 向（整个 3×3×3 邻域）连通；普通方块在
  `SCATTER_RADIUS = 4` 的立方内随机生长连通区域，每次形状不同。
- **硬约束**：`limit` 上限、黑名单（起点与相邻方块都拦截）、不可破坏方块、
  带方块实体方块。
- **创造模式**：默认不连锁；打开“创造模式连锁”后仍要按住触发键，按创造规则
  不掉落、不耗耐久。
- **无手持物判定**：空手、木头、镐走同一套规则；掉落仍按原版。
- **面向玩家的文本**：设置名、提示、命令反馈、按键名全部走语言文件
  （`assets/veinsweep/lang/{en_us,zh_cn,zh_tw}.json`），三种语言键集合必须一致；
  英文为回退语言。文案只描述功能，不写 BFS、API、钩子、伪随机、提交号等技术词。
- **清单描述字段**：`horizon-mod.toml` 的 `description` 是单语言字符串，游戏内
  模组列表直接显示它，不查语言文件。因此该字段写“简体中文 + 空行 + 英文”的
  组合文本；`zh_tw` 只用于设置页、提示与命令反馈。

## 4. 构建与安装

原生 mod 必须在本机编译（启动器用自带的 Rust 工具链和固定 SDK）。

### 启动器界面

1. 把 `publish/veinsweep-0.6.1.hmod`（或整个 `mod/` 文件夹）放进实例的
   `horizon-native-mods/`（游戏里 **选项 → 模组 → Open Mod Folder** 打开的就是这里）。
2. 在启动器的 Mods 页把 VeinSweep 打开（等价于 `horizon-mods.txt` 里写
   `mod.veinsweep:true`）。
3. 点 Build（或 Play，启动器会先构建）。

### 命令行

```powershell
# 构建到默认实例
& "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe" native-build `
    "e:\Project\Game\Minecraft\Horizon\VeinSweep\mod" --trust-native-code

# 指定实例与版本
& "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe" native-build `
    "e:\Project\Game\Minecraft\Horizon\VeinSweep\mod" `
    --instance "My Instance" --version nightly-2026-11-20-0a875da --trust-native-code
```

- `native-build` 需要先把该 Nightly 准备好（`horizon-launcher prepare --version ...`）。
- `--trust-native-code` 表示允许原生代码：它和普通程序一样拥有用户权限，没有沙箱。
- 构建成功后实例会选用一个新的 `native-<commit>-<hash>` 构建。
- 启动测试时**不要再显式传 `--version nightly-...`**，否则会覆盖实例已选的原生
  构建，导致 mod 未加载（模组页显示 “Installed, not built into this game”）。

想自己跑 `cargo check`：先把固定构建的 SDK 放进 `mod/.horizon/sdk/`
（`horizon-mod-api = { path = ".horizon/sdk/api/crates/horizon-mod-api" }` 是唯一允许的写法）。

## 5. 打包与结构校验

```powershell
python publish/pack.py
```

校验规则（按 `horizon_mod_package::Package` / `index::native::Bundle`）：

- 根目录只允许固定文件（`horizon-mod.toml`、`Cargo.toml`、`icon.png` 等），
  其余只能是 `src/`、`assets/`、`data/` 下的文件；`.horizon/`、`target/` 不入包；
- 路径合法、无重复、无 `mod.wasm`、条目数与总大小受限、包大小 ≤64 MiB；
- 清单 `id`/`version`/`api = "0.2"` 与 `[native] game/horizon/mixins` 与脚本常量一致；
- `Cargo.toml` 的 `horizon-mod-api` 必须按 SDK 相对路径依赖。

输出会打印 `size`、`sha256`、`files` 与 `bundle check: OK`；把哈希同步到注册表条目、
`SUBMIT.md` 和本文件（README 不再放发布信息）。

当前分发包：`publish/veinsweep-0.6.1.hmod`，36714 字节，sha256
`af5b5d6bfd2ee7a9cc800246083c46e3d8db3b91ff1542496afc8642170ae70c`。

## 6. 校验与实测要求

发版前至少完成：

1. `python publish/pack.py` → `bundle check: OK`；
2. `native-build` 针对目标 Nightly 完整编译通过；
3. 进游戏实测下列项目（生存模式，默认设置）：
   - 默认触发键（左 Shift）按住能连锁，松开后单次只破一块；
   - 在 选项 → 控制 → VeinSweep 改键后仍生效，实例 `horizon-mods.txt` 写入
     `key.veinsweep.trigger:<键名>`；
   - 设置页 5 项与默认值正确，语言文件无缺键；
   - 三种语言下设置名、提示、命令反馈显示正确；
   - 矿脉 26 向整条带走、普通方块扩散不超出半径 4 与上限、黑名单拦截、
     创造模式门控、空手与持物一致。

推荐的测试世界做法：`fill` 一个 7×7×7 石立方（343 块），玩家传送到立方上方，
`look_at_block` 对准目标，用 `game_key <键> down` + `game_mouse left down` +
`game_wait` + 释放的方式模拟“按住触发键”；用区域方块统计对比前后数量。
注意 `game_break` 会在结束时释放按住的键，不适合按住类测试。

## 7. 发布流程与注意事项

完整的包格式、注册表结构与提交流程见 [publish/SUBMIT.md](publish/SUBMIT.md)。

- 注册表条目 `publish/registry/mods/veinsweep.toml` 的 `authors` / `owners` /
  `owner_ids` / `source` / `license` 已回填（ChangGengCN，41566656，
  `https://github.com/ChangGengCN/VeinSweep`，MIT）；
- 每个新版本都要重新构建、重新打包、更新 sha256，并重新走人工审核；
- 原生 mod 以玩家权限运行，注册表不会自动合并新版本；
- 发布物固定到一个 Horizon 提交，新 Nightly 上需重新构建并发版。

## 8. 参考资料

- Horizon 主仓库（含 mod API 与包格式说明）：<https://github.com/horizon-rust/horizon>
- 官方 mods 注册表仓库：<https://github.com/horizon-rust/horizon-mods>
- 模组提交页：<https://playhorizon.dev/mods/submit>
- 本机启动器：`%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe`
- 上架细节与注册表字段：本仓库 [publish/SUBMIT.md](publish/SUBMIT.md)

## 9. 版本行为记录

- 0.3.0 起不再有“基础方块”白名单：**所有可破坏方块**都能连锁，只保留保护性
  判断（不能破坏的方块、带方块实体的方块、黑名单）。
- 0.4.0 起扩散分两类：矿脉/原木固定 26 向整邻域连通；泥土、石头、沙子等普通
  方块在固定半径的立方内按六面连通随机生长。
- 0.5.0 起连锁只由“按住触发键 + 破坏方块”触发，与手持物无关；新增连锁破坏
  总开关（默认开），关闭后连锁完全停用。
- 0.6.0 触发键默认改为潜行键（左 Shift）；普通方块扩散半径固定为 4，不再作为
  设置项；描述文案改为只讲功能；界面文本新增英文与繁体中文。
- 0.6.1 清单描述改为“简体中文 + 英文”的组合文本（模组列表直接显示两种语言）；
  开发者文档迁移到本文件。
