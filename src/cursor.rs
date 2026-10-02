//! Procedural software cursor modelled on the Codex runtime capture: a
//! white-rimmed rounded arrowhead with a soft shadow inside a fog, tinted per agent.

use cosmic::iced::{Color, widget::image::Handle};

/// Logical size of the square sprite; the arrow tip sits at its centre.
pub const SIZE: f32 = 112.0;
/// Pixels per logical pixel, enough to stay crisp at 150 % scaling.
const SCALE: f32 = 3.0;

/// Arrowhead outline relative to the tip, in logical pixels.
const ARROW: [(f32, f32); 4] = [(0.0, 0.0), (19.0, 5.2), (11.0, 9.6), (8.0, 18.5)];
const CORNER: f32 = 2.2;
const RIM: f32 = 1.7;

fn sd_polygon(p: (f32, f32), v: &[(f32, f32)]) -> f32 {
    let dot = |a: (f32, f32), b: (f32, f32)| a.0 * b.0 + a.1 * b.1;
    let sub = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0, a.1 - b.1);
    let mut d = dot(sub(p, v[0]), sub(p, v[0]));
    let mut sign = 1.0;
    for i in 0..v.len() {
        let j = (i + v.len() - 1) % v.len();
        let e = sub(v[j], v[i]);
        let w = sub(p, v[i]);
        let t = (dot(w, e) / dot(e, e)).clamp(0.0, 1.0);
        let b = (w.0 - e.0 * t, w.1 - e.1 * t);
        d = d.min(dot(b, b));
        let c = [p.1 >= v[i].1, p.1 < v[j].1, e.0 * w.1 > e.1 * w.0];
        if c.iter().all(|&x| x) || c.iter().all(|&x| !x) {
            sign = -sign;
        }
    }
    sign * d.sqrt()
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Premultiplied "over".
fn over(dst: [f32; 4], rgb: [f32; 3], a: f32) -> [f32; 4] {
    [
        rgb[0] * a + dst[0] * (1.0 - a),
        rgb[1] * a + dst[1] * (1.0 - a),
        rgb[2] * a + dst[2] * (1.0 - a),
        a + dst[3] * (1.0 - a),
    ]
}

/// Fog and arrow body take the agent's colour; the rim stays white.
pub fn sprite(tint: Color) -> Handle {
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let fog_rgb = [mix(tint.r, 1.0, 0.35), mix(tint.g, 1.0, 0.35), mix(tint.b, 1.0, 0.35)];
    let body_rgb = [tint.r * 0.62, tint.g * 0.62, tint.b * 0.62];
    let px = (SIZE * SCALE) as u32;
    let fog_center = (8.0, 8.5);
    let mut rgba = Vec::with_capacity((px * px * 4) as usize);
    for y in 0..px {
        for x in 0..px {
            let p = (
                (x as f32 + 0.5) / SCALE - SIZE / 2.0,
                (y as f32 + 0.5) / SCALE - SIZE / 2.0,
            );
            let fog_d = (p.0 - fog_center.0).hypot(p.1 - fog_center.1);
            let fog = 0.36 * (-(fog_d / 17.0).powi(2)).exp() + 0.09 * (-(fog_d / 38.0).powi(2)).exp();
            let mut c = over([0.0; 4], fog_rgb, fog);

            let shadow_d = sd_polygon((p.0, p.1 - 1.4), &ARROW) - CORNER;
            c = over(c, [0.0, 0.0, 0.0], 0.38 * (1.0 - smoothstep(-1.0, 4.5, shadow_d)));

            let d = sd_polygon(p, &ARROW) - CORNER;
            let outer = (0.5 - d * SCALE).clamp(0.0, 1.0);
            let inner = (0.5 - (d + RIM) * SCALE).clamp(0.0, 1.0);
            c = over(c, [0.97, 0.97, 0.98], outer);
            c = over(c, body_rgb, inner);

            let a = c[3];
            let straight = |v: f32| if a > 0.0 { (v / a * 255.0).round() as u8 } else { 0 };
            rgba.extend_from_slice(&[straight(c[0]), straight(c[1]), straight(c[2]), (a * 255.0).round() as u8]);
        }
    }
    Handle::from_rgba(px, px, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_sign() {
        assert!(sd_polygon((8.0, 6.0), &ARROW) < 0.0, "inside the arrow");
        assert!(sd_polygon((-5.0, -5.0), &ARROW) > 0.0, "beyond the tip");
        assert!(sd_polygon((16.0, 16.0), &ARROW) > 0.0, "in the notch");
    }
}
