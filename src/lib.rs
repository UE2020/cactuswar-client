//! An IO game inspired by https://domfights.io and https://game.cxii.org
//! The game has a C++ backend and a Rust frontend compiled into WebAssembly.
//! `wasm-bindgen` is used to automatically generate WASM glue-code.

#![allow(unused_must_use)]

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use num_traits::FromPrimitive;

pub mod wrapper;
#[macro_use]
pub mod macros;
pub mod binary;
pub mod draw;
pub mod engine;
pub mod protocol;
pub mod util;

use draw::*;
use protocol::Protocol;
use util::Lerp;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

#[allow(dead_code)]
fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

#[allow(dead_code)]
fn body() -> web_sys::HtmlElement {
    document().body().expect("document should have a body")
}

#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // compositing layer
    let composite = document.get_element_by_id("composite").unwrap();
    let composite: web_sys::HtmlCanvasElement = composite
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

    let composite_ctx = composite
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let input_element = document.get_element_by_id("chatInput").unwrap();
    let input_element: web_sys::HtmlInputElement = input_element
        .dyn_into::<web_sys::HtmlInputElement>()
        .map_err(|_| ())
        .unwrap();

    let chat_div = document.get_element_by_id("chat").unwrap();
    let chat_div: web_sys::HtmlDivElement = chat_div
        .dyn_into::<web_sys::HtmlDivElement>()
        .map_err(|_| ())
        .unwrap();

    let world = Rc::new(RefCell::new(engine::World {
        yourself: engine::Tank {
            id: 0,
            name: wrapper::query_name(),
            mockup: 0,
            position: util::Vector2 { x: 0., y: 0. },
            net_position: util::Vector2 { x: 0., y: 0. },
            velocity: util::Vector2 { x: 0., y: 0. },
            rotation: 0.,
            light: engine::Light {
                x: 0.,
                y: 0.,
                r: 500.,
                color: String::from("rgba(252, 250, 157, 0.2)"),
            },
            net_rotation: 0.,
            yourself: true,
            health: util::Scalar::new(1.),
            radius: 50,
            damaged: false,
            opacity: util::Scalar::new(1.),
            message: String::new(),
        },
        state: engine::GameState::new(),
        input: engine::Input::new(),
        camera: util::Vector2 { x: 0., y: 0. },
        size: util::Scalar::new(1.),
        canvas,
        ctx,
        composite_ctx,
        composite,
        entities: HashMap::new(),
        mockups: None,
        chat_input: input_element,
        chat_div,
        leaderboard: protocol::LeaderboardPacket {
            entries: Vec::new(),
        },
    }));

    let ws = WebSocket::new(wrapper::query_server_url().as_str()).expect("Failed to connect!");
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let mouse_position = Rc::new(Cell::new((0., 0.)));
    let win_size = Rc::new(Cell::new([1., 1.]));
    let mut frame = 0;

    let mut last_frame_time = js_sys::Date::now();
    let mut delta = 1.;

    // requestAnimationFrame
    let f = Rc::new(RefCell::new(None));
    {
        // To get the moust position use:
        // mouse_position.get().0 / fov_math, mouse_position.get().1 / fov_math
        clone!(mouse_position);
        clone!(win_size);
        clone!(ws);
        clone!(world);

        let g = f.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let mut world = world.borrow_mut();

            frame += 1;
            let now = window().performance().unwrap().now();
            delta = ((now - last_frame_time) / 16.).lerp(delta, 0.7);
            if delta < 1.0 {
                delta = 1.0;
            }
            if now - last_frame_time > 500. {
                delta = 1.0;
            }
            last_frame_time = now;
            // set width and height
            world
                .canvas
                .set_width(window().inner_width().unwrap().as_f64().unwrap() as u32);
            world
                .canvas
                .set_height(window().inner_height().unwrap().as_f64().unwrap() as u32);
            world
                .composite
                .set_width(window().inner_width().unwrap().as_f64().unwrap() as u32);
            world
                .composite
                .set_height(window().inner_height().unwrap().as_f64().unwrap() as u32);

            win_size.set([
                window().inner_width().unwrap().as_f64().unwrap(),
                window().inner_height().unwrap().as_f64().unwrap(),
            ]);

            let fov = match &world.mockups {
                Some(mockups) => mockups[world.yourself.mockup as usize].fov,
                None => 20,
            };
            let design_resolution = [112.5 * fov as f64, 112.5 * fov as f64];
            // fov
            world.ctx.scale(
                (win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]),
                (win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]),
            );

            world.composite_ctx.scale(
                (win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]),
                (win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]),
            );

            let center_x = world.canvas.width() as f64
                / 2.
                / ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]));
            let center_y = world.canvas.height() as f64
                / 2.
                / ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]));

            let fov_math = (win_size.get()[0] + win_size.get()[1])
                / (design_resolution[0] + design_resolution[1]);
            world.ctx.translate(center_x, center_y);
            world.ctx.translate(-world.camera.x, -world.camera.y);
            world.composite_ctx.translate(center_x, center_y);
            world
                .composite_ctx
                .translate(-world.camera.x, -world.camera.y);

            if !world.state.chat_open && !world.state.is_dead() {
                if world.input.W {
                    world.yourself.velocity.y -= 1.;
                } else if world.input.S {
                    world.yourself.velocity.y += 1.;
                }

                if world.input.A {
                    world.yourself.velocity.x -= 1.;
                } else if world.input.D {
                    world.yourself.velocity.x += 1.;
                }
            }

            world.input.mouse_position = util::Vector2 {
                x: ((mouse_position.get().0 / fov_math) + world.camera.x - center_x) as i16,
                y: ((mouse_position.get().1 / fov_math) + world.camera.y - center_y) as i16,
            };
            // clear the canvas
            world.ctx.set_fill_style(v8!("rgba(30, 23, 0, 1.0)"));
            let w = world.size.value as f64;
            let h = world.size.value as f64;
            world.ctx.fill_rect(0., 0., w, h);
            world.composite_ctx.clear_rect(0., 0., w, h);

            world.ctx.set_line_join("round");

            // grid
            draw_grid(&world.ctx, w.ceil(), h.ceil());

            // render
            let shadows = world.draw_entities(delta);

            if ws.ready_state() == 1 && !world.state.is_dead() {
                if !world.state.chat_open {
                    util::talk(&ws, &protocol::InputPacket::from_input(world.input));
                } else {
                    util::talk(
                        &ws,
                        &protocol::InputPacket::from_input(engine::Input::new()),
                    );
                }
            }

            world.yourself.rotation = (world.input.mouse_position.y as f64
                - world.yourself.position.y)
                .atan2(world.input.mouse_position.x as f64 - world.yourself.position.x);

            draw_light_with_shadows(
                &world.ctx,
                &world.composite_ctx,
                world.yourself.position.x,
                world.yourself.position.y,
                1300.,
                "rgba(252, 250, 157, 0.25)",
                shadows,
                world.size.value as u16,
            );

            // gui pass
            world.ctx.translate(-center_x, -center_y);
            world.ctx.translate(world.camera.x, world.camera.y);

            world.composite_ctx.translate(-center_x, -center_y);
            world
                .composite_ctx
                .translate(world.camera.x, world.camera.y);

            world.ctx.scale(
                ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]))
                    .recip(),
                ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]))
                    .recip(),
            );

            world.composite_ctx.scale(
                ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]))
                    .recip(),
                ((win_size.get()[0] + win_size.get()[1])
                    / (design_resolution[0] + design_resolution[1]))
                    .recip(),
            );

            world.ctx.scale(
                (win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.),
                (win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.),
            );

            world.composite_ctx.scale(
                (win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.),
                (win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.),
            );

            let center_x = world.canvas.width() as f64
                / 2.
                / ((win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.));
            let center_y = world.canvas.height() as f64
                / 2.
                / ((win_size.get()[0] + win_size.get()[1]) / (2000. + 2000.));

            world.state.level.update(0.05 * delta as f32);

            world.composite_ctx.set_font("75px \"Fira Sans\"");
            world.composite_ctx.save();
            world
                .composite_ctx
                .set_shadow_blur(((frame as f64 / 50.).sin() + 2.) * 20.);
            world.composite_ctx.set_shadow_color("#f28900");
            world.composite_ctx.set_fill_style(v8!("#f28900"));
            world.composite_ctx.set_stroke_style(v8!("#000000"));
            world.composite_ctx.set_line_width(10.);
            world.composite_ctx.fill_text("CactusWar.io", 50., 100.);
            world.composite_ctx.set_shadow_blur(0.);

            world.yourself.opacity.update(0.5 * delta as f32);
            match world.mockups {
                Some(ref mockups) => {
                    world.composite_ctx.set_font("50px \"Fira Sans\"");

                    world.composite_ctx.set_fill_style(v8!("#ffffff"));

                    let level_percentage = world.state.level.value.fract();

                    let text = &*format!(
                        "Level {} {}",
                        world.state.level.tv as u32, mockups[world.yourself.mockup as usize].name
                    );
                    let bar_length = 800.;
                    const BAR_WIDTH: f64 = 79.;
                    const LONGER_BAR_WIDTH: f64 = BAR_WIDTH + 20.;
                    draw_rect_no_correction(
                        &world.composite_ctx,
                        center_x * 2. - 560. - bar_length / 2.,
                        center_y * 2. - 260.,
                        bar_length + 130.,
                        245.,
                        0.,
                        "#121212aa",
                    );
                    draw_bar(
                        &world.composite_ctx,
                        center_x * 2. - 500. - bar_length / 2.,
                        center_x * 2. - 500. + bar_length / 2.,
                        center_y * 2. - 85.,
                        LONGER_BAR_WIDTH,
                        "#000000",
                    );

                    draw_bar(
                        &world.composite_ctx,
                        center_x * 2. - 500. - bar_length / 2.,
                        (center_x * 2. - 500. - bar_length / 2.)
                            + bar_length * level_percentage as f64,
                        center_y * 2. - 85.,
                        BAR_WIDTH,
                        "#00FFFF",
                    );

                    world.composite_ctx.set_shadow_color("#232323");
                    world.composite_ctx.set_shadow_blur(5.);
                    world.composite_ctx.stroke_text(
                        text,
                        center_x * 2. - 500. - bar_length / 2.,
                        center_y * 2. - 70.,
                    );
                    world.composite_ctx.fill_text(
                        text,
                        center_x * 2. - 500. - bar_length / 2.,
                        center_y * 2. - 70.,
                    );

                    world.composite_ctx.set_font("66px \"Fira Sans\"");
                    let text = if world.yourself.name.as_str().is_empty() {
                        "Unnamed Tank"
                    } else {
                        world.yourself.name.as_str()
                    };
                    world.composite_ctx.stroke_text(
                        text,
                        center_x * 2. - 500. - bar_length / 2.,
                        center_y * 2. - 172.5,
                    );
                    world.composite_ctx.fill_text(
                        text,
                        center_x * 2. - 500. - bar_length / 2.,
                        center_y * 2. - 172.5,
                    );

                    // leaderboard
                    world.composite_ctx.save();
                    world.composite_ctx.set_shadow_blur(0.);
                    world.composite_ctx.set_font("50px \"Fira Sans\"");
                    world.composite_ctx.set_fill_style(v8!("#ffffff"));

                    draw_rect_no_correction(
                        &world.composite_ctx,
                        (center_x * 2. - 500.).floor() - 55.0,
                        20.0,
                        525.0,
                        120.0 + world.leaderboard.entries.len() as f64 * 65.0,
                        0.,
                        "#121212aa",
                    );

                    let text = "Leaderboard";
                    world
                        .composite_ctx
                        .stroke_text(text, (center_x * 2. - 465.).floor(), 80.0);
                    world
                        .composite_ctx
                        .fill_text(text, (center_x * 2. - 465.).floor(), 80.0);

                    world.composite_ctx.set_font("30px \"Fira Sans\"");
                    world.composite_ctx.set_line_width(5.);

                    if let Some(entry) = world.leaderboard.entries.get(0) {
                        let max_level = entry.level;
                        for (index, entry) in world.leaderboard.entries.iter().enumerate() {
                            let text = &*format!(
                                "{}  ➤  Level {}",
                                if entry.name.as_str().is_empty() {
                                    "Unnamed Tank"
                                } else {
                                    entry.name.as_str()
                                },
                                entry.level as u32,
                            );
    
                            let measurement = world.composite_ctx.measure_text(text).unwrap().width();
    
                            let level = entry.level / max_level;
    
                            draw::draw_bar(
                                &world.composite_ctx,
                                (center_x * 2. - 500.).floor(),
                                (center_x * 2. - 500.).floor() + 400.0,
                                (150. + index as f64 * 65.) - 10.0,
                                60.0,
                                "rgba(0, 0, 0, 1.0)",
                            );
    
                            draw::draw_bar(
                                &world.composite_ctx,
                                (center_x * 2. - 500.).floor(),
                                (center_x * 2. - 500.).floor() + (400.0 * level as f64),
                                (150. + index as f64 * 65.) - 10.0,
                                40.0,
                                "rgba(140, 140, 140, 1.0)",
                            );
    
                            world.composite_ctx.stroke_text(
                                text,
                                (center_x * 2. - 500.).floor(),
                                150. + index as f64 * 65.,
                            );
                            world.composite_ctx.fill_text(
                                text,
                                (center_x * 2. - 500.).floor(),
                                150. + index as f64 * 65.,
                            );
                        }
                    }

                    world.composite_ctx.restore();

                    // death screen
                    world
                        .state
                        .death_animation_completion
                        .update(0.2 * delta as f32);
                    if world.state.is_dead() {
                        world.state.death_animation_completion.tv = 1.0;
                        world.yourself.opacity.tv = 0.0;
                        world
                            .composite_ctx
                            .set_global_alpha(world.state.death_animation_completion.value as f64);
                        world
                            .composite_ctx
                            .set_fill_style(v8!("rgba(0, 0, 0, 0.2)"));
                        world
                            .composite_ctx
                            .fill_rect(0.0, 0.0, center_x * 2., center_y * 2.);
                        world.state.death_animation_completion.tv = 1.0;
                        world.composite_ctx.set_font("104px \"Fira Sans\"");
                        world.composite_ctx.set_fill_style(v8!("#ffffff"));
                        let text = "YOU DIED!";
                        let measurement = world.composite_ctx.measure_text(text).unwrap().width();
                        world.composite_ctx.stroke_text(
                            text,
                            center_x - measurement / 2.,
                            center_y,
                        );
                        world
                            .composite_ctx
                            .fill_text(text, center_x - measurement / 2., center_y);

                        world.composite_ctx.set_font("44px \"Fira Sans\"");

                        let duration: humantime::Duration =
                            std::time::Duration::from_secs(world.state.time_alive() as u64).into();
                        let text = format!("Time alive: {}", duration);
                        let text = text.as_str();
                        let measurement = world.composite_ctx.measure_text(text).unwrap().width();
                        world.composite_ctx.stroke_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 100.,
                        );
                        world.composite_ctx.fill_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 100.,
                        );

                        let text = format!("Level: {}", world.state.level.value.floor());
                        let text = text.as_str();
                        let measurement = world.composite_ctx.measure_text(text).unwrap().width();
                        world.composite_ctx.stroke_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 160.,
                        );
                        world.composite_ctx.fill_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 160.,
                        );

                        world.composite_ctx.set_global_alpha(
                            world.state.death_animation_completion.value as f64
                                * (((frame as f64 / 7.5).sin() + 1.) / 2.),
                        );

                        world.composite_ctx.set_font("24px \"Fira Sans\"");
                        let text = "(Press Enter To Continue)";
                        let measurement = world.composite_ctx.measure_text(text).unwrap().width();
                        world.composite_ctx.stroke_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 220.,
                        );
                        world.composite_ctx.fill_text(
                            text,
                            center_x - measurement / 2.,
                            center_y + 220.,
                        );
                    } else {
                        world.state.death_animation_completion.tv = 0.0;
                        world.yourself.opacity.tv = 1.0;
                    }
                }
                None => (),
            }

            world.ctx.restore();

            // set the camera position
            world.camera.x = world
                .camera
                .x
                .lerp(world.yourself.net_position.x, 0.075 * delta);
            world.camera.y = world
                .camera
                .y
                .lerp(world.yourself.net_position.y, 0.075 * delta);

            // Schedule ourself for another requestAnimationFrame callback.
            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<dyn FnMut()>));
        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    // mousemove
    {
        clone!(mouse_position);
        clone!(win_size);
        clone!(world);
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            mouse_position.set((event.page_x() as f64, event.page_y() as f64));
        }) as Box<dyn FnMut(_)>);
        world
            .borrow()
            .composite
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }

    // onmessage
    {
        #[allow(unused_variables)]
        let cloned_ws = ws.clone();
        clone!(world);
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let mut world = world.borrow_mut();
                let array = js_sys::Uint8Array::new(&abuf);
                #[allow(unused_variables)]
                let len = array.byte_length() as usize;
                let data = array.to_vec();
                let mut buf = binary::StreamPeerBuffer::new();
                buf.set_data_array(data);
                match FromPrimitive::from_u8(buf.get_u8()) {
                    Some(protocol::Packet::Census) => {
                        // Decode the census and get your own id
                        // We'll need to check our own id against every entity later on.
                        let census = protocol::Census::decode(buf);
                        let yourself_id = world.yourself.id;

                        world.size.tv = census.arena_size as f32;
                        world.state.level.tv = census.level;

                        // Lets check if any entities need to be removed from our cache.
                        // We can just look at all the entities in our cache that are not in the census.
                        // Think of it as a git diff but we can only see subtractions.

                        world.entities.retain(|k, e| match e {
                            engine::Entity::Bullet(entity) => {
                                if entity.opacity.value < 0.05 {
                                    false
                                } else {
                                    if census.entities.contains_key(k) {
                                        true
                                    } else {
                                        entity.opacity.set_update(0.0, 0.1);
                                        entity.scale.set_update(2.0, 0.1);
                                        true
                                    }
                                }
                            }

                            engine::Entity::Tank(entity) => {
                                if entity.opacity.value < 0.05 {
                                    false
                                } else {
                                    if census.entities.contains_key(k) {
                                        true
                                    } else {
                                        entity.opacity.set_update(0.0, 0.1);
                                        true
                                    }
                                }
                            }

                            engine::Entity::Shape(entity) => {
                                if entity.opacity.value < 0.05 {
                                    false
                                } else {
                                    if census.entities.contains_key(k) {
                                        true
                                    } else {
                                        entity.opacity.set_update(0.0, 0.1);
                                        true
                                    }
                                }
                            }
                        });

                        for (id, entity) in &census.entities {
                            // If the entity is yourself, update `yourself`.
                            if *id == yourself_id {
                                match entity {
                                    protocol::Entity::Tank(t) => {
                                        world.yourself.net_position = util::Vector2 {
                                            x: t.position.x as f64,
                                            y: t.position.y as f64,
                                        };
                                        world.yourself.mockup = t.mockup;
                                        world.yourself.radius = t.radius;
                                        if world.yourself.health.tv > t.health {
                                            world.yourself.damaged = true;
                                        }
                                        world.yourself.health.tv = t.health;
                                        world.yourself.message = t.message.clone();
                                    }
                                    _ => {}
                                }
                            } else {
                                // Check what type the foreign entity is
                                match entity {
                                    // It's a tank!
                                    // Check whether it's in our local cache already
                                    protocol::Entity::Tank(census_entity) => {
                                        // It's in our local cache, lets update our cache.
                                        if world.entities.contains_key(id) {
                                            // `game_entity` is our cached entity.
                                            let game_entity = world.entities.get_mut(id).unwrap();
                                            match game_entity {
                                                engine::Entity::Tank(e) => {
                                                    e.net_position = util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    };
                                                    e.net_rotation = census_entity.rotation as f64;
                                                    e.mockup = census_entity.mockup;
                                                    if !e.yourself {
                                                        e.velocity = util::Vector2 {
                                                            x: census_entity.velocity.x as f64 / 2.,
                                                            y: census_entity.velocity.y as f64 / 2.,
                                                        };
                                                    }
                                                    if census_entity.health < e.health.tv {
                                                        e.damaged = true;
                                                    }
                                                    e.health.tv = census_entity.health;
                                                    e.radius = census_entity.radius;
                                                    e.message = census_entity.message.clone();
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            // it's not in our cache, lets add it.
                                            world.entities.insert(
                                                *id,
                                                engine::Entity::Tank(engine::Tank {
                                                    id: *id,
                                                    name: census_entity.name.clone(),
                                                    position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    net_position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    velocity: util::Vector2 {
                                                        x: census_entity.velocity.x as f64 / 4.,
                                                        y: census_entity.velocity.y as f64 / 4.,
                                                    },
                                                    rotation: census_entity.rotation as f64,
                                                    light: engine::Light {
                                                        x: 0.,
                                                        y: 0.,
                                                        r: 1000.,
                                                        color: String::from(
                                                            "rgba(252, 250, 157, 0.4)",
                                                        ),
                                                    },
                                                    yourself: false,
                                                    net_rotation: census_entity.rotation as f64,
                                                    mockup: census_entity.mockup,
                                                    radius: census_entity.radius,
                                                    health: util::Scalar::new(census_entity.health),
                                                    damaged: false,
                                                    opacity: util::Scalar::new(1.),
                                                    message: census_entity.message.clone(),
                                                }),
                                            );
                                        }
                                    }
                                    protocol::Entity::Shape(census_entity) => {
                                        // It's in our local cache, lets update our cache.
                                        if world.entities.contains_key(id) {
                                            // `game_entity` is our cached entity.
                                            let game_entity = world.entities.get_mut(id).unwrap();
                                            match game_entity {
                                                engine::Entity::Shape(e) => {
                                                    e.net_position = util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    };
                                                    if census_entity.health < e.health {
                                                        e.damaged = true;
                                                        e.needs_redraw = true;
                                                    }
                                                    e.health = census_entity.health;
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            // it's not in our cache, lets add it.
                                            world.entities.insert(
                                                *id,
                                                engine::Entity::Shape(engine::Shape {
                                                    id: *id,
                                                    position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    net_position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    sides: ((js_sys::Math::random() * 10.) + 10.)
                                                        as u8,
                                                    velocity: util::Vector2 { x: 0., y: 0. },
                                                    rotation: census_entity.position.x as f32
                                                        + census_entity.position.y as f32,

                                                    health: census_entity.health,
                                                    damaged: false,

                                                    opacity: util::Scalar::new(1.),
                                                    cached_tex: None,
                                                    needs_redraw: true,
                                                    radius: census_entity.radius,
                                                }),
                                            );
                                        }
                                    }
                                    protocol::Entity::Bullet(census_entity) => {
                                        // It's in our local cache, lets update our cache.
                                        if world.entities.contains_key(id) {
                                            // `game_entity` is our cached entity.
                                            let game_entity = world.entities.get_mut(id).unwrap();
                                            match game_entity {
                                                engine::Entity::Bullet(e) => {
                                                    e.net_position = util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    };

                                                    e.velocity = util::Vector2 {
                                                        x: census_entity.velocity.x as f64 / 3.,
                                                        y: census_entity.velocity.y as f64 / 3.,
                                                    };
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            let color = if census_entity.owner == world.yourself.id
                                            {
                                                String::from("#00e6f2")
                                            } else {
                                                String::from("#f28900")
                                            };

                                            // it's not in our cache, lets add it.
                                            world.entities.insert(
                                                *id,
                                                engine::Entity::Bullet(engine::Bullet {
                                                    id: *id,
                                                    position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    net_position: util::Vector2 {
                                                        x: census_entity.position.x as f64,
                                                        y: census_entity.position.y as f64,
                                                    },
                                                    radius: census_entity.radius,
                                                    velocity: util::Vector2 {
                                                        x: census_entity.velocity.x as f64 / 3.,
                                                        y: census_entity.velocity.y as f64 / 3.,
                                                    },
                                                    opacity: util::Scalar::new(1.),
                                                    scale: util::Scalar::new(1.),
                                                    cached_tex: None,
                                                    color,
                                                }),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(protocol::Packet::Handshake) => {
                        let res = protocol::HandshakePacket::decode(buf);
                        do_success_log!(
                            "Init packet has been acknowledged by the server! Our id is: {}",
                            res.id
                        );
                        do_info_log!("Mockups: {:?}", res.mockups);
                        world.mockups = Some(res.mockups);
                        world.yourself.id = res.id;
                    }
                    Some(protocol::Packet::Death) => {
                        let res = protocol::DeathPacket::decode(buf);
                        do_info_log!(
                            "The server has delivered the unfortunate news of our death. We lived for {} seconds",
                            res.time_alive
                        );
                        world.state.player_state = engine::PlayerState::Dead(res.time_alive);
                    }
                    Some(protocol::Packet::Leaderboard) => {
                        world.leaderboard = protocol::LeaderboardPacket::decode(buf);
                    }
                    None => do_error_log!("Unknown packet id!"),
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    // onopen
    {
        let cloned_ws = ws.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            do_success_log!("WebSocket has opened. Sending init packet.");
            cloned_ws.send_with_u8_array(
                Box::new(protocol::InitPacket {
                    // allocate an InitPacket on the heap
                    name: wrapper::query_name(),
                })
                .encode()
                .cursor
                .get_ref()
                .as_slice(),
            );
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    // onerror
    {
        let onerror_callback = Closure::wrap(Box::new(move |_: web_sys::ErrorEvent| {
            do_error_log!("Failed to connect to WebSocket! Please check your network connection!");
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    // onkeydown
    {
        let cloned_world = world.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut world = cloned_world.borrow_mut();
            match event.key_code() {
                87 => world.input.W = true,
                65 => world.input.A = true,
                83 => world.input.S = true,
                68 => world.input.D = true,
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        window()
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }

    // onmousedown
    {
        let cloned_world = world.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let mut world = cloned_world.borrow_mut();
            world.input.mouse_down = true;
        }) as Box<dyn FnMut(_)>);
        window()
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }

    // onmouseup
    {
        let cloned_world = world.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let mut world = cloned_world.borrow_mut();
            world.input.mouse_down = false;
        }) as Box<dyn FnMut(_)>);
        window()
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }

    // onkeyup
    {
        let cloned_world = world.clone();
        clone!(ws);
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut world = cloned_world.borrow_mut();
            match event.key_code() {
                87 => world.input.W = false,
                65 => world.input.A = false,
                83 => world.input.S = false,
                68 => world.input.D = false,
                13 => {
                    event.prevent_default();
                    match world.state.player_state {
                        engine::PlayerState::Alive => {
                            if world.state.chat_open {
                                // send
                                util::talk(
                                    &ws,
                                    &protocol::MessagePacket {
                                        message: world.chat_input.value(),
                                    },
                                );
                                world.chat_input.set_value("");
                                world.chat_div.style().set_property("display", "none");
                                world.state.chat_open = false;
                            } else {
                                world.state.chat_open = true;
                                world.chat_input.set_value("");
                                world.chat_div.style().set_property("display", "block");
                                world.chat_input.focus();
                            }
                        }
                        engine::PlayerState::Dead(_) => {
                            world.state.player_state = engine::PlayerState::Alive;
                            world.state.death_animation_completion.tv = 0.0;
                            world.yourself.opacity.tv = 1.0;
                            util::talk(&ws, &protocol::RespawnPacket);
                        }
                    }
                }
                27 => {
                    world.state.chat_open = false;
                    world.chat_div.style().set_property("display", "none");
                }
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        window()
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }
}
