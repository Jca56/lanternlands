//! How the interface looks: parchment. Warm paper, dark brown ink, and
//! brown edges, for every menu, dialog, panel and button, in place of
//! Lantern UI 2's own look: this is a game, not a desktop.
//!
//! Only the look is set here. How big the writing and the controls are
//! is left as the player has it.

use lntrn_math::Color;
use lntrn_props::Gradient;
use lntrn_ui::Theme;

/// What panels are made of, and a darker sheet for what lies on them:
/// menus, the headers of lists.
pub const PAPER: Color = Color::hex(0xE8D8B0);
const SHEET: Color = Color::hex(0xD8C493);
/// Buttons and tabs: a shade darker again, to stand out from the page.
const BUTTON: Color = Color::hex(0xC8AD77);
/// What is sunk into the page: slots, fields.
const RECESS: Color = Color::hex(0xB8A06D);
/// Writing, and writing that matters less.
pub const INK: Color = Color::hex(0x2E2014);
const FADED: Color = Color::hex(0x6B5638);
/// What is on, or picked out: sealing-wax red, with writing on it the
/// colour of the lightest paper.
const SEAL: Color = Color::hex(0x9C3D2A);
const LIGHT: Color = Color::hex(0xF7EED6);
/// A row that is picked from a list.
const PICKED: Color = Color::hex(0x7A5A34);
/// Edges: leather round what floats, a softer brown everywhere else.
const LEATHER: Color = Color::hex(0x5D4023);
const EDGE: Color = Color::hex(0x5A4326);
/// Behind everything, where nothing is drawn: the dark of the world's
/// own edge.
const DARK: Color = Color::hex(0x271A1C);
/// How strongly surfaces are shaded top to bottom: paper is nearly flat.
const SHADE: f64 = 0.05;

/// A theme dressed as parchment: its colours, and lines bold enough to
/// see. Its sizes are kept.
pub fn dressed(theme: &Theme) -> Theme {
    Theme {
        bg: DARK,
        title: Gradient::shaded(SHEET, SHADE),
        header: Gradient::shaded(SHEET, SHADE),
        panel: Gradient::shaded(PAPER, SHADE),
        widget: Gradient::shaded(BUTTON, SHADE),
        field: RECESS,
        text: INK,
        text_dim: FADED,
        accent: SEAL,
        accent_text: LIGHT,
        selection: PICKED,
        selection_text: LIGHT,
        focus: LEATHER,
        close: SEAL,
        border_dark: EDGE,
        border_light: LIGHT,
        dialog_outline: LEATHER,
        bevel: true,
        accent_buttons: true,
        gradient: SHADE,
        border_width: 2.0,
        radius: 8.0,
        ..theme.clone()
    }
}

/// Dress a theme as parchment, if it is not already. Gives whether it
/// changed.
pub fn dress(theme: &mut Theme) -> bool {
    let wanted = dressed(theme);
    let changed = *theme != wanted;
    if changed {
        *theme = wanted;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How bright a colour is, 0 to 1, as the eye weighs it.
    fn light(c: Color) -> f64 {
        0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
    }

    #[test]
    fn dressing_changes_the_look_and_keeps_the_sizes() {
        let plain = Theme { text_size: 40.0, widget_height: 70.0, ..Theme::default() };
        let mut theme = plain.clone();
        assert!(dress(&mut theme));
        assert_eq!((theme.text_size, theme.widget_height, theme.heading_size), (40.0, 70.0, plain.heading_size));
        assert_eq!((theme.text, theme.panel.mid()), (INK, Gradient::shaded(PAPER, SHADE).mid()));
        // Dressed once is dressed: nothing to do the second time.
        assert!(!dress(&mut theme));
    }

    #[test]
    fn ink_reads_on_everything_it_is_written_on() {
        let theme = dressed(&Theme::default());
        for (what, ground) in [("the page", theme.panel.mid()), ("a sheet", theme.header.mid()), ("a button", theme.widget.mid()), ("a slot", theme.field)] {
            assert!(light(ground) - light(theme.text) > 0.35, "ink on {what}");
            assert!(light(ground) - light(theme.text_dim) > 0.2, "faded ink on {what}");
        }
        // And light writing on what is picked out.
        assert!(light(theme.accent_text) - light(theme.accent) > 0.45);
        assert!(light(theme.selection_text) - light(theme.selection) > 0.45);
        // Slots are darker than the page they are sunk into, and buttons
        // stand out from it.
        assert!(light(theme.panel.mid()) - light(theme.field) > 0.12 && light(theme.panel.mid()) - light(theme.widget.mid()) > 0.08);
    }
}
