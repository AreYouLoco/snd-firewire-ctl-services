// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! Protocol defined by RME GmbH for Fireface 400.

use glib::Error;

use hinawa::{FwNode, FwTcode, FwReq, FwReqExtManual};

use super::*;

/// The structure to represent unique protocol for Fireface 400.
#[derive(Default)]
pub struct Ff400Protocol;

const MIXER_OFFSET: usize       = 0x000080080000;
const OUTPUT_OFFSET: usize      = 0x000080080f80;
const METER_OFFSET: usize       = 0x000080100000;
const CFG_OFFSET: usize         = 0x000080100514;
const STATUS_OFFSET: usize      = 0x0000801c0000;
const AMP_OFFSET: usize         = 0x0000801c0180;

const ANALOG_INPUT_COUNT: usize = 8;
const SPDIF_INPUT_COUNT: usize = 2;
const ADAT_INPUT_COUNT: usize = 8;
const STREAM_INPUT_COUNT: usize = 18;

const ANALOG_OUTPUT_COUNT: usize = 8;
const SPDIF_OUTPUT_COUNT: usize = 2;
const ADAT_OUTPUT_COUNT: usize = 8;

// TODO: 12 quadlets are read at once for 6 octuple of timecode detected from line input 3.
#[allow(dead_code)]
const LTC_STATUS_OFFSET: usize  = 0x0000801f0000;

const AMP_MIC_IN_CH_OFFSET: u8 = 0;
const AMP_LINE_IN_CH_OFFSET: u8 = 2;
const AMP_OUT_CH_OFFSET: u8 = 4;

impl RmeFfFormerMeterOperation for Ff400Protocol {
    const METER_OFFSET: usize = METER_OFFSET;

    const ANALOG_INPUT_COUNT: usize = ANALOG_INPUT_COUNT;
    const SPDIF_INPUT_COUNT: usize = SPDIF_INPUT_COUNT;
    const ADAT_INPUT_COUNT: usize = ADAT_INPUT_COUNT;
    const STREAM_INPUT_COUNT: usize = STREAM_INPUT_COUNT;

    const ANALOG_OUTPUT_COUNT: usize = ANALOG_OUTPUT_COUNT;
    const SPDIF_OUTPUT_COUNT: usize = SPDIF_OUTPUT_COUNT;
    const ADAT_OUTPUT_COUNT: usize = ADAT_OUTPUT_COUNT;
}

impl Ff400Protocol {
    pub fn write_amp_cmd(
        req: &mut FwReq,
        node: &mut FwNode,
        ch: u8,
        level: i8,
        timeout_ms: u32
    ) -> Result<(), Error> {
        let cmd = ((ch as u32) << 16) | ((level as u32) & 0xff);
        let mut raw = [0; 4];
        raw.copy_from_slice(&cmd.to_le_bytes());
        req.transaction_sync(
            node,
            FwTcode::WriteQuadletRequest,
            AMP_OFFSET as u64,
            raw.len(),
            &mut raw,
            timeout_ms
        )
    }
}

/// The structure to represent status of input gains of Fireface 400.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400InputGainStatus{
    /// The level of gain for input 1 and 2. The value is between 0 and 65 by step 1 to represent
    /// the range from 0 to 65 dB.
    pub mic: [i8;2],
    /// The level of gain for input 3 and 4. The value is between 0 and 36 by step 1 to represent
    /// the range from 0 to 18 dB.
    pub line: [i8;2],
}

impl Ff400Protocol {
    pub fn write_input_mic_gain(
        req: &mut FwReq,
        node: &mut FwNode,
        ch: usize,
        gain: i8,
        timeout_ms: u32
    ) -> Result<(), Error> {
        Self::write_amp_cmd(req, node, AMP_MIC_IN_CH_OFFSET + ch as u8, gain, timeout_ms)
    }

    pub fn write_input_line_gain(
        req: &mut FwReq,
        node: &mut FwNode,
        ch: usize,
        gain: i8,
        timeout_ms: u32
    ) -> Result<(), Error> {
        Self::write_amp_cmd(req, node, AMP_LINE_IN_CH_OFFSET + ch as u8, gain, timeout_ms)
    }

    pub fn init_input_gains(
        req: &mut FwReq,
        node: &mut FwNode,
        status: &Ff400InputGainStatus,
        timeout_ms: u32
    ) -> Result<(), Error> {
        status.mic.iter()
            .enumerate()
            .try_for_each(|(i, gain)| Self::write_input_mic_gain(req, node, i, *gain, timeout_ms))?;

        status.line.iter()
            .enumerate()
            .try_for_each(|(i, gain)| Self::write_input_line_gain(req, node, i, *gain, timeout_ms))?;

        Ok(())
    }

    pub fn write_input_mic_gains(
        req: &mut FwReq,
        node: &mut FwNode,
        status: &mut Ff400InputGainStatus,
        gains: &[i8],
        timeout_ms: u32
    ) -> Result<(), Error> {
        status.mic.iter_mut()
            .zip(gains.iter())
            .enumerate()
            .filter(|(_, (o, n))| !o.eq(n))
            .try_for_each(|(i, (o, n))| {
                Self::write_input_mic_gain(req, node, i, *n, timeout_ms)
                    .map(|_| *o = *n)
            })
    }

