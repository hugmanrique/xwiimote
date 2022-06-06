# Key types
The following list shows the key types reported by each device (a controller or an
extension), as defined in the `xwiimote_event_types` enum. This data is also determined
by which enumeration each key type belongs to in `src/event.rs`, but the tabular form
may be helpful.

Note that `GuitarKey::StarPower` corresponds to the Home button.
The guitar's fret bar and strum bar buttons are not shown.

```
// wiimote: LEFT RIGHT UP DOWN PLUS MINUS HOME     A B                           ONE TWO
// pro    : LEFT RIGHT UP DOWN PLUS MINUS HOME X Y A B TR TL ZR ZL THUMBL THUMBR
// classic: LEFT RIGHT UP DOWN PLUS MINUS HOME X Y A B TR TL ZR ZL
// nunchuk:                                                                              C Z
// drums  :                    PLUS MINUS                                                    
// guitar :                    PLUS MINUS                                                    STAR_POWER FRET_FAR_UP FRET_UP FRET_MID FRET_LOW FRET_FAR_LOW STRUM_BAR_UP STRUM_BAR_LOW 
```