// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! Protocol implementation for Yamaha Go and Terratec Phase 24 FW series.
//!
//! The module includes structure, enumeration, and trait and its implementation for protocol
//! defined by Yamaha and Terratec for Go and 24 FW series.
//!
//! DM1000E is used for Yamaha GO 44 and GO 46.
//!
//! ## Diagram of internal signal flow
//!
//! ```text
//! analog-input-1/2  --+----------------------------------------+-----> stream-output-1/2
//! digital-input-1/2 --|-+--------------------------------------|-+---> stream-output-3/4
//!                     | |                                      | |
//!                     | |      ++=======++                     | |
//!                     +-|----> ||       ||                     | |
//!                       +----> ||       ||                     | |
//!                              || 6 x 2 ||                     | |
//! stream-input-1/2 ---+------> || mixer ||--> mixer-output-1/2 | |
//! stream-input-3/4 ---|-+----> ||       ||           |         | |
//! stream-input-5/6 ---|-|-+--> ||       ||           |         | |
//!                     | | |    ++=======++           |         | |
//!                     +-|-|--------------------------|---------|-|--->
//!                     | +-|--------------------------|---------|-|--->
//!                     | | +--------------------------|---------|-|---> analog-output-1/2
//!                     | | |                          +---------|-|---> (one source only)
//!                     | | |                          |         +-|--->
//!                     | | |                          |         | +--->
//!                     | | |                          |         | |
//!                     +-|-|--------------------------|---------|-|--->
//!                     | +-|--------------------------|---------|-|--->
//!                     | | +--------------------------|---------|-|---> analog-output-3/4
//!                     | | |                          +---------|-|---> (one source only)
//!                     | | |                          |         +-|--->
//!                     | | |                          |         | +--->
//!                     | | |                          |         | |
//!                     +-|-|--------------------------|---------|-|--->
//!                       +-|--------------------------|---------|-|--->
//!                         +--------------------------|---------|-|---> digital-output-1/2
//!                                                    +---------|-|---> (one source only)
//!                                                              +-|--->
//!                                                                +--->
//! ```
//!
//! The protocol implementation for Yamaha GO 44 was written with firmware version below:
//!
//! ```sh
//! $ cargo run --bin bco-bootloader-info -- /dev/fw1
//! protocol:
//!   version: 1
//! bootloader:
//!   timestamp: 2005-12-20T10:10:04+0000
//!   version: 0.0.0
//! hardware:
//!   GUID: 0x0002e24700a0de00
//!   model ID: 0x00000b
//!   revision: 0.0.1
//! software:
//!   timestamp: 2006-04-20T10:57:53+0000
//!   ID: 0x0010000b
//!   revision: 1.29.3359
//! image:
//!   base address: 0x20080000
//!   maximum size: 0x180000
//! ```
//!
//! The protocol implementation for Yamaha GO 46 was written with firmware version below:
//!
//! ```sh
//! $ cargo run --bin bco-bootloader-info -- /dev/fw1
//! protocol:
//!   version: 1
//! bootloader:
//!   timestamp: 2005-12-20T10:10:14+0000
//!   version: 0.0.0
//! hardware:
//!   GUID: 0x000283e700a0de00
//!   model ID: 0x00000c
//!   revision: 0.0.1
//! software:
//!   timestamp: 2006-01-26T02:31:32+0000
//!   ID: 0x0010000c
//!   revision: 1.34.3359
//! image:
//!   base address: 0x20080000
//!   maximum size: 0x180000
//! ```
//!
//! The protocol implementation for Terratec Phase X24 was written with firmware version below:
//!
//! ```sh
//! $ cargo run --bin bco-bootloader-info -- /dev/fw1
//! protocol:
//!   version: 1
//! bootloader:
//!   timestamp: 2005-07-29T02:05:14+0000
//!   version: 0.0.0
//! hardware:
//!   GUID: 0x0062c9c7000aac07
//!   model ID: 0x000007
//!   revision: 0.0.1
//! software:
//!   timestamp: 2005-07-25T01:56:53+0000
//!   ID: 0x00000007
//!   revision: 1.32.3359
//! image:
//!   base address: 0x20080000
//!   maximum size: 0x180000
//! ```

use crate::*;

/// The protocol implementation of media and sampling clock for Yamaha Go 44/46 and PHASE 24/X24 FW;
pub struct GoPhase24ClkProtocol;

