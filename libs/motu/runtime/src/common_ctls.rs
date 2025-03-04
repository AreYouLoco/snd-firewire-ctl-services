// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

use glib::{Error, FileError};

use hinawa::FwReq;
use hinawa::{SndMotu, SndUnitExt};

use alsactl::{ElemId, ElemIfaceType, ElemValue};

use core::card_cntr::CardCntr;
use core::elem_value_accessor::ElemValueAccessor;

use motu_protocols::*;

use super::*;

const PHONE_ASSIGN_NAME: &str = "phone-assign";

pub trait PhoneAssignCtlOperation<T: AssignOperation> {
    fn state(&self) -> &usize;
    fn state_mut(&mut self) -> &mut usize;

    fn load(
        &mut self,
        card_cntr: &mut CardCntr,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32
    ) -> Result<Vec<ElemId>, Error> {
        self.cache(unit, req, timeout_ms)?;

        let labels: Vec<&str> = T::ASSIGN_PORTS
            .iter()
            .map(|e| target_port_to_str(&e.0))
            .collect();
        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, PHONE_ASSIGN_NAME, 0);
        card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)
    }

    fn cache(&mut self, unit: &mut SndMotu, req: &mut FwReq, timeout_ms: u32) -> Result<(), Error> {
        T::get_phone_assign(req, &mut unit.get_node(), timeout_ms).map(|val| *self.state_mut() = val)
    }

    fn read(
        &mut self,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            PHONE_ASSIGN_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || Ok(*self.state() as u32))
                    .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    fn write(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            PHONE_ASSIGN_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_phone_assign(req, &mut unit.get_node(), val as usize, timeout_ms)
                        .map(|_| *self.state_mut() = val as usize)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

fn word_clk_speed_mode_to_str(mode: &WordClkSpeedMode) -> &'static str {
    match mode {
        WordClkSpeedMode::ForceLowRate => "Force 44.1/48.0 kHz",
        WordClkSpeedMode::FollowSystemClk => "Follow to system clock",
    }
}

const WORD_OUT_MODE_NAME: &str = "word-out-mode";

const WORD_OUT_MODES: [WordClkSpeedMode; 2] = [
    WordClkSpeedMode::ForceLowRate,
    WordClkSpeedMode::FollowSystemClk,
];

pub trait WordClkCtlOperation<T: WordClkOperation> {
    fn state(&self) -> &WordClkSpeedMode;
    fn state_mut(&mut self) -> &mut WordClkSpeedMode;

    fn load(
        &mut self,
        card_cntr: &mut CardCntr,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32
    ) -> Result<Vec<ElemId>, Error> {
        self.cache(unit, req, timeout_ms)?;

        let labels: Vec<&str> = WORD_OUT_MODES
            .iter()
            .map(|m| word_clk_speed_mode_to_str(m))
            .collect();
        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, WORD_OUT_MODE_NAME, 0);
        card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)
    }

    fn cache(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32
    ) -> Result<(), Error> {
        T::get_word_out(req, &mut unit.get_node(), timeout_ms).map(|mode| *self.state_mut() = mode)
    }

    fn read(
        &mut self,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            WORD_OUT_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || {
                    let pos = WORD_OUT_MODES
                        .iter()
                        .position(|m| self.state().eq(m))
                        .unwrap();
                    Ok(pos as u32)
                })
                    .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    fn write(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            WORD_OUT_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    let &mode = WORD_OUT_MODES.iter()
                        .nth(val as usize)
                        .ok_or_else(|| {
                            let msg =
                                format!("Invalid argument for index of word clock speed: {}", val);
                            Error::new(FileError::Inval, &msg)
                        })?;
                    T::set_word_out(req, &mut unit.get_node(), mode, timeout_ms)
                        .map(|_| *self.state_mut() = mode)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

fn aesebu_rate_convert_mode_to_str(mode: &AesebuRateConvertMode) -> &'static str{
    match mode {
        AesebuRateConvertMode::None => "None",
        AesebuRateConvertMode::InputToSystem => "input-is-converted",
        AesebuRateConvertMode::OutputDependsInput => "output-depends-on-input",
        AesebuRateConvertMode::OutputDoubleSystem => "output-is-double",
    }
}

const AESEBU_RATE_CONVERT_MODE_NAME: &str = "AES/EBU-rate-convert";

pub trait AesebuRateConvertCtlOperation<T: AesebuRateConvertOperation> {
    fn load(&mut self, card_cntr: &mut CardCntr) -> Result<(), Error> {
        let labels: Vec<&str> = T::AESEBU_RATE_CONVERT_MODES
            .iter()
            .map(|l| aesebu_rate_convert_mode_to_str(l))
            .collect();
        let elem_id = ElemId::new_by_name(
            ElemIfaceType::Card,
            0,
            0,
            AESEBU_RATE_CONVERT_MODE_NAME,
            0,
        );
        let _ = card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)?;

