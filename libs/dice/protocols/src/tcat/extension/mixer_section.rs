// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2020 Takashi Sakamoto

//! Mixer section in protocol extension defined by TCAT for ASICs of DICE.
//!
//! The module includes structure, enumeration, and trait and its implementation for mixer section
//! in protocol extension defined by TCAT for ASICs of DICE.
use super::{*, caps_section::*};

/// The structure for protocol implementation of mixer section.
#[derive(Default)]
pub struct MixerSectionProtocol;

impl MixerSectionProtocol {
    const SATURATION_OFFSET: usize = 0x00;
    const COEFF_OFFSET: usize = 0x04;

    pub fn read_saturation(
        req: &mut FwReq,
        node: &mut FwNode,
        sections: &ExtensionSections,
        caps: &ExtensionCaps,
        timeout_ms: u32
    ) -> Result<Vec<bool>, Error> {
        if !caps.mixer.is_exposed {
            Err(Error::new(ProtocolExtensionError::Mixer, "Mixer is not available"))?
        }

        let mut data = [0;4];
        extension_read(
            req,
            node,
            sections.mixer.offset + Self::SATURATION_OFFSET,
            &mut data,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::Mixer, &e.to_string()))
            .map(|_| {
                let val = u32::from_be_bytes(data);
                (0..caps.mixer.output_count)
                    .map(|i| val & (1 << i) > 0)
                    .collect::<Vec<_>>()
            })
    }

    pub fn read_coef(
        req: &mut FwReq,
        node: &mut FwNode,
        sections: &ExtensionSections,
        caps: &ExtensionCaps,
        dst: usize,
        src: usize,
        timeout_ms: u32
    ) -> Result<u32, Error> {
        if !caps.mixer.is_exposed {
            Err(Error::new(ProtocolExtensionError::Mixer, "Mixer is not available"))?
        }

        let offset = 4 * (src + dst * caps.mixer.input_count as usize);
        let mut data = [0;4];
        extension_read(
            req,
            node,
            sections.mixer.offset + Self::COEFF_OFFSET + offset,
            &mut data,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::Mixer, &e.to_string()))
            .map(|_|  u32::from_be_bytes(data))
    }

    pub fn write_coef(
        req: &mut FwReq,
        node: &mut FwNode,
        sections: &ExtensionSections,
        caps: &ExtensionCaps,
        dst: usize,
        src: usize,
        val: u32,
        timeout_ms: u32
    ) -> Result<(), Error> {
        if caps.mixer.is_readonly {
            Err(Error::new(ProtocolExtensionError::Mixer, "Mixer is immutable"))?
        }

        let offset = 4 * (src + dst * caps.mixer.input_count as usize);
        let mut data = [0;4];
        data.copy_from_slice(&val.to_be_bytes());
        extension_write(
            req,
            node,
            sections.mixer.offset + Self::COEFF_OFFSET + offset,
            &mut data,
            timeout_ms
        )
            .map_err(|e| Error::new(ProtocolExtensionError::Mixer, &e.to_string()))
    }
}