impl MediaClockFrequencyOperation for GoPhase24ClkProtocol {
    const FREQ_LIST: &'static [u32] = &[44100, 48000, 88200, 96000, 192000];
}

const CLK_SRC_FB_ID: u8 = 0x04;

impl SamplingClockSourceOperation for GoPhase24ClkProtocol {
    // NOTE: these destination and source can not be connected actually.
    const DST: SignalAddr = SignalAddr::Subunit(SignalSubunitAddr{
        subunit: MUSIC_SUBUNIT_0,
        plug_id: 0x04,
    });

    const SRC_LIST: &'static [SignalAddr] = &[
        // Internal.
        SignalAddr::Subunit(SignalSubunitAddr{
            subunit: MUSIC_SUBUNIT_0,
            plug_id: 0x03,
        }),
        // S/PDIF input.
        SignalAddr::Unit(SignalUnitAddr::Ext(0x01)),
    ];

    fn read_clk_src(avc: &BebobAvc, timeout_ms: u32) -> Result<usize, Error> {
        let mut op = AudioSelector::new(CLK_SRC_FB_ID, CtlAttr::Current, 0xff);
        avc.status(&AUDIO_SUBUNIT_0_ADDR, &mut op, timeout_ms)
            .map(|_| op.input_plug_id as usize)
    }

    fn write_clk_src(avc: &BebobAvc, val: usize, timeout_ms: u32) -> Result<(), Error> {
        let mut op = AudioSelector::new(CLK_SRC_FB_ID, CtlAttr::Current, val as u8);
        avc.control(&AUDIO_SUBUNIT_0_ADDR, &mut op, timeout_ms)
    }
}

/// The protocol implementation of physical input for coaxial models.
pub struct GoPhase24CoaxPhysInputProtocol;

const INPUT_NOMINAL_LEVEL_FB_ID: u8 = 0x02;

const INPUT_NOMINAL_LEVELS: [i16;3] = [
    0xf400u16 as i16,
    0xfd00u16 as i16,
    0x0000u16 as i16,
];

impl AvcSelectorOperation for GoPhase24CoaxPhysInputProtocol {
    // Unused.
    const FUNC_BLOCK_ID_LIST: &'static [u8] = &[0x00];
    const INPUT_PLUG_ID_LIST: &'static [u8] = &[0x00, 0x01, 0x02];

    fn read_selector(avc: &BebobAvc, idx: usize, timeout_ms: u32) -> Result<usize, Error> {
        if idx > 0 {
            let msg = format!("Invalid argument for index of selector: {}", idx);
            Err(Error::new(FileError::Inval, &msg))?;
        }
        let mut op = AudioFeature::new(
            INPUT_NOMINAL_LEVEL_FB_ID,
            CtlAttr::Current,
            AudioCh::All,
            FeatureCtl::Volume(vec![0xff]),
        );
        avc.status(&AUDIO_SUBUNIT_0_ADDR, &mut op, timeout_ms)?;
        if let FeatureCtl::Volume(data) = op.ctl {
            INPUT_NOMINAL_LEVELS.iter()
                .position(|l| *l == data[0])
                .ok_or_else(|| {
                    let msg = format!("Unexpected value for value of nominal level: 0x{:04x}",
                                      data[0]);
                    Error::new(FileError::Io, &msg)
                })
        } else {
            unreachable!()
        }
    }

    fn write_selector(avc: &BebobAvc, idx: usize, val: usize, timeout_ms: u32) -> Result<(), Error> {
        if idx > 0 {
            let msg = format!("Invalid argument for index of selector: {}", idx);
            Err(Error::new(FileError::Inval, &msg))?;
        }
        let v = INPUT_NOMINAL_LEVELS.iter()
            .nth(val)
            .ok_or_else(|| {
                let msg = format!("Invalid argument for index of nominal level: {}", val);
                Error::new(FileError::Inval, &msg)
            })
            .map(|v| *v)?;
        let mut op = AudioFeature::new(
            INPUT_NOMINAL_LEVEL_FB_ID,
            CtlAttr::Current,
            AudioCh::All,
            FeatureCtl::Volume(vec![v]),
        );
        avc.control(&AUDIO_SUBUNIT_0_ADDR, &mut op, timeout_ms)
    }
}

/// The protocol implementation of physical output for coaxial models.
pub struct GoPhase24CoaxPhysOutputProtocol;