    pub fn write_input_line_gains(
        req: &mut FwReq,
        node: &mut FwNode,
        status: &mut Ff400InputGainStatus,
        gains: &[i8],
        timeout_ms: u32
    ) -> Result<(), Error> {
        status.line.iter_mut()
            .zip(gains.iter())
            .enumerate()
            .filter(|(_, (o, n))| !o.eq(n))
            .try_for_each(|(i, (o, n))| {
                Self::write_input_line_gain(req, node, i, *n, timeout_ms)
                    .map(|_| *o = *n)
            })
    }
}

impl RmeFormerOutputOperation for Ff400Protocol {
    const ANALOG_OUTPUT_COUNT: usize = ANALOG_OUTPUT_COUNT;
    const SPDIF_OUTPUT_COUNT: usize = SPDIF_OUTPUT_COUNT;
    const ADAT_OUTPUT_COUNT: usize = ADAT_OUTPUT_COUNT;

    fn write_output_vol(
        req: &mut FwReq,
        node: &mut FwNode,
        ch: usize,
        vol: i32,
        timeout_ms: u32
    ) -> Result<(), Error> {
        let mut raw = [0; 4];
        raw.copy_from_slice(&vol.to_le_bytes());
        req.transaction_sync(
            node,
            FwTcode::WriteBlockRequest,
            (OUTPUT_OFFSET + ch * 4) as u64,
            raw.len(),
            &mut raw,
            timeout_ms
        )
            .and_then(|_| {
                // The value for level is between 0x3f to 0x00 by step 1 to represent -57 dB
                // (=mute) to +6 dB.
                let level = (0x3f * (vol as i64) / (0x00010000 as i64)) as i8;
                let amp_offset = AMP_OUT_CH_OFFSET + ch as u8;
                Self::write_amp_cmd(req, node, amp_offset, level, timeout_ms)
            })
    }
}


impl RmeFormerMixerOperation for Ff400Protocol {
    const ANALOG_INPUT_COUNT: usize = ANALOG_INPUT_COUNT;
    const SPDIF_INPUT_COUNT: usize = SPDIF_INPUT_COUNT;
    const ADAT_INPUT_COUNT: usize = ADAT_INPUT_COUNT;
    const STREAM_INPUT_COUNT: usize = STREAM_INPUT_COUNT;

    const ANALOG_OUTPUT_COUNT: usize = ANALOG_OUTPUT_COUNT;
    const SPDIF_OUTPUT_COUNT: usize = SPDIF_OUTPUT_COUNT;
    const ADAT_OUTPUT_COUNT: usize = ADAT_OUTPUT_COUNT;

    const MIXER_OFFSET: usize = MIXER_OFFSET as usize;
    const AVAIL_COUNT: usize = 18;
}

/// The enumeration to represent source of sampling clock.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Ff400ClkSrc{
    Internal,
    WordClock,
    Adat,
    Spdif,
    Ltc,
}

impl Default for Ff400ClkSrc {
    fn default() -> Self {
        Self::Internal
    }
}

// NOTE: for first quadlet of status quadlets.
const Q0_SYNC_WORD_CLOCK_MASK: u32          = 0x40000000;
const Q0_LOCK_WORD_CLOCK_MASK: u32          = 0x20000000;
const Q0_EXT_CLK_RATE_MASK: u32             = 0x1e000000;
const  Q0_EXT_CLK_RATE_192000_FLAG: u32     = 0x12000000;
const  Q0_EXT_CLK_RATE_176400_FLAG: u32     = 0x10000000;
const  Q0_EXT_CLK_RATE_128000_FLAG: u32     = 0x0c000000;
const  Q0_EXT_CLK_RATE_96000_FLAG: u32      = 0x0e000000;
const  Q0_EXT_CLK_RATE_88200_FLAG: u32      = 0x0a000000;
const  Q0_EXT_CLK_RATE_64000_FLAG: u32      = 0x08000000;
const  Q0_EXT_CLK_RATE_48000_FLAG: u32      = 0x06000000;
const  Q0_EXT_CLK_RATE_44100_FLAG: u32      = 0x04000000;
const  Q0_EXT_CLK_RATE_32000_FLAG: u32      = 0x02000000;
const Q0_ACTIVE_CLK_SRC_MASK: u32           = 0x01c00000;
const  Q0_ACTIVE_CLK_SRC_INTERNAL_FLAG: u32 = 0x01c00000;
const  Q0_ACTIVE_CLK_SRC_LTC_FLAG: u32      = 0x01400000;
const  Q0_ACTIVE_CLK_SRC_WORD_CLK_FLAG: u32 = 0x01000000;
const  Q0_ACTIVE_CLK_SRC_SPDIF_FLAG: u32    = 0x00c00000;
const  Q0_ACTIVE_CLK_SRC_ADAT_FLAG: u32     = 0x00000000;
const Q0_SYNC_SPDIF_MASK: u32               = 0x00100000;
const Q0_LOCK_SPDIF_MASK: u32               = 0x00040000;
const Q0_SPDIF_RATE_MASK: u32               = 0x0003c000;
const  Q0_SPDIF_RATE_192000_FLAG: u32       = 0x00024000;
const  Q0_SPDIF_RATE_176400_FLAG: u32       = 0x00020000;
const  Q0_SPDIF_RATE_128000_FLAG: u32       = 0x0001c000;
const  Q0_SPDIF_RATE_96000_FLAG: u32        = 0x00018000;
const  Q0_SPDIF_RATE_88200_FLAG: u32        = 0x00014000;
const  Q0_SPDIF_RATE_64000_FLAG: u32        = 0x00010000;
const  Q0_SPDIF_RATE_48000_FLAG: u32        = 0x0000c000;
const  Q0_SPDIF_RATE_44100_FLAG: u32        = 0x00008000;
const  Q0_SPDIF_RATE_32000_FLAG: u32        = 0x00004000;
const Q0_LOCK_ADAT_MASK: u32                = 0x00001000;
const Q0_SYNC_ADAT_MASK: u32                = 0x00000400;

