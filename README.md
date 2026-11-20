# VeinSweep —— Horizon 连锁挖掘模组

按住潜行挖掉一个方块时，把与它相连的**同种方块**一起挖掉。连锁破坏走游戏自己的
`ServerPlayerGameMode.destroyBlock`（`PlayerRef::destroy_block`），权限、工具判定、
掉落、耐久、经验、统计、粒子和音效都与手动挖一块完全一致。

0.3.0 起不再有“基础方块”白名单：**所有可破坏方块**都能连锁。只保留保护性判断
（不能破坏的方块、带方块实体的方块、黑名单）。连锁只由**潜行 + 破坏方块**触发，
与手里拿着什么无关：空手、拿木头、拿镐的行为完全一致。

“同种”按**方块 ID**判定（忽略朝向等方块状态）：`oak_log` 的不同朝向互相连锁，
但 `oak_log` 不会蔓延到 `birch_log`，`dirt` 不会蔓延到 `grass_block` 或 `coarse_dirt`，
`sand` 不会蔓延到 `gravel`；同一族但不同 ID 的方块默认互不蔓延。

> 想发布到 Horizon 官方模组集：见 [publish/SUBMIT.md](publish/SUBMIT.md)。

## 类型与格式（2026-11-18 的新规范）

Horizon 在 2026-11-15 合并了新 mod API。本项目已从旧的 native mixin 格式
（`.hnmod` + `horizon-native.json` + `horizon-mixin.toml`）迁移到新格式：

- 分发包是 **`.hmod`**（zip，根目录 `horizon-mod.toml`）；
- 原生 mod 带根 `Cargo.toml`、`Cargo.lock`（可选）与 `src/`，依赖
  `.horizon/sdk/api/crates/horizon-mod-api`；
- 由启动器用固定工具链针对**预编译游戏**编译，不再整份重编游戏；
- 钩子实现为 `mod_api::Hooks` 的方法，本项目只实现
  `break_block`（服务端，对应 NeoForge 的 `BreakBlockEvent`）、
  `register`（设置项）、`register_commands` / `run_mod_command`（黑名单命令）。

固定构建：Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`
（Nightly 2026-11-20），horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`。

## 项目结构

```
VeinSweep/
├── mod/                      # 构建与打包的 mod 工程
│   ├── horizon-mod.toml      # 清单：id、version、api、[native] 固定构建
│   ├── Cargo.toml
│   ├── src/lib.rs            # 钩子、设置、命令、黑名单
│   ├── assets/veinsweep/lang # 设置项的中英文标签
│   └── .horizon/sdk/         # 固定构建的 SDK（不入库、不入包）
├── publish/
│   ├── pack.py               # 生成 .hmod 并做结构校验
│   ├── veinsweep-0.3.7.hmod  # 分发包（构建产物，不入库）
│   ├── SUBMIT.md             # 发布说明
│   └── registry/             # 注册表条目与图标
└── README.md
```

`.horizon/`、`target/`、`*.hmod` 都在 `.gitignore` 里。

## 构建与安装

原生 mod 必须在本机编译（启动器用自带的 Rust 工具链和固定 SDK）。

### 启动器界面

1. 把 `publish/veinsweep-0.3.7.hmod`（或整个 `mod/` 文件夹）放进实例的
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
- `--trust-native-code` 表示允许原生代码：它和普通程序一样拥有你的用户权限，没有沙箱。
- 构建成功后实例会选用一个新的 `native-<commit>-<hash>` 构建。

想自己跑 `cargo check`：先把固定构建的 SDK 放进 `mod/.horizon/sdk/`
（`horizon-mod-api = { path = ".horizon/sdk/api/crates/horizon-mod-api" }` 是唯一允许的写法）。

## 使用

1. 空手或手持任意物品（工具、方块都行），**按住潜行（Shift）**，挖掉一个方块。
2. 与它相连的**同种方块**（方块 ID 相同，忽略朝向等状态）会按设置的范围和上限一起被挖掉；
   不会跨到泥土、草方块、沙砾等其他种类。
3. 创造模式默认**不连锁**（单击只破一块）：在设置里打开“创造模式连锁”后才连锁，
   且仍要满足潜行等其余触发条件；连锁按创造规则不掉落、不耗耐久。旁观者不触发。

保护性规则（始终生效）：

- 破坏速度小于 0 的方块（基岩等）不参与连锁；
- 带方块实体的方块（箱子、熔炉等）不参与连锁；
- 黑名单中的方块不参与连锁，也不会作为连锁起点；
- 只在同种方块内部扩散：按方块 ID 比较，`dirt` 不会蔓延到 `grass_block` / `coarse_dirt`，
  `oak_log` 不会蔓延到其他原木，`sand` 不会蔓延到 `gravel`；
- 不判断手持物：空手、拿木头、拿镐走同一套范围和规则，工具类型与等级都不改变连锁结果；
  掉落仍按原版规则（例如木镐挖钻石矿会连锁但钻石矿不掉落）。

## 设置页（选项 → 模组 → VeinSweep → Config）

共 6 项，顺序如下：

| 设置 | 类型 | 默认 |
| --- | --- | --- |
| 仅潜行时触发 | 开关 | 开 |
| 最大方块数 | 滑块（8–256，步长 8） | 32 |
| 掉落直接入包 | 开关 | 关（世界散落） |
| 含对角连通 | 开关 | 开（关=仅六面相邻 6 向；开=含棱/角共点，26 向） |
| 保留工具耐久 | 滑块（0–10） | 0 |
| 创造模式连锁 | 开关 | 关 |

“保留工具耐久”表示剩余耐久低于该值时停止连锁，避免把工具用爆；只对带耐久的
工具生效，空手或拿方块/食物不会触发它。

## 黑名单

全部在游戏内完成，不需要手改文件：

```
/veinsweep blacklist add minecraft:dirt
/veinsweep blacklist remove minecraft:dirt
/veinsweep blacklist list
/veinsweep blacklist clear
```

- `add` / `remove` 的方块参数支持 Tab 补全（资源 ID）。
- 黑名单按**方块 ID**匹配，该方块的任何状态都不参与连锁。
- 保存位置：`<游戏目录>/horizon-mod-data/veinsweep/blacklist.txt`，每行一个 ID，
  重启后仍然有效；也允许多个客户端共享同一游戏目录时读取它。

## 文件

- 分发包：`publish/veinsweep-0.3.7.hmod`（31525 字节，sha256
  `a121149296549349853500b3d5f6752168b3efd0e816798d820fb736ef06ff82`）
- 设置项翻译：`assets/veinsweep/lang/zh_cn.json`、`en_us.json`
