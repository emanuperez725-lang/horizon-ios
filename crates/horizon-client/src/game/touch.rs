//! Touch controls (Horizon's own, for iOS; vanilla has none). Behind a screen the first
//! finger is the mouse: the pointer follows it with the left button held, so every screen
//! keeps its mouse behaviour with no touch code of its own. While playing, a finger on the
//! lower left is a stick that tilts the virtual controller's left stick (`Pads::injected`),
//! a finger elsewhere turns the view (a tap uses, or attacks the entity under the crosshair;
//! a finger held still mines), and the buttons and the hotbar act as their mappings. Where
//! the controls sit is `horizon_client::touch`.
use super::{Game, Screen};
use horizon_client::{
    Result,
    hud::{HudBatch, rgba},
    keybinds::{KeyBinds, key_code},
    touch::{Control, STICK_REACH, Zone, buttons, tilt, zone},
};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, Touch, TouchPhase, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

/// A finger lifted sooner than this, within `SLOP` GUI pixels of where it went down, taps;
/// one that stays mines.
const HOLD: Duration = Duration::from_millis(300);
const SLOP: f32 = 6.0;
/// Mouse counts per GUI pixel of a finger turning the view: half an iPhone's screen is about
/// 130 degrees at the default sensitivity.
const LOOK: f32 = 3.0;

/// A finger as the mouse: the pointer goes to it, and the left button goes down and up
/// with it.
pub(super) fn as_mouse(touch: &Touch) -> impl Iterator<Item = WindowEvent> + use<> {
    let device_id = touch.device_id;
    let state = match touch.phase {
        TouchPhase::Started => Some(ElementState::Pressed),
        TouchPhase::Moved => None,
        TouchPhase::Ended | TouchPhase::Cancelled => Some(ElementState::Released),
    };
    let moved = WindowEvent::CursorMoved {
        device_id,
        position: touch.location,
    };
    let button = state.map(|state| WindowEvent::MouseInput {
        device_id,
        state,
        button: MouseButton::Left,
    });
    [Some(moved), button].into_iter().flatten()
}

/// The finger turning the view.
struct Look {
    finger: u64,
    at: (f32, f32),
    since: Instant,
    /// How far it has travelled.
    moved: f32,
    /// Held still long enough: it holds key.attack.
    mining: bool,
}

/// The fingers on the window.
#[derive(Default)]
pub(super) struct Touches {
    /// A finger touched the window: the controls are drawn from then on.
    seen: bool,
    /// The finger that is the mouse behind a screen.
    pointer: Option<u64>,
    /// The finger on the stick, where it went down and where it is.
    stick: Option<(u64, (f32, f32), (f32, f32))>,
    /// The stick is pushed far enough forward to sprint.
    sprint: bool,
    look: Option<Look>,
    /// The fingers holding Jump and Use.
    held: Vec<(u64, Control)>,
}