// NOTE: for second quadlet of status quadlets.
const Q1_WORD_OUT_SINGLE_MASK: u32          = 0x00002000;
const Q1_CONF_CLK_SRC_MASK: u32             = 0x00001c01;
const  Q1_CONF_CLK_SRC_LTC_FLAG: u32        = 0x00001400;
const  Q1_CONF_CLK_SRC_WORD_CLK_FLAG: u32   = 0x00001000;
const  Q1_CONF_CLK_SRC_SPDIF_FLAG: u32      = 0x00000c00;
const  Q1_CONF_CLK_SRC_INTERNAL_FLAG: u32   = 0x00000001;
const  Q1_CONF_CLK_SRC_ADAT_FLAG: u32       = 0x00000000;
const Q1_SPDIF_IN_IFACE_MASK: u32           = 0x00000200;
const Q1_OPT_OUT_SIGNAL_MASK: u32           = 0x00000100;
const Q1_SPDIF_OUT_EMPHASIS_MASK: u32       = 0x00000040;
const Q1_SPDIF_OUT_FMT_MASK: u32            = 0x00000020;
const Q1_CONF_CLK_RATE_MASK: u32            = 0x0000001e;
const  Q1_CONF_CLK_RATE_192000_FLAG: u32    = 0x00000016;
const  Q1_CONF_CLK_RATE_176400_FLAG: u32    = 0x00000010;
const  Q1_CONF_CLK_RATE_128000_FLAG: u32    = 0x00000012;
const  Q1_CONF_CLK_RATE_96000_FLAG: u32     = 0x0000000e;
const  Q1_CONF_CLK_RATE_88200_FLAG: u32     = 0x00000008;
const  Q1_CONF_CLK_RATE_64000_FLAG: u32     = 0x0000000a;
const  Q1_CONF_CLK_RATE_48000_FLAG: u32     = 0x00000006;
const  Q1_CONF_CLK_RATE_44100_FLAG: u32     = 0x00000000;
const  Q1_CONF_CLK_RATE_32000_FLAG: u32     = 0x00000002;

/// The structure to represent status of clock locking.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400ClkLockStatus {
    pub adat: bool,
    pub spdif: bool,
    pub word_clock: bool,
}

impl Ff400ClkLockStatus {
    fn parse(&mut self, quads: &[u32]) {
        self.adat = quads[0] & Q0_LOCK_ADAT_MASK > 0;
        self.spdif = quads[0] & Q0_LOCK_SPDIF_MASK > 0;
        self.word_clock = quads[0] & Q0_LOCK_WORD_CLOCK_MASK > 0;

    }
}

/// The structure to represent status of clock synchronization.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400ClkSyncStatus {
    pub adat: bool,
    pub spdif: bool,
    pub word_clock: bool,
}

impl Ff400ClkSyncStatus {
    fn parse(&mut self, quads: &[u32]) {
        self.adat = quads[0] & Q0_SYNC_ADAT_MASK > 0;
        self.spdif = quads[0] & Q0_SYNC_SPDIF_MASK > 0;
        self.word_clock = quads[0] & Q0_SYNC_WORD_CLOCK_MASK > 0;
    }
}

/// The structure to represent status of clock synchronization.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400Status {
    /// For S/PDIF input.
    pub spdif_in: SpdifInput,
    /// For S/PDIF output.
    pub spdif_out: FormerSpdifOutput,
    /// The type of signal to optical output interface.
    pub opt_out_signal: OpticalOutputSignal,
    /// Whether to fix speed to single even if at double/quadruple rate.
    pub word_out_single: bool,
    /// For status of synchronization to external clocks.
    pub sync: Ff400ClkSyncStatus,
    /// For status of locking to external clocks.
    pub lock: Ff400ClkLockStatus,

    pub spdif_rate: Option<ClkNominalRate>,
    pub active_clk_src: Ff400ClkSrc,
    pub external_clk_rate: Option<ClkNominalRate>,
    pub configured_clk_src: Ff400ClkSrc,
    pub configured_clk_rate: ClkNominalRate,
}

impl Ff400Status {
    const QUADLET_COUNT: usize = 2;

