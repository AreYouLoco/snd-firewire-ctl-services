// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! The implementation for protocol defined by Tascam specific to FireWire series.
//!
//! The crate includes traits, structures, and enumerations for protocol defined by Tascam specific
//! to its FireWire series.
//!
//! # The way to process events for control surface
//!
//! Tascam FW-1884, FW-1082, and FE-8 have control surface, in which the hardware make no
//! superficial change to the surface according to user operation. Instead, the surface notifies
//! the operation to system.
//!
//! In FE-8, the image consists of 32 quadlets. Asynchronous notification is simply sent for one
//! of quadlet to which user operation effects.
//!
//! In FW-1884 and FW-1082, the state of surface is expressed as an image which consists of 64
//! quadlets. One of the quadlets is multiplexed to data block in isochronous packet as well as
//! PCM frame in order.
//!
//! FW-1884 and FW-1082 has shift button to divert some buttons. Furthermore, FW-1082 has some
//! rotaries and buttons which change their role according to encoder mode. The module includes
//! two stuffs to abstract the above design; surface state and machine state. The former is used
//! used to parse surface imageg and detect event and operate LED. The latter is used to monitor
//! current state of each surface item by handling the event, and generate normalized events. It's
//! task of runtime implementation to prepare converter between the machine event and application
//! specific message such as ALSA Sequencer and Open Sound Control.
//!
//! The relationship between the constrol surface, surface state, machine state, and message
//! converter is illustrated in below diagram:
//!
//! ```text
//!                       ++====================================================++
//!                       ||                  Service runtime                   ||
//!                       ||                                                    ||
//! ++==========++  surface image    +---------+   machine event   +---------+  ||
//! ||          || ----------------> |         | ----------------> |         |  ||
//! || surface  ||        ||         | surface |                   | machine |  ||
//! || hardware ||  LED operation    |  state  |   machine event   |  state  |  ||
//! ||          || <---------------- |         | <---------------- |         |  ||
//! ++==========++        ||         +---------+                   +---------+  ||
//!                       ||                                         ^    |     ||
//!                       ||                                      machine event ||
//!                       ||                                         |    v     ||
//!                       ||                                       +---------+  ||
//!                       ||                                       |         |  ||
//!                       ||                                       | message |  ||
//!                       ||                                       |converter|  ||
//!                       ||                                       |         |  ||
//!                       ||                                       +---------+  ||
//!                       ||                                         ^    |     ||
//!                       ||                                   specific message ||
//!                       ||                                         |    |     ||
//!                       ++=========================================|====|=====++
//!                                                                  |    |
//!                                     Inter process communication  |    |
//!                                     (ALSA Sequencer, OSC, etc.)  |    v
//!                                                             ++=============++
//!                                                             || application ||
//!                                                             ++=============++
//! ```

pub mod asynch;
pub mod isoch;

pub mod config_rom;

use glib::Error;

use hinawa::{FwNode, FwReq, FwReqExtManual, FwTcode};

const BASE_OFFSET: u64 = 0xffff00000000;
const HW_INFO_REGISTER_OFFSET: u64 = 0x00;
const HW_INFO_FPGA_OFFSET: u64 = 0x04;
const HW_INFO_ARM_OFFSET: u64 = 0x08;
const HW_INFO_HW_OFFSET: u64 = 0x0c;
const LED_OFFSET: u64 = 0x0404;

fn read_quadlet(
    req: &mut FwReq,
    node: &mut FwNode,
    offset: u64,
    frames: &mut [u8],
    timeout_ms: u32,
) -> Result<(), Error> {
    req.transaction_sync(
        node,
        FwTcode::ReadQuadletRequest,
        BASE_OFFSET + offset,
        4,
        frames,
        timeout_ms,
    )
}

fn write_quadlet(
    req: &mut FwReq,
    node: &FwNode,
    offset: u64,
    frames: &mut [u8],
    timeout_ms: u32,
) -> Result<(), Error> {
    req.transaction_sync(
        node,
        FwTcode::WriteQuadletRequest,
        BASE_OFFSET + offset,
        4,
        frames,
        timeout_ms,
    )
}

/// The structure of hardware information.
#[derive(Debug, Default, Copy, Clone)]
pub struct HardwareInformation {
    pub register: u32,
    pub fpga: u32,
    pub arm: u32,
    pub hardware: u32,
}

/// The structure for protocol implementaion commonly available to Tascam FireWire models.
#[derive(Debug, Default)]
pub struct HardwareInformationProtocol;

