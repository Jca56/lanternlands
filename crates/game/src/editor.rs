//! The map editor's hands: which tool is held, and what the pointer
//! does to the world with it. The tools themselves, and the history
//! that takes them back, are in `sim::edit`.

use lntrn_math::Vec2;
use sim::edit::{self, History};
use sim::{Marker, MonsterKind, PersonKind, PropKind, Terrain, World};

/// The widest the brush goes, in tiles.
pub const SIZE_MAX: u32 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Lay ground.
    Brush,
    /// Take ground away, and what stands on it.
    Eraser,
    /// Flood a stretch of one ground with another.
    Fill,
    /// Stand a prop on a tile.
    Place,
    /// Take a prop off a tile.
    Remove,
    /// Pick a prop up and set it down on another tile.
    Move,
    /// Say where the player appears.
    Start,
    /// Make a tile the home of a monster.
    Spawn,
    /// Make a tile the home of a person.
    Person,
}

impl Tool {
    #[cfg(test)]
    pub const ALL: [Tool; 9] = [Tool::Brush, Tool::Eraser, Tool::Fill, Tool::Place, Tool::Remove, Tool::Move, Tool::Start, Tool::Spawn, Tool::Person];

    pub const fn label(self) -> &'static str {
        match self {
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
            Tool::Fill => "Fill",
            Tool::Place => "Place",
            Tool::Remove => "Remove",
            Tool::Move => "Move",
            Tool::Start => "Start",
            Tool::Spawn => "Spawn",
            Tool::Person => "Person",
        }
    }

    /// Whether the brush's width matters to it.
    pub const fn sized(self) -> bool {
        matches!(self, Tool::Brush | Tool::Eraser)
    }
}

pub struct MapEditor {
    /// Which of the panel's tabs is turned to.
    pub tab: usize,
    pub tool: Tool,
    /// How many tiles across the brush and the eraser are.
    pub size: u32,
    /// The ground the brush and the fill lay.
    pub terrain: Terrain,
    /// What the place tool stands on a tile.
    pub prop: PropKind,
    /// What the spawn tool makes a tile the home of.
    pub monster: MonsterKind,
    /// Who the person tool makes a tile the home of.
    pub person: PersonKind,
    pub history: History,
    /// Where the stroke under way last touched, in tiles.
    stroke: Option<Vec2>,
    /// The tile of the prop the move tool is carrying.
    held: Option<(i32, i32)>,
}

fn tile_under(at: Vec2) -> (i32, i32) {
    (at.x.floor() as i32, at.y.floor() as i32)
}

impl MapEditor {
    pub fn new() -> MapEditor {
        MapEditor { tab: 0, tool: Tool::Brush, size: 1, terrain: Terrain::Grass, prop: PropKind::Tree, monster: MonsterKind::Goblin, person: PersonKind::Smith, history: History::new(), stroke: None, held: None }
    }

    /// The tiles the held tool would change with the pointer at `at`
    /// (a point of the world, in tiles).
    pub fn footprint(&self, at: Vec2) -> Vec<(i32, i32)> {
        if self.tool.sized() { edit::brush((at.x, at.y), self.size) } else { vec![tile_under(at)] }
    }

    /// The button went down with the pointer at `at`: a stroke starts.
    pub fn press(&mut self, world: &mut World, at: Vec2) {
        self.history.begin();
        self.stroke = Some(at);
        let (x, y) = tile_under(at);
        match self.tool {
            // A flood happens where the button went down and nowhere else.
            Tool::Fill => {
                edit::fill(&mut self.history, world, x, y, self.terrain);
            }
            // What is carried is whatever stood where the button went down.
            Tool::Move => self.held = world.prop_at(x, y).map(|_| (x, y)),
            _ => self.touch(world, at, at),
        }
    }

    /// The pointer moved to `at` with the button still down.
    pub fn drag(&mut self, world: &mut World, at: Vec2) {
        if let Some(from) = self.stroke.replace(at) {
            self.touch(world, from, at);
        } else {
            self.stroke = None;
        }
    }

