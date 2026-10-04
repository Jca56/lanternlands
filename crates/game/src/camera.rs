//! Where the view looks. World positions are in tiles; screen positions
//! are physical pixels in the window.

use lntrn_math::{Rect, Vec2};

/// Logical pixels a tile is across at zoom 1.
pub const TILE_PX: f64 = 100.0;
pub const ZOOM_MIN: f64 = 0.4;
pub const ZOOM_MAX: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The point of the world at the middle of the view.
    pub center: Vec2,
    pub zoom: f64,
}

impl Camera {
    pub fn new(center: Vec2) -> Camera {
        Camera { center, zoom: 1.0 }
    }

    /// Physical pixels a tile is across on a display of `scale`.
    pub fn tile_px(&self, scale: f64) -> f64 {
        TILE_PX * self.zoom * scale
    }

    pub fn to_screen(self, world: Vec2, view: Rect, scale: f64) -> Vec2 {
        view.center() + (world - self.center) * self.tile_px(scale)
    }

    pub fn to_world(self, screen: Vec2, view: Rect, scale: f64) -> Vec2 {
        self.center + (screen - view.center()) / self.tile_px(scale)
    }

    /// The part of the world a view of `size` pixels shows.
    pub fn visible(&self, size: Vec2, scale: f64) -> Rect {
        Rect::from_center_size(self.center, size / self.tile_px(scale))
    }

    /// Slide the world by a distance on screen, as a drag does.
    pub fn pan_px(&mut self, delta: Vec2, scale: f64) {
        self.center -= delta / self.tile_px(scale);
    }

    /// Zoom by `factor`, keeping the world under `screen` where it is.
    pub fn zoom_about(&mut self, factor: f64, screen: Vec2, view: Rect, scale: f64) {
        let before = self.to_world(screen, view, scale);
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        self.center += before - self.to_world(screen, view, scale);
    }

    /// Keep the middle of the view inside `bounds`.
    pub fn clamp_to(&mut self, bounds: Rect) {
        self.center = bounds.clamp_point(self.center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Rect = Rect::new(Vec2::new(100.0, 50.0), Vec2::new(1100.0, 650.0));

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-9
    }

    #[test]
    fn the_center_lands_in_the_middle_of_the_view() {
        let cam = Camera::new(Vec2::new(12.5, 7.25));
        assert!(close(cam.to_screen(cam.center, VIEW, 1.25), VIEW.center()));
    }

    #[test]
    fn screen_and_world_round_trip() {
        let mut cam = Camera::new(Vec2::new(-3.0, 40.0));
        cam.zoom = 0.7;
        let p = Vec2::new(321.0, 123.0);
        assert!(close(cam.to_screen(cam.to_world(p, VIEW, 1.4), VIEW, 1.4), p));
    }

    #[test]
    fn a_tile_is_tile_px_across_times_zoom_and_scale() {
        let mut cam = Camera::new(Vec2::ZERO);
        cam.zoom = 1.5;
        let a = cam.to_screen(Vec2::new(0.0, 0.0), VIEW, 1.2);
        let b = cam.to_screen(Vec2::new(1.0, 0.0), VIEW, 1.2);
        assert!((b.x - a.x - TILE_PX * 1.5 * 1.2).abs() < 1e-9);
    }

    #[test]
    fn visible_is_the_view_in_tiles_around_the_center() {
        let cam = Camera::new(Vec2::new(10.0, 10.0));
        let seen = cam.visible(VIEW.size(), 1.0);
        assert!(close(seen.min, Vec2::new(5.0, 7.0)) && close(seen.max, Vec2::new(15.0, 13.0)), "{seen:?}");
    }

    #[test]
    fn a_drag_moves_the_world_with_the_pointer() {
        let mut cam = Camera::new(Vec2::new(5.0, 5.0));
        let grabbed = cam.to_world(Vec2::new(400.0, 300.0), VIEW, 1.0);
        cam.pan_px(Vec2::new(60.0, -35.0), 1.0);
        assert!(close(cam.to_world(Vec2::new(460.0, 265.0), VIEW, 1.0), grabbed));
    }

    #[test]
    fn zooming_keeps_the_world_under_the_pointer() {
        let mut cam = Camera::new(Vec2::new(5.0, 5.0));
        let at = Vec2::new(900.0, 120.0);
        let under = cam.to_world(at, VIEW, 1.3);
        cam.zoom_about(1.25, at, VIEW, 1.3);
        assert!((cam.zoom - 1.25).abs() < 1e-12);
        assert!(close(cam.to_world(at, VIEW, 1.3), under));
    }

    #[test]
    fn zoom_stops_at_its_limits() {
        let mut cam = Camera::new(Vec2::ZERO);
        cam.zoom_about(100.0, VIEW.center(), VIEW, 1.0);
        assert_eq!(cam.zoom, ZOOM_MAX);
        cam.zoom_about(1e-6, VIEW.center(), VIEW, 1.0);
        assert_eq!(cam.zoom, ZOOM_MIN);
    }

    #[test]
    fn clamping_keeps_the_center_in_bounds() {
        let mut cam = Camera::new(Vec2::new(-20.0, 500.0));
        cam.clamp_to(Rect::new(Vec2::ZERO, Vec2::new(48.0, 32.0)));
        assert_eq!(cam.center, Vec2::new(0.0, 32.0));
    }
}