    fn parse(&mut self, quads: &[u32]) {
        assert_eq!(quads.len(), Self::QUADLET_COUNT);

        self.lock.parse(&quads);
        self.sync.parse(&quads);

        self.spdif_rate = match quads[0] & Q0_SPDIF_RATE_MASK {
            Q0_SPDIF_RATE_32000_FLAG => Some(ClkNominalRate::R32000),
            Q0_SPDIF_RATE_44100_FLAG => Some(ClkNominalRate::R44100),
            Q0_SPDIF_RATE_48000_FLAG => Some(ClkNominalRate::R48000),
            Q0_SPDIF_RATE_64000_FLAG => Some(ClkNominalRate::R64000),
            Q0_SPDIF_RATE_88200_FLAG => Some(ClkNominalRate::R88200),
            Q0_SPDIF_RATE_96000_FLAG => Some(ClkNominalRate::R96000),
            Q0_SPDIF_RATE_128000_FLAG => Some(ClkNominalRate::R128000),
            Q0_SPDIF_RATE_176400_FLAG => Some(ClkNominalRate::R176400),
            Q0_SPDIF_RATE_192000_FLAG => Some(ClkNominalRate::R192000),
            _ => None,
        };

        self.active_clk_src = match quads[0] & Q0_ACTIVE_CLK_SRC_MASK {
            Q0_ACTIVE_CLK_SRC_ADAT_FLAG => Ff400ClkSrc::Adat,
            Q0_ACTIVE_CLK_SRC_SPDIF_FLAG => Ff400ClkSrc::Spdif,
            Q0_ACTIVE_CLK_SRC_WORD_CLK_FLAG => Ff400ClkSrc::WordClock,
            Q0_ACTIVE_CLK_SRC_LTC_FLAG => Ff400ClkSrc::Ltc,
            Q0_ACTIVE_CLK_SRC_INTERNAL_FLAG => Ff400ClkSrc::Internal,
            _ => unreachable!(),
        };

        self.external_clk_rate = match quads[0] & Q0_EXT_CLK_RATE_MASK {
            Q0_EXT_CLK_RATE_32000_FLAG => Some(ClkNominalRate::R32000),
            Q0_EXT_CLK_RATE_44100_FLAG => Some(ClkNominalRate::R44100),
            Q0_EXT_CLK_RATE_48000_FLAG => Some(ClkNominalRate::R48000),
            Q0_EXT_CLK_RATE_64000_FLAG => Some(ClkNominalRate::R64000),
            Q0_EXT_CLK_RATE_88200_FLAG => Some(ClkNominalRate::R88200),
            Q0_EXT_CLK_RATE_96000_FLAG => Some(ClkNominalRate::R96000),
            Q0_EXT_CLK_RATE_128000_FLAG => Some(ClkNominalRate::R128000),
            Q0_EXT_CLK_RATE_176400_FLAG => Some(ClkNominalRate::R176400),
            Q0_EXT_CLK_RATE_192000_FLAG => Some(ClkNominalRate::R192000),
            _ => None,
        };

        self.spdif_in.iface = if quads[1] & Q1_SPDIF_IN_IFACE_MASK > 0 {
            SpdifIface::Optical
        } else {
            SpdifIface::Coaxial
        };

        self.spdif_out.format = if quads[1] & Q1_SPDIF_OUT_FMT_MASK > 0 {
            SpdifFormat::Professional
        } else {
            SpdifFormat::Consumer
        };

        self.spdif_out.emphasis = quads[1] & Q1_SPDIF_OUT_EMPHASIS_MASK > 0;

        self.opt_out_signal = if quads[1] & Q1_OPT_OUT_SIGNAL_MASK > 0 {
            OpticalOutputSignal::Spdif
        } else {
            OpticalOutputSignal::Adat
        };

        self.word_out_single = quads[1] & Q1_WORD_OUT_SINGLE_MASK > 0;

        self.configured_clk_src = match quads[1] & Q1_CONF_CLK_SRC_MASK {
            Q1_CONF_CLK_SRC_INTERNAL_FLAG => Ff400ClkSrc::Internal,
            Q1_CONF_CLK_SRC_SPDIF_FLAG => Ff400ClkSrc::Spdif,
            Q1_CONF_CLK_SRC_WORD_CLK_FLAG => Ff400ClkSrc::WordClock,
            Q1_CONF_CLK_SRC_LTC_FLAG => Ff400ClkSrc::Ltc,
            Q1_CONF_CLK_SRC_ADAT_FLAG | _ => Ff400ClkSrc::Adat,
        };

        self.configured_clk_rate = match quads[1] & Q1_CONF_CLK_RATE_MASK {
            Q1_CONF_CLK_RATE_32000_FLAG => ClkNominalRate::R32000,
            Q1_CONF_CLK_RATE_48000_FLAG => ClkNominalRate::R48000,
            Q1_CONF_CLK_RATE_64000_FLAG => ClkNominalRate::R64000,
            Q1_CONF_CLK_RATE_88200_FLAG => ClkNominalRate::R88200,
            Q1_CONF_CLK_RATE_96000_FLAG => ClkNominalRate::R96000,
            Q1_CONF_CLK_RATE_128000_FLAG => ClkNominalRate::R128000,
            Q1_CONF_CLK_RATE_176400_FLAG => ClkNominalRate::R176400,
            Q1_CONF_CLK_RATE_192000_FLAG => ClkNominalRate::R192000,
            Q1_CONF_CLK_RATE_44100_FLAG | _ => ClkNominalRate::R44100,
        };

    }
}

impl Ff400Protocol {
    pub fn read_status(
        req: &mut FwReq,
        node: &mut FwNode,
        status: &mut Ff400Status,
        timeout_ms: u32
    ) -> Result<(), Error> {
        let mut raw = [0; 8];
        req.transaction_sync(
            node,
            FwTcode::ReadBlockRequest,
            STATUS_OFFSET as u64,
            raw.len(),
            &mut raw,
            timeout_ms
        )
            .map(|_| {
                let mut quadlet = [0; 4];
                let mut quads = [0u32; 2];
                quads.iter_mut()
                    .enumerate()
                    .for_each(|(i, quad)| {
                        let pos = i * 4;
                        quadlet.copy_from_slice(&raw[pos..(pos + 4)]);
                        *quad = u32::from_le_bytes(quadlet);
                    });
                status.parse(&quads)
            })
    }
}

