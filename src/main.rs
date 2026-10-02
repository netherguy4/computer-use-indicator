//! Click-through overlay that shows when an agent drives the desktop through
//! computer use, styled after the Codex software cursor: a soft-glow arrow
//! that glides to each target, a click ripple, a glowing screen edge and a
//! status pill. `computer-use-indicator-proxy` sends one JSON datagram per
//! tool call. Surfaces exist only while actions keep arriving, so fullscreen
//! apps keep direct scanout the rest of the time.

mod cursor;

use std::{
    collections::HashMap,
    f32::consts::PI,
    time::{Duration, Instant},
};

use cosmic::{
    Element,
    app::{Core, Settings, Task},
    cctk::sctk::{
        output::OutputInfo,
        reexports::client::protocol::wl_output::WlOutput,
        shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    },
    iced::{
        self, Alignment, Background, Border, Color, Degrees, Length, Radians, Rotation, Shadow,
        Subscription, Vector,
        event::{
            self, PlatformSpecific,
            wayland::{Event as WaylandEvent, OutputEvent},
        },
        gradient::Linear,
        platform_specific::shell::commands::layer_surface::{
            destroy_layer_surface, get_layer_surface,
        },
        runtime::platform_specific::wayland::layer_surface::{
            IcedOutput, SctkLayerSurfaceSettings,
        },
        widget::{Space, column, container, image, pin, row, stack, text},
        window,
    },
};
use serde::Deserialize;

const IDLE_BEFORE_FADE: Duration = Duration::from_secs(8);
const FADE: f32 = 0.35;
/// The proxy delays pointer actions by this much so the glide lands first.
const MAX_GLIDE: f32 = 0.42;
const PULSE: f32 = 0.45;
const EDGE: f32 = 26.0;
const KEYS_SHOWN: f32 = 2.2;
/// Idle motion (sway, breathing) dies out this long after an action so a
/// resting overlay stops redrawing.
const SETTLE: f32 = 2.0;

#[derive(Debug, Clone, Deserialize)]
struct Action {
    #[serde(default)]
    hide: bool,
    #[serde(default)]
    agent: String,
    #[serde(default)]
    tool: String,
    #[serde(default)]
    label: String,
    x: Option<f32>,
    y: Option<f32>,
    #[serde(default)]
    keys: Vec<String>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
enum Keyboard {
    Keys(Vec<String>),
    Text(String),
}

#[derive(Debug, Clone)]
enum Msg {
    Action(Action),
    Output(OutputEvent, WlOutput),
    Tick,
}

struct Output {
    wl: WlOutput,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Output {
    fn contains(&self, (x, y): (f32, f32)) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
}

struct Glide {
    from: (f32, f32),
    ctrl: (f32, f32),
    to: (f32, f32),
    start: Instant,
    duration: f32,
    press: bool,
}

impl Glide {
    fn new(from: (f32, f32), to: (f32, f32), press: bool) -> Self {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let dist = dx.hypot(dy);
        // A gentle arc, like a hand moving a mouse, rather than a straight slide.
        let bend = 0.16 * dist;
        let (nx, ny) = if dist > 0.0 {
            (-dy / dist, dx / dist)
        } else {
            (0.0, 0.0)
        };
        Glide {
            from,
            ctrl: (
                (from.0 + to.0) / 2.0 + nx * bend,
                (from.1 + to.1) / 2.0 + ny * bend,
            ),
            to,
            start: Instant::now(),
            duration: (0.18 + dist / 4000.0).clamp(0.22, MAX_GLIDE),
            press,
        }
    }

    fn progress(&self) -> f32 {
        (self.start.elapsed().as_secs_f32() / self.duration).min(1.0)
    }

