#[cfg(doc)]
use crate::device::{Channels, Device};
use crate::device::{DeviceError, DeviceResult};
use crate::key;
use crate::key::KeyState;
use epoll_rs::{Epoll, Opts};
use fallible_iterator::FallibleIterator;
use num_traits::FromPrimitive;
use std::fs::File;
use std::mem;
use unix_ts::Timestamp;

const MAX_IR_SOURCES: usize = 4;

/// An IR source detected by the IR camera, as reported
/// in [`EventKind::Ir`].
#[derive(Copy, Clone, Debug)]
pub struct IrSource {
    /// The x-axis position.
    pub x: i32,
    /// The y-axis position.
    pub y: i32,
}

impl IrSource {
    unsafe fn from(raw: &xwiimote_sys::event) -> [Option<IrSource>; MAX_IR_SOURCES] {
        const MISSING_SOURCE: i32 = 1023;
        let mut sources: [Option<_>; MAX_IR_SOURCES] = Default::default();

        for (ix, pos) in raw.v.abs.iter().take(MAX_IR_SOURCES).enumerate() {
            if pos.x != MISSING_SOURCE && pos.y != MISSING_SOURCE {
                sources[ix] = Some(IrSource { x: pos.x, y: pos.y })
            }
        }
        sources
    }
}

/// The type of an [`Event`], including its associated data.
#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum EventKind {
    /// The state of a Wii Remote controller key changed.
    ///
    /// Received only if [`Channels::CORE`] is open.
    Key(key::Key, KeyState),
    /// Provides the accelerometer data.
    ///
    /// Received only if [`Channels::ACCELEROMETER`] is open.
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
    /// Received only if [`Channels::IR`] is open.
    Ir([Option<IrSource>; MAX_IR_SOURCES]),
    /// Provides Balance Board weight data. Four sensors report
    /// data for each of the edges of the board.
    ///
    /// Received only if [`Channels::BALANCE_BOARD`] is open.
    BalanceBoard([i32; 4]),
    /// Provides the Motion Plus extension gyroscope data.
    ///
    /// Received only if [`Channels::MOTION_PLUS`] is open.
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
    /// Received only if [`Channels::PRO_CONTROLLER`] is open.
    ProControllerKey(key::ProControllerKey, KeyState),
    /// Reports the movement of an analog stick from
    /// a Wii U Pro controller.
    ///
    /// Received only if [`Channels::PRO_CONTROLLER`] is open.
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
    /// An extension was plugged or unplugged, or some other static
    /// data that cannot be monitored separately changed.
    ///
    /// No payload is provided, hence the application should check
    /// what changed by examining the [`Device`] manually.
    Other,
    /// The state of a Classic controller key changed.
    ///
    /// Received only if [`Channels::CLASSIC_CONTROLLER`] is open.
    ClassicControllerKey(key::ClassicControllerKey, KeyState),
    /// Reports the movement of an analog stick from
    /// a Classic controller.
    ///
    /// Received only if [`Channels::CLASSIC_CONTROLLER`] is open.
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
    /// Received only if [`Channels::NUNCHUK`] is open.
    NunchukKey(key::NunchukKey, KeyState),
    /// Reports the movement of an analog stick from a Nunchuk.
    ///
    /// Received only if [`Channels::NUNCHUK`] is open.
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
    /// Received only if [`Channels::DRUMS`] is open.
    DrumsKey(key::DrumsKey, KeyState),
    /// Reports the movement of an analog stick from a
    /// drums controller.
    ///
    /// Received only if [`Channels::DRUMS`] is open.
    // todo: figure out how many drums, and how to report pressure.
    DrumsMove {},
    /// The state of a guitar controller key changed.
    ///
    /// Received only if [`Channels::GUITAR`] is open.
    GuitarKey(key::GuitarKey, KeyState),
    /// Reports the movement of an analog stick, the whammy bar,
    /// or the fret bar from a guitar controller.
    ///
    /// Received only if [`Channels::GUITAR`] is open.
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
}

/// An event received from an open channel to a [`Device`].
#[derive(Copy, Clone, Debug)]
pub struct Event {
    /// The time at which the kernel generated the event.
    pub time: Timestamp,
    /// The event type.
    pub kind: EventKind,
}

