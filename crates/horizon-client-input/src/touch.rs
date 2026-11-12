//! Touch controls (Horizon's own, for iOS; vanilla has none): where the controls sit on the
//! screen and what a finger takes. The game applies them (`game/touch.rs`). Lengths are GUI
//! pixels.

/// A button's side and its gap to the screen's side edge, clear of an iPhone's notch and
/// rounded corners.
const BUTTON: f32 = 32.0;
const EDGE: f32 = 34.0;
/// How far from where it went down a finger tilts the stick fully.
pub const STICK_REACH: f32 = 24.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    /// Held: key.jump.
    Jump,
    /// Held: key.use (eating, blocking, drawing a bow).
    Use,
    /// Flips key.sneak.
    Sneak,
    Inventory,
    Pause,
    /// A hotbar slot, 0..=8.
    Hotbar(usize),
}

/// What a finger going down takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    Control(Control),
    /// The move stick, centred where the finger went down.
    Stick,
    /// Turning the view.
    Look,
}

/// The buttons of a `w` x `h` screen: `[x, y, width, height]` and the label.
pub fn buttons(w: f32, h: f32) -> [(Control, [f32; 4], &'static str); 5] {
    let x = w - EDGE - BUTTON;
    let row = |n: f32| h - 30.0 - BUTTON - n * (BUTTON + 4.0);
    [
        (Control::Jump, [x, row(0.0), BUTTON, BUTTON], "Jump"),
        (Control::Use, [x, row(1.0), BUTTON, BUTTON], "Use"),
        (Control::Sneak, [x, row(2.0), BUTTON, BUTTON], "Sneak"),
        (
            Control::Inventory,
            [w / 2.0 + 95.0, h - 21.0, 20.0, 20.0],
            "...",
        ),
        (Control::Pause, [w - EDGE - 20.0, 4.0, 20.0, 20.0], "||"),
    ]
}

/// What a finger going down at `(x, y)` of a `w` x `h` screen takes.
pub fn zone(x: f32, y: f32, w: f32, h: f32) -> Zone {
    let on = |[bx, by, bw, bh]: [f32; 4]| (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y);
    if let Some((control, ..)) = buttons(w, h).into_iter().find(|b| on(b.1)) {
        return Zone::Control(control);
    }
    // Hud.extractItemHotbar: 182 pixels wide and 22 high, centred on the bottom edge, its
    // slots 20 apart.
    let left = w / 2.0 - 91.0;
    if y >= h - 22.0 && (left..left + 182.0).contains(&x) {
        return Zone::Control(Control::Hotbar(
            (((x - left - 1.0) / 20.0) as usize).min(8),
        ));
    }
    if x < w * 0.4 && y > h * 0.3 {
        Zone::Stick
    } else {
        Zone::Look
    }
}

/// The stick's tilt for a finger `(dx, dy)` from where it went down: -1..1, up positive.
pub fn tilt(dx: f32, dy: f32) -> [f32; 2] {
    let length = dx.hypot(dy).max(STICK_REACH);
    [dx / length, -dy / length]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An iPhone 11 in landscape at GUI scale 3.
    const SCREEN: (f32, f32) = (598.0, 276.0);

    #[test]
    fn a_finger_takes_what_is_under_it() {
        let (w, h) = SCREEN;
        assert_eq!(zone(50.0, 200.0, w, h), Zone::Stick);
        assert_eq!(zone(400.0, 100.0, w, h), Zone::Look);
        // Above the stick's corner the left side turns the view.
        assert_eq!(zone(50.0, 40.0, w, h), Zone::Look);
        for (control, [x, y, bw, bh], _) in buttons(w, h) {
            assert_eq!(
                zone(x + bw / 2.0, y + bh / 2.0, w, h),
                Zone::Control(control)
            );
            assert!(x >= 0.0 && y >= 0.0 && x + bw <= w && y + bh <= h);
        }
    }

    #[test]
    fn the_hotbar_selects_the_slot_touched() {
        let (w, h) = SCREEN;
        let slot = |x: f32| zone(w / 2.0 + x, h - 5.0, w, h);
        assert_eq!(slot(-90.5), Zone::Control(Control::Hotbar(0)));
        assert_eq!(slot(0.0), Zone::Control(Control::Hotbar(4)));
        assert_eq!(slot(90.5), Zone::Control(Control::Hotbar(8)));
        // Left of the hotbar is the stick's corner, above it the view.
        assert_eq!(slot(-120.0), Zone::Stick);
        assert_eq!(zone(w / 2.0, h - 30.0, w, h), Zone::Look);
    }

    #[test]
    fn the_stick_tilts_up_positive_and_no_further_than_one() {
        assert_eq!(tilt(0.0, 0.0), [0.0, 0.0]);
        assert_eq!(tilt(0.0, -STICK_REACH / 2.0), [0.0, 0.5]);
        assert_eq!(tilt(3.0 * STICK_REACH, 0.0), [1.0, 0.0]);
        assert_eq!(tilt(0.0, 5.0 * STICK_REACH), [0.0, -1.0]);
    }
}
