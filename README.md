# VeinSweep —— Horizon 连锁挖掘模组

用镐挖掉一个矿石、或用斧砍掉一个原木时，把与它**相连的同类方块**一起挖掉。
掉落物、耐久、经验、统计、粒子与音效都走游戏自己的玩家挖矿流程，和手动挖一块完全一致。

默认只处理矿石和原木；石头、泥土、沙子类普通方块可以在设置页里**单独开启**，默认全部关闭，
所以升级后不会突然把地形整片挖掉。

> 想发布到 Horizon 官方模组集（让别人在启动器里直接安装）：见
> [publish/SUBMIT.md](publish/SUBMIT.md)，里面有官方包格式、注册表条目、审核流程和当前限制。

## 它是什么类型的模组

Horizon 不能用 Fabric / NeoForge 的 jar（它的游戏逻辑是 Rust，没有 Minecraft 的 Java 类）。
Horizon 真正的代码扩展机制叫 **native mod**：一个 `horizon-mixin.toml` 加若干 `.rs` 源码，
在编译前注入游戏自己的 Rust 源码，由 launcher 用自带的 Rust 工具链构建。
本项目就是这种 native mod（不是数据包，也不是 `.hmod` 内容模组）。

实现要点：

- 触发点：`Game::native_destroy_block`（玩家开始破坏方块，对应 NeoForge 的 `BlockEvent.BreakEvent`）。
- 连锁破坏：`Game::native_player_break_block`，即游戏自己的 `ServerPlayerGameMode.destroyBlock`，
  官方文档里就是为连锁挖掘 / FTB Ultimine 这类工具准备的接口。
- 该接口目前还没升级为稳定的 `Host` 接口（Horizon 的 `player_break` API 片段仍是 pending），
  所以 `horizon-mixin.toml` 固定到了开发时验证过的 Horizon 提交
  （Nightly 2026-11-13，`9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`）。Horizon 更新后重新构建即可。

## 构建与安装

构建必须在你自己的账户下执行：Horizon 的 native mod 就是“在本机把游戏重新编译一遍”，
launcher 需要能写 `%APPDATA%\horizon-launcher\native\` 和实例目录。

### 推荐：连 jade / jei 一起构建

1. 把整个 `VeinSweep` 文件夹复制到 `<实例>\horizon-native-mods\`
   （游戏里 **选项 → 模组 → Open Mod Folder** 打开的就是这个目录）。
2. 构建这个目录（目录里的每个 native mod 都会编进去）：

```powershell
& "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe" native-build "$env:APPDATA\horizon\horizon-native-mods" --trust-native-code
```

### 只构建 VeinSweep

```powershell
& "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe" native-build "e:\Project\Game\Minecraft\Horizon\VeinSweep" --trust-native-code
```

- `FOLDER` 是官方支持的用法（源码 bundle / 解包后的 `.hnmod` / 一组 mod 的文件夹）。
  构建完成后，实例之后游玩的就是新的 `native-*` 版本；它会重新编译游戏，需要几分钟。
- `--trust-native-code` 表示你确认信任这份源码：native mod 是普通程序，拥有你的用户权限，没有沙箱。
- 开发期想改代码后快速重建，可以先执行一次
  `horizon-launcher settings native-profile dev`（构建快约 6 倍，游戏逻辑约慢一倍）。


## 使用

1. 手持**镐**对着矿石，或手持**斧**对着原木；**空手**也可以对着原木这类不需要工具的方块。
   想连锁石头、泥土、沙子，先到设置页开启“基础方块”（默认关闭）。
2. **按住潜行（Shift）** 挖掉它。
3. 与它相连的同类方块会一起被挖掉，掉落物按正常挖矿散落在原地（可改为直接进背包）。
4. 创造模式也能用，但按创造模式规则不掉落、不耗耐久。

默认只对以下方块生效，避免误伤建筑：

- 矿石：`#minecraft:ores`、`#c:ores`、`#forge:ores`、`#neoforge:ores`，以及名字以 `_ore` 结尾的方块；
- 原木：`#minecraft:logs`、`#c:logs`、`#forge:logs`、`#neoforge:logs`，以及 `_log` / `_wood` / `_stem` / `_hyphae` 结尾的方块；
- 工具必须匹配：镐对 `#minecraft:mineable/pickaxe`、斧对 `#minecraft:mineable/axe`、锹对 `#minecraft:mineable/shovel`；
- 需要正确工具才有掉落的方块（如铁矿），工具等级不够时不会连锁；
- 空手只对**原版空手也能获得掉落**的方块生效（`requires_tool == false`，例如原木、菌柄），
  空手挖铁矿/石头这种本来就不掉东西的情况不会连锁；
- 可选扩展“基础方块”（默认关闭，需在设置页手动开启，一个开关覆盖三类）：
  - 石头类：`#minecraft:base_stone_overworld`、`#minecraft:base_stone_nether`，以及末地石、方解石、平滑玄武岩等；
  - 泥土类：`#minecraft:dirt`，以及草方块、灰化土、菌丝体、苔藓块、泥、泥泞的红树根、灵魂土等；
  - 沙子类：`#minecraft:sand`，以及沙砾、可疑的沙砾等。

默认只连锁矿石和原木。基础方块开启后按**类别**连通：挖草方块会把相邻的泥土、灰化土等一起带走，
而不是只沿同一种方块走；矿石和原木仍按**同名**匹配。木板、砖块等建筑方块始终不在范围内。

## 设置

游戏内：**选项 → 模组 → VeinSweep**。列表里那一行本身就是总开关（点一下变成 `VeinSweep 0.2.1: OFF`，
钩子立刻停用）；点 **Settings** 进入设置页，有 7 个按钮：

