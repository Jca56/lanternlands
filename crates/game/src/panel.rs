//! The map editor's side panel: the tools, the brush's width, the
//! grounds, the props, the monsters and the people to pick from, and
//! undo, redo and save.

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

        ui.heading("Tool");
        for tool in Tool::ALL {
            if ui.selectable(tool.label(), editor.tool == tool).clicked {
                editor.tool = tool;
            }
        }
        if editor.tool.sized() {
            let mut size = editor.size as f64;
            if ui.slider("Size", &mut size, 1.0, SIZE_MAX as f64, 1.0) {
                editor.size = size.round().clamp(1.0, SIZE_MAX as f64) as u32;
            }
        }

        ui.heading("Ground");
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

        ui.heading("Props");
        for (kind, picture) in PropKind::ALL.into_iter().zip(pictures.props) {
            let picked = editor.prop == kind && editor.tool == Tool::Place;
            if pick_row(ui, prop_label(kind), picked, Shown::Picture(picture)) {
                editor.prop = kind;
                editor.tool = Tool::Place;
            }
        }

        ui.heading("Monsters");
        for (kind, picture) in MonsterKind::ALL.into_iter().zip(pictures.monsters) {
            let picked = editor.monster == kind && editor.tool == Tool::Spawn;
            if pick_row(ui, kind.label(), picked, Shown::Picture(picture)) {
                editor.monster = kind;
                editor.tool = Tool::Spawn;
            }
        }

        ui.heading("People");
        for (kind, picture) in PersonKind::ALL.into_iter().zip(pictures.people) {
            let picked = editor.person == kind && editor.tool == Tool::Person;
            if pick_row(ui, &person_label(kind), picked, Shown::Picture(picture)) {
                editor.person = kind;
                editor.tool = Tool::Person;
            }
        }
    });
    asked
}

#[cfg(test)]
mod tests {
    use lntrn_ui::WidgetId;
    use lntrn_ui::testing::Harness;

    use super::*;

    fn id(label: &str) -> WidgetId {
        WidgetId::ROOT.with("map editor").with(label)
    }

    fn frame(h: &mut Harness, editor: &mut MapEditor, unsaved: bool) -> Option<Asked> {
        let mut asked = None;
        h.frame(|ui| asked = draw(ui, editor, unsaved, &Pictures::default()));
        asked
    }

    fn click(h: &mut Harness, editor: &mut MapEditor, unsaved: bool, label: &str) -> Option<Asked> {
        let mut asked = None;
        h.click_on(id(label), |ui| asked = asked.or(draw(ui, editor, unsaved, &Pictures::default())));
        asked
    }

    #[test]
    fn picking_a_tool_holds_it() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        for tool in Tool::ALL {
            click(&mut h, &mut editor, false, tool.label());
            assert_eq!(editor.tool, tool);
        }
    }

    #[test]
    fn picking_a_ground_takes_up_the_brush_unless_the_fill_is_held() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        editor.tool = Tool::Remove;
        frame(&mut h, &mut editor, false);
        click(&mut h, &mut editor, false, "Water");
        assert_eq!((editor.terrain, editor.tool), (Terrain::Water, Tool::Brush));
        editor.tool = Tool::Fill;
        click(&mut h, &mut editor, false, "Sand");
        assert_eq!((editor.terrain, editor.tool), (Terrain::Sand, Tool::Fill));
    }

    #[test]
    fn picking_a_prop_takes_up_the_place_tool() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        click(&mut h, &mut editor, false, "Purple herb");
        assert_eq!((editor.prop, editor.tool), (PropKind::HerbPurple, Tool::Place));
    }

    #[test]
    fn picking_a_monster_takes_up_the_spawn_tool() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        click(&mut h, &mut editor, false, "Goblin");
        assert_eq!((editor.monster, editor.tool), (MonsterKind::Goblin, Tool::Spawn));
    }

    #[test]
    fn picking_a_person_takes_up_the_person_tool() {
        let (mut h, mut editor) = (Harness::new(450.0, 2200.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        click(&mut h, &mut editor, false, "Odo the Guard");
        assert_eq!((editor.person, editor.tool), (PersonKind::Guard, Tool::Person));
    }

    #[test]
    fn save_asks_only_when_there_is_something_to_save() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        frame(&mut h, &mut editor, false);
        assert_eq!(click(&mut h, &mut editor, false, "Save"), None);
        assert_eq!(click(&mut h, &mut editor, true, "Save"), Some(Asked::Save));
    }

    #[test]
    fn undo_and_redo_ask_only_when_there_is_a_step_to_take() {
        let (mut h, mut editor) = (Harness::new(450.0, 1800.0), MapEditor::new());
        let mut world = sim::World::new();
        world.set_terrain(0, 0, Terrain::Grass);
        frame(&mut h, &mut editor, false);
        assert_eq!(click(&mut h, &mut editor, false, "Undo"), None);
        editor.terrain = Terrain::Dirt;
        editor.press(&mut world, Vec2::new(0.5, 0.5));
        editor.release();
        assert_eq!(click(&mut h, &mut editor, false, "Redo"), None);
        assert_eq!(click(&mut h, &mut editor, false, "Undo"), Some(Asked::Undo));
        editor.undo(&mut world);
        assert_eq!(click(&mut h, &mut editor, false, "Redo"), Some(Asked::Redo));
    }

    #[test]
    fn every_ground_and_prop_has_a_row_with_its_own_name() {
        let people = PersonKind::ALL.map(person_label);
        let labels: Vec<&str> = Terrain::ALL.map(terrain_label).into_iter().chain(PropKind::ALL.map(prop_label)).chain(MonsterKind::ALL.map(MonsterKind::label)).chain(Tool::ALL.map(Tool::label)).chain(people.iter().map(String::as_str)).collect();
        let mut unique = labels.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), labels.len(), "two rows share a name, and so a widget id");
    }
}
