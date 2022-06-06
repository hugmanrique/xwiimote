use num_derive::FromPrimitive;

// We provide a key enumeration for each controller and
// extension type. To avoid repetition, we use a macro to define
// the common key variants. A matrix of the buttons reported by each
// device is given in `BUTTONS.md`.
// We use the TT munching technique.
macro_rules! key_enum {
    ($name:ident {$($body:tt)*} ($variant:expr) $($tail:tt)*) => {
        inner_key_enum! {
            $name {
                $($body)*  // Previously-built variants.
                $variant,
            }
        }
        $($tail)* // Unprocessed variants.
    };
    // There are no more variants, emit the enum definition.
    ($name:ident {$($body:tt)*}) => {
        #[derive(Copy, Clone, Debug, FromPrimitive)]
        pub enum $name {
            /// Plus (+) button.
            Plus = 6,
            /// Minus (-) button.
            Minus = 7,
            $($body)*
        }
    };
}

macro_rules! regular_controller_key_enum {
    ($name:ident {$($body:tt)*}) => {
        key_enum!{
            $name {
                /// Left directional pad button.
                Left = 0,
                /// Right directional pad button.
                Right = 1,
                /// Up directional pad button.
                Up = 2,
                /// Down directional pad button.
                Down = 3,
                /// A button.
                A = 4,
                /// B button.
                B = 5,
                /// Home button.
                Home = 8,
                $($body)*
            }
        }
    };
}

macro_rules! gamepad_key_enum {
    ($name:ident {$($body:tt)*}) => {
        regular_controller_key_enum!{
            $name {
                /// Joystick X-axis.
                X = 11,
                /// Joystick Y-axis.
                Y = 12,
                /// TL button.
                TL = 13,
                /// TR button.
                TR = 14,
                /// ZL button.
                ZL = 15,
                /// ZR button.
                ZR = 16,
                $($body)*
            }
        }
    };
}

regular_controller_key_enum!(Key {
    /// 1 button.
    One = 9,
    /// 2 button.
    Two = 10
});

gamepad_key_enum!(ProControllerKey {
    /// Left thumb button.
    ///
    /// Reported if the left analog stick is pressed.
    LeftThumb = 17,
    /// Right thumb button.
    ///
    /// Reported if the right analog stick is pressed.
    RightThumb = 18,
});

gamepad_key_enum!(ClassicControllerKey {});

// This is the only extension that doesn't have the + and - buttons.
#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum NunchukKey {
    /// C button.
    C = 19,
    /// Z button.
    Z = 20,
}

key_enum!(DrumsKey {});

key_enum!(GuitarKey {
    /// The StarPower/Home button.
    StarPower = 8, // same as Key::Home
    /// The guitar strum bar.
    StrumBar = 21, // also 22
    /// The guitar upper-most fret button.
    HighestFretBar = 23,
    /// The guitar second-upper fret button.
    HighFretBar = 24,
    /// The guitar mid fret button.
    MidFretBar = 25,
    /// The guitar second-lowest fret button.
    LowFretBar = 26,
    /// The guitar lowest fret button.
    LowestFretBar = 27,
});

#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum KeyState {
    Up = 0,
    Down,
    AutoRepeat,
}
