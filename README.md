# VeinSweep — Chain Mining

**English** | [简体中文](README.zh-CN.md)

Hold the sneak key (by default; change it in Options → Controls → VeinSweep) while you break a block, and every connected block **of the same type** breaks with it. Ore veins and logs come out in one go; dirt, stone and sand tear out as an irregular patch nearby, a different shape each time.

Chain breaking spreads only through the same block type and ignores what you are holding: bare hands, wood or a pickaxe all behave the same. Every breakable block can chain, while unbreakable blocks such as bedrock and blocks with contents such as chests and furnaces are skipped, and anything on the blacklist is ignored. The settings screen can turn chaining off at any time, change how many blocks one break may take, stop before a tool breaks, and allow chaining in creative mode. Drops always scatter in the world, as normal block breaking does.

The interface supports English, Simplified Chinese and Traditional Chinese and follows the game language (English is used when a translation is missing).

## Usage

1. With an empty hand or any item (tool or block), **hold the trigger key** (the sneak key by default, i.e. Left Shift; change it in Options → Controls → VeinSweep) and break a block.
2. Connected blocks **of the same type** (same block ID, ignoring facing and other states) break with it, following these rules, and never spread into other types such as dirt, grass blocks or gravel:
   - Ores / logs: every connected same-type block in the 3×3×3 neighbourhood, including edge- and corner-adjacent ones, in one go;
   - Ordinary blocks: a random connected patch inside a cube of radius 4, a different shape each time.

   Both are capped by the hard "max blocks" limit.
3. Creative mode does **not** chain by default (a click breaks one block): enable "Creative chaining" in the settings first, and still hold the trigger key; chaining then follows creative rules — no drops, no durability loss. Spectators never trigger it.

Protective rules (always active):

- Blocks with a destroy speed below 0 (bedrock and the like) never chain;
- Blocks with a block entity (chests, furnaces, etc.) never chain;
- Blacklisted blocks never chain and are never used as a starting point;
- Only the same block type spreads: comparing by block ID, `dirt` will not spread into `grass_block` / `coarse_dirt`, `oak_log` will not spread into other logs, `sand` will not spread into `gravel`;
- Held items are not checked: bare hands, wood or a pickaxe use the same range and rules; tool type and tier do not change the result, and drops still follow vanilla rules (a wooden pickaxe chains diamond ore but the ore does not drop).

## Settings (Options → Mods → VeinSweep → Config)

Four entries, in this order:

| Setting | Type | Default |
| --- | --- | --- |
| Chain breaking | Toggle (master switch) | On |
| Max blocks | Slider (8–256, step 8) | 32 |
| Preserve tool durability | Slider (0–10) | 0 |
| Creative chaining | Toggle | Off |

The spread radius for ordinary blocks is fixed at 4 and no longer appears in the settings (ores and logs still use the fixed 3×3×3 neighbourhood).

The trigger key is not on the settings screen but in **Options → Controls → VeinSweep → Chain trigger key**, defaulting to the sneak key (Left Shift); keyboard and mouse buttons are supported. With "Chain breaking" off, neither the trigger key nor the other settings have any effect.

"Preserve tool durability" stops chaining when the remaining durability falls below that value, so a tool does not break; it applies only to tools with durability — bare hands, blocks or food never trigger it. If the tool does break mid-chain, chaining stops at once instead of continuing bare-handed.

## Blacklist

Everything happens in game; no need to edit files by hand:

```
/veinsweep blacklist add minecraft:dirt
/veinsweep blacklist remove minecraft:dirt
/veinsweep blacklist list
/veinsweep blacklist clear
```

- The block argument of `add` / `remove` supports Tab completion (resource ID).
- The blacklist matches by **block ID**; no state of that block can chain.
- Saved to `<game directory>/horizon-mod-data/veinsweep/blacklist.txt`, one ID per line, and it survives restarts; clients that share the same game directory can read it too.

---

Development, build, packaging, verification and publishing notes are in [AGENTS.md](AGENTS.md).