// NOTE: for first quadlet of configuration quadlets.
const Q0_HP_OUT_LEVEL_MASK: u32                 = 0x00060000;
const  Q0_HP_OUT_LEVEL_HIGH_FLAG: u32           = 0x00040000;
const  Q0_HP_OUT_LEVEL_CON_FLAG: u32            = 0x00020000;
const  Q0_HP_OUT_LEVEL_PRO_FLAG: u32            = 0x00000000;
const Q0_LINE_OUT_LEVEL_MASK: u32               = 0x00001c00;
const  Q0_LINE_OUT_LEVEL_CON_FLAG: u32          = 0x00001000;
const  Q0_LINE_OUT_LEVEL_PRO_FLAG: u32          = 0x00000800;
const  Q0_LINE_OUT_LEVEL_HIGH_FLAG: u32         = 0x00000400;
const Q0_INPUT_2_INST_MASK: u32                 = 0x00000200;
const Q0_INPUT_2_PAD_MASK: u32                  = 0x00000100;
const Q0_INPUT_1_POWERING_MASK: u32             = 0x00000080;
const Q0_LINE_IN_LEVEL_MASK: u32                = 0x00000038;
const  Q0_LINE_IN_LEVEL_CON_FLAG: u32           = 0x00000020;
const  Q0_LINE_IN_LEVEL_LOW_FLAG: u32           = 0x00000010;
const  Q0_LINE_IN_LEVEL_PRO_FLAG: u32           = 0x00000008;
const Q0_INPUT_3_INST_MASK: u32                 = 0x00000004;
const Q0_INPUT_3_PAD_MASK: u32                  = 0x00000002;
const Q0_INPUT_0_POWERING_MASK: u32             = 0x00000001;

// NOTE: for second quadlet of configuration quadlets.
const Q1_LINE_OUT_LEVEL_MASK: u32               = 0x00000018;
const  Q1_LINE_OUT_LEVEL_PRO_FLAG: u32          = 0x00000018;
const  Q1_LINE_OUT_LEVEL_HIGH_FLAG: u32         = 0x00000010;
const  Q1_LINE_OUT_LEVEL_CON_FLAG: u32          = 0x00000008;
const Q1_LINE_IN_LEVEL_MASK: u32                = 0x00000003;
const  Q1_LINE_IN_LEVEL_CON_FLAG: u32           = 0x00000003;
const  Q1_LINE_IN_LEVEL_PRO_FLAG: u32           = 0x00000002;
const  Q1_LINE_IN_LEVEL_LOW_FLAG: u32           = 0x00000000;

// NOTE: for third quadlet of configuration quadlets.
const Q2_CONTINUE_AT_ERRORS: u32                = 0x80000000;
const Q2_SPDIF_IN_USE_PREEMBLE: u32             = 0x40000000;
const Q2_MIDI_TX_LOW_OFFSET_MASK: u32           = 0x3c000000;
const   Q2_MIDI_TX_LOW_OFFSET_0180_FLAG: u32    = 0x20000000;
const   Q2_MIDI_TX_LOW_OFFSET_0100_FLAG: u32    = 0x10000000;
const   Q2_MIDI_TX_LOW_OFFSET_0080_FLAG: u32    = 0x08000000;
const   Q2_MIDI_TX_LOW_OFFSET_0000_FLAG: u32    = 0x04000000;
const Q2_MIDI_TX_SUPPRESS_MASK: u32             = 0x03000000;
const Q2_WORD_OUT_SINGLE_SPEED_MASK: u32        = 0x00002000;
const Q2_CLK_SRC_MASK: u32                      = 0x00001c01;
const  Q2_CLK_SRC_LTC_FLAG: u32                 = 0x00001400;
const  Q2_CLK_SRC_WORD_CLK_FLAG: u32            = 0x00001000;
const  Q2_CLK_SRC_SPDIF_FLAG: u32               = 0x00000c00;
const  Q2_CLK_SRC_INTERNAL_FLAG: u32            = 0x00000001;
const  Q2_CLK_SRC_ADAT_FLAG: u32                = 0x00000000;
const Q2_SPDIF_IN_IFACE_OPT_MASK: u32           = 0x00000200;
const Q2_OPT_OUT_SIGNAL_MASK: u32               = 0x00000100;
const Q2_SPDIF_OUT_NON_AUDIO_MASK: u32          = 0x00000080;
const Q2_SPDIF_OUT_EMPHASIS_MASK: u32           = 0x00000040;
const Q2_SPDIF_OUT_FMT_PRO_MASK: u32            = 0x00000020;
const Q2_CLK_AVAIL_RATE_QUADRUPLE_MASK: u32     = 0x00000010;
const Q2_CLK_AVAIL_RATE_DOUBLE_MASK: u32        = 0x00000008;
const Q2_CLK_AVAIL_RATE_BASE_48000_MASK: u32    = 0x00000004;
const Q2_CLK_AVAIL_RATE_BASE_44100_MASK: u32    = 0x00000002;

/// The structure to represent configurations of sampling clock.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400ClkConfig{
    pub primary_src: Ff400ClkSrc,
    avail_rate_44100: bool,
    avail_rate_48000: bool,
    avail_rate_double: bool,
    avail_rate_quadruple: bool,
}

