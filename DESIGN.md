# Explore the World — Design

Working title (from the folder name). Claude's design doc; Alva's scratchpad is `Notes.md` (read-only for Claude, git-ignored).

Status: design agreed for the core; milestone 1 (window, tile renderer, camera) is built.

## Pitch

A single-player, 2D top-down RPG in a handcrafted open world. RuneScape-inspired (ticks, click-to-move, skills you train by doing) but not a clone: no chore skills, active abilities in combat, potions instead of food, mana instead of runes. Exploring further is how you find better ore, rarer herbs, and nastier monsters.

## Pillars

1. **Exploration is the game.** The world is handcrafted. The reward for going over the next hill is better nodes, new ingredients, and tougher fights.
2. **Tick-based and click-driven.** Everything in the simulation happens on a fixed game tick.
3. **You become what you do.** No classes. Every skill has its own level and rises when you use it.
4. **Inspired, not copied.** If a RuneScape system is a chore (woodcutting, firemaking), it does not come along.

## Decisions

Locked with Alva on 2026-10-04.

| Area | Decision |
|---|---|
| Players | Single player only |
| Stack | Rust + wgpu + Lantern UI 2 (path dependency, not modified without asking) |
| Movement | Click-to-move on a tile grid, pathfinding, one step per tick |
| Camera | 3/4 top-down (fronts of trees, buildings, characters visible) |
| Art | Bold flat shapes: thick dark outlines, flat colours, big readable silhouettes. Drawn in Lantern Studio 3. Palette: B Earthy (see Art) |
| World building | In-game map editor (toggle-able mode), our own map file format |
| Combat styles | Melee / ranged / magic triangle designed in from day one; melee gets content first |
| Combat depth | Auto-attack on ticks + abilities on a hotbar (cooldowns; mana for spells) |
| Magic | Mana-based, no runes |
| Healing | Potions (Alchemy). Food gives small buffs, WoW-style, not healing |
| Gathering | Quick nodes: a few ticks, loot, node respawns later. Richer and rarer nodes sit deeper in dangerous territory |
| Progression | Per-skill levels trained by doing, no classes |
| Alchemy inputs | Herbalism (gathering skill) + monster parts |
| Out | Woodcutting, Firemaking, Cooking-as-healing, rune-based magic |
| Setting | Classic medieval fantasy |
| Content | Designed together: Claude proposes here, Alva approves, then it gets built |
| Death | Gravestone run-back: respawn in the last town with equipped gear; the inventory waits in a gravestone where you died. The grave is permanent (no timer), but there is only one: die again before reaching it and the old grave crumbles with its items |
| Renderer | Our own wgpu sprite-batch pass, plugged in through `AppHost::render`; Lantern UI 2 draws the HUD on top |
| Code layout | Cargo workspace: a pure-std headless simulation crate (ticks, world, pathfinding, combat, skills) + the game binary (rendering, UI, editor) |
| Screen | The world fills the window; the HUD floats over it (hotbar along the bottom, vitals in a corner, big panels open on demand) |
| Travel | One brisk speed, no stamina or run energy. Faster travel (mounts, teleports) can be unlocked later as a reward |
| Inventory | Fixed grid of slots; materials stack, gear does not. Bank in towns |
| World layout | One seamless overworld streamed in chunks; dungeons, caves, and building interiors are separate maps entered through doors |

## Skills (approved 2026-10-04)

| Group | Skill | Trained by |
|---|---|---|
| Combat | Melee | Dealing melee damage |
| Combat | Ranged | Dealing ranged damage |
| Combat | Magic | Casting spells |
| Combat | Defence | Taking / blocking hits |
| Combat | Vitality | All combat (max health) |
| Gathering | Mining | Ore and stone nodes |
| Gathering | Herbalism | Herb nodes |
| Crafting | Smithing | Smelting bars, forging weapons and armour |
| Crafting | Alchemy | Brewing potions |

Later candidates: Cooking (buff food), Fishing, Crafting (leather / jewellery), something exploration-flavoured (Cartography? Lockpicking?).

## System sketches (proposals)

