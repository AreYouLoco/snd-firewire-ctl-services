// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2020 Takashi Sakamoto
use glib::{Error, FileError};

use hinawa::SndUnitExt;

use alsactl::{ElemValueExt, ElemValueExtManual};

use alsa_ctl_tlv_codec::items::DbInterval;

use core::card_cntr;
use core::elem_value_accessor::ElemValueAccessor;

use ta1394::{AvcAddr, Ta1394Avc};

use bebob_protocols::{apogee::ensemble::*, *};
use bebob_protocols::bridgeco::{BcoPlugAddr, BcoPlugDirection, BcoPlugAddrUnitType};
use bebob_protocols::bridgeco::BcoCompoundAm824StreamFormat;
use bebob_protocols::bridgeco::ExtendedStreamFormatSingle;

use crate::model::{HP_SRC_NAME, OUT_SRC_NAME};

use bebob_protocols::apogee::ensemble::{EnsembleOperation, EnsembleCmd, HwCmd};

pub struct HwCtl{
    stream: StreamMode,
}

fn stream_mode_to_str(mode: &StreamMode) -> &str {
    match mode {
        StreamMode::Format18x18 => "18x18",
        StreamMode::Format10x10 => "10x10",
        StreamMode::Format8x8 => "8x8",
    }
}

