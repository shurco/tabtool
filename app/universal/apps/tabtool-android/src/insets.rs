//! Window insets reported by the Android host.

/// Insets in density-independent pixels.
///
/// `keyboard` is the height of the IME, or zero when the soft keyboard is closed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Insets {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub keyboard: i32,
}

impl Insets {
    /// Parses `[left, top, right, bottom, keyboard]` as returned by the Kotlin bridge.
    #[must_use]
    pub fn from_array(values: &[i32]) -> Option<Self> {
        match *values {
            [left, top, right, bottom, keyboard] => Some(Self {
                left,
                top,
                right,
                bottom,
                keyboard,
            }),
            _ => None,
        }
    }

    /// Renders the insets as a single JSON object for the spike's debug label.
    #[must_use]
    pub fn to_json(self) -> String {
        serde_json::json!({
            "left": self.left,
            "top": self.top,
            "right": self.right,
            "bottom": self.bottom,
            "keyboard": self.keyboard,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Insets;

    #[test]
    fn parses_five_values() {
        let insets = Insets::from_array(&[1, 2, 3, 4, 5]);

        assert_eq!(
            insets,
            Some(Insets {
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
                keyboard: 5,
            })
        );
    }

    #[test]
    fn rejects_arrays_of_the_wrong_length() {
        assert_eq!(Insets::from_array(&[1, 2, 3, 4]), None);
        assert_eq!(Insets::from_array(&[1, 2, 3, 4, 5, 6]), None);
    }

    #[test]
    fn renders_json_with_every_field() {
        let insets = Insets {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
            keyboard: 5,
        };

        let value: serde_json::Value = serde_json::from_str(&insets.to_json()).unwrap();

        assert_eq!(
            value,
            json!({
                "left": 1,
                "top": 2,
                "right": 3,
                "bottom": 4,
                "keyboard": 5,
            })
        );
    }
}