/// The trait for oepration of hardware information.
impl HardwareInformationProtocol {
    pub fn read_hardware_information(
        req: &mut FwReq,
        node: &mut FwNode,
        info: &mut HardwareInformation,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        let mut quads = [0; 4];
        read_quadlet(req, node, HW_INFO_REGISTER_OFFSET, &mut quads, timeout_ms)
            .map(|_| info.register = u32::from_be_bytes(quads))?;
        read_quadlet(req, node, HW_INFO_FPGA_OFFSET, &mut quads, timeout_ms)
            .map(|_| info.fpga = u32::from_be_bytes(quads))?;
        read_quadlet(req, node, HW_INFO_ARM_OFFSET, &mut quads, timeout_ms)
            .map(|_| info.arm = u32::from_be_bytes(quads))?;
        read_quadlet(req, node, HW_INFO_HW_OFFSET, &mut quads, timeout_ms)
            .map(|_| info.hardware = u32::from_be_bytes(quads))?;
        Ok(())
    }
}

/// The enumeration for surface items.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MachineItem {
    // Channel section.
    Master,
    Ol(usize),
    Rec(usize),
    Signal(usize),
    Rotary(usize),
    Select(usize),
    Solo(usize),
    Mute(usize),
    Input(usize),
    Func(usize),
    Pfl,

    // Global section.
    Read,
    Wrt,
    Tch,
    Latch,
    Wheel,
    Shuttle,
    Computer,
    Clock,
    Up,
    Left,
    Down,
    Right,
    NudgeLeft,
    NudgeRight,
    LocateLeft,
    LocateRight,
    Set,
    In,
    Out,

    // Encoder section.
    Flip,
    Pan, // has bool value in FW-1884, has u16 value in FW-1082.
    Aux(usize),
    EncoderMode, // FW-1082 only.

    // Equalizer section.
    High,
    HighMid,
    LowMid,
    Low,
    Recall,
    Gain,
    Freq,
    Q,

    // Bank section.
    Bank,

    // Transport section.
    Rew,
    Fwd,
    Stop,
    Play,
    Record,

    // Shortcut section.
    Panel,
    Save,
    Revert,
    AllSafe,
    ClrSolo,
    Markers,
    Loop,
    Cut,
    Del,
    Copy,
    Paste,
    Alt,
    Cmd,
    Undo,
    Shift,
    Ctrl,
}

impl Default for MachineItem {
    fn default() -> Self {
        Self::Master
    }
}

impl std::fmt::Display for MachineItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Master => write!(f, "master"),
            Self::Ol(ch) => write!(f, "ol {}", ch),
            Self::Rec(ch) => write!(f, "rec {}", ch),
            Self::Signal(ch) => write!(f, "signal {}", ch),
            Self::Rotary(ch) => write!(f, "rotary {}", ch),
            Self::Select(ch) => write!(f, "select {}", ch),
            Self::Solo(ch) => write!(f, "solo {}", ch),
            Self::Mute(ch) => write!(f, "mute {}", ch),
            Self::Input(ch) => write!(f, "input {}", ch),
            Self::Func(ch) => write!(f, "func {}", ch),
            Self::Pfl => write!(f, "pfl"),
            Self::Read => write!(f, "read"),
            Self::Wrt => write!(f, "wrt"),
            Self::Tch => write!(f, "tch"),
            Self::Latch => write!(f, "latch"),
            Self::Wheel => write!(f, "wheel"),
            Self::Shuttle => write!(f, "Shuttle"),
            Self::Computer => write!(f, "computer"),
            Self::Clock => write!(f, "clock"),
            Self::Up => write!(f, "up"),
            Self::Left => write!(f, "left"),
            Self::Down => write!(f, "down"),
            Self::Right => write!(f, "right"),
            Self::NudgeLeft => write!(f, "nudge left"),
            Self::NudgeRight => write!(f, "nudge right"),
            Self::LocateLeft => write!(f, "locate left"),
            Self::LocateRight => write!(f, "locate right"),
            Self::Set => write!(f, "set"),
            Self::In => write!(f, "in"),
            Self::Out => write!(f, "out"),
            Self::Flip => write!(f, "flip"),
            Self::Pan => write!(f, "pan"),
            Self::Aux(ch) => write!(f, "aux {}", ch),
            Self::EncoderMode => write!(f, "encoder model"),
            Self::High => write!(f, "high"),
            Self::HighMid => write!(f, "high-mid"),
            Self::LowMid => write!(f, "low-mid"),
            Self::Low => write!(f, "low"),
            Self::Recall => write!(f, "recall"),
            Self::Gain => write!(f, "gain"),
            Self::Freq => write!(f, "freq"),
            Self::Q => write!(f, "q"),
            Self::Bank => write!(f, "bank"),
            Self::Rew => write!(f, "rew"),
            Self::Fwd => write!(f, "fwd"),
            Self::Stop => write!(f, "stop"),
            Self::Play => write!(f, "play"),
            Self::Record => write!(f, "record"),
            Self::Panel => write!(f, "panel"),
            Self::Save => write!(f, "save"),
            Self::Revert => write!(f, "revert"),
            Self::AllSafe => write!(f, "all safe"),
            Self::ClrSolo => write!(f, "clr solo"),
            Self::Markers => write!(f, "markers"),
            Self::Loop => write!(f, "loop"),
            Self::Cut => write!(f, "cut"),
            Self::Del => write!(f, "del"),
            Self::Copy => write!(f, "copy"),
            Self::Paste => write!(f, "paste"),
            Self::Alt => write!(f, "alt"),
            Self::Cmd => write!(f, "cmd"),
            Self::Undo => write!(f, "undo"),
            Self::Shift => write!(f, "shift"),
            Self::Ctrl => write!(f, "ctrl"),
        }
    }
}