| 按钮 | 作用 | 默认 |
| --- | --- | --- |
| 触发方式 | 潜行时 / 总是 | 潜行时 |
| 最大方块数 | 8 → 16 → 32 → 64 → 128 → 256 循环 | 32 |
| 掉落方式 | 世界散落 / 直接入包 | 世界散落 |
| 连通范围 | 六面（6 向）/ 含对角（26 向） | 六面 |
| 保留工具耐久 | 0–10 循环，剩多少耐久时停止连锁 | 2 |
| 空手触发 | 允许 / 禁止 | 允许 |
| 基础方块 | 允许 / 禁止，是否连锁石头、泥土、沙子类普通方块 | 禁止 |

这些设置以单文件形式保存在
`<实例文件夹>\horizon-mods\.horizon-mod-data\veinsweep\<键名>`，删除对应文件即恢复默认。

## 行为细节

- **数量上限**：一次最多连锁“最大方块数”个方块（不含你手动挖的那块）。
- **空手**：主手为空时也能触发，但只对原版空手就能拿到掉落的方块生效（例如原木、菌柄）；
  矿石、石头等需要正确工具的方块空手不连锁。空手不消耗耐久；可在设置页把“空手触发”关掉。
- **基础方块**：石头、泥土、沙子类默认关闭。开启后仍遵守工具规则：
  石头类要镐、泥土/沙子类要锹（空手时按上面的“空手”规则）。
  连通按类别计算：泥土类会把泥土、草方块、灰化土等一起连带，石头类和沙子类同理；
  不会跨类别（挖泥土不会连到石头或沙子）。
- **耐久**：手持工具时，每块都按正常挖矿扣耐久；剩余耐久低于“保留工具耐久”时立刻停止，不会把工具用爆。
- **掉落与拾取**：默认与世界散落一致；选“直接入包”时，掉落直接进背包，放不下的从脚下掉出。
- **正常流程**：每块都经过玩家权限、冒险模式 `can_break`、出生点保护、区域保护（`native_may_build`）等检查；
  被保护或被拒绝的方块会被跳过。
- **不递归**：连锁过程中的每次破坏都会再次触发破坏钩子，模组内部用标记拦住，不会无线扩散。
- **多人**：客户端与服务端各跑各的模组；只有服务端（含单人世界、LAN 主机）的模组真正改方块，
  所以联机时服务端也要装。

## 卸载

临时停用：**选项 → 模组** 里把 `VeinSweep 0.2.1` 点成 OFF（钩子立刻不再做事）。

彻底移除：把 `<实例>\horizon-native-mods\veinsweep\` 删掉（如果装过），再用不带本模组的版本重新构建一次
`horizon-launcher native-build --trust-native-code`。因为 native mod 是编译进游戏的，只删文件不重建，
游戏里仍然带着它（只是没有源码了）。

## 实测结果

在本机（Nightly 2026-11-13 对应的提交）用 Horizon 自带的控制接口在测试世界逐项跑过：

| 场景 | 结果 |
| --- | --- |
| 钻石镐 + 潜行，挖 3×3×3 铁矿的中心一块 | 27 块全部挖掉 |
| 钻石镐 + 潜行，挖 3×3×3 石头的中心一块 | 只挖掉 1 块，石头不连锁 |
| 钻石斧挖铁矿 | 只挖掉 1 块，工具不匹配 |
| 不潜行挖铁矿 | 只挖掉 1 块 |
| 生存模式、上限设 8，挖同类矿簇 | 恰好再连锁 8 块，每块各扣 1 点耐久，raw_iron 正常掉落并被拾取 |
| 钻石斧 + 潜行，砍 3×3×3 原木 | 连锁 8 块（受上限约束） |
| 在模组列表里把 VeinSweep 关掉 | 只挖掉 1 块 |

以上是 1.0.0 工具模式的实测结果。0.2.0 的泥土连锁在游戏内复测过：
纯泥土 9×9 平面、上限 64 时，中心一块周围按六向连锁了 64 块，说明邻居枚举和 BFS 本身没问题；
“沿斜线单向扩展”出现在草方块/泥土混合的自然地表，原因是当时要求邻居方块名与起点完全相同，
草方块在斜坡上恰好连成一条线。0.2.1 把基础方块改成按类别匹配后修复。
0.2.1 已用官方 `horizon-mixins apply` + launcher 自带 Rust 工具链完整编译通过，
产物中确认是 `VeinSweep 0.2.1`；按类别连通的最终游戏内效果仍建议你实测一次。

## 常见问题

- **构建报 `E_TARGET_NOT_FOUND` / `E_HOOK_CATALOG`**：Horizon 已升级，模组固定的提交与当前游戏不一致。
  用当前实例的提交号更新 `horizon-mixin.toml` 的 `compatibility.horizon_revision`，然后重新构建。
  当前提交号可看 `%APPDATA%\horizon-launcher\native\source\` 下的文件夹名。
- **完全不触发**：确认没按着 Shift 时挖（默认潜行触发），确认手里是镐/斧（或空手对着原木），
  确认方块是矿石或原木。
- **空手为什么不连锁泥土/石头**：“基础方块”默认关闭，需要先在设置页开启；
  石头需要镐，空手本来就不掉东西，所以即使开启基础方块，空手也不会连锁石头。
- **升级后为什么还是只连锁矿石/原木**：0.2.1 的“基础方块”默认关闭，保持原有行为；
  想挖石头/泥土/沙子，到设置页把它改成“允许”。
- **只想在特定方块上生效**：目前规则写在 `veinsweep.rs` 的 `chain_scope` 和三个 `is_*_like` 里，改完重新构建。
