// Where the Corner panel goes inside the game window. All in logical pixels.
use crate::gamewin::Frame;

/// Gap from the game window's edges for the default top-right spot.
const MARGIN: f64 = 16.0;

/// The panel's top-left for a game frame. `spot` is where the player dragged it:
/// (0, 0) top-left of the game window to (1, 1) bottom-right, so it keeps its
/// place when the window is resized. None is the default top-right spot.
pub fn place(frame: Frame, panel: (f64, f64), spot: Option<(f64, f64)>) -> (f64, f64) {
    let free = ((frame.w - panel.0).max(0.0), (frame.h - panel.1).max(0.0));
    match spot {
        Some((fx, fy)) => (frame.x + fx.clamp(0.0, 1.0) * free.0, frame.y + fy.clamp(0.0, 1.0) * free.1),
        None => (frame.x + (free.0 - MARGIN).max(0.0), frame.y + MARGIN.min(free.1)),
    }
}

/// The spot for a panel the player dropped at (x, y), kept inside the game window.
pub fn spot_of(frame: Frame, panel: (f64, f64), at: (f64, f64)) -> (f64, f64) {
    let free = ((frame.w - panel.0).max(1.0), (frame.h - panel.1).max(1.0));
    (((at.0 - frame.x) / free.0).clamp(0.0, 1.0), ((at.1 - frame.y) / free.1).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: Frame = Frame { x: 100.0, y: 50.0, w: 1600.0, h: 900.0 };

    #[test]
    fn defaults_to_the_top_right_inside_the_game() {
        assert_eq!(place(GAME, (436.0, 500.0), None), (100.0 + 1600.0 - 436.0 - 16.0, 66.0));
    }

    #[test]
    fn a_dragged_spot_survives_a_resize() {
        let spot = spot_of(GAME, (400.0, 400.0), (100.0, 550.0));
        assert_eq!(spot, (0.0, 1.0));
        let bigger = Frame { w: 2000.0, h: 1200.0, ..GAME };
        assert_eq!(place(bigger, (400.0, 400.0), Some(spot)), (100.0, 50.0 + 800.0));
    }

    #[test]
    fn drops_outside_are_pulled_back_in() {
        assert_eq!(spot_of(GAME, (400.0, 400.0), (5000.0, -20.0)), (1.0, 0.0));
    }
}
