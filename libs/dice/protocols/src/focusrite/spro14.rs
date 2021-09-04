// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! Protocol specific to Focusrite Saffire Pro 14.
//!
//! The module includes structure, enumeration, and trait and its implementation for protocol
//! defined by Focusrite for Saffire Pro 14.

use crate::tcat::extension::*;
use crate::tcat::tcd22xx_spec::*;

/// The structure to represent state of TCD22xx on Saffire Pro 14.
#[derive(Debug)]
pub struct SPro14State{
    tcd22xx: Tcd22xxState,
}

impl Default for SPro14State {
    fn default() -> Self {
        SPro14State {
            tcd22xx: Default::default(),
        }
    }
}

impl Tcd22xxSpec for SPro14State {
    const INPUTS: &'static [Input] = &[
        Input{id: SrcBlkId::Ins0, offset: 0, count: 4, label: None},
        Input{id: SrcBlkId::Aes, offset: 6, count: 2, label: Some("S/PDIF-coax")},
    ];
    const OUTPUTS: &'static [Output] = &[
        Output{id: DstBlkId::Ins0, offset: 0, count: 4, label: None},
        Output{id: DstBlkId::Aes, offset: 6, count: 2, label: Some("S/PDIF-coax")},
    ];
    // NOTE: The first 2 entries in router section are used to display signal detection.
    const FIXED: &'static [SrcBlk] = &[
        SrcBlk{id: SrcBlkId::Ins0, ch: 0},
        SrcBlk{id: SrcBlkId::Ins0, ch: 1},
    ];
}

impl AsMut<Tcd22xxState> for SPro14State {
    fn as_mut(&mut self) -> &mut Tcd22xxState {
        &mut self.tcd22xx
    }
}

impl AsRef<Tcd22xxState> for SPro14State {
    fn as_ref(&self) -> &Tcd22xxState {
        &self.tcd22xx
    }
}
