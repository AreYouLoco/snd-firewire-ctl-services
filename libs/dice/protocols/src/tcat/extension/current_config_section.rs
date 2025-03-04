// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2020 Takashi Sakamoto

//! Current configuration section in protocol extension defined by TCAT for ASICs of DICE.
//!
//! The module includes structure, enumeration, and trait and its implementation for current
//! configuration section in protocol extension defined by TCAT for ASICs of DICE.

use super::{*, cmd_section::*, caps_section::*};
use super::router_entry::*;
use super::stream_format_entry::*;

/// The structure of protocol implementation of current configuration section.
#[derive(Default)]
pub struct CurrentConfigSectionProtocol;

impl CurrentConfigSectionProtocol {
    const LOW_ROUTER_CONFIG_OFFSET: usize = 0x0000;
    const LOW_STREAM_CONFIG_OFFSET: usize = 0x1000;
    const MID_ROUTER_CONFIG_OFFSET: usize = 0x2000;
    const MID_STREAM_CONFIG_OFFSET: usize = 0x3000;
    const HIGH_ROUTER_CONFIG_OFFSET: usize = 0x4000;
    const HIGH_STREAM_CONFIG_OFFSET: usize = 0x5000;

    pub fn read_current_router_entries(
        req: &mut FwReq,
        node: &mut FwNode,
        sections: &ExtensionSections,
        caps: &ExtensionCaps,
        mode: RateMode,
        timeout_ms: u32
    ) -> Result<Vec<RouterEntry>, Error> {
        let offset = match mode {
            RateMode::Low => Self::LOW_ROUTER_CONFIG_OFFSET,
            RateMode::Middle => Self::MID_ROUTER_CONFIG_OFFSET,
            RateMode::High => Self::HIGH_ROUTER_CONFIG_OFFSET,
        };

        let mut data = [0;4];
        let offset = sections.current_config.offset + offset;
        extension_read(
            req,
            node,
            offset,
            &mut data,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::CurrentConfig, &e.to_string()))?;

        let entry_count = std::cmp::min(u32::from_be_bytes(data) as usize,
                                        caps.router.maximum_entry_count as usize);

        read_router_entries(
            req,
            node,
            caps,
            offset + 4,
            entry_count,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::CurrentConfig, &e.to_string()))
    }

    pub fn read_current_stream_format_entries(
        req: &mut FwReq,
        node: &mut FwNode,
        sections: &ExtensionSections,
        caps: &ExtensionCaps,
        mode: RateMode,
        timeout_ms: u32
    ) -> Result<(Vec<FormatEntry>, Vec<FormatEntry>), Error> {
        let offset = match mode {
            RateMode::Low => Self::LOW_STREAM_CONFIG_OFFSET,
            RateMode::Middle => Self::MID_STREAM_CONFIG_OFFSET,
            RateMode::High => Self::HIGH_STREAM_CONFIG_OFFSET,
        };
        let offset = sections.current_config.offset + offset;
        read_stream_format_entries(
            req,
            node,
            caps,
            offset,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::CurrentConfig, &e.to_string()))
    }
}
