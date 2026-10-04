# Explore the World — Design

Working title (from the folder name). Claude's design doc; Alva's scratchpad is `Notes.md` (read-only for Claude, git-ignored).

Status: design agreed for the core; milestones 1 (window, tile renderer, camera), 2 (map editor), 3 (ticks and walking) and 4 (bag, skills, mining) are done.

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
| Movement | Click-to-move on a tile grid, pathfinding, one step per 0.4 s tick, eight directions with no corner cutting |
| Camera | 3/4 top-down (fronts of trees, buildings, characters visible) |
| Art | Bold flat shapes: thick dark outlines, flat colours, big readable silhouettes. Drawn in Lantern Studio 3. Palette: B Earthy (see Art) |
| World building | In-game map editor (toggle-able mode), our own map file format |
| Combat styles | Melee / ranged / magic triangle designed in from day one; melee gets content first |
| Combat depth | Auto-attack on ticks + abilities on a hotbar (cooldowns; mana for spells) |
| Magic | Mana-based, no runes |
| Healing | Potions (Alchemy). Food gives small buffs, WoW-style, not healing |
| Gathering | Quick nodes: a few ticks, loot, node respawns later. Richer and rarer nodes sit deeper in dangerous territory. A node is worked in swings, each with a chance that climbs with level; it gives a few, then is spent until it comes back |
| Tools | No pickaxe needed yet: mining is bare-handed until equipment exists (milestone 6), when pickaxes make you faster and unlock harder rocks |
| Progression | Per-skill levels trained by doing, no classes. Short and punchy: the cap is level 50, and each level should unlock something |
| Alchemy inputs | Herbalism (gathering skill) + monster parts |
| Out | Woodcutting, Firemaking, Cooking-as-healing, rune-based magic |
| Setting | Classic medieval fantasy |
| Content | Designed together: Claude proposes here, Alva approves, then it gets built |
| Death | Gravestone run-back: respawn in the last town with equipped gear; the inventory waits in a gravestone where you died. The grave is permanent (no timer), but there is only one: die again before reaching it and the old grave crumbles with its items |
| Renderer | Our own wgpu sprite-batch pass, plugged in through `AppHost::render`; Lantern UI 2 draws the HUD on top |
| Code layout | Cargo workspace: a pure-std headless simulation crate (ticks, world, pathfinding, combat, skills) + the game binary (rendering, UI, editor) |
| Screen | The world fills the window; the HUD floats over it (hotbar along the bottom, vitals in a corner, big panels open on demand) |
| Travel | One brisk speed, no stamina or run energy. Faster travel (mounts, teleports) can be unlocked later as a reward |
| Inventory | 30 slots, 5 across and 6 down (taller than wide); materials stack, gear does not. Bank in towns |
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

## Content so far (proposed numbers, all tunable)

Built so the systems can be played; Alva has not signed these numbers off.

**Levels.** Experience to be level L, with n = L - 1: `20 n + 8 n^2.5`.

| Level | 2 | 5 | 10 | 20 | 30 | 40 | 50 |
|---|---|---|---|---|---|---|---|
| Experience in all | 28 | 336 | 2,124 | 12,942 | 36,776 | 76,708 | 135,436 |

Every skill starts at level 1.

**Items.** Copper ore (stacks).

**Nodes.**

| Node | Skill | Level | Gives | Holds | A swing | Chance | Experience | Comes back |
|---|---|---|---|---|---|---|---|---|
| Copper rock | Mining | 1 | Copper ore | 3 | 3 ticks (1.2 s) | 55%, +3% a level, at most 95% | 12 each | 75 ticks (30 s) |

So the first rock is level 2, and a level-1 miner empties a rock in about 6.5 s on average.

## System sketches (proposals)

- **Tick.** One constant for tick length: 0.4 s, chosen by Alva over RuneScape's 0.6 s. The simulation only advances on ticks; rendering interpolates between them so movement looks smooth.
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

Milestones 1 to 4, as of 2026-10-04.

