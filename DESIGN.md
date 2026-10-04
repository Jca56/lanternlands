# Lanternlands — Design

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
| Combat depth | Auto-attack on ticks + abilities on a hotbar (cooldowns; mana for spells). Built in that order: fighting first, abilities right after |
| Abilities | Extra, on top of auto-attacks: an ability fires on the player's next tick and only its own cooldown limits it (no shared cooldown). The first four are melee: Heavy Strike, Flurry, Kick, Guard. All of them are there from the start for now; how they are unlocked is still to design |
| Hotbar | Six big buttons along the bottom middle, worked with the keys 1 to 6 or a press |
| Monsters | They attack on sight: get near and they come for you. They give up when drawn too far from home |
| Recovery | Health comes back slowly always, and quickly out of combat. Potions will be for the middle of a fight |
| Drops | Monster parts only (Alchemy ingredients), straight into the bag. No coins or gear from monsters yet |
| Magic | Mana-based, no runes |
| Healing | Potions (Alchemy). Food gives small buffs, WoW-style, not healing |
| Gathering | Quick nodes: a few ticks, loot, node respawns later. Richer and rarer nodes sit deeper in dangerous territory. A node is worked in swings, each with a chance that climbs with level; it gives a few, then is spent until it comes back |
| Tools | Mining is bare-handed to begin with; a pickaxe makes you faster and later rocks will need one. A pickaxe carried in the bag is enough: the best one you have is used, with no swapping it into your hand |
| Smithing | At stations on the map, placed with the editor: ore is smelted into bars at a furnace, bars are hammered into gear at an anvil. Next to each thing that can be made are buttons for 1, 5 and All |
| Metals | Bronze first: a bronze bar takes one copper ore and one tin ore, so there is a tin rock too. Iron and beyond are later content on the same system |
| Equipment | The full set of slots from the start: weapon, shield, head, body, legs, hands, feet |
| Worn gear | Everything shows on the character, in every pose and facing: sword in hand, shield, helmet, body and leg armour, gauntlets, boots |
| Progression | Per-skill levels trained by doing, no classes. Short and punchy: the cap is level 50, and each level should unlock something |
| Alchemy inputs | Herbalism (gathering skill) + monster parts |
| Brewing | At a cauldron on the map, placed with the editor like the furnace and anvil, with the same panel and its 1, 5 and All |
| Drinking | A press on a potion in the bag drinks it. Potions are not on the hotbar |
| Potion limits | Both: a potion takes a bag slot each (they do not stack), and after drinking one there is a short wait before the next of its kind |
| First potions | Healing, Strength, Stoneskin, Regeneration |
| Out | Woodcutting, Firemaking, Cooking-as-healing, rune-based magic |
| Setting | Classic medieval fantasy |
| Name | **Lanternlands** (Alva, 2026-10-04). Lantern-themed because a lantern mechanic is wanted in the game eventually. Before that the working title was "Explore the World", which the project folder may still be called. The repository is `github.com/Jca56/lanternlands` |
| Content | Designed together: Claude proposes here, Alva approves, then it gets built |
| Death | Gravestone run-back: respawn in the last town with equipped gear; the inventory waits in a gravestone where you died. The grave is permanent (no timer), but there is only one: die again before reaching it and the old grave crumbles with its items |
| Renderer | Our own wgpu sprite-batch pass, plugged in through `AppHost::render`; Lantern UI 2 draws the HUD on top |
| Code layout | Cargo workspace: a pure-std headless simulation crate (ticks, world, pathfinding, combat, skills) + the game binary (rendering, UI, editor) |
| Screen | The world fills the window; the HUD floats over it (hotbar along the bottom, vitals in a corner, big panels open on demand) |
| Saving | Always, by itself: there is no save button. What is earned is saved the moment it is got, the rest every ten seconds while anything is happening, and again on leaving |
| Characters | Three slots, each a named character, picked on a title screen. Saves live in `~/.lantern/save/lanternlands/` (a new Lantern convention Alva agreed to) |
| Title screen | A painting drawn in Lantern Studio behind the menu; the game's name is written over it as text, since the name is not final |
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

**Items.** Materials, which stack: copper ore, tin ore, bronze bar, goblin tooth, purple herb, cyan herb. One to a slot: the bronze pickaxe, sword, shield, helmet, chestplate, greaves, gauntlets and boots, and every potion.

