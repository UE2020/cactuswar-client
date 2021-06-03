use crate::draw::*;
use crate::util::*;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::*;

use std::f64::consts::PI;
use crate::do_info_log;
use crate::wrapper;

/// A structure that holds all the currently pressed keys.
#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Debug, Copy, Clone)]
pub struct Input {
    pub W: bool,
    pub A: bool,
    pub S: bool,
    pub D: bool,
    pub mouse_down: bool,
    pub mouse_position: Vector2<i16>,
}

impl Input {
    /// Create an instance of Keys with no keys pressed.
    pub fn new() -> Self {
        Self {
            W: false,
            A: false,
            S: false,
            D: false,
            mouse_down: false,
            mouse_position: Vector2 { x: 0, y: 0 },
        }
    }
}

/// The Draw trait provides a basic outline for how drawable entities work.
pub trait Draw {
    /// Draw the entity.
    fn draw(&mut self, ctx: &CanvasRenderingContext2d);
}

/// A domtank of any class.
/// Domtanks will be rendered based on their mockup id.
pub struct Tank {
    pub id: u32,
    pub name: String,
    pub position: Vector2<f64>,
    pub net_position: Vector2<f64>,
    pub net_rotation: f64,
    pub velocity: Vector2<f64>,
    pub rotation: f64,
    pub light: Light,
    pub yourself: bool,
    pub mockup: u8,
    pub health: Scalar<f32>,
    pub radius: u16,
    pub damaged: bool,
    pub opacity: Scalar<f32>
}

impl Tank {
    fn draw(&mut self, ctx: &CanvasRenderingContext2d, mockups: &Option<Mockups>) {
        ctx.set_global_alpha(self.opacity.value as f64);
        self.position.x = self.position.x.lerp(self.net_position.x, 0.05);
        self.position.y = self.position.y.lerp(self.net_position.y, 0.05);

        if !self.yourself {
            self.rotation = lerp_angle(self.rotation, self.net_rotation, 0.3);

            ctx.set_font("900 48px \"Overpass\"");
            ctx.save();
            ctx.set_fill_style(v8!("#ffffff"));
            ctx.set_stroke_style(v8!("#000000"));
            ctx.set_line_width(20.);
            // measure text
            let measurement = ctx.measure_text(self.name.as_str()).unwrap().width();
            ctx.stroke_text(self.name.as_str(), self.position.x - measurement/2., self.position.y - self.radius as f64 - 80.);
            ctx.fill_text(self.name.as_str(), self.position.x - measurement/2., self.position.y - self.radius as f64 - 80.);
            ctx.restore();
        }

        self.opacity.update(0.2);
        self.health.update(0.2);

        self.position.x += self.velocity.x;
        self.position.y += self.velocity.y;

        self.velocity.x *= 0.8;
        self.velocity.y *= 0.8;

        match mockups {
            Some(mockups) => {
                let my_tank = &mockups[self.mockup as usize];
                for barrel in my_tank.barrels.iter() {
                    ctx.save();
                    ctx.translate(self.position.x, self.position.y);
                    ctx.rotate(self.rotation + barrel.angle as f64);
                    ctx.translate(self.radius as f64 * self.opacity.value as f64 * barrel.length as f64, 0. * 2.);
                    draw_rect_no_rotation(ctx, 0., 0., self.radius as f64 * self.opacity.value as f64 * barrel.length as f64 * 2., self.radius as f64 * self.opacity.value as f64 * barrel.width as f64 * 2., "rgba(20, 20, 20, 1.0)");
                    ctx.restore();
                }
            }
            None => ()
        }

        let random_chance = js_sys::Math::random() < 0.85;
        let color = if self.damaged {
            if random_chance {
                "#780000"
            } else {
                "#8a4900"
            }
        } else {
            "rgba(50, 50, 50, 1.0)"
        };


        draw_circle(
            ctx,
            self.position.x,
            self.position.y,
            self.radius as f64  * self.opacity.value as f64,
            color,
        );

        // health (percentage)
        const BAR_LENGTH: f64 = 200.;
        const BAR_DISTANCE: f64 = 80.;
        const BAR_WIDTH: f64 = 10.;
        const LONGER_BAR_WIDTH: f64 = BAR_WIDTH + (10. * 2.);
        draw_bar(ctx, self.position.x - BAR_LENGTH / 2., self.position.x + BAR_LENGTH / 2., self.position.y + self.radius as f64 + BAR_DISTANCE, LONGER_BAR_WIDTH, "#000000");
        draw_bar(ctx, self.position.x - BAR_LENGTH / 2., (self.position.x - BAR_LENGTH / 2.) + BAR_LENGTH * self.health.value as f64, self.position.y + self.radius as f64 + BAR_DISTANCE, BAR_WIDTH, "#3ea832");

        self.damaged = false;

        self.light = Light {
            x: self.position.x,
            y: self.position.y,
            r: 1300.,
            color: String::from("rgba(252, 250, 157, 0.35)"),
        };
        ctx.set_global_alpha(1.0);
    }
}

