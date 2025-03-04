// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2020 Takashi Sakamoto

use {
    glib::Error,
    hinawa::SndEfw,
    alsactl::{ElemId, ElemIfaceType, ElemValue},
    core::{card_cntr::*, elem_value_accessor::*},
    efw_protocols::{hw_info::*, robot_guitar::*},
};

#[derive(Default)]
pub struct GuitarCtl;

const MANUAL_CHARGE_NAME: &str = "guitar-manual-chage";
const AUTO_CHARGE_NAME: &str = "guitar-auto-chage";
const SUSPEND_TO_CHARGE: &str = "guitar-suspend-to-charge";

impl GuitarCtl {
    const MIN_SEC: i32 = 0;
    const MAX_SEC: i32 = 60 * 60;   // = One hour.
    const STEP_SEC: i32 = 1;

    pub fn load(&mut self, hwinfo: &HwInfo, card_cntr: &mut CardCntr) -> Result<(), Error> {
        let has_guitar_charge = hwinfo.caps.iter().find(|&e| *e == HwCap::GuitarCharging).is_some();

        if has_guitar_charge {
            let elem_id = ElemId::new_by_name(
                ElemIfaceType::Card, 0, 0, MANUAL_CHARGE_NAME, 0);
            let _ = card_cntr.add_bool_elems(&elem_id, 1, 1, true)?;

            let elem_id = ElemId::new_by_name(
                ElemIfaceType::Card, 0, 0, AUTO_CHARGE_NAME, 0);
            let _ = card_cntr.add_bool_elems(&elem_id, 1, 1, true)?;

            let elem_id = ElemId::new_by_name(
                ElemIfaceType::Card, 0, 0, SUSPEND_TO_CHARGE, 0);
            let _ = card_cntr.add_int_elems(&elem_id, 1,
                Self::MIN_SEC, Self::MAX_SEC, Self::STEP_SEC, 1, None, true)?;
        }

        Ok(())
    }

    pub fn read(
        &mut self,
        unit: &mut SndEfw,
        elem_id: &ElemId,
        elem_value: &mut ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MANUAL_CHARGE_NAME => {
                ElemValueAccessor::<bool>::set_val(elem_value, || {
                    unit.get_charge_state(timeout_ms)
                        .map(|s| s.manual_charge)
                })?;
                Ok(true)
            }
            AUTO_CHARGE_NAME => {
                ElemValueAccessor::<bool>::set_val(elem_value, || {
                    unit.get_charge_state(timeout_ms)
                        .map(|s| s.auto_charge)
                })?;
                Ok(true)
            }
            SUSPEND_TO_CHARGE => {
                ElemValueAccessor::<i32>::set_val(elem_value, || {
                    unit.get_charge_state(timeout_ms)
                        .map(|s| s.suspend_to_charge as i32)
                })?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(
        &mut self,
        unit: &mut SndEfw,
        elem_id: &ElemId,
        _: &ElemValue,
        new: &ElemValue,
        timeout_ms: u32,
    ) -> Result<bool, Error> {
        match elem_id.get_name().as_str() {
            MANUAL_CHARGE_NAME => {
                ElemValueAccessor::<bool>::get_val(new, |val| {
                    let mut state = unit.get_charge_state(timeout_ms)?;
                    state.manual_charge = val;
                    unit.set_charge_state(&state, timeout_ms)
                })?;
                Ok(true)
            }
            AUTO_CHARGE_NAME => {
                ElemValueAccessor::<bool>::get_val(new, |val| {
                    let mut state = unit.get_charge_state(timeout_ms)?;
                    state.auto_charge = val;
                    unit.set_charge_state(&state, timeout_ms)
                })?;
                Ok(true)
            }
            SUSPEND_TO_CHARGE => {
                ElemValueAccessor::<i32>::get_val(new, |val| {
                    let mut state = unit.get_charge_state(timeout_ms)?;
                    state.suspend_to_charge = val as u32;
                    unit.set_charge_state(&state, timeout_ms)
                })?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
