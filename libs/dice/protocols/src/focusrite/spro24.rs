// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! Protocol specific to Focusrite Saffire Pro 24 and Pro 24 DSP.
//!
//! The module includes structure, enumeration, and trait and its implementation for protocol
//! defined by Focusrite for Saffire Pro 24 and Pro 24 DSP.

use crate::tcat::extension::*;
use crate::tcat::tcd22xx_spec::*;

const INPUTS: [Input; 4] = [
    Input{id: SrcBlkId::Ins0, offset: 0, count: 4, label: None},
    Input{id: SrcBlkId::Aes, offset: 6, count: 2, label: Some("S/PDIF-coax")},
    // NOTE: share the same optical interface.
    Input{id: SrcBlkId::Adat, offset: 0, count: 8, label: None},
    Input{id: SrcBlkId::Aes, offset: 4, count: 2, label: Some("S/PDIF-opt")},
];

const OUTPUTS: [Output; 2] = [
    Output{id: DstBlkId::Ins0, offset: 0, count: 6, label: None},
    Output{id: DstBlkId::Aes, offset: 6, count: 2, label: Some("S/PDIF-coax")},
];

// NOTE: The first 4 entries in router section are used to display hardware metering.
const FIXED: [SrcBlk; 4] = [
    SrcBlk{id: SrcBlkId::Ins0, ch: 0},
    SrcBlk{id: SrcBlkId::Ins0, ch: 1},
    SrcBlk{id: SrcBlkId::Ins0, ch: 2},
    SrcBlk{id: SrcBlkId::Ins0, ch: 3},
];

/// The structure to represent state of TCD22xx on Saffire Pro 24.
#[derive(Debug)]
pub struct SPro24State {
    tcd22xx: Tcd22xxState,
}

impl Default for SPro24State {
    fn default() -> Self {
        Self {
            tcd22xx: Default::default(),
        }
    }
}

impl Tcd22xxSpec for SPro24State {
    const INPUTS: &'static [Input] = &INPUTS;
    const OUTPUTS: &'static [Output] = &OUTPUTS;
    const FIXED: &'static [SrcBlk] = &FIXED;
}

impl AsMut<Tcd22xxState> for SPro24State {
    fn as_mut(&mut self) -> &mut Tcd22xxState {
        &mut self.tcd22xx
    }
}

impl AsRef<Tcd22xxState> for SPro24State {
    fn as_ref(&self) -> &Tcd22xxState {
        &self.tcd22xx
    }
}

/// The structure to represent state of TCD22xx on Saffire Pro 24 DSP.
#[derive(Debug)]
pub struct SPro24DspState {
    tcd22xx: Tcd22xxState,
}

impl Default for SPro24DspState {
    fn default() -> Self {
        Self {
            tcd22xx: Default::default(),
        }
    }
}

impl Tcd22xxSpec for SPro24DspState {
    const INPUTS: &'static [Input] = &INPUTS;
    const OUTPUTS: &'static [Output] = &OUTPUTS;
    const FIXED: &'static [SrcBlk] = &FIXED;
}

impl AsMut<Tcd22xxState> for SPro24DspState {
    fn as_mut(&mut self) -> &mut Tcd22xxState {
        &mut self.tcd22xx
    }
}

impl AsRef<Tcd22xxState> for SPro24DspState {
    fn as_ref(&self) -> &Tcd22xxState {
        &self.tcd22xx
    }
}
