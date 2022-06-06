use crate::key;
use crate::key::KeyState;
use std::time::Instant;

pub enum EventKind {
    /// The state of a Wii Remote controller key changed.
    ///
    /// Received only if `Channels::CORE` is open.
    Key(key::Key, KeyState),
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
    MotionPlus {
        /// The x-axis rotational speed.
        x: i32,
        /// The y-axis rotational speed.
        y: i32,
        /// The z-axis rotational speed.
        z: i32,
    },
    /// The state of a Wii U Pro controller key changed.
    ///
    /// Received only if `Channels::PRO_CONTROLLER` is open.
    ProControllerKey(key::ProControllerKey, KeyState),
    /// Reports the movement of an analog stick from
    /// a Wii U Pro controller.
    ///
    /// Received only if `Channels::PRO_CONTROLLER` is open.
    ProControllerMove {
        /// The left analog stick absolute x-axis position.
        left_x: i32,
        /// The left analog stick absolute y-axis position.
        left_y: i32,
        /// The right analog stick absolute x-axis position.
        right_x: i32,
        /// The right analog stick absolute y-axis position.
        right_y: i32,
    },
    // todo: document
    Watch {}, // todo: rename to connect/disconnect
    /// The state of a Classic controller key changed.
    ///
    /// Received only if `Channels::CLASSIC_CONTROLLER` is open.
    ClassicControllerKey(key::ClassicControllerKey, KeyState),
    /// Reports the movement of an analog stick from
    /// a Classic controller.
    ///
    /// Received only if `Channels::CLASSIC_CONTROLLER` is open.
    ClassicControllerMove {
        /// The left analog stick x-axis absolute position.
        left_x: i32,
        /// The left analog stick y-axis absolute position.
        left_y: i32,
        /// The right analog stick x-axis absolute position.
        right_x: i32,
        /// The right analog stick y-axis absolute position.
        right_y: i32,
        /// The TL trigger absolute position, ranging from 0 to 63.
        ///
        /// Many controller do not have analog controllers, in
        /// which case this value is either 0 or 63.
        left_trigger: u8,
        /// The TR trigger absolute position, ranging from 0 to 63.
        ///
        /// Many controller do not have analog controllers, in
        /// which case this value is either 0 or 63.
        right_trigger: u8,
    },
    /// The state of a Nunchuk key changed.
    ///
    /// Received only if `Channels::NUNCHUK` is open.
    NunchukKey(key::NunchukKey, KeyState),
    /// Reports the movement of an analog stick from a Nunchuk.
    ///
    /// Received only if `Channels::NUNCHUK` is open.
    NunchukMove {
        /// The x-axis absolute position.
        x: i32,
        /// The y-axis absolute position.
        y: i32,
        /// The x-axis acceleration.
        x_acceleration: i32,
        /// The y-axis acceleration.
        y_acceleration: i32,
    },
    /// The state of a drums controller key changed.
    ///
    /// Received only if `Channels::DRUMS` is open.
    DrumsKey(key::DrumsKey, KeyState),
    /// Reports the movement of an analog stick from a
    /// drums controller.
    ///
    /// Received only if `Channels::DRUMS` is open.
    // todo: figure out how many drums, and how to report pressure.
    DrumsMove {},
    /// The state of a guitar controller key changed.
    ///
    /// Received only if `Channels::GUITAR` is open.
    GuitarKey(key::GuitarKey, KeyState),
    /// Reports the movement of an analog stick, the whammy bar,
    /// or the fret bar from a guitar controller.
    ///
    /// Received only if `Channels::GUITAR` is open.
    GuitarMove {
        /// The x-axis analog stick position.
        x: i32,
        /// The y-axis analog stick position.
        y: i32,
        /// The whammy bar position.
        whammy_bar: i32,
        /// The fret bar absolute position.
        fret_bar: i32,
    },
    /// The device was removed.
    ///
    /// Received only if watch mode was enabled via `Device::watch`.
    // todo, can we close the iterator instead?
    Removed,
}

pub struct Event {
    pub time: Instant,
    pub kind: EventKind,
}

pub(crate) struct EventStream {
    handle: *mut xwiimote_sys::iface,
    raw_event: xwiimote_sys::event,
}

impl EventStream {
    // todo: document that the caller is responsible for opening
    //       the channels.
    pub fn new(handle: *mut xwiimote_sys::iface) -> Self {
        Self {
            handle,
            raw_event: xwiimote_sys::event::default(),
        }
    }
}

impl Iterator for EventStream {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
