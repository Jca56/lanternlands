//! The game as `lntrn-app` runs it: what is uploaded once the GPU is
//! there, and the world's own pass drawn under the interface each frame.

use super::*;

impl AppHost for Game {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, images: &mut Images) {
        self.renderer = Some(WorldRenderer::new(gpu, format, &self.sprites.sheet));
        for (slot, kind) in self.pictures.props.iter_mut().zip(PropKind::ALL) {
            *slot = self.look.prop(kind).map(|id| images.add(gpu, self.sprites.picture(id)));
        }
        for (slot, image) in self.item_pictures.0.iter_mut().zip(&self.item_images) {
            *slot = image.as_ref().map(|image| images.add(gpu, image));
        }
        for (slot, image) in self.ability_pictures.0.iter_mut().zip(&self.ability_images) {
            *slot = image.as_ref().map(|image| images.add(gpu, image));
        }
        for (slot, kind) in self.pictures.monsters.iter_mut().zip(MonsterKind::ALL) {
            let standing = self.look.monster(kind).sprite(Facing::Down, Pose::Idle);
            *slot = standing.map(|(id, _)| images.add(gpu, self.sprites.picture(id)));
        }
        for (slot, kind) in self.pictures.people.iter_mut().zip(PersonKind::ALL) {
            let standing = self.look.person(kind).sprite(Facing::Down, Pose::Idle);
            *slot = standing.map(|(id, _)| images.add(gpu, self.sprites.picture(id)));
        }
        self.title_picture = self.title_image.as_ref().map(|image| images.add(gpu, image));
    }

    fn after_rebuild(&mut self, _: &Gpu, _: &mut Images, shell: &mut Shell<Self>) -> bool {
        self.keep_house(shell)
    }

    fn exiting(&mut self, _: Exit) {
        // However the game ended, the character in play is kept.
        self.autosave();
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        if self.mode == Mode::Title {
            // The painting covers the window; there is no world to draw.
            return;
        }
        let extras = self.extras();
        let Some(renderer) = self.renderer.as_mut().filter(|_| cx.window == 0) else { return };
        // The world's area, in whole pixels inside the window.
        let window = Rect::new(Vec2::ZERO, Vec2::new(cx.size[0] as f64, cx.size[1] as f64));
        let view = self.view.round().intersection(&window);
        if view.is_empty() {
            return;
        }
        let rect = [view.min.x as u32, view.min.y as u32, view.width() as u32, view.height() as u32];
        let sim = &self.sim;
        self.scene.build(&sim.world, &self.camera, view.size(), self.scale, &self.sprites, &self.look, &extras, &|x, y| sim.is_spent((x, y)));
        renderer.prepare(cx.gpu, &self.scene, [rect[2], rect[3]]);

        // Nothing more changes this frame: the passes only read.
        let renderer: &'f WorldRenderer = renderer;
        let picture = cx.graph.transient(TexDesc::color("world", rect[2], rect[3], WorldRenderer::TARGET_FORMAT));
        let backbuffer = cx.backbuffer;
        cx.graph.add_node("world", &[], &[picture], move |_, encoder, views| renderer.draw(encoder, views.get(picture)));
        cx.graph.add_node("world to window", &[picture], &[backbuffer], move |gpu, encoder, views| renderer.composite(gpu, encoder, views.get(picture), views.get(backbuffer), rect));
    }
}
