// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2020 Takashi Sakamoto
use glib::Error;

use hinawa::FwReq;
use hinawa::SndMotu;

use alsactl::{ElemId, ElemValue};

use core::card_cntr::{CardCntr, CtlModel, NotifyModel};

use motu_protocols::{register_dsp::*, version_2::*, *};

use super::{common_ctls::*, register_dsp_ctls::*, v2_ctls::*};

const TIMEOUT_MS: u32 = 100;

#[derive(Default)]
pub struct F828mk2{
    req: FwReq,
    clk_ctls: ClkCtl,
    opt_iface_ctl: OptIfaceCtl,
    phone_assign_ctl: PhoneAssignCtl,
    word_clk_ctl: WordClkCtl,
    mixer_output_ctl: MixerOutputCtl,
    mixer_return_ctl: MixerReturnCtl,
    mixer_source_ctl: MixerSourceCtl,
    output_ctl: OutputCtl,
    line_input_ctl: LineInputCtl,
}

#[derive(Default)]
struct PhoneAssignCtl(usize, Vec<ElemId>);

impl PhoneAssignCtlOperation<F828mk2Protocol> for PhoneAssignCtl {
    fn state(&self) -> &usize {
        &self.0
    }

    fn state_mut(&mut self) -> &mut usize {
        &mut self.0
    }
}

#[derive(Default)]
struct WordClkCtl(WordClkSpeedMode, Vec<ElemId>);

impl WordClkCtlOperation<F828mk2Protocol> for WordClkCtl {
    fn state(&self) -> &WordClkSpeedMode {
        &self.0
    }

    fn state_mut(&mut self) -> &mut WordClkSpeedMode {
        &mut self.0
    }
}

#[derive(Default)]
struct ClkCtl;

impl V2ClkCtlOperation<F828mk2Protocol> for ClkCtl {}

#[derive(Default)]
struct OptIfaceCtl((usize, usize), Vec<ElemId>);

impl V2OptIfaceCtlOperation<F828mk2Protocol> for OptIfaceCtl {
    fn state(&self) -> &(usize, usize) {
        &self.0
    }

    fn state_mut(&mut self) -> &mut (usize, usize) {
        &mut self.0
    }
}

#[derive(Default)]
struct MixerOutputCtl(RegisterDspMixerOutputState, Vec<ElemId>);

impl RegisterDspMixerOutputCtlOperation<F828mk2Protocol> for MixerOutputCtl {
    fn state(&self) -> &RegisterDspMixerOutputState {
        &self.0
    }

    fn state_mut(&mut self) -> &mut RegisterDspMixerOutputState {
        &mut self.0
    }
}

#[derive(Default)]
struct MixerReturnCtl(RegisterDspMixerReturnState, Vec<ElemId>);

impl RegisterDspMixerReturnCtlOperation<F828mk2Protocol> for MixerReturnCtl {
    fn state(&self) -> &RegisterDspMixerReturnState {
        &self.0
    }

    fn state_mut(&mut self) -> &mut RegisterDspMixerReturnState {
        &mut self.0
    }
}

#[derive(Default)]
struct MixerSourceCtl(RegisterDspMixerMonauralSourceState, Vec<ElemId>);

impl RegisterDspMixerMonauralSourceCtlOperation<F828mk2Protocol> for MixerSourceCtl {
    fn state(&self) -> &RegisterDspMixerMonauralSourceState {
        &self.0
    }

    fn state_mut(&mut self) -> &mut RegisterDspMixerMonauralSourceState {
        &mut self.0
    }
}

#[derive(Default)]
struct OutputCtl(RegisterDspOutputState, Vec<ElemId>);

impl RegisterDspOutputCtlOperation<F828mk2Protocol> for OutputCtl {
    fn state(&self) -> &RegisterDspOutputState {
        &self.0
    }

    fn state_mut(&mut self) -> &mut RegisterDspOutputState {
        &mut self.0
    }
}

#[derive(Default)]
struct LineInputCtl(Traveler828mk2LineInputState, Vec<ElemId>);

impl Traveler828mk2LineInputCtlOperation<F828mk2Protocol> for LineInputCtl {
    fn state(&self) -> &Traveler828mk2LineInputState {
        &self.0
    }

    fn state_mut(&mut self) -> &mut Traveler828mk2LineInputState {
        &mut self.0
    }
}

impl CtlModel<SndMotu> for F828mk2 {
    fn load(&mut self, unit: &mut SndMotu, card_cntr: &mut CardCntr) -> Result<(), Error> {
        self.clk_ctls.load(card_cntr)?;
        self.opt_iface_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|mut elem_id_list| self.opt_iface_ctl.1.append(&mut elem_id_list))?;
        self.phone_assign_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|mut elem_id_list| self.phone_assign_ctl.1.append(&mut elem_id_list))?;
        self.word_clk_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|mut elem_id_list| self.word_clk_ctl.1.append(&mut elem_id_list))?;
        self.mixer_output_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|elem_id_list| self.mixer_output_ctl.1 = elem_id_list)?;
        self.mixer_return_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|elem_id_list| self.mixer_return_ctl.1 = elem_id_list)?;
        self.mixer_source_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|elem_id_list| self.mixer_source_ctl.1 = elem_id_list)?;
        self.output_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|elem_id_list| self.output_ctl.1 = elem_id_list)?;
        self.line_input_ctl.load(card_cntr, unit, &mut self.req, TIMEOUT_MS)
            .map(|elem_id_list| self.line_input_ctl.1 = elem_id_list)?;
        Ok(())
    }

    fn read(
        &mut self,
        unit: &mut SndMotu,
        elem_id: &ElemId,
        elem_value: &mut ElemValue
    ) -> Result<bool, Error> {
        if self.clk_ctls.read(unit, &mut self.req, elem_id, elem_value, TIMEOUT_MS)? {
            Ok(true)
        } else if self.opt_iface_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.phone_assign_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.word_clk_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.mixer_output_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.mixer_return_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.mixer_source_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.output_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else if self.line_input_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn write(
        &mut self,
        unit: &mut SndMotu,
        elem_id: &ElemId,
        _: &ElemValue,
        new: &ElemValue
    ) -> Result<bool, Error> {
        if self.clk_ctls.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.opt_iface_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.phone_assign_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.word_clk_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.mixer_output_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.mixer_return_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.mixer_source_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.output_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else if self.line_input_ctl.write(unit, &mut self.req, elem_id, new, TIMEOUT_MS)? {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl NotifyModel<SndMotu, u32> for F828mk2 {
    fn get_notified_elem_list(&mut self, elem_id_list: &mut Vec<ElemId>) {
        elem_id_list.extend_from_slice(&self.word_clk_ctl.1);
    }

    fn parse_notification(&mut self, unit: &mut SndMotu, msg: &u32) -> Result<(), Error> {
        if *msg & F828mk2Protocol::NOTIFY_PORT_CHANGE > 0 {
            self.word_clk_ctl.cache(unit, &mut self.req, TIMEOUT_MS)?;
        }
        // TODO: what kind of event is preferable for NOTIFY_FOOTSWITCH_MASK?
        Ok(())
    }

    fn read_notified_elem(
        &mut self,
        _: &SndMotu,
        elem_id: &ElemId,
        elem_value: &mut ElemValue
    ) -> Result<bool, Error> {
        if self.word_clk_ctl.read(elem_id, elem_value)? {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
