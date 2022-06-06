use std::time::Instant;

macro_rules! key_enum {
    ($name:ident {$($vals:tt)*}) => {
        pub enum $name {
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
            /// Plus (+) button.
            Plus,
            /// Minus (-) button.
            Minus,
            /// Home button.
            Home,
            $($vals)*
        }
    };
}

key_enum!(Key {
    /// 1 button.
    One,
    /// 2 button.
    Two,
});

// wiimote: LEFT, RIGHT, UP, DOWN, PLUS, MINUS, HOME,       A, B,                                ONE, TWO
// pro    : LEFT, RIGHT, UP, DOWN, PLUS, MINUS, HOME, X, Y, A, B, TR, TL, ZR, ZL, THUMBL, THUMBR
// classic: LEFT, RIGHT, UP, DOWN, PLUS, MINUS, HOME, X, Y, A, B, TR, TL, ZR, ZL
// nunchuk:                                                                                               C, Z
// drums  :                        PLUS, MINUS
// guitar :                        PLUS,        HOME,
// In total, 21 buttons

key_enum!(ProControllerKey {});

pub enum OtherKey {
    // Wiimote // todo: note that the power button is not reported

    // Classic (also includes the above), Nunchuk, Wii-U Pro and others.
    /// Joystick X-axis.
    X,
    /// Joystick Y-axis.
    Y,
    /// Left trigger range.
    LeftTrigger,
    /// Right trigger range.
    RightTrigger,
    /// ZL button.
    ZL,
    /// ZR button.
    ZR,
    /// Left thumb button.
    ///
    /// Reported if the left analog stick is pressed.
    LeftThumb,
    /// Right thumb button.
    ///
    /// Reported if the right analog stick is pressed.
    RightThumb,
    /// Extra C button (e.g. in the Nunchuk).
    C,
    /// Extra Z button (e.g. in the Nunchuk).
    Z,
    // Guitar Hero guitars
    // todo: can we clean up strum bar events and have a single key?
    // todo: improve strum docs.
    /// The guitar strum bar was moved up.
    StrumBarUp,
    /// The guitar strum bar was moved down.
    StrumBarDown,
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
}

pub enum KeyState {
    Up = 0,
    Down,
    AutoRepeat,
}

pub enum EventKind {
    /// The state of a Wii Remote controller key changed.
    ///
    /// Received only if `Channels::CORE` is open.
    Key(Key, KeyState),
    /// Provides the accelerometer data.
    ///
    /// Received only if `Channels::ACCELEROMETER` is open.
    Accelerometer {
        /// The x-axis acceleration.
        x: i32,
        /// The y-axis acceleration.
        y: i32,
        /// The z-axis acceleration.
        z: i32,
    },
    /// Provides the IR camera data.
    ///
    /// The camera can track up to four IR sources. The index
    /// of each source within the array is maintained across
    /// events.
    ///
    /// Received only if `Channels::IR` is open.
    // todo: use [Option(source)]
    Ir {},
    /// Provides Balance Board weight data. Four sensors report
    /// data for each of the edges of the board.
    ///
    /// Received only if `Channels::BALANCE_BOARD` is open.
    BalanceBoard([i32; 4]),
    /// Provides the Motion Plus extension gyroscope data.
    ///
    /// Received only if `Channels::MOTION_PLUS` is open.
    MotionPlus {},
    /// The state of a Wii U Pro controller key changed.
    ///
    /// Received only if `Channels::PRO_CONTROLLER` is open.
    ProControllerKey {},
    /// Reports the movement of an analog stick from
    /// a Wii U Pro controller.
    ///
    /// Received only if `Channels::PRO_CONTROLLER` is open.
    ProControllerMove {},
    // todo: document
    Watch {}, // todo: rename to connect/disconnect
    /// The state of a Classic controller key changed.
    ///
    /// Received only if `Channels::CLASSIC_CONTROLLER` is open.
    ClassicControllerKey {},
    /// Reports the movement of an analog stick from
    /// a Classic controller.
    ///
    /// Received only if `Channels::CLASSIC_CONTROLLER` is open.
    ClassicControllerMove {},
    /// The state of a Nunchuk key changed.
    ///
    /// Received only if `Channels::NUNCHUK` is open.
    NunchukKey {},
    /// Reports the movement of an analog stick from a Nunchuk.
    ///
    /// Received only if `Channels::NUNCHUK` is open.
    NunchukMove {},
    /// The state of a drums controller key changed.
    ///
    /// Received only if `Channels::DRUMS` is open.
    DrumsKey {},
    /// Reports the movement of an analog stick from a
    /// drums controller.
    ///
    /// Received only if `Channels::DRUMS` is open.
    DrumsMove {},
    /// The state of a guitar controller key changed.
    ///
    /// Received only if `Channels::GUITAR` is open.
    GuitarKey {},
    /// Reports the movement of an analog stick from a
    /// guitar controller.
    ///
    /// Received only if `Channels::GUITAR` is open.
    GuitarMove {},
    /// The device was removed.
    ///
    /// Received only if watch mode was enabled via `Device::watch`.
    Removed,
}

pub struct Event {
    pub time: Instant,
    pub kind: EventKind,
}
