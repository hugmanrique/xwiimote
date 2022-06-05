enum Key {
    // Wiimote // todo: note that the power button is not reported
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
    /// 1 button.
    One,
    /// 2 button.
    Two,
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

enum KeyState {
    Up = 0,
    Down,
    AutoRepeat,
}

pub enum Event {
    Key { key: Key, state: KeyState },
    AbsMotion { x: u32, y: u32, z: u32 },
}

/*pub struct Event {
    time: Instant,
}*/