- **Layout.** A Cargo workspace: `crates/sim` (pure std: terrain, the chunked world, props and markers, pathfinding, the ticking player, the edit tools and their history, map files, a test map) and `crates/game` (the binary `explore-the-world`: camera, sprite sheet packer, scene builder, the wgpu passes, the map editor and its panel, the Lantern UI 2 host). Art sources in `art/`, what the game loads in `assets/`.
- **Window.** Lantern UI 2 by path, unmodified. One area with no header and no title bar; the world fills it and the interface will draw over it.
- **World pass.** The world is drawn into a picture of its own that holds the art's sRGB numbers and blends them as they are, the way Lantern Studio composites, then that picture is put on the window. Blending in linear light instead made shadows visibly paler than the approved art.
- **Ground.** Drawn on the dual grid: one quad per square between four tile centres. Kinds stack grass < dirt < sand < water; a layer covers its own tiles and those of every kind above it, so no gap opens where two meet. The shader shapes each quad from which corners it covers, with corners rounded to half a tile and a rim just inside the edge. There is no transition art.
- **Sprites.** Listed in `assets/sprites.toml` (file, rect, anchor). Art is 200 px per tile, twice the style sheet, so it stays sharp up to full zoom. Packed at startup into one sheet with smaller copies for zooming out; drawn back to front by where they stand.
- **Decoration.** Grass tufts are scattered by a hash of the tile, on about one bare grass tile in five. They are not world data.
- **The map.** `assets/maps/overworld/` is the real map: the game loads it and the editor saves to it. It started as `sim::testmap`, a made-up 48 x 32 map, written out once (Alva's choice); that generator stays for the tests.
- **Map files.** One compact binary file per 32 x 32 chunk (Alva's choice), named for the chunk (`-1_0.chunk`), about 500 bytes each for the test map. Versioned, in tagged sections so layers can be added. Ground and props are stored by name, not number, so kinds can be added or reordered without breaking saved maps; an unknown name is an error, never a silent hole. Only changed chunks are written, each whole and then moved into place; a chunk erased to nothing loses its file.
- **Map editor.** F2 docks a panel on the left (a real Lantern UI 2 area, so its edge drags): undo, redo, save; the tools (brush, eraser, fill, place, remove, move, start); the brush's width, 1 to 9 tiles; the grounds; the props with their pictures. The left button works the held tool on the world, with the tiles it would touch lit under the pointer. A whole stroke is one undo step, 500 steps deep. Quitting with unsaved changes asks first.
- **Ticks and walking.** A tick is 0.4 s and a step takes one tick (Alva's choice). Steps go eight ways; a diagonal one needs both tiles beside it free, so no corner is cut. Water, the void, trees and rocks cannot be stood on; herbs can. A click finds the shortest way (A*, diagonal first then straight) and the player follows it a tile a tick, finding a new way if something gets in it. Clicking where they cannot stand walks them to the tile right beside it. The simulation knows only tiles and ticks; the game slides the sprite between tiles over the tick.
- **The player's sprite.** `assets/sprites/player-01.png` (source in `art/sprites/`): three drawn facings (down, up, side; left is the side mirrored), each with an idle pose and a walk of stride, pass, other stride. A tile's step is a stride then a pass, and strides take turns. A diagonal step shows the side view. The manifest's `[grids.name]` table cuts such a sheet into sprites.
- **The view while playing.** Always on the player, gliding after them; the wheel still zooms. Nothing redraws while the player and the view are at rest.
- **Player start.** A marker in the map (the editor's Start tool; a see-through player shows it while editing), saved in its chunk's file. With none, the player starts on the nearest standable tile to the map's middle.
- **Skills, bag, nodes.** In `sim`: nine skills with their own experience (cap 50), a 30-slot bag (drawn 5 across, 6 down) where materials stack, and props that are nodes. Clicking a node walks the player beside it and sets them to work: a swing every few ticks, a seeded dice roll for each, experience and an item on a hit, and the node spent after it has given what it holds until its time is up. Walking off drops the work; a full bag stops it. The simulation reports what happened as events and the game decides how to show them.
- **The interface while playing** (`hud.rs`): Skills and Bag buttons in the bottom-right corner (also K and I), panels that stand on them, what was just got floating up from the player, and what the game has to say (a level gained, a full bag) in big letters at the top. Lantern UI 2's own toasts are not used while playing: they land on the buttons.
- **More drawings.** The player's sheet gained a wind-up and a strike for each facing; a spent rock is the rock without its copper; items have 128 px icons in `assets/items/` (sources in `art/items/`).
- **Controls for now.** Click to walk, or to work a rock. Wheel zooms about the pointer, F11 fullscreen, Ctrl+Q quits. F2 opens and closes the editor; in it, middle-drag or arrows / WASD move the view, Ctrl+Z undoes, Ctrl+Shift+Z or Ctrl+Y redoes, Ctrl+S saves.
- **How it is checked.** 201 tests. The render tests draw the test map with no window and compare pixels with the palette and with colours read off the style sheet (ground colours, rims, the void, a tree's trunk, canopy and shadow, no gaps between ground quads, the window showing the same numbers). `EXPLORE_RENDER_OUT=<folder> cargo test -p game` keeps those pictures as PNGs. The editor is driven end to end with Lantern UI 2's headless harness: pretend keys and clicks open the panel, pick rows, paint, undo, save, and quit through the dialog, against a scratch copy of the map. Whole frames (the world pass and the interface over it, put together as `lntrn-app` does) are drawn with no window too, so the panels and the bag can be looked at and their pixels checked.

### Calls Claude made, Alva's to overrule

1. Ground edges are rounded by the shader rather than hand-drawn transition tiles. The tile grid shows as rounded steps on diagonals.
2. The stacking order grass < dirt < sand < water.
3. Diagonal neighbours of the same ground join through the corner, so a diagonal path is one path.
4. Dirt and water have rims (from the style sheet); sand has none; grass has a dark green one only at the edge of the world.
5. Where there is no ground it is the outline colour, `#271a1c`.
6. Props stand with their anchor on the middle of their tile.
7. A tile is 100 logical pixels at zoom 1; zoom runs 0.4 to 2.
8. No title bar.
9. Painting ground leaves what stands on it; erasing takes the ground and what stood there.
10. The fill floods tiles joined by their sides, and does nothing where there is no ground.
11. Placing a prop on a tile that has one replaces it.
12. Picking a ground takes up the brush (or keeps the fill if that is held); picking a prop takes up the place tool.
13. While editing, the view may leave the map by half a screen, to paint new ground past its edge.
14. The move tool carries the prop under the pointer tile by tile: it follows onto any tile it can stand on (ground, nothing there already) and waits on its last good tile over those it cannot. It never replaces another prop.
15. The player is drawn without the sword: weapons come back as equipment drawn over the body, not baked into every frame.
16. The first step after standing still starts at once instead of waiting for the next tick to come round. Nothing else ticks yet; when monsters do, whether a click waits for the tick is a feel question to revisit.
17. The game stands still while the editor is open. Closing it stops the player and moves them off any ground that can no longer be stood on.
18. While playing the view cannot be moved off the player.
19. Where the player is is not saved yet: every launch starts at the start.
20. Nothing the player earns is saved yet: the bag and the skills start empty every launch, like their position.
21. A node can be worked from any of the eight tiles around it.
22. At work the player winds up and strikes every tick, though the roll comes only when a swing is done.
23. The game says things itself, at the top of the screen, rather than through Lantern UI 2's toasts.
24. The dice are seeded from the clock when the game starts.
25. Props are still the four placeholder kinds; the rock is a copper rock. They become data-defined objects when they start to do things (mining, picking).

## Open questions

- Bank size
- The Skills panel's look: Alva does not like Lantern UI 2's stock progress bars there. To be redrawn as our own rows (it is one function in `hud.rs`); what they should look like is not decided
- The XP curve's numbers (proposed above), and what each level unlocks
- Quests and NPC dialogue: when, and how deep
- Where assets live for a deployed binary (it finds them beside itself, else in the source tree)
- Ground edges: keep the shader's rounded steps, or hand-drawn transitions?
- Save system
- Audio
- Real name for the game

## Milestones (proposal)

0. Art style test sheet (terrain, ore rock, herb, player, goblin) to lock the look, tile size, and palette. **Done 2026-10-04**: style approved, palette B Earthy
1. Window, tile renderer, camera. **Done 2026-10-04**: Alva ran it ("It looks beautiful!")
2. Map editor: paint, place, save, load. **Done 2026-10-04**: Alva used it ("that's so cool!"), asked for a move tool, and that "works great"
3. Tick simulation, click-to-move pathfinding. **Done 2026-10-04** with full walk cycles: Alva "spent so much time just walking around"
4. Inventory, skills and XP, Mining nodes. **Done 2026-10-04**: Alva played it ("it works!"), asked for the bag to be 5 across and 6 down, and does not like the stock progress bars in the Skills panel (to be redrawn later)
5. Combat: melee auto-attack, abilities, monsters, drops, death
6. Smithing and equipment
7. Herbalism, Alchemy, potions
8. NPCs, shops, bank, dialogue
