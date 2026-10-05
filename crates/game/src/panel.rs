//! The map editor's side panel: undo, redo and save, and under them
//! three tabs. Ground has the brush, the fill, the eraser and the
//! grounds to lay; Props has the props to stand on tiles, in groups
//! that fold away; Living has the monsters and the people to settle,
//! and the player's start.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{FILL, ImageHandle, Sense, Ui};
use sim::{MonsterKind, PersonKind, PropKind, Terrain};

use crate::editor::{MapEditor, SIZE_MAX, Tool};
use crate::palette::terrain_style;

/// How tall a row with a picture is, in logical pixels.
const PICK_ROW: f64 = 65.0;

/// Something a button on the panel asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    Undo,
    Redo,
    Save,
}

/// The props' pictures, uploaded for the interface to draw.
#[derive(Default)]
pub struct Pictures {
    pub props: [Option<ImageHandle>; PropKind::ALL.len()],
    pub monsters: [Option<ImageHandle>; MonsterKind::ALL.len()],
    pub people: [Option<ImageHandle>; PersonKind::ALL.len()],
}

pub const fn terrain_label(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Grass => "Grass",
        Terrain::Dirt => "Dirt",
        Terrain::Sand => "Sand",
        Terrain::Water => "Water",
    }
}

pub const fn prop_label(kind: PropKind) -> &'static str {
    match kind {
        PropKind::Tree => "Tree",
        PropKind::Rock => "Copper rock",
        PropKind::TinRock => "Tin rock",
        PropKind::Furnace => "Furnace",
        PropKind::Anvil => "Anvil",
        PropKind::Cauldron => "Cauldron",
        PropKind::HerbPurple => "Purple herb",
        PropKind::HerbCyan => "Cyan herb",
        PropKind::BankChest => "Bank chest",
        PropKind::Crates => "Crates",
        PropKind::Barrel => "Barrel",
        PropKind::Signpost => "Signpost",
        PropKind::Well => "Well",
        PropKind::Fence => "Fence",
    }
}

/// The panel's tabs, and the tool taken up on turning to each.
pub const TABS: [&str; 3] = ["Ground", "Props", "Living"];
const TAB_TOOL: [Tool; 3] = [Tool::Brush, Tool::Place, Tool::Spawn];
/// The tools each tab has buttons for. Picking something from a tab's
/// lists takes up the tool that puts it down, button or no.
pub const TAB_TOOLS: [&[Tool]; 3] = [&[Tool::Brush, Tool::Fill, Tool::Eraser], &[Tool::Place, Tool::Move, Tool::Remove], &[Tool::Start, Tool::Remove]];
/// What the lists of the living are headed.
pub const MONSTERS: &str = "Monsters";
pub const PEOPLE: &str = "People";

/// The groups the props are listed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Nature,
    Nodes,
    Stations,
    Village,
}

impl Group {
    pub const ALL: [Group; 4] = [Group::Nature, Group::Nodes, Group::Stations, Group::Village];

    pub const fn label(self) -> &'static str {
        match self {
            Group::Nature => "Nature",
            Group::Nodes => "Rocks and herbs",
            Group::Stations => "Stations",
            Group::Village => "Village",
        }
    }

    /// The group a kind of prop is listed in.
    pub const fn of(kind: PropKind) -> Group {
        match kind {
            PropKind::Tree => Group::Nature,
            PropKind::Rock | PropKind::TinRock | PropKind::HerbPurple | PropKind::HerbCyan => Group::Nodes,
            PropKind::Furnace | PropKind::Anvil | PropKind::Cauldron | PropKind::BankChest => Group::Stations,
            PropKind::Crates | PropKind::Barrel | PropKind::Signpost | PropKind::Well | PropKind::Fence => Group::Village,
        }
    }
}

/// A person's row: who they are and what they do.
pub fn person_label(kind: PersonKind) -> String {
    format!("{} the {}", kind.label(), kind.calling())
}

/// What a pick row shows beside its name.
enum Shown {
    Swatch(Color),
    Picture(Option<ImageHandle>),
}