**Nodes.**

| Node | Skill | Level | Gives | Holds | A swing | Chance | Experience | Comes back |
|---|---|---|---|---|---|---|---|---|
| Copper rock | Mining | 1 | Copper ore | 3 | 3 ticks (1.2 s) | 55%, +3% a level, at most 95% | 12 each | 75 ticks (30 s) |
| Tin rock | Mining | 1 | Tin ore | 3 | 3 ticks (1.2 s) | 55%, +3% a level, at most 95% | 12 each | 75 ticks (30 s) |
| Purple herb | Herbalism | 1 | Purple herb | 2 | 2 ticks (0.8 s) | always | 10 each | 75 ticks (30 s) |
| Cyan herb | Herbalism | 3 | Cyan herb | 2 | 2 ticks (0.8 s) | always | 14 each | 75 ticks (30 s) |

So the first rock is level 2, and a level-1 miner empties a rock in about 6.5 s on average. A bronze pickaxe in the bag takes a tick off every swing (2 ticks, 0.8 s).

**Smithing.** All of it trains Smithing.

| Made | At | From | Level | Experience | Takes |
|---|---|---|---|---|---|
| Bronze bar | Furnace | 1 copper ore + 1 tin ore | 1 | 10 | 3 ticks (1.2 s) |
| Bronze pickaxe | Anvil | 2 bars | 1 | 30 | 4 ticks (1.6 s) |
| Bronze sword | Anvil | 2 bars | 2 | 30 | 4 ticks |
| Bronze boots | Anvil | 1 bar | 2 | 15 | 4 ticks |
| Bronze gauntlets | Anvil | 1 bar | 3 | 15 | 4 ticks |
| Bronze helmet | Anvil | 2 bars | 3 | 30 | 4 ticks |
| Bronze shield | Anvil | 3 bars | 4 | 45 | 4 ticks |
| Bronze greaves | Anvil | 3 bars | 4 | 45 | 4 ticks |
| Bronze chestplate | Anvil | 5 bars | 5 | 75 | 4 ticks |

**Alchemy.** Brewed at a cauldron, 3 ticks (1.2 s) each; all of it trains Alchemy.

| Potion | From | Level | Experience | What it does |
|---|---|---|---|---|
| Healing | 1 purple herb + 1 goblin tooth | 1 | 20 | Gives back 15 health at once |
| Strength | 1 cyan herb + 1 goblin tooth | 2 | 25 | +2 to the most a blow does, for 150 ticks (60 s) |
| Stoneskin | 1 purple herb + 1 cyan herb | 3 | 30 | +4 armour, for 150 ticks (60 s) |
| Regeneration | 1 cyan herb + 2 goblin teeth | 4 | 35 | 1 health every 2 ticks for 75 ticks (30 s), in a fight or out: 37 in all |

After drinking one, the next of its kind waits 15 ticks (6 s). A healing potion is not drunk at full health.

**Gear.** What each piece adds while it is worn. Accuracy counts as Melee levels when the player strikes, armour as Defence levels when they are struck at (4% a level either way); damage is added to the most a blow does.

| Piece | Slot | Accuracy | Damage | Armour |
|---|---|---|---|---|
| Bronze sword | Weapon | +2 | +2 | |
| Bronze shield | Shield | | | +2 |
| Bronze helmet | Head | | | +1 |
| Bronze chestplate | Body | | | +3 |
| Bronze greaves | Legs | | | +2 |
| Bronze gauntlets | Hands | | | +1 |
| Bronze boots | Feet | | | +1 |

So the whole set is +10 armour: a goblin, which lands 6 blows in 10 on a bare level-1 character, lands 2 in 10 on one in full bronze. With the sword, blows do 1 to 4 instead of 1 to 2 and land 68% of the time instead of 60%. Over 200 fights each against one goblin, the armoured character took less than a fifth of the damage and won in a little over half the time.

Forging is 15 experience a bar. The levels are set so that making each piece once, in this order, smelting its bars first, is always enough experience for the next: the whole set is 19 bars (19 copper and 19 tin ore) and ends at Smithing 5, with no filler to make.

**Monsters.**

