// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

pub mod spro40_model;
pub mod liquids56_model;
pub mod spro24_model;
pub mod spro24dsp_model;
pub mod spro14_model;
pub mod spro26_model;

use glib::{Error, FileError};

use hinawa::FwReq;
use hinawa::{SndDice, SndUnitExt};

use alsactl::{ElemId, ElemIfaceType, ElemValue, ElemValueExt, ElemValueExtManual};

use dice_protocols::tcat::extension::*;
use dice_protocols::focusrite::*;

use core::card_cntr::*;
use core::elem_value_accessor::*;

const VOL_NAME: &str = "output-group-volume";
const VOL_HWCTL_NAME: &str = "output-group-volume-hwctl";
const VOL_MUTE_NAME: &str = "output-group-volume-mute";
const MUTE_NAME: &str = "output-group-mute";
const DIM_NAME: &str = "output-group-dim";
const DIM_HWCTL_NAME: &str= "output-group-dim-hwctl";
const MUTE_HWCTL_NAME: &str = "output-group-mute-hwctl";

trait OutGroupCtlOperation<T: SaffireproOutGroupOperation> {
    fn state(&self) -> &OutGroupState;
    fn state_mut(&mut self) -> &mut OutGroupState;

    const LEVEL_MIN: i32 = 0x00;
    const LEVEL_MAX: i32 = 0x7f;
    const LEVEL_STEP: i32 = 0x01;

    fn load(
        &mut self,
        card_cntr: &mut CardCntr,
        unit: &mut SndDice,
        req: &mut FwReq,
        sections: &ExtensionSections,
        timeout_ms: u32
    ) -> Result<Vec<ElemId>, Error> {

        let mut node = unit.get_node();
        let mut state = T::create_out_group_state();
        T::read_out_group_mute(req, &mut node, sections, &mut state, timeout_ms)?;
        T::read_out_group_dim(req, &mut node, sections, &mut state, timeout_ms)?;
        T::read_out_group_vols(req, &mut node, sections, &mut state, timeout_ms)?;
        T::read_out_group_vol_mute_hwctls(req, &mut node, sections, &mut state, timeout_ms)?;
        T::read_out_group_dim_mute_hwctls(req, &mut node, sections, &mut state, timeout_ms)?;

        *self.state_mut() = state;

        let mut notified_elem_id_list = Vec::new();

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, MUTE_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, 1, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, DIM_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, 1, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        let output_count = T::ENTRY_COUNT;

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, VOL_NAME, 0);
        card_cntr.add_int_elems(&elem_id, 1, Self::LEVEL_MIN, Self::LEVEL_MAX, Self::LEVEL_STEP,
                                output_count, None, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, VOL_MUTE_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, output_count, true)?;