impl Default for Ff400ClkConfig {
    fn default() -> Self {
        Self{
            primary_src: Ff400ClkSrc::default(),
            avail_rate_44100: true,
            avail_rate_48000: true,
            avail_rate_double: true,
            avail_rate_quadruple: true,
        }
    }
}

impl Ff400ClkConfig {
    fn build(&self, quads: &mut [u32]) {
        let mask = match self.primary_src {
            Ff400ClkSrc::Internal => Q2_CLK_SRC_INTERNAL_FLAG,
            Ff400ClkSrc::Ltc => Q2_CLK_SRC_LTC_FLAG,
            Ff400ClkSrc::WordClock => Q2_CLK_SRC_WORD_CLK_FLAG,
            Ff400ClkSrc::Adat => Q2_CLK_SRC_ADAT_FLAG,
            Ff400ClkSrc::Spdif => Q2_CLK_SRC_SPDIF_FLAG,
        };
        quads[2] |= mask;

        if self.avail_rate_44100 {
            quads[2] |= Q2_CLK_AVAIL_RATE_BASE_44100_MASK;
        }
        if self.avail_rate_48000 {
            quads[2] |= Q2_CLK_AVAIL_RATE_BASE_48000_MASK;
        }
        if self.avail_rate_double {
            quads[2] |= Q2_CLK_AVAIL_RATE_DOUBLE_MASK;
        }
        if self.avail_rate_quadruple {
            quads[2] |= Q2_CLK_AVAIL_RATE_QUADRUPLE_MASK;
        }
    }

    fn parse(&mut self, quads: &[u32]) {
        self.primary_src = match quads[2] & Q2_CLK_SRC_MASK {
            Q2_CLK_SRC_INTERNAL_FLAG => Ff400ClkSrc::Internal,
            Q2_CLK_SRC_LTC_FLAG => Ff400ClkSrc::Ltc,
            Q2_CLK_SRC_WORD_CLK_FLAG => Ff400ClkSrc::WordClock,
            Q2_CLK_SRC_SPDIF_FLAG => Ff400ClkSrc::Spdif,
            Q2_CLK_SRC_ADAT_FLAG | _ => Ff400ClkSrc::Adat,
        };

        self.avail_rate_44100 = quads[2] & Q2_CLK_AVAIL_RATE_BASE_44100_MASK > 0;
        self.avail_rate_48000 = quads[2] & Q2_CLK_AVAIL_RATE_BASE_48000_MASK > 0;
        self.avail_rate_double = quads[2] & Q2_CLK_AVAIL_RATE_DOUBLE_MASK > 0;
        self.avail_rate_quadruple = quads[2] & Q2_CLK_AVAIL_RATE_QUADRUPLE_MASK > 0;
    }
}

/// The structure to represent configuration for analog inputs.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400AnalogInConfig{
    /// The nominal level of audio signal for input 5, 6, 7 and 8.
    pub line_level: FormerLineInNominalLevel,
    /// Whether to deliver +48 V powering for input 1 and 2.
    pub phantom_powering: [bool;2],
    /// Whether to use input 3 and 4 for instrument.
    pub insts: [bool;2],
    /// Whether to attenuate signal level from input 3 and 4.
    pub pad: [bool;2],
}

impl Ff400AnalogInConfig {
    fn build(&self, quads: &mut [u32]) {
        match self.line_level {
            FormerLineInNominalLevel::Low => {
                quads[0] |= Q0_LINE_IN_LEVEL_LOW_FLAG;
                quads[1] |= Q1_LINE_IN_LEVEL_LOW_FLAG;
            }
            FormerLineInNominalLevel::Consumer => {
                quads[0] |= Q0_LINE_IN_LEVEL_CON_FLAG;
                quads[1] |= Q1_LINE_IN_LEVEL_CON_FLAG;
            }
            FormerLineInNominalLevel::Professional => {
                quads[0] |= Q0_LINE_IN_LEVEL_PRO_FLAG;
                quads[1] |= Q1_LINE_IN_LEVEL_PRO_FLAG;
            }
        }

        if self.phantom_powering[0] {
            quads[0] |= Q0_INPUT_0_POWERING_MASK;
        }
        if self.phantom_powering[1] {
            quads[0] |= Q0_INPUT_1_POWERING_MASK;
        }

        if self.insts[0] {
            quads[0] |= Q0_INPUT_2_INST_MASK;
        }
        if self.insts[1] {
            quads[0] |= Q0_INPUT_3_INST_MASK;
        }

        if self.pad[0] {
            quads[0] |= Q0_INPUT_2_PAD_MASK;
        }
        if self.pad[1] {
            quads[0] |= Q0_INPUT_3_PAD_MASK;
        }
    }