- **Tick.** One constant for tick length (start at RuneScape's 0.6 s, tune by feel once it is playable). The simulation only advances on ticks; rendering interpolates between them so movement looks smooth.
- **World.** Tile grid stored in chunks. Layers: terrain, objects (rocks, herbs, buildings, scenery), collision, spawns. Trees exist as scenery, not as a skill.
- **Interaction.** Click a thing: path to a tile next to it, then act (mine, pick, attack, talk).
- **Combat.** Each weapon has an attack speed in ticks. Accuracy roll, then damage roll. Abilities have cooldowns in ticks; spells also cost mana. Triangle: melee beats ranged, ranged beats magic, magic beats melee.
- **Recovery.** Potions heal and restore mana. Food grants a small timed buff. Slow regen out of combat.
- **Editor.** Paint terrain, place objects, nodes, monster spawns, and NPCs; save and load map files.

## Art

Style test 01: `art/concepts/style-test-01.png` (editable source `style-test-01.lstudio` next to it). **Approved by Alva 2026-10-04: the style is right, and the palette is B Earthy.**

The style, as drawn in the test:

- 100 px per tile at authoring size; each scene is 6 x 6 tiles. Player is about 1.3 tiles tall, goblin about 1.1. (Tile size and on-screen zoom still to be confirmed in the running game.)
- One outline colour: 5 px on main silhouettes, 3 to 4 px on details.
- Two flat tones per object (base + one highlight), no gradients.
- Terrain has no dark outline; edges get a slightly darker rim instead (path, pond).
- Drop shadows are flat translucent ellipses.

### Palette: B Earthy

Muted, warm, old-school. In the test sheet B is the A panel pushed through two adjustment layers (saturation -35, then temperature +20 / brightness -4). The values below are sampled from the result and are the real palette from here on: new art is drawn with these directly, not with adjustment layers.

| Use | Colour | | Use | Colour |
|---|---|---|---|---|
| Outline | `#271a1c` | | Skin | `#e0bd97` |
| Grass | `#73a14c` | | Hair (brown) | `#68411f` |
| Grass, dark patch | `#699645` | | Tunic blue | `#5572ac` |
| Grass, light patch | `#80ad5a` | | Tunic blue, highlight | `#718cbd` |
| Grass tuft | `#4f7e35` | | Tunic blue, shade | `#4b6399` |
| Dirt path | `#bd9d6b` | | Leather belt | `#5d4023` |
| Dirt rim / mound | `#947340` | | Boots | `#322213` |
| Pebble | `#a58558` | | Gold | `#c19b4a` |
| Sand | `#ccb98a` | | Steel | `#dedad6` |
| Water | `#5e96b8` | | Goblin skin | `#7ba047` |
| Water rim | `#457295` | | Goblin eye | `#dcc158` |
| Water highlight | `#abc8d5` | | Wood / rags | `#785230` |
| Foliage | `#4b7f39` | | Wood, light (club) | `#866039` |
| Foliage highlight | `#669b4d` | | Stone | `#8a8685` |
| Herb leaf | `#3d7442` | | Stone, light | `#b2aeac` |
| Flower, purple | `#b768b7` | | Gravestone | `#9a9795` |
| Flower, cyan | `#61b1b4` | | Copper ore | `#c78246` / `#d49c63` |

## How it is built

Milestone 1, as of 2026-10-04.

- **Layout.** A Cargo workspace: `crates/sim` (pure std: terrain, the chunked world, props, a test map) and `crates/game` (the binary `explore-the-world`: camera, sprite sheet packer, scene builder, the wgpu passes, the Lantern UI 2 host). Art sources in `art/`, what the game loads in `assets/`.
- **Window.** Lantern UI 2 by path, unmodified. One area with no header and no title bar; the world fills it and the interface will draw over it.
- **World pass.** The world is drawn into a picture of its own that holds the art's sRGB numbers and blends them as they are, the way Lantern Studio composites, then that picture is put on the window. Blending in linear light instead made shadows visibly paler than the approved art.
- **Ground.** Drawn on the dual grid: one quad per square between four tile centres. Kinds stack grass < dirt < sand < water; a layer covers its own tiles and those of every kind above it, so no gap opens where two meet. The shader shapes each quad from which corners it covers, with corners rounded to half a tile and a rim just inside the edge. There is no transition art.
- **Sprites.** Listed in `assets/sprites.toml` (file, rect, anchor). Art is 200 px per tile, twice the style sheet, so it stays sharp up to full zoom. Packed at startup into one sheet with smaller copies for zooming out; drawn back to front by where they stand.
- **Decoration.** Grass tufts are scattered by a hash of the tile, on about one bare grass tile in five. They are not world data.
- **Test map.** `sim::testmap` is a made-up 48 x 32 map to look at until the editor exists. Not content.
- **Controls for now.** Wheel zooms about the pointer, middle-drag or arrows / WASD move the view, F11 fullscreen, Ctrl+Q quits.
- **How it is checked.** 58 tests. The render tests draw the test map with no window and compare pixels with the palette and with colours read off the style sheet (ground colours, rims, the void, a tree's trunk, canopy and shadow, no gaps between ground quads, the window showing the same numbers). `EXPLORE_RENDER_OUT=<folder> cargo test -p game` keeps those pictures as PNGs.

### Calls Claude made, Alva's to overrule

1. Ground edges are rounded by the shader rather than hand-drawn transition tiles. The tile grid shows as rounded steps on diagonals.
2. The stacking order grass < dirt < sand < water.
3. Diagonal neighbours of the same ground join through the corner, so a diagonal path is one path.
4. Dirt and water have rims (from the style sheet); sand has none; grass has a dark green one only at the edge of the world.
5. Where there is no ground it is the outline colour, `#271a1c`.
6. Props stand with their anchor on the middle of their tile.
7. A tile is 100 logical pixels at zoom 1; zoom runs 0.4 to 2.
8. No title bar.

## Open questions

- Inventory size, bank size
- Travel speed in tiles per tick
- Level cap and XP curve
- Quests and NPC dialogue: when, and how deep
- Tile size on screen and zoom: built as 100 logical px per tile, zoom 0.4 to 2; right once Alva has seen it?
- Where assets live for a deployed binary (it finds them beside itself, else in the source tree)
- Ground edges: keep the shader's rounded steps, or hand-drawn transitions?
- Save system
- Audio
- Real name for the game

## Milestones (proposal)

0. Art style test sheet (terrain, ore rock, herb, player, goblin) to lock the look, tile size, and palette. **Done 2026-10-04**: style approved, palette B Earthy
1. Window, tile renderer, camera. **Built 2026-10-04**, awaiting Alva's look
2. Map editor: paint, place, save, load
3. Tick simulation, click-to-move pathfinding
4. Inventory, skills and XP, Mining nodes
5. Combat: melee auto-attack, abilities, monsters, drops, death
6. Smithing and equipment
7. Herbalism, Alchemy, potions
8. NPCs, shops, bank, dialogue