        if T::HAS_VOL_HWCTL {
            let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, VOL_HWCTL_NAME, 0);
            card_cntr.add_bool_elems(&elem_id, 1, output_count, true)?;
        }

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, DIM_HWCTL_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, output_count, true)?;

        let elem_id = ElemId::new_by_name(ElemIfaceType::Card, 0, 0, MUTE_HWCTL_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, output_count, true)?;

        Ok(notified_elem_id_list)
    }

    fn read(
        &mut self,
        elem_id: &ElemId,
        elem_value: &mut ElemValue
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            VOL_MUTE_NAME => {
                elem_value.set_bool(&self.state().vol_mutes);
                Ok(true)
            }
            VOL_HWCTL_NAME => {
                elem_value.set_bool(&self.state().vol_hwctls);
                Ok(true)
            }
            DIM_HWCTL_NAME => {
                elem_value.set_bool(&self.state().dim_hwctls);
                Ok(true)
            }
            MUTE_HWCTL_NAME => {
                elem_value.set_bool(&self.state().mute_hwctls);
                Ok(true)
            }
            _ => self.read_notified_elem(elem_id, elem_value),
        }
    }

    fn write(
        &mut self,
        unit: &mut SndDice,
        req: &mut FwReq,
        sections: &ExtensionSections,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MUTE_NAME => {
                ElemValueAccessor::<bool>::get_val(elem_value, |val| {
                    T::write_out_group_mute(
                        req,
                        &mut unit.get_node(),
                        sections,
                        self.state_mut(),
                        val,
                        timeout_ms
                    )
                })
                .map(|_| true)
            }
            DIM_NAME => {
                ElemValueAccessor::<bool>::get_val(elem_value, |val| {
                    T::write_out_group_dim(
                        req,
                        &mut unit.get_node(),
                        sections,
                        self.state_mut(),
                        val,
                        timeout_ms
                    )
                })
                .map(|_| true)
            }
            VOL_NAME => {
                let mut vals = vec![0i32; T::ENTRY_COUNT];
                elem_value.get_int(&mut vals);
                let vols: Vec<i8> = vals.iter()
                    .map(|&v| (Self::LEVEL_MAX - v) as i8)
                    .collect();
                T::write_out_group_vols(
                    req,
                    &mut unit.get_node(),
                    sections,
                    self.state_mut(),
                    &vols,
                    timeout_ms
                )
                .map(|_| true)
            }
            VOL_MUTE_NAME => {
                let mut vol_mutes = vec![false; T::ENTRY_COUNT];
                elem_value.get_bool(&mut vol_mutes);
                let vol_hwctls = self.state().vol_hwctls.clone();
                T::write_out_group_vol_mute_hwctls(
                    req,
                    &mut unit.get_node(),
                    sections,
                    self.state_mut(),
                    &vol_mutes,
                    &vol_hwctls,
                    timeout_ms
                )
                .map(|_| true)
            }
            VOL_HWCTL_NAME => {
                let mut vol_hwctls = self.state().vol_hwctls.clone();
                elem_value.get_bool(&mut vol_hwctls);
                let vol_mutes = vec![false; T::ENTRY_COUNT];
                T::write_out_group_vol_mute_hwctls(
                    req,
                    &mut unit.get_node(),
                    sections,
                    self.state_mut(),
                    &vol_mutes,
                    &vol_hwctls,
                    timeout_ms
                )
                .map(|_| true)
            }
            DIM_HWCTL_NAME => {
                let mut dim_hwctls = vec![false; T::ENTRY_COUNT];
                elem_value.get_bool(&mut dim_hwctls);
                let mute_hwctls = self.state().mute_hwctls.clone();
                T::write_out_group_dim_mute_hwctls(
                    req,
                    &mut unit.get_node(),
                    sections,
                    self.state_mut(),
                    &dim_hwctls,
                    &mute_hwctls,
                    timeout_ms
                )?;
                Ok(true)
            }
            MUTE_HWCTL_NAME => {
                let mut mute_hwctls = vec![false; T::ENTRY_COUNT];
                elem_value.get_bool(&mut mute_hwctls);
                let dim_hwctls = self.state().dim_hwctls.clone();
                T::write_out_group_dim_mute_hwctls(
                    req,
                    &mut unit.get_node(),
                    sections,
                    self.state_mut(),
                    &dim_hwctls,
                    &mute_hwctls,
                    timeout_ms
                )?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn parse_notification(
        &mut self,
        unit: &mut SndDice,
        req: &mut FwReq,
        sections: &ExtensionSections,
        msg: u32,
        timeout_ms: u32
    ) -> Result<(), Error> {
        if T::has_dim_mute_change(msg) {
            let mut node = unit.get_node();
            let state = self.state_mut();
            T::read_out_group_mute(req, &mut node, sections, state, timeout_ms)?;
            T::read_out_group_dim(req, &mut node, sections, state, timeout_ms)?;
        }

        if T::has_vol_change(msg) {
            let state = self.state_mut();
            T::read_out_group_knob_value(
                req,
                &mut unit.get_node(),
                sections,
                state,
                timeout_ms
            )?;

            let vol = state.hw_knob_value;
            let hwctls = state.vol_hwctls.clone();
            state.vols.iter_mut()
                .zip(hwctls.iter())
                .filter(|(_, &hwctl)| hwctl)
                .for_each(|(v, _)| *v = vol);
        }

        Ok(())
    }

    fn read_notified_elem(
        &self,
        elem_id: &ElemId,
        elem_value: &ElemValue
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MUTE_NAME => {
                elem_value.set_bool(&[self.state().mute_enabled]);
                Ok(true)
            }
            DIM_NAME => {
                elem_value.set_bool(&[self.state().dim_enabled]);
                Ok(true)
            }
            VOL_NAME => {
                let vols: Vec<i32> = self.state().vols.iter()
                    .map(|&v| Self::LEVEL_MAX - (v as i32))
                    .collect();
                elem_value.set_int(&vols);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

const MIC_INPUT_LEVEL_NAME: &str = "mic-input-level";
const LINE_INPUT_LEVEL_NAME: &str = "line-input-level";

fn mic_input_level_to_str(level: &SaffireproMicInputLevel) -> &'static str {
    match level {
        SaffireproMicInputLevel::Line => "line",
        SaffireproMicInputLevel::Instrument => "instrument",
    }
}

fn line_input_level_to_str(level: &SaffireproLineInputLevel) -> &'static str {
    match level {
        SaffireproLineInputLevel::Low => "low",
        SaffireproLineInputLevel::High => "high",
    }
}

trait SaffireproInputCtlOperation<T: SaffireproInputOperation> {
    const MIC_LEVELS: [SaffireproMicInputLevel; 2] = [
        SaffireproMicInputLevel::Line,
        SaffireproMicInputLevel::Instrument,
    ];

    const LINE_LEVELS: [SaffireproLineInputLevel; 2] = [
        SaffireproLineInputLevel::Low,
        SaffireproLineInputLevel::High,
    ];

    fn load(&mut self, card_cntr: &mut CardCntr) -> Result<(), Error> {
        let labels: Vec<&str> = Self::MIC_LEVELS
            .iter()
            .map(|l| mic_input_level_to_str(l))
            .collect();
        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, MIC_INPUT_LEVEL_NAME, 0);
        let _ = card_cntr
            .add_enum_elems(&elem_id, 1, T::MIC_INPUT_COUNT, &labels, None, true)?;

        let labels: Vec<&str> = Self::LINE_LEVELS
            .iter()
            .map(|l| line_input_level_to_str(l))
            .collect();
        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, LINE_INPUT_LEVEL_NAME, 0);
        let _ = card_cntr
            .add_enum_elems(&elem_id, 1, T::LINE_INPUT_COUNT, &labels, None, true)?;

        Ok(())
    }

    fn read(
        &mut self,
        unit: &mut SndDice,
        req: &mut FwReq,
        sections: &ExtensionSections,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
        timeout_ms: u32
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MIC_INPUT_LEVEL_NAME => {
                let mut levels = vec![SaffireproMicInputLevel::default(); T::MIC_INPUT_COUNT];
                T::read_mic_level(
                    req,
                    &mut unit.get_node(),
                    sections,
                    &mut levels,
                    timeout_ms
                )?;
                ElemValueAccessor::<u32>::set_vals(elem_value, T::MIC_INPUT_COUNT, |idx| {
                    let pos = Self::MIC_LEVELS
                        .iter()
                        .position(|l| l.eq(&levels[idx]))
                        .unwrap();
                    Ok(pos as u32)
                })
                    .map(|_| true)
            }
            LINE_INPUT_LEVEL_NAME => {
                let mut levels = vec![SaffireproLineInputLevel::default(); T::LINE_INPUT_COUNT];
                T::read_line_level(
                    req,
                    &mut unit.get_node(),
                    sections,
                    &mut levels,
                    timeout_ms
                )?;
                ElemValueAccessor::<u32>::set_vals(elem_value, T::MIC_INPUT_COUNT, |idx| {
                    let pos = Self::LINE_LEVELS
                        .iter()
                        .position(|l| l.eq(&levels[idx]))
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
        unit: &mut SndDice,
        req: &mut FwReq,
        sections: &ExtensionSections,
        elem_id: &ElemId,
        elem_value: &ElemValue,
        timeout_ms: u32
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MIC_INPUT_LEVEL_NAME => {
                let mut vals = vec![0; T::MIC_INPUT_COUNT];
                elem_value.get_enum(&mut vals);
                let mut levels = vec![SaffireproMicInputLevel::default(); T::MIC_INPUT_COUNT];
                vals
                    .iter()
                    .enumerate()
                    .try_for_each(|(i, &val)| {
                        Self::MIC_LEVELS
                            .iter()
                            .nth(val as usize)
                            .ok_or_else(|| {
                                let msg = format!("Invalid index for mic input levels: {}", val);
                                Error::new(FileError::Inval, &msg)
                            })
                            .map(|&l| levels[i] = l)
                    })?;
                T::write_mic_level(
                    req,
                    &mut unit.get_node(),
                    sections,
                    &levels,
                    timeout_ms,
                )
                    .map(|_| true)
            }
            LINE_INPUT_LEVEL_NAME => {
                let mut vals = vec![0; T::LINE_INPUT_COUNT];
                elem_value.get_enum(&mut vals);
                let mut levels = vec![SaffireproLineInputLevel::default(); T::LINE_INPUT_COUNT];
                vals
                    .iter()
                    .enumerate()
                    .try_for_each(|(i, &val)| {
                        Self::LINE_LEVELS
                            .iter()
                            .nth(val as usize)
                            .ok_or_else(|| {
                                let msg = format!("Invalid index for line input levels: {}", val);
                                Error::new(FileError::Inval, &msg)
                            })
                            .map(|&l| levels[i] = l)
                    })?;
                T::write_line_level(
                    req,
                    &mut unit.get_node(),
                    sections,
                    &levels,
                    timeout_ms,
                )
                    .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}