/// The state machine of control surface.
#[derive(Default, Debug)]
pub struct MachineState {
    /// The boolean value of each item.
    bool_items: Vec<bool>,
    /// The u16 value of each item.
    u16_items: Vec<u16>,
    /// Between 0-3.
    bank: u16,
    /// One of Rew, Fwd, Stop, Play, and Record.
    transport: MachineItem,
}

/// The event of state machine.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ItemValue {
    Bool(bool),
    U16(u16),
}

const BANK_MIN: u16 = 0;
const BANK_MAX: u16 = 3;

/// The trait for operation of state machine.
pub trait MachineStateOperation {
    const BOOL_ITEMS: &'static [MachineItem];
    const U16_ITEMS: &'static [MachineItem];
    const HAS_TRANSPORT: bool;
    const HAS_BANK: bool;

    const BANK_MIN: u16 = BANK_MIN;
    const BANK_MAX: u16 = BANK_MAX;

    const U16_ITEM_MIN: u16 = 0;
    const U16_ITEM_MAX: u16 = 0xffffu16;

    const TRANSPORT_ITEMS: [MachineItem; 5] = [
        MachineItem::Rew,
        MachineItem::Fwd,
        MachineItem::Stop,
        MachineItem::Play,
        MachineItem::Record,
    ];

    const EQ_BAND_ITEMS: [MachineItem; 4] = [
        MachineItem::High,
        MachineItem::HighMid,
        MachineItem::LowMid,
        MachineItem::Low,
    ];

    fn initialize_machine(state: &mut MachineState) {
        state.bool_items = vec![false; Self::BOOL_ITEMS.len()];
        state.u16_items = vec![0; Self::U16_ITEMS.len()];
        state.bank = 0;
        state.transport = MachineItem::Stop;
    }

    fn get_machine_current_values(state: &MachineState) -> Vec<(MachineItem, ItemValue)> {
        let mut machine_values = Vec::new();

        Self::BOOL_ITEMS
            .iter()
            .zip(state.bool_items.iter())
            .for_each(|(&item, &value)| machine_values.push((item, ItemValue::Bool(value))));

        Self::U16_ITEMS
            .iter()
            .zip(state.u16_items.iter())
            .for_each(|(&item, &value)| machine_values.push((item, ItemValue::U16(value))));

        if Self::HAS_BANK {
            machine_values.push((MachineItem::Bank, ItemValue::U16(state.bank)));
        }

        if Self::HAS_TRANSPORT {
            Self::TRANSPORT_ITEMS.iter().for_each(|&item| {
                machine_values.push((item, ItemValue::Bool(item.eq(&state.transport))));
            });
        }

        machine_values
    }

