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
        pub enum $name {
            /// Plus (+) button.
            Plus,
            /// Minus (-) button.
            Minus,
            $($body)*
        }
    };
}

macro_rules! regular_controller_key_enum {
    ($name:ident {$($body:tt)*}) => {
        key_enum!{
            $name {
                /// Left directional pad button.
                Left,
                /// Right directional pad button.
                Right,
                /// Up directional pad button.
                Up,
                /// Down directional pad button.
                Down,
                /// A button.
                A,
                /// B button.
                B,
                /// Home button.
                Home,
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
                X,
                /// Joystick Y-axis.
                Y,
                /// TL button.
                TL,
                /// TR button.
                TR,
                /// ZL button.
                ZL,
                /// ZR button.
                ZR,
                $($body)*
            }
        }
    };
}

regular_controller_key_enum!(Key {
    /// 1 button.
    One,
    /// 2 button.
    Two
});

gamepad_key_enum!(ProControllerKey {
    /// Left thumb button.
    ///
    /// Reported if the left analog stick is pressed.
    LeftThumb,
    /// Right thumb button.
    ///
    /// Reported if the right analog stick is pressed.
    RightThumb,
});

gamepad_key_enum!(ClassicControllerKey {});

pub enum NunchukKey {
    /// C button.
    C,
    /// Z button.
    Z,
}

key_enum!(DrumsKey {});

key_enum!(GuitarKey {
    /// The StarPower/Home button.
    StarPower,
    /// The guitar strum bar.
    StrumBar,
    /// The guitar upper-most fret button.
    HighestFretBar,
    /// The guitar second-upper fret button.
    HighFretBar,
    /// The guitar mid fret button.
    MidFretBar,
    /// The guitar second-lowest fret button.
    LowFretBar,
    /// The guitar lowest fret button.
    LowestFretBar,
});

pub enum KeyState {
    Up = 0,
    Down,
    AutoRepeat,
}