    fn parse(&mut self, quads: &[u32]) {
        let pair = (quads[0] & Q0_LINE_IN_LEVEL_MASK, quads[1] & Q1_LINE_IN_LEVEL_MASK);
        self.line_level = match pair {
            (Q0_LINE_IN_LEVEL_LOW_FLAG, Q1_LINE_IN_LEVEL_LOW_FLAG) => FormerLineInNominalLevel::Low,
            (Q0_LINE_IN_LEVEL_CON_FLAG, Q1_LINE_IN_LEVEL_CON_FLAG) => FormerLineInNominalLevel::Consumer,
            (Q0_LINE_IN_LEVEL_PRO_FLAG, Q1_LINE_IN_LEVEL_PRO_FLAG) => FormerLineInNominalLevel::Professional,
            _ => unreachable!(),
        };

        self.phantom_powering[0] = quads[0] & Q0_INPUT_0_POWERING_MASK > 0;
        self.phantom_powering[1] = quads[0] & Q0_INPUT_1_POWERING_MASK > 0;

        self.insts[0] = quads[0] & Q0_INPUT_2_INST_MASK > 0;
        self.insts[1] = quads[0] & Q0_INPUT_3_INST_MASK > 0;

        self.pad[0] = quads[0] & Q0_INPUT_2_PAD_MASK > 0;
        self.pad[1] = quads[0] & Q0_INPUT_3_PAD_MASK > 0;
    }
}

/// The enumeration to represent low offset of destination address for MIDI messages.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Ff400MidiTxLowOffset {
    /// Between 0x0000 to 0x007c.
    A0000,
    /// Between 0x0080 to 0x00fc.
    A0080,
    /// Between 0x0100 to 0x017c.
    A0100,
    /// Between 0x0180 to 0x01fc.
    A0180,
}

impl Default for Ff400MidiTxLowOffset {
    fn default() -> Self {
        Self::A0000
    }
}

impl Ff400MidiTxLowOffset {
    fn build(&self, quads: &mut [u32]) {
        quads[2] |= match self {
            Self::A0000 => Q2_MIDI_TX_LOW_OFFSET_0000_FLAG,
            Self::A0080 => Q2_MIDI_TX_LOW_OFFSET_0080_FLAG,
            Self::A0100 => Q2_MIDI_TX_LOW_OFFSET_0100_FLAG,
            Self::A0180 => Q2_MIDI_TX_LOW_OFFSET_0180_FLAG,
        };
    }

    fn parse(&mut self, quads: &[u32]) {
        *self = match quads[2] & Q2_MIDI_TX_LOW_OFFSET_MASK {
            Q2_MIDI_TX_LOW_OFFSET_0180_FLAG => Self::A0180,
            Q2_MIDI_TX_LOW_OFFSET_0100_FLAG => Self::A0100,
            Q2_MIDI_TX_LOW_OFFSET_0080_FLAG => Self::A0080,
            Q2_MIDI_TX_LOW_OFFSET_0000_FLAG => Self::A0000,
            _ => unreachable!(),
        }
    }
}

/// The structure to represent configurations.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Ff400Config{
    /// The low offset of destination address for MIDI messages.
    midi_tx_low_offset: Ff400MidiTxLowOffset,
    /// Whether to enable transaction for MIDI messages.
    midi_tx_enable: bool,
    /// For sampling clock.
    pub clk: Ff400ClkConfig,
    /// For analog inputs.
    pub analog_in: Ff400AnalogInConfig,
    /// The nominal level of audio signal for output 1, 2, 3, 4, 5, and 6.
    pub line_out_level: LineOutNominalLevel,
    /// The nominal level of audio signal for headphone output.
    pub hp_out_level: LineOutNominalLevel,
    /// For S/PDIF input.
    pub spdif_in: SpdifInput,
    /// For S/PDIF output.
    pub spdif_out: FormerSpdifOutput,
    /// The type of signal to optical output interface.
    pub opt_out_signal: OpticalOutputSignal,
    /// Whether to fix speed to single even if at double/quadruple rate.
    pub word_out_single: bool,
    /// Whether to continue audio processing against any synchronization corruption.
    continue_at_errors: bool,
}

impl Default for Ff400Config {
    fn default() -> Self {
        Self{
            midi_tx_low_offset: Default::default(),
            midi_tx_enable: true,
            clk: Default::default(),
            analog_in: Default::default(),
            line_out_level: Default::default(),
            hp_out_level: Default::default(),
            spdif_in: Default::default(),
            spdif_out: Default::default(),
            opt_out_signal: Default::default(),
            word_out_single: Default::default(),
            continue_at_errors: true,
        }
    }
}

impl Ff400Config {
    const QUADLET_COUNT: usize = 3;