impl Event {
    /// Parses the given raw event.
    ///
    /// # Safety
    /// Assumes that `raw` contains a valid event, as
    /// returned by [`xwiimote_sys::event_dispatch`].
    unsafe fn parse(raw: &xwiimote_sys::event) -> Self {
        let time = Timestamp::new(raw.time.tv_sec, raw.time.tv_usec as u32 * 1000);
        let kind = match raw.type_ {
            xwiimote_sys::EVENT_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::Key(key, state)
            }
            xwiimote_sys::EVENT_ACCEL => {
                let acc = raw.v.abs[0];
                EventKind::Accelerometer {
                    x: acc.x,
                    y: acc.y,
                    z: acc.z,
                }
            }
            xwiimote_sys::EVENT_IR => EventKind::Ir(IrSource::from(raw)),
            xwiimote_sys::EVENT_BALANCE_BOARD => {
                let weights = raw.v.abs;
                EventKind::BalanceBoard([weights[0].x, weights[1].x, weights[2].x, weights[3].x])
            }
            xwiimote_sys::EVENT_MOTION_PLUS => {
                let rot_speed = raw.v.abs[0];
                EventKind::MotionPlus {
                    x: rot_speed.x,
                    y: rot_speed.y,
                    z: rot_speed.z,
                }
            }
            xwiimote_sys::EVENT_PRO_CONTROLLER_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::ProControllerKey(key, state)
            }
            xwiimote_sys::EVENT_PRO_CONTROLLER_MOVE => {
                let pos = raw.v.abs;
                EventKind::ProControllerMove {
                    left_x: pos[0].x,
                    left_y: pos[0].y,
                    right_x: pos[1].x,
                    right_y: pos[1].y,
                }
            }
            xwiimote_sys::EVENT_WATCH => EventKind::Other,
            xwiimote_sys::EVENT_CLASSIC_CONTROLLER_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::ClassicControllerKey(key, state)
            }
            xwiimote_sys::EVENT_CLASSIC_CONTROLLER_MOVE => {
                let pos = raw.v.abs;
                EventKind::ClassicControllerMove {
                    left_x: pos[0].x,
                    left_y: pos[0].y,
                    right_x: pos[1].x,
                    right_y: pos[1].y,
                    left_trigger: pos[2].x as u8,
                    right_trigger: pos[2].y as u8,
                }
            }
            xwiimote_sys::EVENT_NUNCHUK_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::NunchukKey(key, state)
            }
            xwiimote_sys::EVENT_NUNCHUK_MOVE => {
                let values = raw.v.abs;
                EventKind::NunchukMove {
                    x: values[0].x,
                    y: values[0].y,
                    x_acceleration: values[1].x,
                    y_acceleration: values[1].y,
                }
            }
            xwiimote_sys::EVENT_DRUMS_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::DrumsKey(key, state)
            }
            xwiimote_sys::EVENT_DRUMS_MOVE => todo!(),
            xwiimote_sys::EVENT_GUITAR_KEY => {
                let (key, state) = Self::parse_key(raw);
                EventKind::GuitarKey(key, state)
            }
            xwiimote_sys::EVENT_GONE => panic!("unexpected removal event"),
            type_id => panic!("unexpected event type {}", type_id),
        };
        Event { time, kind }
    }
    unsafe fn parse_key<T: FromPrimitive>(raw: &xwiimote_sys::event) -> (T, key::KeyState) {
        let data = raw.v.key;
        (
            T::from_u32(data.code).unwrap_or_else(|| panic!("unknown key code {}", data.code)),
            key::KeyState::from_u32(data.state)
                .unwrap_or_else(|| panic!("unknown key state {}", data.state)),
        )
    }
}

/// Watches for events on the file descriptor used by a [`Device`].
///
/// The kinds of events returned by [`Self::next()`] depend on the open
/// channels with the device. See each [`EventKind`] variant for the
/// required channels to receive events of a certain type.
pub struct EventStream {
    handle: *mut xwiimote_sys::iface,
    epoll: Epoll,
    last_event: xwiimote_sys::event,
}

impl EventStream {
    pub(crate) fn open(handle: *mut xwiimote_sys::iface) -> DeviceResult<Self> {
        // We watch the device for read availability to avoid busy-waiting.
        let dev_fd = unsafe { xwiimote_sys::iface_get_fd(handle) };
        let epoll = Epoll::new()?;
        unsafe { epoll.add_raw_fd::<File>(dev_fd, Opts::IN | Opts::HUP | Opts::ERR)? };

        Ok(Self {
            handle,
            epoll,
            last_event: Default::default(),
        })
    }
}

impl FallibleIterator for EventStream {
    type Item = Event;
    type Error = DeviceError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            let ep_event = self.epoll.wait_one()?;
            return match ep_event.events {
                Opts::IN => {
                    const EAGAIN: i32 = -11; // -libc::EAGAIN
                    let err_code = unsafe {
                        xwiimote_sys::iface_dispatch(
                            self.handle,
                            &mut self.last_event,
                            mem::size_of::<xwiimote_sys::event>(),
                        )
                    };
                    match err_code {
                        0 => {
                            if self.last_event.type_ == xwiimote_sys::EVENT_GONE {
                                // We were watching for hot-plug events, and
                                // the device was closed.
                                return Ok(None);
                            }

                            let event = unsafe { Event::parse(&self.last_event) };
                            Ok(Some(event))
                        }
                        EAGAIN => continue, // Poll until readable again
                        err => Err(DeviceError(err)),
                    }
                }
                Opts::HUP | Opts::ERR => Ok(None),
                event => panic!("Unexpected epoll event {:?}", event),
            };
        }
    }
}