impl<'a> HwCtl {
    const STREAM_MODE_NAME: &'a str = "stream-mode";

    const STREAM_MODES: [StreamMode; 3] = [
        StreamMode::Format18x18,
        StreamMode::Format10x10,
        StreamMode::Format8x8,
    ];

    pub fn new() -> Self {
        HwCtl {
            stream: Default::default(),
        }
    }

    pub fn load(&mut self, avc: &BebobAvc, card_cntr: &mut card_cntr::CardCntr, timeout_ms: u32)
        -> Result<(), Error>
    {
        let plug_addr = BcoPlugAddr::new_for_unit(BcoPlugDirection::Output, BcoPlugAddrUnitType::Isoc,
                                                  0);
        let mut op = ExtendedStreamFormatSingle::new(&plug_addr);
        avc.status(&AvcAddr::Unit, &mut op, timeout_ms)?;
        let info = op.stream_format.as_bco_compound_am824_stream()?;
        let count = info.entries.iter()
            .filter(|entry| entry.format == BcoCompoundAm824StreamFormat::MultiBitLinearAudioRaw)
            .fold(0, |count, entry| count + entry.count as usize);
        self.stream = match count {
            18 => StreamMode::Format18x18,
            10 => StreamMode::Format10x10,
            _ => StreamMode::Format8x8,
        };

        let labels: Vec<&str> = Self::STREAM_MODES.iter()
            .map(|m| stream_mode_to_str(m))
            .collect();
        let elem_id = alsactl::ElemId::new_by_name(
            alsactl::ElemIfaceType::Card,
            0,
            0,
            Self::STREAM_MODE_NAME,
            0,
        );
        let _ = card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)?;

        Ok(())
    }

    pub fn read(&mut self, elem_id: &alsactl::ElemId, elem_value: &mut alsactl::ElemValue)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::STREAM_MODE_NAME => {
                let pos = Self::STREAM_MODES.iter()
                    .position(|m| m.eq(&self.stream))
                    .unwrap();
                elem_value.set_enum(&[pos as u32]);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(&mut self, unit: &hinawa::SndUnit, avc: &BebobAvc, elem_id: &alsactl::ElemId,
                 _: &alsactl::ElemValue, new: &alsactl::ElemValue, timeout_ms: u32)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::STREAM_MODE_NAME => {
                let mut vals = [0];
                new.get_enum(&mut vals);
                let &mode = Self::STREAM_MODES.iter()
                    .nth(vals[0] as usize)
                    .ok_or_else(|| {
                        let msg = format!("Invalid index of mode of stream: {}", vals[0]);
                        Error::new(FileError::Inval, &msg)
                    })?;
                unit.lock()?;
                let cmd = EnsembleCmd::Hw(HwCmd::StreamMode(mode));
                let mut op = EnsembleOperation::new(cmd);
                let res = avc.control(&AvcAddr::Unit, &mut op, timeout_ms);
                let _ = unit.unlock();
                res.map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

fn opt_iface_mode_to_str(mode: &OptIfaceMode) -> &str {
    match mode {
        OptIfaceMode::Spdif => "S/PDIF",
        OptIfaceMode::Adat => "ADAT/SMUX",
    }
}

pub struct OpticalCtl{
    output: OptIfaceMode,
}

impl<'a> OpticalCtl {
    const OUT_MODE_NAME: &'a str = "output-optical-mode";

    const MODES: [OptIfaceMode;2] = [
        OptIfaceMode::Spdif,
        OptIfaceMode::Adat,
    ];

    pub fn new() -> Self {
        OpticalCtl {
            output: Default::default(),
        }
    }

    pub fn load(&mut self, avc: &BebobAvc, card_cntr: &mut card_cntr::CardCntr, timeout_ms: u32)
        -> Result<(), Error>
    {
        // Transfer initialized data.
        let cmd = EnsembleCmd::OutputOptIface(self.output);
        let mut op = EnsembleOperation::new(cmd);
        avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;

        let labels: Vec<&str> = Self::MODES.iter()
            .map(|m| opt_iface_mode_to_str(m))
            .collect();
        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer,
                                                   0, 0, Self::OUT_MODE_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, 1, &labels, None, true)?;

        Ok(())
    }

    pub fn read(&mut self, elem_id: &alsactl::ElemId, elem_value: &mut alsactl::ElemValue)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::OUT_MODE_NAME => {
                ElemValueAccessor::<u32>::set_val(elem_value, || {
                    let pos = Self::MODES.iter()
                        .position(|m| *m == self.output)
                        .unwrap();
                    Ok(pos as u32)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(&mut self, avc: &BebobAvc, elem_id: &alsactl::ElemId,
                 _: &alsactl::ElemValue, new: &alsactl::ElemValue, timeout_ms: u32)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::OUT_MODE_NAME => {
                ElemValueAccessor::<u32>::get_val(new, |val| {
                    let mode = Self::MODES.iter()
                        .nth(val as usize)
                        .ok_or_else(||{
                            let msg = format!("Invalid index for mode of optical interface: {}",
                                              val);
                            Error::new(FileError::Inval, &msg)
                        })
                        .map(|m| *m)?;
                    let cmd = EnsembleCmd::OutputOptIface(mode);
                    let mut op = EnsembleOperation::new(cmd);
                    avc.control(&AvcAddr::Unit, &mut op, timeout_ms)
                        .map(|_| self.output = mode)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

fn output_nominal_level_to_str(level: &OutputNominalLevel) -> &str {
    match level {
        OutputNominalLevel::Professional => "+4dB",
        OutputNominalLevel::Consumer => "-10dB",
    }
}

pub struct OutputCtl {
    levels: [OutputNominalLevel; 8],
}

impl<'a> OutputCtl {
    const OUT_LEVEL_NAME: &'a str = "output-level";

    const OUT_LABELS: &'a [&'a str] = &[
        "analog-1", "analog-2", "analog-3", "analog-4", "analog-5", "analog-6", "analog-7",
        "analog-8",
    ];

    const NOMINAL_LEVELS: [OutputNominalLevel; 2] = [
        OutputNominalLevel::Professional,
        OutputNominalLevel::Consumer,
    ];

    pub fn new() -> Self {
        Self {
            levels: Default::default(),
        }
    }

    pub fn load(&mut self, avc: &BebobAvc, card_cntr: &mut card_cntr::CardCntr, timeout_ms: u32)
        -> Result<(), Error>
    {
        // Transfer initialized data.
        self.levels.iter()
            .enumerate()
            .try_for_each(|(i, &l)| {
                let cmd = EnsembleCmd::OutputNominalLevel(i, l);
                let mut op = EnsembleOperation::new(cmd);
                avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;
                Ok(())
            })?;

        let labels: Vec<&str> = Self::NOMINAL_LEVELS.iter()
            .map(|l| output_nominal_level_to_str(l))
            .collect();
        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer, 0, 0, Self::OUT_LEVEL_NAME, 0);
        card_cntr.add_enum_elems(&elem_id, 1, Self::OUT_LABELS.len(), &labels, None, true)
            .map(|_| ())?;

        Ok(())
    }

    pub fn read(&mut self, elem_id: &alsactl::ElemId, elem_value: &mut alsactl::ElemValue)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::OUT_LEVEL_NAME => {
                ElemValueAccessor::<u32>::set_vals(elem_value, Self::OUT_LABELS.len(), |i| {
                    let pos = Self::NOMINAL_LEVELS.iter()
                        .position(|l| *l == self.levels[i])
                        .unwrap();
                    Ok(pos as u32)
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(&mut self, avc: &BebobAvc, elem_id: &alsactl::ElemId,
                 old: &alsactl::ElemValue, new: &alsactl::ElemValue, timeout_ms: u32)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::OUT_LEVEL_NAME => {
                 ElemValueAccessor::<u32>::get_vals(new, old, Self::OUT_LABELS.len(), |idx, val| {
                    let &level = Self::NOMINAL_LEVELS.iter()
                        .nth(val as usize)
                        .ok_or_else(|| {
                            let msg = format!("Invalid value for index of input nominal level: {}",
                                              val);
                            Error::new(FileError::Inval, &msg)
                        })?;

                    let cmd = EnsembleCmd::OutputNominalLevel(idx, level);
                    let mut op = EnsembleOperation::new(cmd);
                    avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;
                    self.levels[idx] = level;
                    Ok(())
                })
                .map(|_| true)
            }
            _ => Ok(false),
        }
    }
}

pub struct MixerCtl {
    mixers: [[i32; 36]; 4],
}

impl<'a> MixerCtl {
    const MIXER_LABELS: &'a [&'a str] = &["mixer-1", "mixer-2", "mixer-3", "mixer-4"];

    const MIXER_SRC_LABELS: &'a [&'a str] = &[
        // = EnsembleCmd::MixerSrc0.
        "analog-1", "analog-2", "analog-3", "analog-4",
        "analog-5", "analog-6", "analog-7", "analog-8",
        "stream-1",
        // = EnsembleCmd::MixerSrc1.
        "stream-2", "stream-3", "stream-4",
        "stream-5", "stream-6", "stream-7", "stream-8",
        "stream-9", "stream-10",
        // = EnsembleCmd::MixerSrc2.
        "stream-11", "stream-12",
        "stream-13", "stream-14", "stream-15", "stream-16",
        "stream-17", "stream-18",
        "adat-1",
        // = EnsembleCmd::MixerSrc3.
        "adat-2", "adat-3", "adat-4",
        "adat-5", "adat-6", "adat-7", "adat-8",
        "spdif-1", "spdif-2",
    ];

    const MIXER_SRC_GAIN_NAME: &'a str = "mixer-source-gain";

    const GAIN_MIN: i32 = 0;
    const GAIN_MAX: i32 = 0x7fff;
    const GAIN_STEP: i32 = 0xff;
    const GAIN_TLV: DbInterval = DbInterval{min: -4800, max: 0, linear: false, mute_avail: true};

    pub fn new() -> Self {
        let mut mixers = [[0; 36]; 4];

        mixers.iter_mut()
            .enumerate()
            .for_each(|(i, mixer)| {
                mixer.iter_mut()
                    .enumerate()
                    .filter(|(j, _)| i % 2 == j % 2)
                    .for_each(|(_, v)| {
                        *v = Self::GAIN_MAX;
                    });
            });

        MixerCtl{mixers}
    }

    fn write_pair(&mut self, avc: &BebobAvc, index: usize, vals: &[i32], pos: usize, timeout_ms: u32)
        -> Result<(), Error>
    {
        let mut args = Vec::new();
        args.push((index / 2) as u8);

        let mut idx = 0;
        let params = (pos..(pos + 9)).fold([0; 18], |mut params, i| {
            let (l, r) = match index % 2 {
                0 => (vals[i] as i16, self.mixers[index + 1][i] as i16),
                _ => (self.mixers[index - 1][i] as i16, vals[i] as i16),
            };
            params[idx] = l;
            params[idx + 1] = r;
            idx += 2;
            params
        });


        let p = index / 2;
        let cmd = match pos / 9 {
            3 => EnsembleCmd::MixerSrc3(p, params),
            2 => EnsembleCmd::MixerSrc2(p, params),
            1 => EnsembleCmd::MixerSrc1(p, params),
            _ => EnsembleCmd::MixerSrc0(p, params),
        };

        let mut op = EnsembleOperation::new(cmd);
        avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;

        self.mixers[index].copy_from_slice(&vals[0..Self::MIXER_SRC_LABELS.len()]);

        Ok(())
    }

    pub fn load(&mut self, avc: &BebobAvc, card_cntr: &mut card_cntr::CardCntr, timeout_ms: u32)
        -> Result<(), Error>
    {
        // Transfer initialized data.
        let mixers = self.mixers;
        (0..4).try_for_each(|i| {
            mixers.iter().enumerate().try_for_each(|(j, vals)| {
                self.write_pair(avc, j, vals, i * 9, timeout_ms)
            })
        })?;

        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer,
                                                   0, 0, Self::MIXER_SRC_GAIN_NAME, 0);
        let _ = card_cntr.add_int_elems(&elem_id, Self::MIXER_LABELS.len(),
                                        Self::GAIN_MIN, Self::GAIN_MAX, Self::GAIN_STEP,
                                        Self::MIXER_SRC_LABELS.len(),
                                        Some(&Into::<Vec<u32>>::into(Self::GAIN_TLV)), true)?;

        Ok(())
    }

    pub fn read(&mut self, elem_id: &alsactl::ElemId, elem_value: &mut alsactl::ElemValue)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::MIXER_SRC_GAIN_NAME => {
                let vals = &self.mixers[elem_id.get_index() as usize];
                elem_value.set_int(vals);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(&mut self, avc: &BebobAvc, elem_id: &alsactl::ElemId,
                 old: &alsactl::ElemValue, new: &alsactl::ElemValue, timeout_ms: u32)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            Self::MIXER_SRC_GAIN_NAME => {
                let len = Self::MIXER_SRC_LABELS.len();
                let mut vals = vec![0;len * 2];
                new.get_int(&mut vals[0..len]);
                old.get_int(&mut vals[len..]);
                let index = elem_id.get_index() as usize;
                for i in 0..4 {
                    let p = i * 9;
                    if vals[p..(p + 9)] != vals[(len + p)..(len + p + 9)] {
                        self.write_pair(avc, index, &vals, p, timeout_ms)?;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub struct RouteCtl {
    out: [u32; 18],
    cap: [u32; 18],
    hp: [u32; 2],
}

impl<'a> RouteCtl {
    const PORT_LABELS: &'a [&'a str] = &[
        // From external interfaces.
        "analog-1", "analog-2", "analog-3", "analog-4",
        "analog-5", "analog-6", "analog-7", "analog-8",
        // For host computer.
        "stream-1", "stream-2", "stream-3", "stream-4",
        "stream-5", "stream-6", "stream-7", "stream-8",
        "stream-9", "stream-10", "stream-11", "stream-12",
        "stream-13", "stream-14", "stream-15", "stream-16",
        "stream-17", "stream-18",
        // From external interfaces.
        "spdif-1", "spdif-2",
        "adat-1", "adat-2", "adat-3", "adat-4",
        "adat-5", "adat-6", "adat-7", "adat-8",
        // From internal multiplexers.
        "mixer-1", "mixer-2", "mixer-3", "mixer-4",
    ];

    const OUT_LABELS: &'a [&'a str] = &[
        "analog-1", "analog-2", "analog-3", "analog-4",
        "analog-5", "analog-6", "analog-7", "analog-8",
        "spdif-1", "spdif-2",
        "adat-1", "adat-2", "adat-3", "adat-4",
        "adat-5", "adat-6", "adat-7", "adat-8",
    ];

    const OUT_SRC_LABELS: &'a [&'a str] = &[
        "analog-1", "analog-2", "analog-3", "analog-4",
        "analog-5", "analog-6", "analog-7", "analog-8",
        "stream-1", "stream-2", "stream-3", "stream-4",
        "stream-5", "stream-6", "stream-7", "stream-8",
        "stream-9", "stream-10", "stream-11", "stream-12",
        "stream-13", "stream-14", "stream-15", "stream-16",
        "stream-17", "stream-18",
        "spdif-1", "spdif-2",
        "adat-1", "adat-2", "adat-3", "adat-4",
        "adat-5", "adat-6", "adat-7", "adat-8",
        "mixer-1", "mixer-2", "mixer-3", "mixer-4",
    ];

    const CAP_LABELS: &'a [&'a str] = &[
        "stream-1", "stream-2", "stream-3", "stream-4",
        "stream-5", "stream-6", "stream-7", "stream-8",
        "stream-9", "stream-10", "stream-11", "stream-12",
        "stream-13", "stream-14", "stream-15", "stream-16",
        "stream-17", "stream-18",
    ];

    const CAP_SRC_LABELS: &'a [&'a str] = &[
        "analog-1", "analog-2", "analog-3", "analog-4", "analog-5", "analog-6", "analog-7",
        "analog-8", "spdif-1", "spdif-2", "adat-1", "adat-2", "adat-3", "adat-4", "adat-5",
        "adat-6", "adat-7", "adat-8",
    ];

    const HP_LABELS: &'a [&'a str] = &["hp-2", "hp-1"];

    const HP_SRC_LABELS: &'a [&'a str] = &[
        "analog-1/2",
        "analog-3/4",
        "analog-5/6",
        "analog-7/8",
        "spdif-1/2",
        "none",
    ];

    const CAP_SRC_NAME: &'a str = "capture-source";

    pub fn new() -> Self {
        let mut out = [0; 18];
        for (i, v) in out.iter_mut().enumerate() {
            *v = (i + 8) as u32;
        }

        let mut cap = [0; 18];
        for (i, v) in cap.iter_mut().enumerate() {
            *v = i as u32;
        }

        let hp = [1, 0];

        RouteCtl { out, cap, hp }
    }

    fn update_route(&mut self, avc: &BebobAvc, dst: &str, src: &str, timeout_ms: u32)
        -> Result<(), Error>
    {
        if let Some(d) = Self::PORT_LABELS.iter().position(|&x| x == dst) {
            if let Some(s) = Self::PORT_LABELS.iter().position(|&x| x == src) {
                let cmd = EnsembleCmd::IoRouting(d, s);
                let mut op = EnsembleOperation::new(cmd);
                avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;
                Ok(())
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        }
    }

    fn update_hp_source(&mut self, avc: &BebobAvc, dst: usize, src: usize, timeout_ms: u32)
        -> Result<(), Error>
    {
        let cmd = EnsembleCmd::HpSrc(dst, src);
        let mut op = EnsembleOperation::new(cmd);
        avc.control(&AvcAddr::Unit, &mut op, timeout_ms)?;
        self.hp[dst] = src as u32;
        Ok(())
    }

    pub fn load(&mut self, avc: &BebobAvc, card_cntr: &mut card_cntr::CardCntr, timeout_ms: u32)
        -> Result<(), Error>
    {
        // Transfer initialized data.
        Self::OUT_LABELS.iter().enumerate().try_for_each(|(i, dst)| {
            let src = Self::OUT_SRC_LABELS[8 + i];
            self.update_route(avc, dst, src, timeout_ms)
        })?;

        Self::CAP_LABELS.iter().enumerate().try_for_each(|(i, dst)| {
            let src = Self::CAP_SRC_LABELS[i];
            self.update_route(avc, dst, src, timeout_ms)
        })?;

        (0..Self::HP_LABELS.len()).try_for_each(|i| {
            self.update_hp_source(avc, i, i, timeout_ms)
        })?;

        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer, 0, 0, OUT_SRC_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, Self::OUT_LABELS.len(),
                                         Self::OUT_SRC_LABELS, None, true)?;

        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer,
                                                   0, 0, Self::CAP_SRC_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, Self::CAP_LABELS.len(),
                                         Self::CAP_SRC_LABELS, None, true)?;

        let elem_id = alsactl::ElemId::new_by_name(alsactl::ElemIfaceType::Mixer, 0, 0, HP_SRC_NAME, 0);
        let _ = card_cntr.add_enum_elems(&elem_id, 1, Self::HP_LABELS.len(),
                                         Self::HP_SRC_LABELS, None, true)?;

        Ok(())
    }

    pub fn read(&mut self, elem_id: &alsactl::ElemId, elem_value: &mut alsactl::ElemValue)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            OUT_SRC_NAME => {
                elem_value.set_enum(&self.out);
                Ok(true)
            }
            Self::CAP_SRC_NAME => {
                elem_value.set_enum(&self.cap);
                Ok(true)
            }
            HP_SRC_NAME => {
                elem_value.set_enum(&self.hp);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn write(&mut self, avc: &BebobAvc, elem_id: &alsactl::ElemId,
                 old: &alsactl::ElemValue, new: &alsactl::ElemValue, timeout_ms: u32)
        -> Result<bool, Error>
    {
        match elem_id.get_name().as_str() {
            OUT_SRC_NAME => {
                ElemValueAccessor::<u32>::get_vals(new, old, Self::OUT_LABELS.len(), |idx, val| {
                    let dst = Self::OUT_LABELS[idx];
                    let src = Self::OUT_SRC_LABELS[val as usize];
                    self.update_route(avc, dst, src, timeout_ms)?;
                    self.out[idx] = val;
                    Ok(())
                })?;
                Ok(true)
            }
            Self::CAP_SRC_NAME => {
                ElemValueAccessor::<u32>::get_vals(new, old, Self::CAP_LABELS.len(), |idx, val| {
                    let dst = Self::CAP_LABELS[idx];
                    let src = Self::CAP_SRC_LABELS[val as usize];
                    self.update_route(avc, dst, src, timeout_ms)?;
                    self.cap[idx] = val;
                    Ok(())
                })?;
                Ok(true)
            }
            HP_SRC_NAME => {
                ElemValueAccessor::<u32>::get_vals(new, old, Self::HP_LABELS.len(), |idx, val| {
                    self.update_hp_source(avc, idx, val as usize, timeout_ms)?;
                    Ok(())
                })?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