    fn change_machine_value(
        state: &mut MachineState,
        input: &(MachineItem, ItemValue),
    ) -> Vec<(MachineItem, ItemValue)> {
        let mut outputs = Vec::new();

        if let ItemValue::Bool(value) = input.1 {
            // Normal items.
            let _ = Self::BOOL_ITEMS
                .iter()
                .zip(state.bool_items.iter_mut())
                .find(|(i, v)| input.0.eq(i) && !value.eq(v))
                .map(|(_, v)| {
                    *v = value;
                    outputs.push((input.0, ItemValue::Bool(*v)));
                });

            // One of transport items should be enabled.
            if Self::HAS_TRANSPORT
                && Self::TRANSPORT_ITEMS
                    .iter()
                    .find(|i| input.0.eq(i))
                    .is_some()
            {
                if input.0 != state.transport {
                    outputs.push((state.transport, ItemValue::Bool(false)));
                    outputs.push((input.0, ItemValue::Bool(true)));
                    state.transport = input.0;
                }
            }

            // None of, or one of equalizer band items should be enabled.
            if Self::EQ_BAND_ITEMS.iter().find(|i| input.0.eq(i)).is_some() {
                if value {
                    Self::BOOL_ITEMS
                        .iter()
                        .zip(state.bool_items.iter_mut())
                        .filter(|(i, v)| {
                            !input.0.eq(i)
                                && **v
                                && Self::EQ_BAND_ITEMS.iter().find(|item| item.eq(i)).is_some()
                        })
                        .for_each(|(i, v)| {
                            *v = false;
                            outputs.push((*i, ItemValue::Bool(*v)));
                        });
                }
            }
        } else if let ItemValue::U16(value) = input.1 {
            let _ = Self::U16_ITEMS
                .iter()
                .zip(state.u16_items.iter_mut())
                .find(|(i, v)| input.0.eq(i) && !value.eq(v))
                .map(|(_, v)| {
                    *v = value;
                    outputs.push((input.0, ItemValue::U16(*v)));
                });

            if Self::HAS_BANK && input.0 == MachineItem::Bank {
                if state.bank != value && value <= Self::BANK_MAX {
                    state.bank = value;
                    outputs.push((MachineItem::Bank, ItemValue::U16(state.bank)));
                }
            }
        }

        outputs
    }
}

/// The trait for operation of constol surface.
pub trait SurfaceImageOperation<T> {
    fn initialize_surface_state(state: &mut T);

    fn decode_surface_image(
        state: &T,
        image: &[u32],
        index: u32,
        before: u32,
        after: u32,
    ) -> Vec<(MachineItem, ItemValue)>;

    fn feedback_to_surface(
        state: &mut T,
        machine_value: &(MachineItem, ItemValue),
        req: &mut FwReq,
        node: &mut FwNode,
        timeout_ms: u32,
    ) -> Result<(), Error>;

    fn finalize_surface(
        state: &mut T,
        req: &mut FwReq,
        node: &mut FwNode,
        timeout_ms: u32,
    ) -> Result<(), Error>;
}

/// The structure for common state of surface.
#[derive(Default, Debug)]
struct SurfaceCommonState {
    stateful_items: Vec<bool>,
}

/// The structure of boolean value in surface image.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
struct SurfaceBoolValue(usize, u32); // index, mask.

/// The structure of u16 value in surface image.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
struct SurfaceU16Value(usize, u32, usize); // index, mask, shift

fn detect_stateful_bool_action(
    bool_val: &SurfaceBoolValue,
    index: u32,
    before: u32,
    after: u32,
) -> bool {
    bool_val.0 == index as usize && (before ^ after) & bool_val.1 > 0 && before & bool_val.1 > 0
}

fn detect_bool_action(bool_val: &SurfaceBoolValue, index: u32, before: u32, after: u32) -> bool {
    bool_val.0 == index as usize && (before ^ after) & bool_val.1 > 0
}

fn detect_bool_value(bool_val: &SurfaceBoolValue, before: u32) -> bool {
    before & bool_val.1 > 0
}

fn detect_u16_action(u16_val: &SurfaceU16Value, index: u32, before: u32, after: u32) -> bool {
    u16_val.0 == index as usize && (before ^ after) & u16_val.1 > 0
}

fn detect_u16_value(u16_val: &SurfaceU16Value, after: u32) -> u16 {
    ((after & u16_val.1) >> u16_val.2) as u16
}

fn detect_u16_value_in_image(u16_val: &SurfaceU16Value, image: &[u32]) -> u16 {
    ((image[u16_val.0] & u16_val.1) >> u16_val.2) as u16
}

/// The trait for operation to convert between image and machine value.
trait SurfaceImageCommonOperation {
    const STATEFUL_ITEMS: &'static [(SurfaceBoolValue, MachineItem)];
    const STATELESS_ITEMS: &'static [(SurfaceBoolValue, MachineItem)];
    const ROTARIES: &'static [(SurfaceU16Value, MachineItem)];
    const FADERS: &'static [(SurfaceBoolValue, SurfaceU16Value, MachineItem)];