| Monster | Level | Health | Attack | Defence | Most a blow does | A blow every | Sees you at | Gives up at | Wanders | Leaves | Comes back |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Goblin | 2 | 12 | 1 | 1 | 2 | 4 ticks (1.6 s) | 4 tiles | 8 tiles from home | 3 tiles from home | Goblin tooth, 1 time in 2 | 60 ticks (24 s) |

**Fighting.**

- The player strikes every 3 ticks (1.2 s), bare-handed, from any of the eight tiles around.
- A blow lands 60% of the time, 4% more or less for each level the attacker's Melee (a monster's Attack) is over or under the defender's Defence; never under 10% or over 95%.
- A blow that lands does 1 up to the most it can, each as likely. The player's most is 2, plus 1 for every 3 Melee levels.
- Health is 20, plus 6 for every Vitality level past the first.
- Experience: 4 Melee and 1 Vitality for each point of damage done; 4 Defence for each blow that misses the player.
- A fight lasts until 12 ticks (4.8 s) after the last blow given or taken. In one, health comes back 1 every 8 ticks; out of one, a twentieth of the whole every tick (empty to full in 8 s).

Measured over 4,000 fights each: a new character against one goblin always wins, in about 15 s, down about 5 of their 20 health at the worst moment (17 at the very worst). Against two goblins at once they win about 7 times in 10.

**Abilities.** All melee, bare-handed, usable from level 1.

| Ability | What it does | Aimed | Cooldown |
|---|---|---|---|
| Heavy Strike | One blow that always lands, for the most a plain blow does up to twice that (2 to 4 at Melee 1) | at a monster | 20 ticks (8 s) |
| Flurry | Three blows on three ticks, each as likely to land as a plain one, for 1 up to half a plain blow's most | at a monster | 30 ticks (12 s) |
| Kick | The monster can neither move nor strike for 6 ticks (2.4 s). No damage; 4 Melee experience | at a monster | 38 ticks (15.2 s) |
| Guard | For 10 ticks (4 s) blows against the player do half, rounded down, so a blow of 1 is stopped. 4 Defence experience for each blow met | nobody | 50 ticks (20 s) |

Experience for the damage abilities do is the same as for plain blows.

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

Milestones 1 to 5b (fighting, abilities) and the save game, as of 2026-10-04.