    fn build(&self, quads: &mut [u32]) {
        assert_eq!(quads.len(), Self::QUADLET_COUNT);

        self.midi_tx_low_offset.build(quads);

        if !self.midi_tx_enable {
            quads[2] |= Q2_MIDI_TX_SUPPRESS_MASK;
        }

        self.clk.build(quads);
        self.analog_in.build(quads);

        match self.line_out_level {
            LineOutNominalLevel::High => {
                quads[0] |= Q0_LINE_OUT_LEVEL_HIGH_FLAG;
                quads[1] |= Q1_LINE_OUT_LEVEL_HIGH_FLAG;
            }
            LineOutNominalLevel::Consumer => {
                quads[0] |= Q0_LINE_OUT_LEVEL_CON_FLAG;
                quads[1] |= Q1_LINE_OUT_LEVEL_CON_FLAG;
            }
            LineOutNominalLevel::Professional => {
                quads[0] |= Q0_LINE_OUT_LEVEL_PRO_FLAG;
                quads[1] |= Q1_LINE_OUT_LEVEL_PRO_FLAG;
            }
        }

        match self.hp_out_level {
            LineOutNominalLevel::High => {
                quads[0] |= Q0_HP_OUT_LEVEL_HIGH_FLAG;
            }
            LineOutNominalLevel::Consumer => {
                quads[0] |= Q0_HP_OUT_LEVEL_CON_FLAG;
            }
            LineOutNominalLevel::Professional => {
                quads[0] |= Q0_HP_OUT_LEVEL_PRO_FLAG;
            }
        }

        if self.spdif_in.iface == SpdifIface::Optical {
            quads[2] |= Q2_SPDIF_IN_IFACE_OPT_MASK;
        }
        if self.spdif_in.use_preemble {
            quads[2] |= Q2_SPDIF_IN_USE_PREEMBLE;
        }

        if self.opt_out_signal == OpticalOutputSignal::Spdif {
            quads[2] |= Q2_OPT_OUT_SIGNAL_MASK;
        }
        if self.spdif_out.format == SpdifFormat::Professional {
            quads[2] |= Q2_SPDIF_OUT_FMT_PRO_MASK;
        }
        if self.spdif_out.emphasis {
            quads[2] |= Q2_SPDIF_OUT_EMPHASIS_MASK;
        }
        if self.spdif_out.non_audio {
            quads[2] |= Q2_SPDIF_OUT_NON_AUDIO_MASK;
        }

        if self.word_out_single {
            quads[2] |= Q2_WORD_OUT_SINGLE_SPEED_MASK;
        }

        if self.continue_at_errors {
            quads[2] |= Q2_CONTINUE_AT_ERRORS;
        }
    }

    #[allow(dead_code)]
    fn parse(&mut self, quads: &[u32]) {
        assert_eq!(quads.len(), Self::QUADLET_COUNT);

        self.midi_tx_low_offset.parse(quads);
        self.midi_tx_enable = quads[2] & Q2_MIDI_TX_SUPPRESS_MASK == 0;

        self.clk.parse(quads);
        self.analog_in.parse(quads);

        let pair = (quads[0] & Q0_LINE_OUT_LEVEL_MASK, quads[1] & Q1_LINE_OUT_LEVEL_MASK);
        self.line_out_level = match pair {
            (Q0_LINE_OUT_LEVEL_HIGH_FLAG, Q1_LINE_OUT_LEVEL_HIGH_FLAG) => LineOutNominalLevel::High,
            (Q0_LINE_OUT_LEVEL_CON_FLAG, Q1_LINE_OUT_LEVEL_CON_FLAG) => LineOutNominalLevel::Consumer,
            (Q0_LINE_OUT_LEVEL_PRO_FLAG, Q1_LINE_OUT_LEVEL_PRO_FLAG) => LineOutNominalLevel::Professional,
            _ => unreachable!(),
        };

        self.hp_out_level = match quads[0] & Q0_HP_OUT_LEVEL_MASK {
            Q0_HP_OUT_LEVEL_HIGH_FLAG => LineOutNominalLevel::High,
            Q0_HP_OUT_LEVEL_CON_FLAG => LineOutNominalLevel::Consumer,
            Q0_HP_OUT_LEVEL_PRO_FLAG => LineOutNominalLevel::Professional,
            _ => unreachable!(),
        };

        self.spdif_in.iface = if quads[2] & Q2_SPDIF_IN_IFACE_OPT_MASK > 0 {
            SpdifIface::Optical
        } else {
            SpdifIface::Coaxial
        };
        self.spdif_in.use_preemble = quads[2] & Q2_SPDIF_IN_USE_PREEMBLE > 0;

        self.spdif_out.format = if quads[2] & Q2_SPDIF_OUT_FMT_PRO_MASK > 0 {
            SpdifFormat::Professional
        } else {
            SpdifFormat::Consumer
        };
        self.spdif_out.emphasis = quads[2] & Q2_SPDIF_OUT_EMPHASIS_MASK > 0;
        self.spdif_out.non_audio = quads[2] & Q2_SPDIF_OUT_NON_AUDIO_MASK > 0;

        self.opt_out_signal = if quads[2] & Q2_OPT_OUT_SIGNAL_MASK > 0 {
            OpticalOutputSignal::Spdif
        } else {
            OpticalOutputSignal::Adat
        };

        self.word_out_single = quads[2] & Q2_WORD_OUT_SINGLE_SPEED_MASK > 0;
        self.continue_at_errors = quads[2] & Q2_CONTINUE_AT_ERRORS > 0;
    }

    /// Although the configuration registers are write-only, some of them are available in status
    /// registers.
    pub fn init(&mut self, status: &Ff400Status) {
        self.clk.primary_src = status.configured_clk_src;
        self.spdif_in = status.spdif_in;
        self.spdif_out = status.spdif_out;
        self.opt_out_signal = status.opt_out_signal;
        self.word_out_single = status.word_out_single;
    }
}

impl Ff400Protocol {
    pub fn write_cfg(
        req: &mut FwReq,
        node: &mut FwNode,
        cfg: &Ff400Config,
        timeout_ms: u32
    ) -> Result<(), Error> {
        let mut quads = [0u32; 3];
        cfg.build(&mut quads);

        let mut raw = [0; 12];
        quads.iter()
            .enumerate()
            .for_each(|(i, quad)| {
                let pos = i * 4;
                raw[pos..(pos + 4)].copy_from_slice(&quad.to_le_bytes())
            });
        req.transaction_sync(
            node,
            FwTcode::WriteBlockRequest,
            CFG_OFFSET as u64,
            raw.len(),
            &mut raw,
            timeout_ms
        )
    }
}