    fn initialize_surface_common_state(state: &mut SurfaceCommonState) {
        state.stateful_items = vec![Default::default(); Self::STATEFUL_ITEMS.len()];
    }

    fn decode_surface_image_common(
        machine_values: &mut Vec<(MachineItem, ItemValue)>,
        state: &SurfaceCommonState,
        image: &[u32],
        index: u32,
        before: u32,
        after: u32,
    ) {
        Self::STATEFUL_ITEMS
            .iter()
            .zip(state.stateful_items.iter())
            .filter(|((bool_val, _), _)| {
                detect_stateful_bool_action(bool_val, index, before, after)
            })
            .for_each(|((_, item), &s)| machine_values.push((*item, ItemValue::Bool(!s))));

        Self::STATELESS_ITEMS
            .iter()
            .filter(|(bool_val, _)| detect_bool_action(bool_val, index, before, after))
            .for_each(|(bool_val, item)| {
                let value = detect_bool_value(bool_val, before);
                machine_values.push((*item, ItemValue::Bool(value)));
            });

        Self::ROTARIES
            .iter()
            .filter(|(u16_val, _)| detect_u16_action(u16_val, index, before, after))
            .for_each(|(u16_val, item)| {
                let value = detect_u16_value(u16_val, after);
                machine_values.push((*item, ItemValue::U16(value)));
            });

        Self::FADERS
            .iter()
            .filter(|(bool_val, _, _)| detect_bool_action(bool_val, index, before, after))
            .for_each(|(_, u16_val, item)| {
                let value = detect_u16_value_in_image(u16_val, image);
                machine_values.push((*item, ItemValue::U16(value)));
            });
    }

    fn feedback_to_surface_common(
        state: &mut SurfaceCommonState,
        machine_value: &(MachineItem, ItemValue),
    ) {
        Self::STATEFUL_ITEMS
            .iter()
            .zip(state.stateful_items.iter_mut())
            .find(|((_, item), _)| machine_value.0.eq(item))
            .map(|((_, _), s)| *s = !*s);
    }
}

#[derive(Default, Debug)]
struct LedState(Vec<u16>);

fn operate_led(
    req: &mut FwReq,
    node: &mut FwNode,
    pos: u16,
    enable: bool,
    timeout_ms: u32,
) -> Result<(), Error> {
    let mut frame = [0; 4];
    frame[0..2].copy_from_slice(&(enable as u16).to_be_bytes());
    frame[2..4].copy_from_slice(&pos.to_be_bytes());
    write_quadlet(req, node, LED_OFFSET, &mut frame, timeout_ms)
}

fn operate_led_cached(
    state: &mut LedState,
    req: &mut FwReq,
    node: &mut FwNode,
    pos: u16,
    enable: bool,
    timeout_ms: u32,
) -> Result<(), Error> {
    operate_led(req, node, pos, enable, timeout_ms).map(|_| {
        if !enable {
            state.0.retain(|&p| p != pos);
        } else if state.0.iter().find(|&p| *p == pos).is_none() {
            state.0.push(pos);
        }
    })
}

fn clear_leds(
    state: &mut LedState,
    req: &mut FwReq,
    node: &mut FwNode,
    timeout_ms: u32,
) -> Result<(), Error> {
    let cache = state.0.to_vec();
    cache
        .iter()
        .try_for_each(|&pos| operate_led_cached(state, req, node, pos, false, timeout_ms))
}

/// The trait for operation of FireWire LED.
pub trait FireWireLedOperation {
    const POSITIONS: &'static [u16];

    fn operate_firewire_led(
        req: &mut FwReq,
        node: &mut FwNode,
        enable: bool,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        operate_led(req, node, Self::POSITIONS[0], enable, timeout_ms)
    }
}

/// The trait for operation of normal LED in surface.
trait SurfaceNormalLedOperation {
    const NORMAL_LEDS: &'static [(&'static [MachineItem], &'static [u16])];

    fn find_normal_led_pos(item: &MachineItem) -> Option<u16> {
        Self::NORMAL_LEDS
            .iter()
            .find(|(items, _)| {
                if items.len() == 1 {
                    items[0].eq(item)
                } else {
                    items.iter().find(|i| item.eq(i)).is_some()
                }
            })
            .map(|(_, positions)| positions[0])
    }
}