    /// The button came up, or the stroke was cut short: what it did is
    /// one step of the history.
    pub fn release(&mut self) {
        self.held = None;
        if self.stroke.take().is_some() {
            self.history.commit();
        }
    }

    /// The tile of the prop being carried by the move tool, if one is.
    pub fn carrying(&self) -> Option<(i32, i32)> {
        self.held
    }

    #[cfg(test)]
    fn stroking(&self) -> bool {
        self.stroke.is_some()
    }

    pub fn undo(&mut self, world: &mut World) -> bool {
        (self.stroke, self.held) = (None, None);
        self.history.undo(world)
    }

    pub fn redo(&mut self, world: &mut World) -> bool {
        self.release();
        self.history.redo(world)
    }

    /// What every tool but the fill does as the pointer goes from
    /// `from` to `to`.
    fn touch(&mut self, world: &mut World, from: Vec2, to: Vec2) {
        let (x, y) = tile_under(to);
        match self.tool {
            Tool::Brush => edit::paint_line(&mut self.history, world, (from.x, from.y), (to.x, to.y), self.size, Some(self.terrain)),
            Tool::Eraser => edit::paint_line(&mut self.history, world, (from.x, from.y), (to.x, to.y), self.size, None),
            Tool::Fill => {}
            Tool::Place => {
                edit::place(&mut self.history, world, x, y, self.prop);
            }
            // What stands on the tile goes first; with nothing standing,
            // whoever lives there does, monster or person.
            Tool::Remove => {
                if !edit::remove(&mut self.history, world, x, y) {
                    edit::set_spawn(&mut self.history, world, x, y, None);
                    edit::set_person(&mut self.history, world, x, y, None);
                }
            }
            Tool::Spawn => {
                edit::set_spawn(&mut self.history, world, x, y, Some(self.monster));
            }
            Tool::Person => {
                edit::set_person(&mut self.history, world, x, y, Some(self.person));
            }
            Tool::Start => {
                edit::set_marker(&mut self.history, world, Marker::PlayerStart, x, y);
            }
            // The carried prop follows the pointer onto any tile it can
            // stand on, and waits on its last one over those it can't.
            Tool::Move => {
                if let Some(from) = self.held
                    && edit::move_prop(&mut self.history, world, from, (x, y))
                {
                    self.held = Some((x, y));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field() -> World {
        let mut world = World::new();
        for y in 0..10 {
            for x in 0..10 {
                world.set_terrain(x, y, Terrain::Grass);
            }
        }
        world
    }

    fn count(world: &World, terrain: Terrain) -> usize {
        (0..10).flat_map(|y| (0..10).map(move |x| (x, y))).filter(|&(x, y)| world.terrain(x, y) == Some(terrain)).count()
    }

    #[test]
    fn press_drag_release_is_one_undo() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        editor.terrain = Terrain::Dirt;
        editor.press(&mut world, Vec2::new(1.5, 1.5));
        editor.drag(&mut world, Vec2::new(5.5, 1.5));
        editor.drag(&mut world, Vec2::new(5.5, 6.5));
        editor.release();
        assert_eq!(count(&world, Terrain::Dirt), 10);
        assert!(editor.undo(&mut world));
        assert_eq!(count(&world, Terrain::Dirt), 0);
        assert!(!editor.undo(&mut world));
        assert!(editor.redo(&mut world));
        assert_eq!(count(&world, Terrain::Dirt), 10);
    }

    #[test]
    fn the_footprint_is_what_a_press_changes() {
        for (tool, size) in [(Tool::Brush, 1), (Tool::Brush, 4), (Tool::Eraser, 5), (Tool::Fill, 3), (Tool::Place, 7)] {
            // (Remove and Move change nothing on bare ground; their own tests cover them.)
            let (mut world, mut editor) = (field(), MapEditor::new());
            (editor.tool, editor.size, editor.terrain) = (tool, size, Terrain::Sand);
            let at = Vec2::new(4.3, 5.8);
            let footprint = editor.footprint(at);
            let before: Vec<_> = (0..10).flat_map(|y| (0..10).map(move |x| (x, y))).map(|(x, y)| world.tile(x, y)).collect();
            editor.press(&mut world, at);
            editor.release();
            if tool == Tool::Fill {
                // A flood goes as far as the ground does; it starts under the pointer.
                assert_eq!(footprint, vec![(4, 5)]);
                assert_eq!(count(&world, Terrain::Sand), 100);
                continue;
            }
            for y in 0..10 {
                for x in 0..10 {
                    let changed = world.tile(x, y) != before[(y * 10 + x) as usize];
                    assert_eq!(changed, footprint.contains(&(x, y)), "{tool:?} size {size} at tile ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn a_drag_with_no_press_does_nothing() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        editor.terrain = Terrain::Water;
        editor.drag(&mut world, Vec2::new(3.5, 3.5));
        editor.release();
        assert_eq!(count(&world, Terrain::Water), 0);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn the_fill_does_not_follow_a_drag() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.set_terrain(5, 5, Terrain::Water);
        (editor.tool, editor.terrain) = (Tool::Fill, Terrain::Sand);
        editor.press(&mut world, Vec2::new(0.5, 0.5));
        editor.drag(&mut world, Vec2::new(5.5, 5.5));
        editor.release();
        assert_eq!(world.terrain(5, 5), Some(Terrain::Water), "the pond the drag crossed is not flooded");
        assert_eq!(count(&world, Terrain::Sand), 99);
    }

    #[test]
    fn place_and_remove_work_tile_by_tile_along_a_drag() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        (editor.tool, editor.prop) = (Tool::Place, PropKind::Rock);
        editor.press(&mut world, Vec2::new(2.5, 2.5));
        editor.drag(&mut world, Vec2::new(3.5, 2.5));
        editor.release();
        assert_eq!(world.props_in(0, 0, 10, 10).count(), 2);
        editor.tool = Tool::Remove;
        editor.press(&mut world, Vec2::new(2.5, 2.5));
        editor.release();
        assert_eq!(world.props_in(0, 0, 10, 10).count(), 1);
        // Two strokes, two undos.
        editor.undo(&mut world);
        editor.undo(&mut world);
        assert_eq!(world.props_in(0, 0, 10, 10).count(), 0);
    }

    /// Where the one prop of `kind` on the field stands.
    fn where_is(world: &World, kind: PropKind) -> Vec<(i32, i32)> {
        world.props_in(-5, -5, 15, 15).filter(|p| p.kind == kind).map(|p| (p.x, p.y)).collect()
    }

    #[test]
    fn the_move_tool_carries_a_prop_to_where_the_button_comes_up() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.place_prop(PropKind::Tree, 2, 2);
        editor.tool = Tool::Move;
        editor.press(&mut world, Vec2::new(2.4, 2.6));
        assert_eq!(editor.carrying(), Some((2, 2)));
        // It follows the pointer as it goes.
        editor.drag(&mut world, Vec2::new(3.5, 2.5));
        assert_eq!(where_is(&world, PropKind::Tree), vec![(3, 2)]);
        editor.drag(&mut world, Vec2::new(7.5, 6.5));
        editor.release();
        assert_eq!(where_is(&world, PropKind::Tree), vec![(7, 6)]);
        assert_eq!(editor.carrying(), None);

        // The whole carry is one step.
        assert!(editor.undo(&mut world));
        assert_eq!(where_is(&world, PropKind::Tree), vec![(2, 2)]);
        assert!(!editor.undo(&mut world));
        assert!(editor.redo(&mut world));
        assert_eq!(where_is(&world, PropKind::Tree), vec![(7, 6)]);
    }

    #[test]
    fn a_carried_prop_waits_at_the_edge_of_where_it_cannot_stand() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.place_prop(PropKind::Tree, 2, 2);
        world.place_prop(PropKind::Rock, 5, 2);
        editor.tool = Tool::Move;
        editor.press(&mut world, Vec2::new(2.5, 2.5));
        editor.drag(&mut world, Vec2::new(4.5, 2.5));
        // Over the rock, then off the map: it stays on its last good tile.
        editor.drag(&mut world, Vec2::new(5.5, 2.5));
        assert_eq!(where_is(&world, PropKind::Tree), vec![(4, 2)]);
        editor.drag(&mut world, Vec2::new(-3.5, 2.5));
        assert_eq!(where_is(&world, PropKind::Tree), vec![(4, 2)]);
        assert_eq!(where_is(&world, PropKind::Rock), vec![(5, 2)], "the rock was not disturbed");
        // Back over good ground it comes along again.
        editor.drag(&mut world, Vec2::new(6.5, 3.5));
        editor.release();
        assert_eq!(where_is(&world, PropKind::Tree), vec![(6, 3)]);
    }

    #[test]
    fn the_move_tool_on_bare_ground_carries_nothing() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.place_prop(PropKind::Tree, 2, 2);
        editor.tool = Tool::Move;
        editor.press(&mut world, Vec2::new(6.5, 6.5));
        assert_eq!(editor.carrying(), None);
        // Dragging across the tree does not pick it up on the way.
        editor.drag(&mut world, Vec2::new(2.5, 2.5));
        editor.drag(&mut world, Vec2::new(8.5, 8.5));
        editor.release();
        assert_eq!(where_is(&world, PropKind::Tree), vec![(2, 2)]);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn a_prop_put_back_where_it_was_leaves_nothing_to_undo() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.place_prop(PropKind::Rock, 4, 4);
        editor.tool = Tool::Move;
        editor.press(&mut world, Vec2::new(4.5, 4.5));
        editor.drag(&mut world, Vec2::new(6.5, 4.5));
        editor.drag(&mut world, Vec2::new(4.5, 4.5));
        editor.release();
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn the_start_tool_puts_the_players_start_under_the_pointer() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        editor.tool = Tool::Start;
        editor.press(&mut world, Vec2::new(3.5, 4.5));
        editor.release();
        assert_eq!(world.marker(Marker::PlayerStart), Some((3, 4)));
        // Dragged, it follows; off the ground, it stays on the last good tile.
        editor.press(&mut world, Vec2::new(6.5, 6.5));
        editor.drag(&mut world, Vec2::new(7.5, 6.5));
        editor.drag(&mut world, Vec2::new(-2.5, 6.5));
        editor.release();
        assert_eq!(world.marker(Marker::PlayerStart), Some((7, 6)));
        editor.undo(&mut world);
        assert_eq!(world.marker(Marker::PlayerStart), Some((3, 4)));
    }

    #[test]
    fn the_spawn_tool_makes_homes_and_remove_takes_them_after_any_prop() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        world.place_prop(PropKind::HerbCyan, 4, 4);
        editor.tool = Tool::Spawn;
        editor.press(&mut world, Vec2::new(4.5, 4.5));
        editor.drag(&mut world, Vec2::new(5.5, 4.5));
        editor.release();
        assert_eq!(world.spawns(), vec![(MonsterKind::Goblin, 4, 4), (MonsterKind::Goblin, 5, 4)]);
        // Remove: first the herb on the tile, then the home under it.
        editor.tool = Tool::Remove;
        editor.press(&mut world, Vec2::new(4.5, 4.5));
        editor.release();
        assert_eq!((world.prop_at(4, 4), world.spawn_at(4, 4)), (None, Some(MonsterKind::Goblin)));
        editor.press(&mut world, Vec2::new(4.5, 4.5));
        editor.release();
        assert_eq!(world.spawns(), vec![(MonsterKind::Goblin, 5, 4)]);
        editor.undo(&mut world);
        assert_eq!(world.spawn_at(4, 4), Some(MonsterKind::Goblin));
    }

    #[test]
    fn undo_mid_stroke_ends_the_stroke() {
        let (mut world, mut editor) = (field(), MapEditor::new());
        editor.terrain = Terrain::Dirt;
        editor.press(&mut world, Vec2::new(1.5, 1.5));
        assert!(editor.undo(&mut world));
        assert!(!editor.stroking());
        // The button is still down, but the stroke is over: no more paint.
        editor.drag(&mut world, Vec2::new(8.5, 8.5));
        assert_eq!(count(&world, Terrain::Dirt), 0);
    }
}
