use crate::wrapper;
use wasm_bindgen::prelude::*;
use web_sys::*;
use wasm_bindgen::JsCast;

const OUTLINE_WIDTH: f64 = 9.;

/// Draw a light.
///
/// It's recommended that the light color has an opacity of 0.3.
pub fn draw_light(ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64, color: &str) {
    ctx.save();
    let grd = ctx
        .create_radial_gradient(r + x, r + y, r / 300., r + x, r + y, r)
        .unwrap();
    grd.add_color_stop(0., color);
    grd.add_color_stop(1., "rgba(0,0,0,0)");
    ctx.set_fill_style(v8!(grd));
    ctx.translate(-r, -r);
    ctx.fill_rect(x, y, r * 2., r * 2.);
    ctx.restore();
}

/// Draw a light with shadows
///
/// It's recommended that the light color has an opacity of 0.3.
pub fn draw_light_with_shadows(ctx: &CanvasRenderingContext2d, off_ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64, color: &str, shadows: Vec<crate::engine::Quadrilateral>) {   
    off_ctx.clear_rect(0., 0., 12000., 12000.); 
    draw_light(&off_ctx, x, y, r, color);

    off_ctx.save();
    off_ctx.begin_path();
    for shadow in shadows {
        off_ctx.move_to(shadow.0.x, shadow.0.y);
        off_ctx.line_to(shadow.1.x, shadow.1.y);
        off_ctx.line_to(shadow.3.x, shadow.3.y);
        off_ctx.line_to(shadow.2.x, shadow.2.y);
        off_ctx.line_to(shadow.0.x, shadow.0.y);
    }
    off_ctx.clip();
    off_ctx.clear_rect(0., 0., 12000., 12000.);
    off_ctx.restore();
}

/// Draw a rectangle (with rotation).
pub fn draw_rect(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
    color: &str,
) {
    ctx.save();
    ctx.set_line_width(OUTLINE_WIDTH);
    ctx.set_fill_style(v8!(color));
    ctx.translate(x, y);
    ctx.rotate(r);
    ctx.fill_rect(-w / 2., -h / 2., w, h);
    ctx.set_stroke_style(&wrapper::pSBC(-0.4, color, false, true)); // mic colors using pSBC
    ctx.stroke_rect((-w + 5.) / 2., (-h + 5.) / 2., w, h);
    ctx.restore();
}

/// Draw a rectangle (without rotation).
pub fn draw_rect_no_rotation(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    color: &str,
) {
    ctx.save();
    ctx.set_line_width(OUTLINE_WIDTH);
    ctx.set_fill_style(v8!(color));
    ctx.translate(x, y);
    ctx.fill_rect(-w / 2., -h / 2., w, h);
    ctx.set_stroke_style(&wrapper::pSBC(-0.4, color, false, true)); // mic colors using pSBC
    ctx.stroke_rect((-w + 5.) / 2., (-h + 5.) / 2., w, h);
    ctx.restore();
}

/// Draw a grid.
pub fn draw_grid(ctx: &CanvasRenderingContext2d, width: f64, height: f64) {
    ctx.set_stroke_style(v8!("rgba(20, 14, 0, 1.0)"));
    ctx.set_line_width(OUTLINE_WIDTH);
    // grid
    for x in 0..width as u64 / 100 {
        ctx.move_to(x as f64 * 100., 0.);
        ctx.line_to(x as f64 * 100., height);
    }
    for y in 0..height as u64 / 100 {
        ctx.move_to(0., y as f64 * 100.);
        ctx.line_to(width, y as f64 * 100.);
    }
    ctx.stroke();
}

/// Draw a polygon with a radius, an amount of sides, and an angle.
#[allow(dead_code)]
fn regular_polygon(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    sides: u8,
    angle: f64,
) {
    if sides < 3 {
        return;
    };
    ctx.begin_path();
    let _a = (std::f64::consts::PI * 2.) / sides as f64;

    for i in 0..sides {
        let theta = (i as f64 / sides as f64) * 2. * std::f64::consts::PI;
        let x2 = x + radius * (theta + angle).cos();
        let y2 = y + radius * (theta + angle).sin();
        ctx.line_to(x2, y2);
    }
    ctx.close_path();
}

/// Draw a polygon with the specified parameters.
/// An outline color will be automatically selected.
pub fn draw_polygon(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    sides: u8,
    angle: f64,
    color: &str,
) {
    ctx.save();
    ctx.set_fill_style(v8!(color));
    ctx.set_line_width(OUTLINE_WIDTH);
    ctx.set_stroke_style(&wrapper::pSBC(-0.4, color, false, true)); // mic colors using pSBC
    regular_polygon(ctx, x, y, radius, sides, angle);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
}

/// Draw a polygon with a radius, an amount of sides, and an angle.
#[allow(dead_code)]
fn regular_star(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    inner_radius: f64,
    sides: u8,
    angle: f64,
) {
    if sides < 3 {
        return;
    };
    ctx.begin_path();
    let _a = (std::f64::consts::PI * 2.) / sides as f64;

    for i in 0..sides {
        let theta = (i as f64 / sides as f64) * 2. * std::f64::consts::PI;
        if i % 2 == 0 {
            let x2 = x + radius * (theta + angle).cos();
            let y2 = y + radius * (theta + angle).sin();
            ctx.line_to(x2, y2);
        } else {
            let x2 = x + inner_radius * (theta + angle).cos();
            let y2 = y + inner_radius * (theta + angle).sin();
            ctx.line_to(x2, y2);
        }
    }
    ctx.close_path();
}

/// Draw a star
/// Draw a polygon with the specified parameters.
/// An outline color will be automatically selected.
pub fn draw_star(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    inner_radius: f64,
    sides: u8,
    angle: f64,
    color: &str,
) {
    ctx.save();
    ctx.set_fill_style(v8!(color));
    ctx.set_line_width(OUTLINE_WIDTH);
    ctx.set_stroke_style(&wrapper::pSBC(-0.4, color, false, true)); // mix colors using pSBC
    regular_star(ctx, x, y, radius, inner_radius, sides, angle);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
}

/// Draw a circle.
pub fn draw_circle(ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64, color: &str) {
    ctx.save();
    ctx.set_fill_style(v8!(color));
    ctx.set_stroke_style(&wrapper::pSBC(-0.4, color, false, true)); // mix colors using pSBC
    ctx.set_line_width(OUTLINE_WIDTH * 2.);
    ctx.begin_path();
    ctx.arc(x, y, r, 0., 2. * std::f64::consts::PI);
    ctx.stroke();
    ctx.fill();
    ctx.close_path();
    ctx.restore();
}