    fn position(&self) -> (f32, f32) {
        let t = self.progress();
        // Critically damped spring shape, normalised to land exactly at t = 1.
        let spring = |t: f32| 1.0 - (1.0 + 6.0 * t) * (-6.0 * t).exp();
        let e = spring(t) / spring(1.0);
        let u = 1.0 - e;
        let p = |a: f32, c: f32, b: f32| u * u * a + 2.0 * u * e * c + e * e * b;
        (
            p(self.from.0, self.ctrl.0, self.to.0),
            p(self.from.1, self.ctrl.1, self.to.1),
        )
    }
}

struct App {
    core: Core,
    outputs: Vec<Output>,
    /// Shown surface -> index into `outputs`.
    surfaces: HashMap<window::Id, usize>,
    agent: String,
    label: String,
    cursor: Option<(f32, f32)>,
    glide: Option<Glide>,
    pulse: Option<(Instant, (f32, f32))>,
    keyboard: Option<(Instant, Keyboard)>,
    last_action: Instant,
    shown_at: Instant,
    fading: Option<Instant>,
    /// Agent the sprite was tinted for.
    sprite: (String, image::Handle),
}

fn socket_path() -> std::path::PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is not set");
    std::path::Path::new(&dir).join("computer-use-indicator.sock")
}

fn socket_sub() -> Subscription<Msg> {
    Subscription::run(|| {
        iced::stream::channel(16, async |mut tx| {
            use cosmic::iced::futures::SinkExt;

            let path = socket_path();
            let _ = std::fs::remove_file(&path);
            let sock = tokio::net::UnixDatagram::bind(&path).expect("bind indicator socket");
            let mut buf = vec![0u8; 4096];
            loop {
                let Ok(n) = sock.recv(&mut buf).await else {
                    continue;
                };
                if let Ok(action) = serde_json::from_slice::<Action>(&buf[..n]) {
                    let _ = tx.send(Msg::Action(action)).await;
                }
            }
        })
    })
}

fn using_computer(agent: &str) -> String {
    let russian = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|l| !l.is_empty()))
        .is_some_and(|l| l.starts_with("ru"));
    if russian {
        format!("{agent} использует компьютер")
    } else {
        format!("{agent} is using your computer")
    }
}

fn agent_color(agent: &str) -> Color {
    match agent.to_lowercase().as_str() {
        "claude" => Color::from_rgb8(0xE0, 0x8A, 0x67),
        "codex" => Color::from_rgb8(0x4C, 0x9D, 0xFF),
        "opencode" => Color::from_rgb8(0x8B, 0xD4, 0x7E),
        _ => cosmic::theme::active().cosmic().accent_color().into(),
    }
}

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

impl App {
    fn upsert_output(&mut self, wl: WlOutput, info: &OutputInfo) {
        let (Some((x, y)), Some((width, height))) = (info.logical_position, info.logical_size)
        else {
            return;
        };
        let output = Output {
            wl,
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: height as f32,
        };
        match self.outputs.iter_mut().find(|o| o.wl == output.wl) {
            Some(existing) => *existing = output,
            None => self.outputs.push(output),
        }
    }

    fn settling(&self) -> bool {
        self.last_action.elapsed().as_secs_f32() < SETTLE + MAX_GLIDE
    }

    fn visible(&self) -> bool {
        !self.surfaces.is_empty()
    }

    fn cursor_position(&self) -> Option<(f32, f32)> {
        self.glide.as_ref().map(Glide::position).or(self.cursor)
    }

    fn show(&mut self) -> Task<Msg> {
        self.fading = None;
        if self.visible() {
            return Task::none();
        }
        self.shown_at = Instant::now();
        let tasks = self.outputs.iter().enumerate().map(|(index, output)| {
            let id = window::Id::unique();
            self.surfaces.insert(id, index);
            get_layer_surface(SctkLayerSurfaceSettings {
                id,
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::None,
                input_zone: Some(Vec::new()),
                anchor: Anchor::all(),
                output: IcedOutput::Output(output.wl.clone()),
                namespace: "computer-use-indicator".into(),
                size: None,
                exclusive_zone: -1,
                ..Default::default()
            })
        });
        Task::batch(tasks.collect::<Vec<_>>())
    }

    fn hide(&mut self) -> Task<Msg> {
        // A hidden cursor reappears where it was, like Codex's resting cursor.
        if let Some(glide) = self.glide.take() {
            self.cursor = Some(glide.to);
        }
        self.pulse = None;
        self.keyboard = None;
        self.fading = None;
        Task::batch(
            self.surfaces
                .drain()
                .map(|(id, _)| destroy_layer_surface(id)),
        )
    }