/// A tall row to pick one thing from a list: what it looks like on the
/// left, its name beside it, lit when it is the one picked. Returns
/// whether it was clicked.
fn pick_row(ui: &mut Ui, label: &str, picked: bool, shown: Shown) -> bool {
    let id = ui.id(label);
    let rect = ui.alloc(Vec2::new(FILL, ui.m.px(PICK_ROW)));
    let mut r = ui.interact(id, rect, Sense::CLICK);
    ui.focusable(id, rect);
    ui.key_click(id, &mut r);
    if picked {
        ui.fill_shaded(rect, ui.theme.shaded(ui.theme.selection));
    } else if r.hovered || r.held {
        ui.fill(rect, ui.theme.hover(ui.theme.panel.mid()));
    }
    let pad = ui.m.pad;
    let side = rect.height() - pad;
    let frame = Rect::from_min_size(rect.min + Vec2::new(pad, pad * 0.5), Vec2::new(side, side));
    match shown {
        Shown::Swatch(color) => {
            ui.draw.rounded_rect(frame, ui.m.radius, color);
            ui.draw.stroke_rect(frame, ui.m.border, ui.m.radius, ui.theme.border_dark);
        }
        Shown::Picture(Some(image)) => {
            // As big as fits the frame without stretching.
            let scale = (side / image.width.max(1) as f64).min(side / image.height.max(1) as f64);
            let fitted = Rect::from_center_size(frame.center(), Vec2::new(image.width as f64, image.height as f64) * scale).round();
            ui.draw.image(fitted, image, 0.0, Color::WHITE);
        }
        Shown::Picture(None) => {}
    }
    let style = ui.text_style();
    let text = Rect::new(Vec2::new(frame.max.x + pad, rect.min.y), rect.max);
    ui.text_in_rect(label, &style, text, if picked { ui.theme.selection_text } else { ui.theme.text });
    ui.focus_ring(id, rect);
    r.clicked
}

/// A button that is only there to press when `enabled`.
fn button(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    if enabled {
        ui.button(label).clicked
    } else {
        ui.button_disabled(label);
        false
    }
}

/// A row of buttons for a tab's tools, the held one lit.
fn tool_row(ui: &mut Ui, editor: &mut MapEditor, tools: &[Tool]) {
    let labels: Vec<&str> = tools.iter().map(|tool| tool.label()).collect();
    let mut held = tools.iter().position(|tool| *tool == editor.tool).unwrap_or(usize::MAX);
    if ui.tabs(&mut held, &labels) {
        editor.tool = tools[held];
    }
}

/// Draw the panel. `unsaved` is whether the map has changes to save.
pub fn draw(ui: &mut Ui, editor: &mut MapEditor, unsaved: bool, pictures: &Pictures) -> Option<Asked> {
    let mut asked = None;
    ui.scroll_area("map editor", None, |ui| {
        ui.row(|ui| {
            if button(ui, "Undo", editor.history.can_undo()) {
                asked = Some(Asked::Undo);
            }
            if button(ui, "Redo", editor.history.can_redo()) {
                asked = Some(Asked::Redo);
            }
            if button(ui, "Save", unsaved) {
                asked = Some(Asked::Save);
            }
        });
        ui.space(ui.m.gap);
        if ui.tabs(&mut editor.tab, &TABS) {
            editor.tool = TAB_TOOL[editor.tab];
        }
        ui.space(ui.m.gap);
        let tab = editor.tab.min(TABS.len() - 1);
        tool_row(ui, editor, TAB_TOOLS[tab]);
        match tab {
            0 => ground_tab(ui, editor),
            1 => props_tab(ui, editor, pictures),
            _ => living_tab(ui, editor, pictures),
        }
    });
    asked
}

/// The brush's width, and the grounds to lay.
fn ground_tab(ui: &mut Ui, editor: &mut MapEditor) {
    if editor.tool.sized() {
        let mut size = editor.size as f64;
        if ui.slider("Size", &mut size, 1.0, SIZE_MAX as f64, 1.0) {
            editor.size = size.round().clamp(1.0, SIZE_MAX as f64) as u32;
        }
    }
    for terrain in Terrain::ALL {
        let [r, g, b] = terrain_style(terrain).fill.map(f64::from);
        let picked = editor.terrain == terrain && matches!(editor.tool, Tool::Brush | Tool::Fill);
        if pick_row(ui, terrain_label(terrain), picked, Shown::Swatch(Color::rgb(r, g, b))) {
            editor.terrain = terrain;
            // Picking a ground means to lay it: with the fill if that
            // is what is held, else with the brush.
            if editor.tool != Tool::Fill {
                editor.tool = Tool::Brush;
            }
        }
    }
}

/// The props, group by group, each group under a header that folds it
/// away.
fn props_tab(ui: &mut Ui, editor: &mut MapEditor, pictures: &Pictures) {
    for group in Group::ALL {
        ui.collapsing(group.label(), |ui| {
            for (kind, picture) in PropKind::ALL.into_iter().zip(pictures.props).filter(|(kind, _)| Group::of(*kind) == group) {
                let picked = editor.prop == kind && editor.tool == Tool::Place;
                if pick_row(ui, prop_label(kind), picked, Shown::Picture(picture)) {
                    editor.prop = kind;
                    editor.tool = Tool::Place;
                }
            }
        });
    }
}

