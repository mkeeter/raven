use chrono::{Datelike, Timelike};
use std::mem::offset_of;
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct DatetimePorts {
    year: U16<BigEndian>,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    day_of_week: u8,
    day_of_year: U16<BigEndian>,
    is_dst: u8,
    _pad: [u8; 5],
}

impl Ports for DatetimePorts {
    const BASE: u8 = 0xc0;
}

impl DatetimePorts {
    const YEAR: u8 = Self::BASE | offset_of!(Self, year) as u8;
    const MONTH: u8 = Self::BASE | offset_of!(Self, month) as u8;
    const DAY: u8 = Self::BASE | offset_of!(Self, day) as u8;
    const HOUR: u8 = Self::BASE | offset_of!(Self, hour) as u8;
    const MINUTE: u8 = Self::BASE | offset_of!(Self, minute) as u8;
    const SECOND: u8 = Self::BASE | offset_of!(Self, second) as u8;
    const DAY_OF_WEEK: u8 = Self::BASE | offset_of!(Self, day_of_week) as u8;
    const DAY_OF_YEAR: u8 = Self::BASE | offset_of!(Self, day_of_year) as u8;
    const IS_DST: u8 = Self::BASE | offset_of!(Self, is_dst) as u8;
}

pub struct Datetime;

impl Datetime {
    pub fn deo(&mut self, _vm: &mut Uxn, _target: u8) {
        // Time in Varvara, just like in real live, cannot be changed
    }
    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        let d = vm.dev_mut::<DatetimePorts>();
        let t = chrono::Local::now();
        match target {
            DatetimePorts::YEAR => d.year.set(t.year().try_into().unwrap()),
            DatetimePorts::MONTH => d.month = t.month().try_into().unwrap(),
            DatetimePorts::DAY => d.day = t.day().try_into().unwrap(),
            DatetimePorts::HOUR => d.hour = t.hour().try_into().unwrap(),
            DatetimePorts::MINUTE => d.minute = t.minute().try_into().unwrap(),
            DatetimePorts::SECOND => d.second = t.second().try_into().unwrap(),
            DatetimePorts::DAY_OF_WEEK => {
                d.day_of_week =
                    t.weekday().num_days_from_sunday().try_into().unwrap()
            }
            DatetimePorts::DAY_OF_YEAR => {
                d.day_of_year.set(t.ordinal().try_into().unwrap())
            }
            DatetimePorts::IS_DST => {
                // https://github.com/chronotope/chrono/issues/1562
                d.is_dst = 0u8; // TODO this is not correct
            }

            _ => (),
        }
    }
}