- **Layout.** A Cargo workspace: `crates/sim` (pure std: terrain, the chunked world, props and markers, pathfinding, the ticking player, the edit tools and their history, map files, a test map) and `crates/game` (the binary `lanternlands`: camera, sprite sheet packer, scene builder, the wgpu passes, the map editor and its panel, the Lantern UI 2 host). Art sources in `art/`, what the game loads in `assets/`.
- **Window.** Lantern UI 2 by path, unmodified. One area with no header and no title bar; the world fills it and the interface will draw over it.
- **World pass.** The world is drawn into a picture of its own that holds the art's sRGB numbers and blends them as they are, the way Lantern Studio composites, then that picture is put on the window. Blending in linear light instead made shadows visibly paler than the approved art.
- **Ground.** Drawn on the dual grid: one quad per square between four tile centres. Kinds stack grass < dirt < sand < water; a layer covers its own tiles and those of every kind above it, so no gap opens where two meet. The shader shapes each quad from which corners it covers, with corners rounded to half a tile and a rim just inside the edge. There is no transition art.
- **Sprites.** Listed in `assets/sprites.toml` (file, rect, anchor). Art is 200 px per tile, twice the style sheet, so it stays sharp up to full zoom. Packed at startup into one sheet with smaller copies for zooming out; drawn back to front by where they stand.
- **Decoration.** Grass tufts are scattered by a hash of the tile, on about one bare grass tile in five. They are not world data.
- **The map.** `assets/maps/overworld/` is the real map: the game loads it and the editor saves to it. It started as `sim::testmap`, a made-up 48 x 32 map, written out once (Alva's choice); that generator stays for the tests.
- **Map files.** One compact binary file per 32 x 32 chunk (Alva's choice), named for the chunk (`-1_0.chunk`), about 500 bytes each for the test map. Versioned, in tagged sections so layers can be added. Ground and props are stored by name, not number, so kinds can be added or reordered without breaking saved maps; an unknown name is an error, never a silent hole. Only changed chunks are written, each whole and then moved into place; a chunk erased to nothing loses its file.
- **Map editor.** F2 docks a panel on the left (a real Lantern UI 2 area, so its edge drags): undo, redo, save; the tools (brush, eraser, fill, place, remove, move, start, spawn); the brush's width, 1 to 9 tiles; the grounds; the props and the monsters with their pictures. Remove takes a prop first, then a monster's home; a see-through monster stands on each home while editing. The left button works the held tool on the world, with the tiles it would touch lit under the pointer. A whole stroke is one undo step, 500 steps deep. Quitting with unsaved changes asks first.
- **Ticks and walking.** A tick is 0.4 s and a step takes one tick (Alva's choice). Steps go eight ways; a diagonal one needs both tiles beside it free, so no corner is cut. Water, the void, trees and rocks cannot be stood on; herbs can. A click finds the shortest way (A*, diagonal first then straight) and the player follows it a tile a tick, finding a new way if something gets in it. Clicking where they cannot stand walks them to the tile right beside it. The simulation knows only tiles and ticks; the game slides the sprite between tiles over the tick.
- **The player's sprite.** `assets/sprites/player-01.png` (source in `art/sprites/`): three drawn facings (down, up, side; left is the side mirrored), each with an idle pose and a walk of stride, pass, other stride. A tile's step is a stride then a pass, and strides take turns. A diagonal step shows the side view. The manifest's `[grids.name]` table cuts such a sheet into sprites.
- **The view while playing.** Always on the player, gliding after them; the wheel still zooms. Nothing redraws while the player and the view are at rest.
- **Player start.** A marker in the map (the editor's Start tool; a see-through player shows it while editing), saved in its chunk's file. With none, the player starts on the nearest standable tile to the map's middle.
- **Skills, bag, nodes.** In `sim`: nine skills with their own experience (cap 50), a 30-slot bag (drawn 5 across, 6 down) where materials stack, and props that are nodes. Clicking a node walks the player beside it and sets them to work: a swing every few ticks, a seeded dice roll for each, experience and an item on a hit, and the node spent after it has given what it holds until its time is up. Walking off drops the work; a full bag stops it. The simulation reports what happened as events and the game decides how to show them.
- **The interface while playing** (`hud.rs`): Skills and Bag buttons in the bottom-right corner (also K and I), panels that stand on them, the items just got floating up from the player, and what the game has to say (a level gained, a full bag) in big letters at the top. Lantern UI 2's own toasts are not used while playing: they land on the buttons.
- **Monsters and fighting** (`sim::monsters`, `sim::combat`). The map says where each monster lives (the editor's Spawn tool; a fourth section in the chunk file). A monster wanders near its home, comes for a player it can see, strikes when beside them, gives up and mends when the player is too far from its home, and comes back some time after it falls. Clicking a monster walks the player to it and sets them at it from the tile beside it, following if it moves; a player set upon while doing nothing fights back by themselves. What a monster leaves goes straight into the bag. Dying leaves the bag in a grave where the player fell (kept in the save), wakes them whole at the start, and crumbles any older grave; clicking the gravestone walks back and takes what fits.
- **Two clocks.** The world ticks steadily (monsters, health, rocks coming back) whenever there is anything for it to do. The player's own tick starts the moment they click, so a click never waits. Both are 0.4 s.
- **Fighting on screen.** The player's health is a bar in the top-left corner with its numbers, and (Alva's ask) a bar over their own head while they are hurt or in a fight. A monster that is hurt or after the player has a bar over its head. What a blow did rises over whoever it was aimed at, in big letters: a white number or a grey "miss" over a monster the player struck at, a red "-N" or a grey "miss" over the player when a monster struck at them. The monster the player is set on has a red ring on the ground under it (`assets/sprites/target-ring.png`, drawn in the world just before the monster so it lies under its feet). The goblin's sheet (`assets/sprites/goblin-01.png`) is laid out like the player's: three facings, a walk, and a swing of its club. The player swings with the same two poses they mine with, once for each blow.
- **Abilities and the hotbar** (`sim::abilities`, `hotbar.rs`). An ability asked for is used on the player's next tick, which from a standstill comes at once; one aimed at a monster waits until the player is beside it, and is let go if the monster is. Asked for with no target, it goes for the nearest monster that is after the player, or says "No target". Cooldowns are counted in the world's ticks, so they run down whatever the player does, and are not saved. The bar is six 90 px buttons on light plates (so the icons' dark outlines read), each with its key on a chip in the corner; a cooling button goes dark with the seconds left written on it, one asked for and waiting has a light edge, and Guard's has a blue edge while the guard is up. Icons are 128 px, in `assets/abilities/` (sources in `art/abilities/`). What is on the bar is fixed for now.
- **Smithing** (`sim::making`, `making.rs`). A furnace and an anvil are props, placed with the editor like any other; a click walks the player beside one and they are then at it until they walk off (or press Esc, which steps back from a station before it leaves the game). While they are at it the panel for it is open on the left: a row for each thing made there, with its picture on a light plate, what it takes against what is carried (red when short), and buttons for 1, 5 and All, or the Smithing level it wants in red where the smith is not up to it. A thing is made every few ticks, its makings leaving the bag as it goes in, until the number asked for is made, the makings run out or there is no room; a hammer swing is drawn for each. What can be made is one table, `RECIPES`.
- **Equipment** (`sim::gear`, `gear_panel.rs`). Seven slots. A press on a piece in the bag puts it on, trading places with whatever was worn there, so there is always room; a press on a worn piece in the gear panel (the Gear button, or G) takes it off into the bag, if there is room. The panel lays the slots out like a body and adds up what is worn. Gear stays on through death, is saved in a section of its own (a save from before gear simply has none), and is saved the moment it changes.
- **Gear on the character.** Each slot has a sheet laid out exactly like the player's own (`assets/sprites/gear-*.png`), drawn over the body in a fixed order: legs, feet, body, hands, head, shield, weapon. The shield has a second sheet drawn under the body, for the views where it shows from behind. The sheets are plain greys and the game multiplies them by the metal's colour, so a new metal is a colour, not eight more sheets. Greaves, boots, chestplate and gauntlets were made in Lantern Studio from the player's own sheet (each body part picked out by its colour and filled grey, so they fit every pose by construction); helmet, sword and shield are drawn and placed for each of the 18 frames. The source is `art/sprites/player-gear.lstudio`.
- **Sheets of cells are trimmed.** A grid's cell is packed as only the part of it that is drawn, with its anchor moved to match, and a cell with nothing in it is no sprite. The gear sheets are mostly empty, so this keeps the packed sheet small; it shrank the player's and the goblin's too.
- **Herbs and potions** (`sim::potions`, `tonics.rs`). The purple and cyan herbs already on the map are nodes now: a click walks onto one (they are not in the way) and picks it, which never fails; a picked one is drawn as its leaves until it grows back. A cauldron is a third station, with the same panel as the furnace and anvil; a station names the skill it takes, so the panel says Alchemy there. A press on a potion in the bag drinks it (the same press that puts gear on). What a lasting potion is doing shows under the health bar, its picture and the seconds left; a potion in the bag whose kind is still waiting is dark with its seconds on it. What potions are doing is counted in the world's ticks, ends with death, and is not saved.
- **More drawings.** The player's sheet gained a wind-up and a strike for each facing; a spent rock is the rock without its copper; items have 128 px icons in `assets/items/` (sources in `art/items/`).
- **Characters and saves.** The game opens on a title screen (`title.rs`): the painting `assets/title/title-01.png` (source in `art/title/`) cut to fill the window, the game's name, three character slots and Quit. An empty slot asks for a name (1 to 16 letters) and starts a new character; a held one continues; Delete asks first. A save (`sim::save`) is one small binary file per character, `character-N.save`, in sections like a chunk file's, with skills and items by name: who they are and how long they have been played, where they stand, their skills and bag, the nodes they left spent with the time each has left, and their grave. Health and monsters are not saved: both come back whole. It is written whole and then moved into place. Resuming on a map that has changed stands the player as near where they were as they can be and forgets nodes that are gone. Esc goes back to the title screen. The map belongs to no character: all three play on the same one.
- **Controls for now.** Click to walk, to work a rock, to fight a monster, to use a furnace or an anvil, or to open the grave. 1 to 6 for the hotbar; I, K and G for the bag, the skills and the gear. Esc for the title screen. Wheel zooms about the pointer, F11 fullscreen, Ctrl+Q quits. F2 opens and closes the editor; in it, middle-drag or arrows / WASD move the view, Ctrl+Z undoes, Ctrl+Shift+Z or Ctrl+Y redoes, Ctrl+S saves.
- **How it is checked.** 350 tests. The render tests draw the test map with no window and compare pixels with the palette and with colours read off the style sheet (ground colours, rims, the void, a tree's trunk, canopy and shadow, no gaps between ground quads, the window showing the same numbers). `LANTERNLANDS_RENDER_OUT=<folder> cargo test -p game` keeps those pictures as PNGs. The editor is driven end to end with Lantern UI 2's headless harness: pretend keys and clicks open the panel, pick rows, paint, undo, save, and quit through the dialog, against a scratch copy of the map. Whole frames (the world pass and the interface over it, put together as `lntrn-app` does) are drawn with no window too, so the panels, the bag, a fight and a grave can be looked at and their pixels checked. Fights are played through end to end the same way: a goblin is put beside the player and time is let pass.

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
16. The first step after standing still starts at once instead of waiting for the next tick to come round. Now that monsters tick, the world keeps a steady clock of its own and the player's starts on the click.
17. The game stands still while the editor is open. Closing it stops the player and moves them off any ground that can no longer be stood on.
18. While playing the view cannot be moved off the player.
19. Esc leaves for the title screen at once, without asking: nothing can be lost, since it is all saved.
20. Spent rocks are each character's own: one character emptying a rock does not empty it for another.
21. A node can be worked from any of the eight tiles around it.
22. One swing is drawn for each roll at a rock and each blow in a fight (Alva, 2026-10-04: the constant swinging was wrong): the arm is drawn back as the tick before ends and strikes as the blow's tick begins, so the strike and its number come together. Between swings they stand ready. Monsters swing the same way.
23. The game says things itself, at the top of the screen, rather than through Lantern UI 2's toasts.
24. The dice are seeded from the clock when the game starts.
25. A character's name is 1 to 16 letters.
26. The title painting shows only what is in the game (the hero, trees, a copper rock, herbs, a path, a pond, hills and mountains): no castle or other place that has not been designed.
27. Time played counts only while playing, not on the title screen or in the map editor.
28. Props are still the four placeholder kinds; the rock is a copper rock. They become data-defined objects when they start to do things (mining, picking).
29. All of the fighting numbers above, and the goblin's.
30. Until there are towns, dying wakes the player at the map's start marker.
31. Dying with an empty bag leaves no grave, but still crumbles an older one.
32. A grave gives back what fits in the bag and keeps the rest.
33. A player set upon while standing still fights back by themselves. Busy with something else (walking, mining), they carry on with it and take the blows.
34. A monster the player strikes comes for them even from further off than it can see.
35. Health is not saved: a character always starts a session whole, with every monster alive and at home.
36. A dead monster is simply gone until it comes back: no body, no dying animation yet.
37. Experience gained is not shown as it comes (Alva, 2026-10-04: it floated over the player and hid the damage numbers). Where it goes instead is an open question; the Skills panel shows the totals.
38. A tile holds one body. The player walks round monsters and stops beside one that stands where they were going; monsters go round each other and never step onto the player; a dead monster waits to come back while someone stands on its home; a monster that finds the player on its tile (they woke there) steps off before anything else. Before this the player could end up standing on the goblin they clicked, and every number piled up in one place.
39. The first blow lands when the player has got there: set at a monster on the tick of their last step, they strike on the next tick, as the step ends. Standing beside it already, they strike at once.
40. The monster the player is set on (going for it, or fighting it, clicked or not) has a red ring on the ground under it for as long as it is their target; the gold ring on the ground is only for walking.
41. Every ability number above, and which button each is on (1 Heavy Strike, 2 Flurry, 3 Kick, 4 Guard).
42. A button fires on the press, not the release.
43. A Kick does no damage, and a kicked monster shows only the word "stunned": no stars over its head yet.
44. Abilities use the two swing drawings there are: no kick drawing, no guard stance yet.
45. Dying keeps the cooldowns running but drops the guard and anything asked for.
46. Every smithing number above, and the pickaxe's tick.
47. The rock that was just "Rock" is the copper rock; the tin rock is the same rock with tin in it, and a spent one of either looks the same.
48. Smelting is drawn with the same hammer swing as forging.
49. The making panel stays open after a batch is done, to make more; it shows only what that station makes.
50. Icons with dark outlines stand on light plates wherever the ground behind them is dark (the hotbar, the making panel, the gear panel).
51. Every gear number above. Nothing needs a level to be worn yet.
52. The sword is always in the hand nearest the viewer in the side views (the sheet is mirrored for facing left), and the shield on the far side.
53. The chestplate covers the tunic and leaves the sleeves and the belt showing; the helmet leaves the face open with a nose guard.
54. In the strike poses the sword ends low and close to the body: a cell of the player's sheet has no room for a longer reach.
55. Gear is put on and taken off with one press, in a fight too; there is no dragging yet.
56. A panel that would reach over the hotbar stands on top of it instead of on the buttons.
57. The gear panel's empty slots say what goes in them, dimly.
58. Every herb and potion number above.
59. The herbs are still called by their colours, on the map and in the bag; real names are for Alva to choose.
60. No bottles: a potion is just its herb and its monster part. Nothing is left over after drinking.
61. A herb is picked with the same swing the player mines and fights with.
62. A healing potion pressed at full health stays in the bag rather than being wasted.
63. Potions are told apart by a white mark as well as their colour: a cross, an arrow, a shield, a leaf.

## Open questions

- Bank size
- Real names for the herbs
- Whether gear should need levels to wear, and what iron and later metals cost and give
- How abilities are unlocked (all are there from the start for now), and whether the bar can be rearranged
- Where experience gained is shown, now that it no longer floats over the player
- The Skills panel's look: Alva does not like Lantern UI 2's stock progress bars there. To be redrawn as our own rows (it is one function in `hud.rs`); what they should look like is not decided
- The XP curve's numbers (proposed above), and what each level unlocks
- Quests and NPC dialogue: when, and how deep
- Where assets live for a deployed binary (it finds them beside itself, else in the source tree)
- Ground edges: keep the shader's rounded steps, or hand-drawn transitions?
- Audio
- The lantern mechanic: what the lantern does (not designed yet)

## Milestones (proposal)

0. Art style test sheet (terrain, ore rock, herb, player, goblin) to lock the look, tile size, and palette. **Done 2026-10-04**: style approved, palette B Earthy
1. Window, tile renderer, camera. **Done 2026-10-04**: Alva ran it ("It looks beautiful!")
2. Map editor: paint, place, save, load. **Done 2026-10-04**: Alva used it ("that's so cool!"), asked for a move tool, and that "works great"
3. Tick simulation, click-to-move pathfinding. **Done 2026-10-04** with full walk cycles: Alva "spent so much time just walking around"
4. Inventory, skills and XP, Mining nodes. **Done 2026-10-04**: Alva played it ("it works!"), asked for the bag to be 5 across and 6 down, and does not like the stock progress bars in the Skills panel (to be redrawn later)
4b. Save game: title screen, three named characters, autosave. **Done 2026-10-04**: Alva tried it ("It works!")
5. Fighting: goblins, melee auto-attack, health, drops, death and the grave, the editor's Spawn tool. **Done 2026-10-04**: Alva played it ("it's good", "cool!"), had the floating experience text taken out, asked for one swing per blow, a red ring on the monster clicked and a health bar over the player's own head
5b. Abilities: the hotbar, cooldowns, Heavy Strike, Flurry, Kick, Guard. **Done 2026-10-04**: Alva played it ("super cool! Combat works nicely!")
6a. Smithing: tin rock, furnace, anvil, bronze bars, the bronze set and a pickaxe forged into the bag, the making panel. **Done 2026-10-04**: Alva smithed the whole set ("Heck yeah! Awesome!")
6b. Equipment: the seven slots, the gear panel, what gear does in a fight, gear drawn on the character and tinted by its metal, gear kept through death and saved. **Done 2026-10-04**: Alva wore it all, screenshot and all
7. Herbalism, Alchemy, potions: herbs as nodes, the cauldron, the four first potions, drinking from the bag, what a potion does shown while it lasts. **Done 2026-10-04**: Alva tried it ("it works!") and found that continuing a game started at a new character's health, since fixed
8. NPCs, shops, bank, dialogue