pub struct Shape {
    pub id: u32,
    pub position: Vector2<f64>,
    pub net_position: Vector2<f64>,
    pub velocity: Vector2<f64>,
    pub rotation: f32,
    pub sides: u8,
    pub health: f32,
    pub damaged: bool,

    pub opacity: Scalar<f32>,
    pub cached_tex: Option<web_sys::HtmlCanvasElement>,
    pub needs_redraw: bool
}

impl Draw for Shape {
    fn draw(&mut self, ctx: &CanvasRenderingContext2d) {
        self.position.x = self.position.x.lerp(self.net_position.x, 0.05);
        self.position.y = self.position.y.lerp(self.net_position.y, 0.05);

        self.opacity.update(0.1);

        if self.sides % 2 != 0 {
            self.sides += 1;
        }

        let tex = {
            match &self.cached_tex {
                Some(canvas) => {
                    canvas.clone()
                }
                None => {
                    let off_can = crate::document().create_element("canvas").unwrap();
                    let off_can: web_sys::HtmlCanvasElement = off_can
                        .dyn_into::<web_sys::HtmlCanvasElement>()
                        .map_err(|_| ())
                        .unwrap();
                    off_can.set_width(150 * 2);
                    off_can.set_height(150 * 2);
                    off_can.clone()
                }
            }
        };

        if self.needs_redraw {
            let off_ctx = tex
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            let random_chance = js_sys::Math::random() < 0.85;

            let color = if self.damaged {
                if random_chance {
                    "#780000"
                } else {
                    "#8a4900"
                }
            } else {
                "#002606"
            };
            let darker_color = if self.damaged {
                if random_chance {
                    "#570000"
                } else {
                    "#572e00"
                }
            } else {
                "#001d03"
            };

            draw_star(
                &off_ctx,
                150.,
                150.,
                70.,
                150.,
                self.sides,
                self.rotation as f64,
                "#1a1a1a",
            );

            draw_circle(&off_ctx, 150., 150., 100., color);

            draw_star(
                &off_ctx,
                150.,
                150.,
                40.,
                70.,
                self.sides,
                self.rotation as f64,
                darker_color,
            );

            if self.damaged {
                self.needs_redraw = true;
            } else {
                self.needs_redraw = false;
            }
            self.damaged = false;
        }

        ctx.set_global_alpha(self.opacity.value as f64);
        ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
            &tex,
            self.position.x - tex.width() as f64 * self.opacity.value as f64 / 2.,
            self.position.y - tex.height() as f64 * self.opacity.value as f64 / 2.,
            tex.width() as f64 * self.opacity.value as f64,
            tex.height() as f64 * self.opacity.value as f64,
        );
        ctx.set_global_alpha(1.);

        self.cached_tex = Some(tex);

    }
}

#[derive(Debug)]
pub struct Bullet {
    pub id: u32,
    pub position: Vector2<f64>,
    pub net_position: Vector2<f64>,
    pub velocity: Vector2<f64>,
    pub radius: u16,

    pub opacity: Scalar<f32>,
    pub scale: Scalar<f32>,

    pub cached_tex: Option<web_sys::HtmlCanvasElement>,

    pub color: String
}

impl Draw for Bullet {
    fn draw(&mut self, ctx: &CanvasRenderingContext2d) {
        self.position.x = self.position.x.lerp(self.net_position.x, 0.05);
        self.position.y = self.position.y.lerp(self.net_position.y, 0.05);

        self.position.x += self.velocity.x;
        self.position.y += self.velocity.y;

        self.opacity.update(0.3);
        self.scale.update(0.3);

        match &self.cached_tex {
            Some(canvas) => {
                ctx.set_global_alpha(self.opacity.value as f64);
                ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
                    &canvas,
                    self.position.x - canvas.width() as f64 * self.scale.value as f64 / 2.,
                    self.position.y - canvas.height() as f64 * self.scale.value as f64 / 2.,
                    canvas.width() as f64 * self.scale.value as f64,
                    canvas.height() as f64 * self.scale.value as f64,
                );
                ctx.set_global_alpha(1.);
            }
            None => {
                let off_can = crate::document().create_element("canvas").unwrap();
                let off_can: web_sys::HtmlCanvasElement = off_can
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .map_err(|_| ())
                    .unwrap();
                let off_ctx = off_can
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                off_can.set_width((self.radius as u32 + 9) * 2 + 300);
                off_can.set_height((self.radius as u32 + 9) * 2 + 300);

                off_ctx.set_shadow_blur(100.);
                off_ctx.set_shadow_color(self.color.as_str());
                draw_circle(
                    &off_ctx,
                    self.radius as f64 + 9. + 150.,
                    self.radius as f64 + 9. + 150.,
                    self.radius as f64,
                    self.color.as_str(),
                );
                off_ctx.set_shadow_blur(0.);

                ctx.save();
                ctx.set_global_alpha(self.opacity.value as f64);
                ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
                    &off_can,
                    self.position.x - off_can.width() as f64 * self.scale.value as f64 / 2.,
                    self.position.y - off_can.height() as f64 * self.scale.value as f64 / 2.,
                    off_can.width() as f64 * self.scale.value as f64,
                    off_can.height() as f64 * self.scale.value as f64,
                );
                ctx.restore();
                self.cached_tex = Some(off_can);
            }
        }
    }
}