impl AvcSelectorOperation for GoPhase24CoaxPhysOutputProtocol {
    const FUNC_BLOCK_ID_LIST: &'static [u8] = &[
        0x01,   // analog-output-1/2
        0x03,   // digital-output-1/2
    ];
    const INPUT_PLUG_ID_LIST: &'static [u8] = &[
        0x00,   // stream-input-1/2
        0x01,   // stream-input-3/4
        0x02,   // analog-input-1/2
        0x03,   // digital-input-1/2
        0x04,   // mixer-output-1/2
        0x05,   // stream-input-5/6
    ];
}
/// The protocol implementation of physical output for optical models.
pub struct GoPhase24OptPhysOutputProtocol;

impl AvcLevelOperation for GoPhase24OptPhysOutputProtocol {
    const ENTRIES: &'static [(u8, AudioCh)] = &[
        (0x01, AudioCh::Each(0)), // analog-output-1
        (0x01, AudioCh::Each(1)), // analog-output-2
        (0x01, AudioCh::Each(2)), // analog-output-3
        (0x01, AudioCh::Each(3)), // analog-output-4
    ];
}

impl AvcMuteOperation for GoPhase24OptPhysOutputProtocol {}

impl AvcSelectorOperation for GoPhase24OptPhysOutputProtocol {
    const FUNC_BLOCK_ID_LIST: &'static [u8] = &[
        0x01,   // analog-output-1/2
        0x02,   // analog-output-3/4
        0x03,   // digital-output-1/2
    ];
    const INPUT_PLUG_ID_LIST: &'static [u8] = &[
        0x00,   // stream-input-1/2
        0x01,   // stream-input-3/4
        0x02,   // analog-input-1/2
        0x03,   // digital-input-1/2
        0x04,   // mixer-output-1/2
        0x05,   // stream-input-5/6
    ];
}

/// The protocol implementation of mixer source gain for coaxial model.
pub struct GoPhase24CoaxHeadphoneProtocol;

impl AvcSelectorOperation for GoPhase24CoaxHeadphoneProtocol {
    const FUNC_BLOCK_ID_LIST: &'static [u8] = &[0x02];
    const INPUT_PLUG_ID_LIST: &'static [u8] = &[
        0x00,   // stream-input-1/2
        0x01,   // stream-input-3/4
        0x02,   // analog-input-1/2
        0x03,   // digital-input-1/2
        0x04,   // mixer-output-1/2
        0x05,   // stream-input-5/6
    ];
}

/// The protocol implementation of mixer source gain.
pub struct GoPhase24MixerSourceProtocol;

impl AvcLevelOperation for GoPhase24MixerSourceProtocol {
    const ENTRIES: &'static [(u8, AudioCh)] = &[
        (0x06, AudioCh::Each(0)), // analog-input-1/2
        (0x06, AudioCh::Each(1)), // analog-input-1/2
        (0x07, AudioCh::Each(0)), // digital-input-1/2
        (0x07, AudioCh::Each(1)), // digital-input-1/2
        (0x03, AudioCh::Each(0)), // stream-input-1/2
        (0x03, AudioCh::Each(1)), // stream-input-1/2
        (0x04, AudioCh::Each(0)), // stream-input-3/4
        (0x04, AudioCh::Each(1)), // stream-input-3/4
        (0x05, AudioCh::Each(0)), // stream-input-5/6
        (0x05, AudioCh::Each(1)), // stream-input-5/6
    ];
}

impl AvcMuteOperation for GoPhase24MixerSourceProtocol {}

/// The protocol implementation of mixer output volume for coaxial models.
pub struct GoPhase24CoaxMixerOutputProtocol;

impl AvcLevelOperation for GoPhase24CoaxMixerOutputProtocol {
    const ENTRIES: &'static [(u8, AudioCh)] = &[
        (0x01, AudioCh::Each(0)),
        (0x01, AudioCh::Each(1)),
    ];
}

impl AvcMuteOperation for GoPhase24CoaxMixerOutputProtocol {}

/// The protocol implementation of mixer output volume for optical models.
pub struct GoPhase24OptMixerOutputProtocol;

impl AvcLevelOperation for GoPhase24OptMixerOutputProtocol {
    const ENTRIES: &'static [(u8, AudioCh)] = &[
        (0x02, AudioCh::Each(0)),
        (0x02, AudioCh::Each(1)),
    ];
}

impl AvcMuteOperation for GoPhase24OptMixerOutputProtocol {}
