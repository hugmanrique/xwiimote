#[cfg(doc)]
use crate::Channels;
use crate::IoBlocker;
use crate::{Device, Result};
use futures::Stream;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};
use std::{io, mem};

// Keys

// We provide a key enumeration for each controller and extension type.
// To avoid repetition, we use a macro to define the common key variants.
// A matrix of the buttons reported by each device is given in `BUTTONS.md`.
// This macro definition uses the TT munching technique.
macro_rules! key_enum {
    ($doc:expr, $name:ident {$($body:tt)*} ($variant:expr) $($tail:tt)*) => {
        inner_key_enum! {
            $doc:expr,
            $name {
                $($body)*  // Previously-built variants.
                $variant,
            }
        }
        $($tail)* // Unprocessed variants.
    };
    // There are no more variants, emit the enum definition.
    ($doc:expr, $name:ident {$($body:tt)*}) => {
        #[derive(Copy, Clone, Debug, FromPrimitive)]
        #[doc = $doc]
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
    ($doc:expr, $name:ident {$($body:tt)*}) => {
        key_enum!{
            $doc,
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
    ($doc:expr, $name:ident {$($body:tt)*}) => {
        regular_controller_key_enum!{
            $doc,
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

regular_controller_key_enum!(
    "The keys of a Wii Remote",
    Key {
        /// 1 button.
        One = 9,
        /// 2 button.
        Two = 10
    }
);

gamepad_key_enum!(
    "The keys of a Wii U Pro controller",
    ProControllerKey {
        /// Left thumb button.
        ///
        /// Reported if the left analog stick is pressed.
        LeftThumb = 17,
        /// Right thumb button.
        ///
        /// Reported if the right analog stick is pressed.
        RightThumb = 18,
    }
);

gamepad_key_enum!("The keys of a Classic controller", ClassicControllerKey {});

/// The keys of a Nunchuk.
// This is the only extension that doesn't have the + and - buttons.
#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum NunchukKey {
    /// C button.
    C = 19,
    /// Z button.
    Z = 20,
}

key_enum!("The keys of a drums controller.", DrumsKey {});

key_enum!("The keys of a guitar controller.",
    GuitarKey {
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
    }
);

/// The state of a key.
#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum KeyState {
    /// The key is released.
    Up = 0,
    /// The key is held down.
    Down,
    /// The key is [held down](`Self::Down`), and was reported as so in
    /// the previous event for the same key.
    AutoRepeat,
}

// Event kinds

const MAX_IR_SOURCES: usize = 4;

/// An IR source detected by the IR camera, as reported in [`EventKind::Ir`].
#[derive(Copy, Clone, Debug)]
pub struct IrSource {
    /// The x-axis position.
    pub x: i32,
    /// The y-axis position.
    pub y: i32,
}

impl IrSource {
    /// Parses the IR source data from the given event.
    ///
    /// # Safety
    /// Assumes `raw` points to an event of type [`xwiimote_sys::EVENT_IR`].
    unsafe fn parse(raw: &xwiimote_sys::event) -> [Option<IrSource>; MAX_IR_SOURCES] {
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
    Key(Key, KeyState),
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
    ProControllerKey(ProControllerKey, KeyState),
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
    ClassicControllerKey(ClassicControllerKey, KeyState),
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
    NunchukKey(NunchukKey, KeyState),
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
    DrumsKey(DrumsKey, KeyState),
    /// Reports the movement of an analog stick from a
    /// drums controller.
    ///
    /// Received only if [`Channels::DRUMS`] is open.
    // todo: figure out how many drums, and how to report pressure.
    DrumsMove {},
    /// The state of a guitar controller key changed.
    ///
    /// Received only if [`Channels::GUITAR`] is open.
    GuitarKey(GuitarKey, KeyState),
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
    pub time: SystemTime,
    /// The event type.
    pub kind: EventKind,
}

impl Event {
    /// Parses the event.
    ///
    /// # Safety
    /// Assumes that `raw` is an object returned by [`xwiimote_sys::event_dispatch`].
    unsafe fn parse(raw: &xwiimote_sys::event) -> Self {
        // Rust does not provide a way to create a `SystemTime` directly.
        let since_epoch = Duration::new(raw.time.tv_sec as u64, raw.time.tv_usec as u32 * 1000);
        let time = SystemTime::UNIX_EPOCH + since_epoch;

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
            xwiimote_sys::EVENT_IR => EventKind::Ir(IrSource::parse(raw)),
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
            xwiimote_sys::EVENT_GONE => panic!("unexpected removal event"), // handled by `EventStream`
            type_id => panic!("unexpected event type {}", type_id),
        };
        Event { time, kind }
    }

    unsafe fn parse_key<T: FromPrimitive>(raw: &xwiimote_sys::event) -> (T, KeyState) {
        let data = raw.v.key;
        let key =
            T::from_u32(data.code).unwrap_or_else(|| panic!("unknown key code {}", data.code));
        let state = KeyState::from_u32(data.state)
            .unwrap_or_else(|| panic!("unknown key state {}", data.state));
        (key, state)
    }
}

/// Watches for events from a [`Device`].
///
/// The kinds of streamed events depend on the open channels with
/// the device. See the description of each [`EventKind`] variant
/// for the channels needed to receive events of a certain kind.
pub struct EventStream<'a> {
    device: &'a Device,
    // Reuse event across
    last_event: xwiimote_sys::event,
    // Whether the epoll interest is currently registered. Used to
    // prevent a double-close when dropping the stream.
    have_interest: bool,
}

impl<'a> EventStream<'a> {
    const EPOLL_EVENTS: libc::c_int = libc::EPOLLIN | libc::EPOLLHUP | libc::EPOLLPRI;

    /// Creates a new stream over the events from the device.
    pub(crate) fn try_new(device: &'a Device) -> Result<Self> {
        // Watch the device fd for read availability to avoid busy-waiting.
        let fd = unsafe { xwiimote_sys::iface_get_fd(device.handle) };
        IoBlocker::get().add_interest(fd, Self::EPOLL_EVENTS)?;

        Ok(Self {
            device,
            last_event: Default::default(),
            have_interest: true,
        })
    }

    /// Removes interest for the [`Device`] file events.
    fn remove_interest(&mut self) -> Result<()> {
        if self.have_interest {
            self.have_interest = false;

            let fd = unsafe { xwiimote_sys::iface_get_fd(self.device.handle) };
            IoBlocker::get().remove_interest(fd, Self::EPOLL_EVENTS)
        } else {
            Ok(())
        }
    }
}

impl Stream for EventStream<'_> {
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.have_interest {
            // We stop reading events once a disconnect event is received.
            return Poll::Ready(None);
        }

        // Attempt to read a single incoming event.
        let res_code = unsafe {
            xwiimote_sys::iface_dispatch(
                self.device.handle,
                &mut self.last_event,
                mem::size_of::<xwiimote_sys::event>(),
            )
        };

        const PENDING: libc::c_int = -libc::EAGAIN;
        let result = match res_code {
            0 => {
                if self.last_event.type_ == xwiimote_sys::EVENT_GONE {
                    // We were watching for hot-plug events, and the device
                    // was closed. No more events are coming.
                    self.remove_interest().err().map(|why| Err(why))
                } else {
                    let event = unsafe { Event::parse(&self.last_event) };
                    Some(Ok(event))
                }
            }
            PENDING => {
                // No event is available, arrange for `wake` to be called once
                // an event is available.
                let fd = unsafe { xwiimote_sys::iface_get_fd(self.device.handle) };
                IoBlocker::get().set_callback(fd, cx.waker().clone());
                return Poll::Pending;
            }
            // Failure, perhaps the device was disconnected.
            _ => Some(Err(io::Error::last_os_error())),
        };
        Poll::Ready(result)
    }
}

impl Drop for EventStream<'_> {
    fn drop(&mut self) {
        self.remove_interest()
            .expect("failed to remove interest for device fd");
    }
}