/// A structure used for rendering lights.
#[derive(Clone)]
pub struct Light {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub color: String,
}

impl Draw for Light {
    fn draw(&mut self, ctx: &CanvasRenderingContext2d) {
        draw_light(ctx, self.x, self.y, self.r, &*self.color);
    }
}

/// The Entity enum allows the World class to store every Entity in 1 hashmap.
#[allow(dead_code)]
pub enum Entity {
    Tank(Tank),
    Shape(Shape),
    Bullet(Bullet),
}

/// Represents a shadow triangle (for lighting)
pub struct Quadrilateral(pub Vector2<f64>, pub Vector2<f64>, pub Vector2<f64>, pub Vector2<f64>);

pub type Mockups = Vec<crate::protocol::TankMockup>;

/// The World class manages the game state. Examples are:
/// * Input
/// * Rendering
/// * Lerp
/// * Entities
pub struct World {
    pub input: Input,
    pub camera: Vector2<f64>,
    pub size: Scalar<f32>,

    pub canvas: HtmlCanvasElement,
    pub ctx: CanvasRenderingContext2d,

    pub composite: HtmlCanvasElement,
    pub composite_ctx: CanvasRenderingContext2d,

    pub yourself: Tank,
    pub entities: HashMap<u32, Entity>,

    pub mockups: Option<Mockups>,
}

impl World {
    /// Draw all entities that aren't comprised of UI.
    pub fn draw_entities(&mut self) -> Vec<Quadrilateral> {
        self.size.update(0.075);
        let mut lights: Vec<Light> = Vec::new();

        let mut tanks = Vec::new();
        let mut cacti = Vec::new();
        let mut bullets = Vec::new();
        let mut shadows = Vec::new();
        
        for entity in self.entities.values_mut() {
            match entity {
                Entity::Tank(e) => {
                    tanks.push(e);
                    //e.draw(&self.ctx);
                    //lights.push(e.light.clone());
                }
                Entity::Shape(e) => cacti.push(e),
                Entity::Bullet(e) => bullets.push(e),
            }
        }

        for cactus in cacti {
            let angle = (self.yourself.position.y - cactus.position.y).atan2(self.yourself.position.x - cactus.position.x);
            let right_angle = angle + PI/2.;
            let left_angle = angle - PI/2.;

            let right_point = Vector2 {
                x: (right_angle.cos() * 100. * cactus.opacity.value as f64) + cactus.position.x,
                y: (right_angle.sin() * 100. * cactus.opacity.value as f64) + cactus.position.y
            };

            let left_point = Vector2 {
                x: (left_angle.cos() * 100. * cactus.opacity.value as f64) + cactus.position.x,
                y: (left_angle.sin() * 100. * cactus.opacity.value as f64) + cactus.position.y
            };

            // Now lets make a massive quad for every shadow...?
            let right_angle = (self.yourself.position.y - right_point.y).atan2(self.yourself.position.x - right_point.x);
            let left_angle = (self.yourself.position.y - left_point.y).atan2(self.yourself.position.x - left_point.x);

            let right_point2 = Vector2 {
                x: right_point.x - (right_angle.cos() * 2200.),
                y: right_point.y - (right_angle.sin() * 2200.),
            };

            let left_point2 = Vector2 {
                x: left_point.x - (left_angle.cos() * 2200.),
                y: left_point.y - (left_angle.sin() * 2200.),
            };


            shadows.push(Quadrilateral(right_point, left_point, right_point2, left_point2));

            cactus.draw(&self.ctx);
            
        }

        for tank in tanks {
            tank.draw(&self.ctx, &self.mockups);
        }

        for bullet in bullets {
            bullet.draw(&self.ctx);
        }

        self.yourself.draw(&self.ctx, &self.mockups);
        shadows
    }
}
