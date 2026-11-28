//! The focused window's place on its output as brightness of the 1-6 keys.
//!
//! The keys are the output cut into six equal slices, left to right; each
//! shines by how much of its slice the window covers.

use crate::protocol::WIDTH_KEY_COUNT;

/// Weight of the focused window; the others are dimmer to tell them apart.
const FOCUSED: f64 = 1.0;
const UNFOCUSED: f64 = 0.1;

/// A window's horizontal span in logical pixels from the output's left edge.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Span {
    pub left: f64,
    pub right: f64,
    pub focused: bool,
}

/// Brightness of each key on an output `width` wide: the largest of its
/// slice's coverage by each window, scaled by the window's weight. Whatever
/// lies outside the output is clipped.
pub fn levels(
    spans: &[Span],
    width: f64,
) -> [u8; WIDTH_KEY_COUNT] {
    let slice = width / WIDTH_KEY_COUNT as f64;
    let mut levels = [0; WIDTH_KEY_COUNT];
    for (key, level) in levels.iter_mut().enumerate() {
        let from = key as f64 * slice;
        for span in spans {
            let covered = (span.right.min(from + slice) - span.left.max(from)).max(0.0);
            let weight = if span.focused { FOCUSED } else { UNFOCUSED };
            *level = (*level).max((255.0 * weight * covered / slice).round() as u8);
        }
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focused(
        left: f64,
        right: f64,
    ) -> Span {
        Span {
            left,
            right,
            focused: true,
        }
    }

    fn other(
        left: f64,
        right: f64,
    ) -> Span {
        Span {
            left,
            right,
            focused: false,
        }
    }

    #[test]
    fn test_halves_and_whole() {
        assert_eq!(
            levels(&[focused(0.0, 600.0)], 1200.0),
            [255, 255, 255, 0, 0, 0]
        );
        assert_eq!(
            levels(&[focused(600.0, 1200.0)], 1200.0),
            [0, 0, 0, 255, 255, 255]
        );
        assert_eq!(levels(&[focused(0.0, 1200.0)], 1200.0), [255; 6]);
        assert_eq!(levels(&[], 1200.0), [0; 6]);
    }

    #[test]
    fn test_partial_coverage() {
        assert_eq!(
            levels(&[focused(0.0, 300.0)], 1200.0),
            [255, 128, 0, 0, 0, 0]
        );
        assert_eq!(
            levels(&[focused(100.0, 200.0)], 1200.0),
            [128, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_clipping() {
        assert_eq!(
            levels(&[focused(-500.0, 400.0)], 1200.0),
            [255, 255, 0, 0, 0, 0]
        );
        assert_eq!(
            levels(&[focused(1000.0, 5000.0)], 1200.0),
            [0, 0, 0, 0, 0, 255]
        );
        assert_eq!(levels(&[focused(-900.0, -100.0)], 1200.0), [0; 6]);
    }

    #[test]
    fn test_mixed_focused_unfocused_empty() {
        // Focused on the first sixth, a neighbour on the second and third,
        // the rest empty.
        assert_eq!(
            levels(&[focused(0.0, 200.0), other(200.0, 600.0)], 1200.0),
            [255, 26, 26, 0, 0, 0]
        );
        // Where they overlap the brighter wins; a dim window under a focused one is hidden.
        assert_eq!(
            levels(&[other(0.0, 1200.0), focused(200.0, 400.0)], 1200.0),
            [26, 255, 26, 26, 26, 26]
        );
    }
}
