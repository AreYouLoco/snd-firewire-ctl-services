// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

use glib::{Error, FileError};

use hinawa::FwReq;
use hinawa::{SndMotu, SndUnitExt};

use alsactl::{ElemId, ElemIfaceType, ElemValue, ElemValueExt, ElemValueExtManual};

use alsa_ctl_tlv_codec::items::DbInterval;

use core::card_cntr::*;
use core::elem_value_accessor::*;

use motu_protocols::register_dsp::*;

use super::model::*;

const MIXER_OUTPUT_VOLUME_NAME: &str = "mixer-output-volume";
const MIXER_OUTPUT_MUTE_NAME: &str = "mixer-output-mute";
const MIXER_OUTPUT_DST_NAME: &str = "mixer-output-destination";

fn copy_int_to_elem_value<T: Copy + Into<i32>>(elem_value: &mut ElemValue, data: &[T]) {
    let vals: Vec<i32> = data.iter().map(|&val| val.into()).collect();
    elem_value.set_int(&vals);
}

pub trait RegisterDspMixerOutputCtlOperation<T: RegisterDspMixerOutputOperation> {
    fn state(&self) -> &RegisterDspMixerOutputState;
    fn state_mut(&mut self) -> &mut RegisterDspMixerOutputState;

    const VOL_TLV: DbInterval = DbInterval {
        min: 0,
        max: 63,
        linear: true,
        mute_avail: false,
    };

    fn load(
        &mut self,
        card_cntr: &mut CardCntr,
        unit: &mut SndMotu,
        req: &mut FwReq,
        timeout_ms: u32,
    ) -> Result<Vec<ElemId>, Error> {
        T::read_mixer_output_state(req, &mut unit.get_node(), self.state_mut(), timeout_ms)?;

        let mut notified_elem_id_list = Vec::new();

        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, MIXER_OUTPUT_VOLUME_NAME, 0);
        card_cntr.add_int_elems(
            &elem_id,
            1,
            T::MIXER_OUTPUT_VOLUME_MIN as i32,
            T::MIXER_OUTPUT_VOLUME_MAX as i32,
            T::MIXER_OUTPUT_VOLUME_STEP as i32,
            T::MIXER_COUNT,
            Some(&Vec::<u32>::from(&Self::VOL_TLV)),
            true,
        )
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, MIXER_OUTPUT_MUTE_NAME, 0);
        card_cntr.add_bool_elems(&elem_id, 1, T::MIXER_COUNT, true)
            .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;

        if T::OUTPUT_DESTINATIONS.len() > 0 {
            let labels: Vec<&str> = T::OUTPUT_DESTINATIONS.iter()
                .map(|p| target_port_to_str(p))
                .collect();
            let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, MIXER_OUTPUT_DST_NAME, 0);
            card_cntr.add_enum_elems(&elem_id, 1, T::MIXER_COUNT, &labels, None, true)
                .map(|mut elem_id_list| notified_elem_id_list.append(&mut elem_id_list))?;
        }

        Ok(notified_elem_id_list)
    }

    fn read(&mut self, elem_id: &ElemId, elem_value: &mut ElemValue) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MIXER_OUTPUT_VOLUME_NAME => {
                copy_int_to_elem_value(elem_value, &self.state().volume);
                Ok(true)
            }
            MIXER_OUTPUT_MUTE_NAME => {
                elem_value.set_bool(&self.state().mute);
                Ok(true)
            }
            MIXER_OUTPUT_DST_NAME => {
                ElemValueAccessor::<u32>::set_vals(elem_value, T::MIXER_COUNT, |idx| {
                    let val = T::OUTPUT_DESTINATIONS
                        .iter()
                        .position(|p| self.state().destination[idx].eq(p))
                        .unwrap();
                    Ok(val as u32)
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
            MIXER_OUTPUT_VOLUME_NAME => {
                let mut vals = vec![0; T::MIXER_COUNT];
                elem_value.get_int(&mut vals);
                let vols: Vec<u8> = vals.iter().map(|&vol| vol as u8).collect();
                T::write_mixer_output_volume(
                    req,
                    &mut unit.get_node(),
                    &vols,
                    self.state_mut(),
                    timeout_ms
                )
                    .map(|_| true)
            }
            MIXER_OUTPUT_MUTE_NAME => {
                let mut mute = vec![false; T::MIXER_COUNT];
                elem_value.get_bool(&mut mute);
                T::write_mixer_output_mute(
                    req,
                    &mut unit.get_node(),
                    &mute,
                    self.state_mut(),
                    timeout_ms
                )
                    .map(|_| true)
            }
            MIXER_OUTPUT_DST_NAME => {
                let mut vals = vec![0; T::MIXER_COUNT];
                elem_value.get_enum(&mut vals);
                let mut dst = Vec::new();
                vals
                    .iter()
                    .try_for_each(|&val| {
                        T::OUTPUT_DESTINATIONS
                            .iter()
                            .nth(val as usize)
                            .ok_or_else(|| {
                                let msg = format!("Invalid index for ourput destination: {}", val);
                                Error::new(FileError::Inval, &msg)
                            })
                            .map(|&port| dst.push(port))
                    })?;
                T::write_mixer_output_destination(
                    req,
                    &mut unit.get_node(),
                    &dst,
                    self.state_mut(),
                    timeout_ms
                )
                    .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}