    fn on_action(&mut self, action: Action) -> Task<Msg> {
        if self.sprite.0 != action.agent {
            self.sprite = (
                action.agent.clone(),
                cursor::sprite(agent_color(&action.agent)),
            );
        }
        self.agent = action.agent;
        self.label = action.label;
        self.last_action = Instant::now();
        self.keyboard = match (action.keys.is_empty(), action.text) {
            (false, _) => Some((Instant::now(), Keyboard::Keys(action.keys))),
            (true, Some(text)) => Some((Instant::now(), Keyboard::Text(text))),
            (true, None) => None,
        };
        if let (Some(x), Some(y)) = (action.x, action.y) {
            // First appearance glides in from a short distance away.
            let from = self.cursor_position().unwrap_or((x + 90.0, y + 120.0));
            let press = matches!(action.tool.as_str(), "click" | "drag");
            self.glide = Some(Glide::new(from, (x, y), press));
        }
        self.show()
    }

    fn tick(&mut self) -> Task<Msg> {
        if let Some(glide) = &self.glide
            && glide.progress() >= 1.0
        {
            if glide.press {
                self.pulse = Some((Instant::now(), glide.to));
            }
            self.cursor = Some(glide.to);
            self.glide = None;
        }
        if self
            .pulse
            .is_some_and(|(at, _)| at.elapsed().as_secs_f32() > PULSE)
        {
            self.pulse = None;
        }
        if self
            .keyboard
            .as_ref()
            .is_some_and(|(at, _)| at.elapsed().as_secs_f32() > KEYS_SHOWN)
        {
            self.keyboard = None;
        }
        match self.fading {
            Some(at) if at.elapsed().as_secs_f32() > FADE => self.hide(),
            None if self.visible() && self.last_action.elapsed() > IDLE_BEFORE_FADE => {
                self.fading = Some(Instant::now());
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// 0..1 envelope: fade in on show, fade out after idling.
    fn opacity(&self) -> f32 {
        let fade_in = (self.shown_at.elapsed().as_secs_f32() / 0.2).min(1.0);
        let fade_out = self
            .fading
            .map_or(1.0, |at| 1.0 - (at.elapsed().as_secs_f32() / FADE).min(1.0));
        fade_in * fade_out
    }

    fn edge_glow(&self, color: Color, alpha: f32) -> Vec<Element<'_, Msg>> {
        let strip = |angle: f32, horizontal: bool| {
            let gradient = Linear::new(Radians::from(Degrees(angle)))
                .add_stop(0.0, with_alpha(color, 0.55 * alpha))
                .add_stop(0.35, with_alpha(color, 0.18 * alpha))
                .add_stop(1.0, with_alpha(color, 0.0));
            let space = if horizontal {
                Space::new().width(Length::Fill).height(EDGE)
            } else {
                Space::new().width(EDGE).height(Length::Fill)
            };
            container(space).style(move |_| container::Style {
                background: Some(Background::Gradient(gradient.into())),
                ..Default::default()
            })
        };
        let fill = || Space::new().width(Length::Fill).height(Length::Fill);
        vec![
            column![strip(180.0, true), fill()].into(),
            column![fill(), strip(0.0, true)].into(),
            row![strip(90.0, false), fill()].into(),
            row![fill(), strip(270.0, false)].into(),
        ]
    }

    /// Keycaps for `press_key`, a typing line with a caret for `type_text`.
    fn keyboard_bubble(&self, color: Color, alpha: f32) -> Option<Element<'_, Msg>> {
        let (at, keyboard) = self.keyboard.as_ref()?;
        let age = at.elapsed().as_secs_f32();
        let alpha = alpha * (age / 0.12).min(1.0) * ((KEYS_SHOWN - age) / 0.3).clamp(0.0, 1.0);
        let white = move |a: f32| {
            cosmic::theme::Text::Color(Color {
                a: a * alpha,
                ..Color::WHITE
            })
        };

        let content: Element<'_, Msg> = match keyboard {
            Keyboard::Keys(keys) => {
                let mut caps = row![].spacing(5).align_y(Alignment::Center);
                for (i, key) in keys.iter().enumerate() {
                    if i > 0 {
                        caps = caps.push(text("+").size(12).class(white(0.5)));
                    }
                    let cap = container(
                        text(key.clone())
                            .size(13)
                            .font(cosmic::font::semibold())
                            .class(white(0.95)),
                    )
                    .padding([3, 9])
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.10 * alpha).into()),
                        border: Border {
                            color: Color::from_rgba(1.0, 1.0, 1.0, 0.30 * alpha),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        shadow: Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5 * alpha),
                            offset: Vector::new(0.0, 2.0),
                            blur_radius: 0.0,
                        },
                        ..Default::default()
                    });
                    caps = caps.push(cap);
                }
                caps.into()
            }
            Keyboard::Text(typed) => {
                let shown = match typed.char_indices().rev().nth(39) {
                    Some((i, _)) => format!("…{}", &typed[i..]),
                    None => typed.clone(),
                };
                let blink = if ((age * 2.5) as u32).is_multiple_of(2) {
                    1.0
                } else {
                    0.0
                };
                let caret =
                    container(Space::new().width(2).height(15)).style(move |_| container::Style {
                        background: Some(with_alpha(color, blink * alpha).into()),
                        ..Default::default()
                    });
                row![
                    text("⌨").size(14).class(white(0.7)),
                    row![text(shown).size(13).class(white(0.95)), caret].align_y(Alignment::Center),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            }
        };

        Some(
            container(content)
                .padding([6, 10])
                .max_width(420)
                .style(move |_| container::Style {
                    background: Some(Color::from_rgba(0.09, 0.09, 0.11, 0.88 * alpha).into()),
                    border: Border {
                        color: with_alpha(color, 0.55 * alpha),
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    shadow: Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.35 * alpha),
                        offset: Vector::new(0.0, 4.0),
                        blur_radius: 16.0,
                    },
                    ..Default::default()
                })
                .into(),
        )
    }

    fn pill(&self, color: Color, alpha: f32) -> Element<'_, Msg> {
        let breath = if self.settling() {
            0.65 + 0.35 * (self.last_action.elapsed().as_secs_f32() * PI).sin().abs()
        } else {
            1.0
        };
        let dot = container(Space::new().width(8).height(8)).style(move |_| container::Style {
            background: Some(with_alpha(color, breath * alpha).into()),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: with_alpha(color, 0.8 * alpha),
                offset: Vector::ZERO,
                blur_radius: 8.0,
            },
            ..Default::default()
        });
        let white = |a: f32| {
            cosmic::theme::Text::Color(Color {
                a: a * alpha,
                ..Color::WHITE
            })
        };
        let content = row![
            dot,
            text(using_computer(&self.agent))
                .size(14)
                .font(cosmic::font::semibold())
                .class(white(0.95)),
            text(self.label.clone()).size(14).class(white(0.6)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(content)
            .padding([9, 18])
            .style(move |_| container::Style {
                background: Some(Color::from_rgba(0.09, 0.09, 0.11, 0.86 * alpha).into()),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.12 * alpha),
                    width: 1.0,
                    radius: 999.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.35 * alpha),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 24.0,
                },
                ..Default::default()
            })
            .into()
    }
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Msg;
    const APP_ID: &'static str = "io.github.netherguy4.ComputerUseIndicator";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: ()) -> (Self, Task<Msg>) {
        // Frosted-glass themes would otherwise blur every screen behind the overlay.
        core.set_auto_blur(Default::default());
        let app = App {
            core,
            outputs: Vec::new(),
            surfaces: HashMap::new(),
            agent: String::new(),
            label: String::new(),
            cursor: None,
            glide: None,
            pulse: None,
            keyboard: None,
            last_action: Instant::now(),
            shown_at: Instant::now(),
            fading: None,
            sprite: (String::new(), cursor::sprite(agent_color(""))),
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Msg) -> Task<Msg> {
        match message {
            Msg::Action(action) if action.hide => self.hide(),
            Msg::Action(action) => self.on_action(action),
            Msg::Tick => self.tick(),
            Msg::Output(OutputEvent::Created(Some(info)) | OutputEvent::InfoUpdate(info), wl) => {
                self.upsert_output(wl, &info);
                Task::none()
            }
            Msg::Output(OutputEvent::Removed, wl) => {
                // Surfaces map to output indices, so drop them before reindexing.
                let task = self.hide();
                self.outputs.retain(|o| o.wl != wl);
                task
            }
            Msg::Output(OutputEvent::Created(None), _) => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let outputs = event::listen_with(|event, _, _| match event {
            iced::Event::PlatformSpecific(PlatformSpecific::Wayland(WaylandEvent::Output(
                event,
                wl,
            ))) => Some(Msg::Output(event, wl)),
            _ => None,
        });
        let mut subs = vec![socket_sub(), outputs];
        if self.visible() {
            let busy = self.settling()
                || self.glide.is_some()
                || self.pulse.is_some()
                || self.fading.is_some()
                || self.keyboard.as_ref().is_some_and(|(at, _)| {
                    let age = at.elapsed().as_secs_f32();
                    age < 0.12 || age > KEYS_SHOWN - 0.3
                });
            let period = if busy { 16 } else { 250 };
            subs.push(iced::time::every(Duration::from_millis(period)).map(|_| Msg::Tick));
        }
        Subscription::batch(subs)
    }

    fn view(&self) -> Element<'_, Msg> {
        Space::new().into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Msg> {
        let Some(output) = self.surfaces.get(&id).and_then(|i| self.outputs.get(*i)) else {
            return Space::new().into();
        };
        let alpha = self.opacity();
        let color = agent_color(&self.agent);
        let mut layers = self.edge_glow(color, alpha);

        if let Some((at, (px, py))) = self.pulse
            && output.contains((px, py))
        {
            let t = (at.elapsed().as_secs_f32() / PULSE).min(1.0);
            let r = 10.0 + 30.0 * (1.0 - (1.0 - t).powi(3));
            let ring = container(Space::new().width(r * 2.0).height(r * 2.0)).style(move |_| {
                container::Style {
                    border: Border {
                        color: with_alpha(color, 0.9 * (1.0 - t) * alpha),
                        width: 2.5,
                        radius: r.into(),
                    },
                    ..Default::default()
                }
            });
            layers.push(pin(ring).x(px - output.x - r).y(py - output.y - r).into());
        }

        let cursor = self.cursor_position();
        if let Some((cx, cy)) = cursor
            && output.contains((cx, cy))
        {
            // Damped pendulum sway around the tip after landing, as in Codex.
            let t = self.last_action.elapsed().as_secs_f32();
            let sway = if self.glide.is_none() && self.settling() {
                0.07 * (t * 5.0).sin() * (-t / 0.7).exp()
            } else {
                0.0
            };
            let pressed = self
                .pulse
                .is_some_and(|(at, _)| at.elapsed().as_secs_f32() < 0.12);
            let size = cursor::SIZE * if pressed { 0.88 } else { 1.0 };
            let sprite = image(self.sprite.1.clone())
                .width(size)
                .height(size)
                .rotation(Rotation::Floating(Radians(sway)))
                .opacity(alpha);
            layers.push(
                pin(sprite)
                    .x(cx - output.x - size / 2.0)
                    .y(cy - output.y - size / 2.0)
                    .into(),
            );
        }

        // One pill, on the screen the agent is working on.
        let pill_here = match cursor {
            Some(point) => output.contains(point),
            None => self.surfaces.get(&id) == self.surfaces.values().min(),
        };
        if pill_here {
            let mut top = column![self.pill(color, alpha)]
                .spacing(10)
                .align_x(Alignment::Center);
            match (cursor, self.keyboard_bubble(color, alpha)) {
                // Keystrokes go to the focused field, usually where the cursor last clicked.
                (Some((cx, cy)), Some(bubble)) => {
                    layers.push(
                        pin(bubble)
                            .x(cx - output.x + 18.0)
                            .y(cy - output.y + 24.0)
                            .into(),
                    );
                }
                (None, Some(bubble)) => top = top.push(bubble),
                _ => {}
            }
            layers.push(
                container(top)
                    .center_x(Length::Fill)
                    .padding(iced::padding::top(42))
                    .into(),
            );
        }

        stack(layers).into()
    }

    fn style(&self) -> Option<iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<App>(Settings::default().no_main_window(true), ())
}