        Ok(())
    }

    fn read(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            AESEBU_RATE_CONVERT_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || {
                    T::get_aesebu_rate_convert_mode(req, &mut unit.get_node(), timeout_ms)
                        .map(|val| val as u32)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    fn write(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            AESEBU_RATE_CONVERT_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_aesebu_rate_convert_mode(req, &mut unit.get_node(), val as usize, timeout_ms)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

fn level_meters_hold_time_mode_to_string(mode: &LevelMetersHoldTimeMode) -> &'static str {
    match mode {
        LevelMetersHoldTimeMode::Off => "off",
        LevelMetersHoldTimeMode::Sec2 => "2sec",
        LevelMetersHoldTimeMode::Sec4 => "4sec",
        LevelMetersHoldTimeMode::Sec10 => "10sec",
        LevelMetersHoldTimeMode::Sec60 => "1min",
        LevelMetersHoldTimeMode::Sec300 => "5min",
        LevelMetersHoldTimeMode::Sec480 => "8min",
        LevelMetersHoldTimeMode::Infinite => "infinite",
    }
}

fn level_meters_aesebu_mode_to_string(mode: &LevelMetersAesebuMode) -> &'static str {
    match mode {
        LevelMetersAesebuMode::Output => "output",
        LevelMetersAesebuMode::Input => "input",
    }
}

fn level_meters_programmable_mode_to_string(mode: &LevelMetersProgrammableMode) -> &'static str {
    match mode {
        LevelMetersProgrammableMode::AnalogOutput => "analog-output",
        LevelMetersProgrammableMode::AdatInput => "ADAT-input",
        LevelMetersProgrammableMode::AdatOutput => "ADAT-output",
    }
}

const PEAK_HOLD_TIME_MODE_NAME: &str = "meter-peak-hold-time";
const CLIP_HOLD_TIME_MODE_NAME: &str = "meter-clip-hold-time";
const AESEBU_MODE_NAME: &str = "AES/EBU-meter";
const PROGRAMMABLE_MODE_NAME: &str = "programmable-meter";

#[derive(Default)]
pub struct LevelMeterState(usize, usize);

pub trait LevelMetersCtlOperation<T: LevelMetersOperation> {
    fn state(&self) -> &LevelMeterState;
    fn state_mut(&mut self) -> &mut LevelMeterState;

    fn load(
        &mut self,
        card_cntr: &mut CardCntr,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32
    ) -> Result<Vec<ElemId>, Error> {
        self.cache(unit, req, timeout_ms)?;

        let mut notified_elem_id_list = Vec::new();

        let labels: Vec<&str> = T::LEVEL_METERS_HOLD_TIME_MODES
            .iter()
            .map(|l| level_meters_hold_time_mode_to_string(&l))
            .collect();
        let elem_id =
            ElemId::new_by_name(ElemIfaceType::Card, 0, 0, PEAK_HOLD_TIME_MODE_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)?;

        let elem_id =
            ElemId::new_by_name(ElemIfaceType::Card, 0, 0, CLIP_HOLD_TIME_MODE_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)?;

        let labels: Vec<&str> = T::LEVEL_METERS_AESEBU_MODES
            .iter()
            .map(|l| level_meters_aesebu_mode_to_string(&l))
            .collect();
        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, AESEBU_MODE_NAME, 0);
        card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        let labels: Vec<&str> = T::LEVEL_METERS_PROGRAMMABLE_MODES
            .iter()
            .map(|l| level_meters_programmable_mode_to_string(&l))
            .collect();
        let elem_id =
            ElemId::new_by_name(ElemIfaceType::Card, 0, 0, PROGRAMMABLE_MODE_NAME, 0);
        card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        Ok(notified_elem_id_list)
    }

    fn cache(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32
    ) -> Result<(), Error> {
        T::get_level_meters_aesebu_mode(req, &mut unit.get_node(), timeout_ms).map(|idx| {
            self.state_mut().0 = idx;
        })?;

        T::get_level_meters_programmable_mode(req, &mut unit.get_node(), timeout_ms).map(|idx| {
            self.state_mut().1 = idx;
        })?;

        Ok(())
    }

    fn read(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            PEAK_HOLD_TIME_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || {
                    T::get_level_meters_peak_hold_time_mode(req, &mut unit.get_node(), timeout_ms)
                        .map(|val| val as u32)
                })
                .map(|_| true)
            }
            CLIP_HOLD_TIME_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || {
                    T::get_level_meters_clip_hold_time_mode(req, &mut unit.get_node(), timeout_ms)
                        .map(|val| val as u32)
                })
                .map(|_| true)
            }
            _ => self.refer(elem_id, elem_value),
        }
    }

    fn refer(&mut self, elem_id: &ElemId, elem_value: &mut ElemValue) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            AESEBU_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || Ok(self.state().0 as u32))
                .map(|_| true)
            }
            PROGRAMMABLE_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || Ok(self.state().1 as u32))
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    fn write(
        &mut self,
        unit: &mut SndMotu,
        req: &mut FwReq,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            PEAK_HOLD_TIME_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_level_meters_peak_hold_time_mode(req, &mut unit.get_node(), val as usize, timeout_ms)
                })
                .map(|_| true)
            }
            CLIP_HOLD_TIME_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_level_meters_clip_hold_time_mode(req, &mut unit.get_node(), val as usize, timeout_ms)
                })
                .map(|_| true)
            }
            AESEBU_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_level_meters_aesebu_mode(req, &mut unit.get_node(), val as usize, timeout_ms)
                        .map(|_| self.state_mut().0 = val as usize)
                })
                .map(|_| true)
            }
            PROGRAMMABLE_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(elem_value, |val| {
                    T::set_level_meters_programmable_mode(req, &mut unit.get_node(), val as usize, timeout_ms)
                        .map(|_| self.state_mut().1 = val as usize)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}