impl Game<'_> {
    /// A finger on the window (`WindowEvent::Touch`).
    pub(super) fn touch(&mut self, event_loop: &ActiveEventLoop, id: WindowId, touch: &Touch) {
        let Some(batch) = self.gui_scale() else {
            return;
        };
        self.touch.seen = true;
        self.last_input = Instant::now();
        let down = touch.phase == TouchPhase::Started;
        let up = matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled);
        let playing = !self.pad_in_menu() && self.player.is_some() && self.pending_save.is_none();
        if !playing || self.touch.pointer == Some(touch.id) {
            // Behind a screen: the pointer follows the first finger, with the left button held.
            if down && self.touch.pointer.is_none() {
                self.touch.pointer = Some(touch.id);
            }
            if self.touch.pointer != Some(touch.id) {
                return;
            }
            if up {
                self.touch.pointer = None;
            }
            for event in as_mouse(touch) {
                self.window_event(event_loop, id, event);
            }
            return;
        }
        let at = (
            touch.location.x as f32 / batch.scale,
            touch.location.y as f32 / batch.scale,
        );
        let size = (batch.width, batch.height);
        if let Err(e) = self.touch_play(touch.id, at, size, down, up) {
            log::warn!("Horizon: touch: {e}");
        }
    }

    /// While playing: a finger going down takes a control, the stick or the view, and keeps
    /// it until it lifts.
    fn touch_play(
        &mut self,
        finger: u64,
        at: (f32, f32),
        (w, h): (f32, f32),
        down: bool,
        up: bool,
    ) -> Result<()> {
        if !self.captured {
            self.capture(true);
        }
        if down {
            match zone(at.0, at.1, w, h) {
                Zone::Control(control @ (Control::Jump | Control::Use)) => {
                    self.touch.held.push((finger, control));
                    self.touch_hold(control, true)?;
                }
                Zone::Control(Control::Sneak) => {
                    // Flips sneaking, as a controller's stick click does (game/pad.rs).
                    let sneaking = self.keys.contains(&KeyCode::ShiftLeft);
                    self.mapped_key(KeyCode::ShiftLeft, !sneaking, false)?;
                }
                Zone::Control(Control::Inventory) => {
                    self.touch_key("key.inventory", true)?;
                    self.touch_key("key.inventory", false)?;
                }
                Zone::Control(Control::Pause) => self.pad_back()?,
                Zone::Control(Control::Hotbar(slot)) => self.selected = slot,
                Zone::Stick if self.touch.stick.is_none() => {
                    self.touch.stick = Some((finger, at, at));
                }
                // A second finger on the stick's side turns the view too.
                Zone::Stick | Zone::Look if self.touch.look.is_none() => {
                    self.touch.look = Some(Look {
                        finger,
                        at,
                        since: Instant::now(),
                        moved: 0.0,
                        mining: false,
                    });
                }
                Zone::Stick | Zone::Look => {}
            }
            return Ok(());
        }
        if let Some(i) = self.touch.held.iter().position(|(f, _)| *f == finger) {
            if up {
                let (_, control) = self.touch.held.remove(i);
                self.touch_hold(control, false)?;
            }
        } else if let Some((_, origin, _)) = self.touch.stick.filter(|s| s.0 == finger) {
            self.touch.stick = (!up).then_some((finger, origin, at));
            self.pad.injected.left = if up {
                [0.0; 2]
            } else {
                tilt(at.0 - origin.0, at.1 - origin.1)
            };
            // Pushed twice as far forward as a full tilt: sprint.
            let sprint = !up && origin.1 - at.1 > 2.0 * STICK_REACH;
            if sprint != self.touch.sprint {
                self.touch.sprint = sprint;
                self.touch_key("key.sprint", sprint)?;
            }
        } else if let Some(mut look) = self.touch.look.take_if(|l| l.finger == finger) {
            let (dx, dy) = (at.0 - look.at.0, at.1 - look.at.1);
            look.at = at;
            look.moved += dx.hypot(dy);
            self.mouse
                .add_motion(f64::from(dx * LOOK), f64::from(dy * LOOK));
            if !up {
                self.touch.look = Some(look);
            } else if look.mining {
                self.play_mouse(MouseButton::Left, false)?;
            } else if look.moved < SLOP && look.since.elapsed() < HOLD {
                // A tap clicks: key.attack on an entity, key.use on anything else.
                let button = if self.pick_entity_hit().is_some() {
                    MouseButton::Left
                } else {
                    MouseButton::Right
                };
                self.play_mouse(button, true)?;
                self.play_mouse(button, false)?;
            }
        }
        Ok(())
    }

    /// Once per loop iteration: a finger held still starts mining, and a screen that opened
    /// takes the fingers off the controls.
    pub(super) fn touch_frame(&mut self) -> Result<()> {
        if self.pad_in_menu() {
            if self.touch.stick.take().is_some() {
                self.pad.injected.left = [0.0; 2];
            }
            if std::mem::take(&mut self.touch.sprint) {
                self.touch_key("key.sprint", false)?;
            }
            if self.touch.look.take().is_some_and(|l| l.mining) {
                self.play_mouse(MouseButton::Left, false)?;
            }
            for (_, control) in std::mem::take(&mut self.touch.held) {
                self.touch_hold(control, false)?;
            }
            return Ok(());
        }
        if let Some(look) = &mut self.touch.look
            && !look.mining
            && look.moved < SLOP
            && look.since.elapsed() >= HOLD
        {
            look.mining = true;
            self.play_mouse(MouseButton::Left, true)?;
        }
        Ok(())
    }

    /// Jump and Use, held for as long as their finger stays.
    fn touch_hold(&mut self, control: Control, down: bool) -> Result<()> {
        match control {
            Control::Jump => self.touch_key("key.jump", down),
            _ => self.play_mouse(MouseButton::Right, down),
        }
    }

    /// A control as the default key of its mapping, like a controller's button (game/pad.rs).
    fn touch_key(&mut self, mapping: &str, down: bool) -> Result<()> {
        match key_code(KeyBinds::default_of(mapping)) {
            Some(key) => self.mapped_key(key, down, false),
            None => Ok(()),
        }
    }

    /// The controls over the HUD, while playing, once a finger touched the window.
    pub(super) fn draw_touch(&self, batch: &mut HudBatch) {
        if !self.touch.seen || self.screen != Screen::Playing {
            return;
        }
        let atlas = &self.gui.atlas;
        for (_, [x, y, w, h], label) in buttons(batch.width, batch.height) {
            batch.fill(atlas, x, y, w, h, rgba(0, 0, 0, 80));
            let width = HudBatch::text_width(atlas, label);
            batch.text(
                atlas,
                label,
                (x + (w - width) / 2.0).round(),
                (y + (h - 8.0) / 2.0).round(),
                rgba(255, 255, 255, 200),
                false,
            );
        }
        if let Some((_, origin, at)) = self.touch.stick {
            let [tx, ty] = tilt(at.0 - origin.0, at.1 - origin.1);
            let r = STICK_REACH;
            batch.fill(
                atlas,
                origin.0 - r,
                origin.1 - r,
                2.0 * r,
                2.0 * r,
                rgba(0, 0, 0, 60),
            );
            batch.fill(
                atlas,
                origin.0 + tx * r - 8.0,
                origin.1 - ty * r - 8.0,
                16.0,
                16.0,
                rgba(255, 255, 255, 120),
            );
        }
    }
}
