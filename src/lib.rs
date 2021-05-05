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
use engine::Draw;
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
            health: 1.,
            radius: 50,
            damaged: false,
            opacity: util::Scalar::new(1.)
        },
        input: engine::Input::new(),
        camera: util::Vector2 { x: 0., y: 0. },
        canvas,
        ctx,
        composite_ctx,
        composite,
        entities: HashMap::new(),
        mockups: None
    }));


    let ws = WebSocket::new(wrapper::query_server_url().as_str()).expect("Failed to connect!");
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let mouse_position = Rc::new(Cell::new((0., 0.)));
    let win_size = Rc::new(Cell::new([1., 1.]));

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
            let design_resolution = [2250., 2250.];
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
            world.composite_ctx.translate(-world.camera.x, -world.camera.y);

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

            world.input.mouse_position = util::Vector2 {
                x: ((mouse_position.get().0 / fov_math) + world.camera.x - center_x) as i16,
                y: ((mouse_position.get().1 / fov_math) + world.camera.y - center_y) as i16,
            };
            // clear the canvas
            world.ctx.set_fill_style(v8!("rgba(30, 23, 0, 1.0)"));
            let w = 12000.;
            let h = 12000.;
            world.ctx.fill_rect(0., 0., w, h);
            world.composite_ctx.clear_rect(0., 0., w, h);

            world.ctx.set_line_join("round");

            // grid
            draw_grid(&world.ctx, 12000., 12000.);

            // render
            let shadows = world.draw_entities();

            if ws.ready_state() == 1 {
                ws.send_with_u8_array(
                    Box::new(protocol::InputPacket::from_input(world.input))
                        .encode()
                        .cursor
                        .get_ref()
                        .as_slice(),
                );
            }

            world.yourself.rotation = (world.input.mouse_position.y as f64
                - world.yourself.position.y)
                .atan2(world.input.mouse_position.x as f64 - world.yourself.position.x);

            
            /*world.ctx.save();
            world.ctx.begin_path();
            for shadow in shadows {
                world.ctx.move_to(shadow.0.x, shadow.0.y);
                world.ctx.line_to(shadow.1.x, shadow.1.y);
                world.ctx.line_to(shadow.3.x, shadow.3.y);
                world.ctx.line_to(shadow.2.x, shadow.2.y);
                world.ctx.line_to(shadow.0.x, shadow.0.y);
            }
            world.ctx.clip();*/
            //world.ctx.set_global_composite_operation("source-out");
            draw_light_with_shadows(&world.ctx, &world.composite_ctx, world.yourself.position.x, world.yourself.position.y, 1300., "rgba(252, 250, 157, 0.25)", shadows);
            //world.ctx.set_global_composite_operation("source-over");
            // gui pass
            world.ctx.translate(-center_x, -center_y);
            world.ctx.translate(world.camera.x, world.camera.y);

            world.composite_ctx.translate(-center_x, -center_y);
            world.composite_ctx.translate(world.camera.x, world.camera.y);

            world.ctx.set_font("bold 48px Ubuntu");
            world.ctx.save();
            world.ctx.set_fill_style(v8!("#ffffff"));
            world.ctx.set_stroke_style(v8!("#000000"));
            world.ctx.set_line_width(20.);
            world.ctx.stroke_text("CactusWar.io Alpha", 50., 100.);
            world.ctx.fill_text("CactusWar.io Alpha", 50., 100.);

            match world.mockups {
                Some(ref mockups) => {
                    let measurement = world.ctx.measure_text(format!("Level 0 {}", mockups[world.yourself.mockup as usize].name).as_str()).unwrap().width();
                    world.ctx.stroke_text(format!("Level 0 {}", mockups[world.yourself.mockup as usize].name).as_str(), center_x - measurement/2., 100.);
                    world.ctx.fill_text(format!("Level 0 {}", mockups[world.yourself.mockup as usize].name).as_str(), center_x - measurement/2., 100.);
                }
                None => ()
            }

            world.ctx.restore();
            //world.ctx.save();
            //world.ctx.set_global_composite_operation("overlay");
            //world.ctx.set_fill_style(v8!("rgba(255, 255, 255, 0.5)"));
            //world.ctx.fill_rect(0., 0., 12000., 12000.);
            //world.ctx.restore();

            // set the camera position
            world.camera.x = world.camera.x.lerp(world.yourself.net_position.x, 0.075);
            world.camera.y = world.camera.y.lerp(world.yourself.net_position.y, 0.075);

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
                                        entity.opacity.tv = 0.0;
                                        entity.scale.tv = 2.0;
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
                                        entity.opacity.tv = 0.0;
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
                                        entity.opacity.tv = 0.0;
                                        true
                                    }
                                }
                            }

                            _ => census.entities.contains_key(k),
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
                                        if world.yourself.health > t.health {
                                            world.yourself.damaged = true;
                                        }
                                        world.yourself.health = t.health;
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
                                                            x: census_entity.velocity.x as f64 / 4.,
                                                            y: census_entity.velocity.y as f64 / 4.,
                                                        };
                                                    }
                                                    if census_entity.health < e.health {
                                                        e.damaged = true;
                                                    }
                                                    e.health = census_entity.health;
                                                    e.radius = census_entity.radius;
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
                                                    velocity: util::Vector2 { x: census_entity.velocity.x as f64 / 4., y: census_entity.velocity.y as f64 / 4. },
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
                                                    health: census_entity.health,
                                                    damaged: false,
                                                    opacity: util::Scalar::new(1.)
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
                                                        e.cached_tex = None;
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
                    },
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
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut world = cloned_world.borrow_mut();
            match event.key_code() {
                87 => world.input.W = false,
                65 => world.input.A = false,
                83 => world.input.S = false,
                68 => world.input.D = false,
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        window()
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener to canvas!");
        closure.forget();
    }
}