/// The monsters and the people to settle on tiles.
fn living_tab(ui: &mut Ui, editor: &mut MapEditor, pictures: &Pictures) {
    ui.collapsing(MONSTERS, |ui| {
        for (kind, picture) in MonsterKind::ALL.into_iter().zip(pictures.monsters) {
            let picked = editor.monster == kind && editor.tool == Tool::Spawn;
            if pick_row(ui, kind.label(), picked, Shown::Picture(picture)) {
                editor.monster = kind;
                editor.tool = Tool::Spawn;
            }
        }
    });
    ui.collapsing(PEOPLE, |ui| {
        for (kind, picture) in PersonKind::ALL.into_iter().zip(pictures.people) {
            let picked = editor.person == kind && editor.tool == Tool::Person;
            if pick_row(ui, &person_label(kind), picked, Shown::Picture(picture)) {
                editor.person = kind;
                editor.tool = Tool::Person;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use lntrn_ui::WidgetId;
    use lntrn_ui::testing::Harness;

    use super::*;

    /// A widget of the panel, by the names it is under.
    fn id(path: &[&str]) -> WidgetId {
        path.iter().fold(WidgetId::ROOT.with("map editor"), |id, name| id.with(name))
    }

    fn frame(h: &mut Harness, editor: &mut MapEditor, unsaved: bool) -> Option<Asked> {
        let mut asked = None;
        h.frame(|ui| asked = draw(ui, editor, unsaved, &Pictures::default()));
        asked
    }

    fn click_id(h: &mut Harness, editor: &mut MapEditor, unsaved: bool, id: WidgetId) -> Option<Asked> {
        let mut asked = None;
        h.click_on(id, |ui| asked = asked.or(draw(ui, editor, unsaved, &Pictures::default())));
        asked
    }

    fn click(h: &mut Harness, editor: &mut MapEditor, unsaved: bool, path: &[&str]) -> Option<Asked> {
        click_id(h, editor, unsaved, id(path))
    }

    /// Turn to a tab, and lay its contents out.
    fn turn_to(h: &mut Harness, editor: &mut MapEditor, tab: usize) {
        click_id(h, editor, false, id(&[TABS[tab]]).with_index(tab));
        frame(h, editor, false);
        assert_eq!(editor.tab, tab);
    }

    fn opened() -> (Harness, MapEditor) {
        let (mut h, mut editor) = (Harness::new(450.0, 1400.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        (h, editor)
    }

    #[test]
    fn turning_to_a_tab_takes_up_its_tool_and_its_buttons_hold_the_others() {
        let (mut h, mut editor) = opened();
        for tab in [1, 2, 0] {
            turn_to(&mut h, &mut editor, tab);
            assert_eq!(editor.tool, TAB_TOOL[tab], "the {} tab", TABS[tab]);
            for (i, tool) in TAB_TOOLS[tab].iter().enumerate() {
                click_id(&mut h, &mut editor, false, id(&[tool.label()]).with_index(i));
                assert_eq!(editor.tool, *tool);
            }
        }
        // Between them the tabs reach every tool: by a button, or by
        // picking what the tool puts down.
        for tool in Tool::ALL {
            assert!(TAB_TOOLS.iter().any(|tools| tools.contains(&tool)) || TAB_TOOL.contains(&tool) || tool == Tool::Person, "{tool:?} is out of reach");
        }
    }

    #[test]
    fn picking_a_ground_takes_up_the_brush_unless_the_fill_is_held() {
        let (mut h, mut editor) = opened();
        editor.tool = Tool::Remove;
        frame(&mut h, &mut editor, false);
        click(&mut h, &mut editor, false, &["Water"]);
        assert_eq!((editor.terrain, editor.tool), (Terrain::Water, Tool::Brush));
        editor.tool = Tool::Fill;
        click(&mut h, &mut editor, false, &["Sand"]);
        assert_eq!((editor.terrain, editor.tool), (Terrain::Sand, Tool::Fill));
    }

    #[test]
    fn picking_a_prop_from_its_group_takes_up_the_place_tool() {
        let (mut h, mut editor) = opened();
        turn_to(&mut h, &mut editor, 1);
        editor.tool = Tool::Move;
        for (kind, group) in [(PropKind::HerbPurple, Group::Nodes), (PropKind::BankChest, Group::Stations), (PropKind::Fence, Group::Village), (PropKind::Tree, Group::Nature)] {
            assert_eq!(Group::of(kind), group);
            click(&mut h, &mut editor, false, &[group.label(), prop_label(kind)]);
            assert_eq!((editor.prop, editor.tool), (kind, Tool::Place));
        }
    }

    #[test]
    fn a_group_folds_away_and_comes_back() {
        let (mut h, mut editor) = opened();
        turn_to(&mut h, &mut editor, 1);
        let (header, row) = (id(&[Group::Village.label()]), id(&[Group::Village.label(), "Well"]));
        let open = h.rect_of(id(&[Group::Stations.label()])).expect("a header").min.y;
        assert!(h.rect_of(row).is_some_and(|r| r.min.y > open));
        // Folding Nature away moves everything under it up by its one row.
        click_id(&mut h, &mut editor, false, id(&[Group::Nature.label()]));
        frame(&mut h, &mut editor, false);
        assert!(h.rect_of(id(&[Group::Stations.label()])).expect("still there").min.y < open);
        click_id(&mut h, &mut editor, false, id(&[Group::Nature.label()]));
        frame(&mut h, &mut editor, false);
        assert_eq!(h.rect_of(id(&[Group::Stations.label()])).expect("still there").min.y, open);
        // A folded group's rows are not there at all.
        click_id(&mut h, &mut editor, false, header);
        frame(&mut h, &mut editor, false);
        assert_eq!(h.rect_of(row), None);
        assert!(h.rect_of(header).is_some());
    }

    #[test]
    fn picking_a_monster_takes_up_the_spawn_tool_and_a_person_the_person_tool() {
        let (mut h, mut editor) = opened();
        turn_to(&mut h, &mut editor, 2);
        editor.tool = Tool::Start;
        click(&mut h, &mut editor, false, &[MONSTERS, "Goblin"]);
        assert_eq!((editor.monster, editor.tool), (MonsterKind::Goblin, Tool::Spawn));
        click(&mut h, &mut editor, false, &[PEOPLE, "Odo the Guard"]);
        assert_eq!((editor.person, editor.tool), (PersonKind::Guard, Tool::Person));
        click(&mut h, &mut editor, false, &[PEOPLE, "Tilda the Banker"]);
        assert_eq!(editor.person, PersonKind::Banker);
    }

    #[test]
    fn save_asks_only_when_there_is_something_to_save() {
        let (mut h, mut editor) = opened();
        assert_eq!(click(&mut h, &mut editor, false, &["Save"]), None);
        assert_eq!(click(&mut h, &mut editor, true, &["Save"]), Some(Asked::Save));
    }

    #[test]
    fn undo_and_redo_ask_only_when_there_is_a_step_to_take() {
        let (mut h, mut editor) = opened();
        let mut world = sim::World::new();
        world.set_terrain(0, 0, Terrain::Grass);
        assert_eq!(click(&mut h, &mut editor, false, &["Undo"]), None);
        editor.terrain = Terrain::Dirt;
        editor.press(&mut world, Vec2::new(0.5, 0.5));
        editor.release();
        assert_eq!(click(&mut h, &mut editor, false, &["Redo"]), None);
        assert_eq!(click(&mut h, &mut editor, false, &["Undo"]), Some(Asked::Undo));
        editor.undo(&mut world);
        assert_eq!(click(&mut h, &mut editor, false, &["Redo"]), Some(Asked::Redo));
    }

    #[test]
    fn every_list_names_each_thing_once_and_every_prop_is_in_a_group() {
        let unique = |labels: Vec<String>, what: &str| {
            let mut sorted = labels.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), labels.len(), "two {what} share a name, and so a widget id");
        };
        unique(Terrain::ALL.map(terrain_label).iter().chain(&TABS).chain(&["Undo", "Redo", "Save", "Size"]).map(|s| s.to_string()).collect(), "things on the Ground tab");
        for group in Group::ALL {
            let kinds: Vec<PropKind> = PropKind::ALL.into_iter().filter(|kind| Group::of(*kind) == group).collect();
            assert!(!kinds.is_empty(), "nothing is listed under {}", group.label());
            unique(kinds.into_iter().map(|kind| prop_label(kind).to_owned()).collect(), "props");
        }
        unique(Group::ALL.map(|g| g.label().to_owned()).to_vec(), "groups");
        unique(MonsterKind::ALL.map(|k| k.label().to_owned()).to_vec(), "monsters");
        unique(PersonKind::ALL.map(person_label).to_vec(), "people");
        for tools in TAB_TOOLS {
            unique(tools.iter().map(|tool| tool.label().to_owned()).collect(), "tools of a tab");
        }
    }
}
