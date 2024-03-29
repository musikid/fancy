/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use log::debug;

use std::io::{Error, Seek, SeekFrom, Write};

use super::RcWrapper;
use crate::nbfc::*;

#[derive(Debug)]
/// Contains information about writing to the EC for a fan.
struct FanWriteConfig {
    write_register: u8,
    reset_required: bool,
    reset_value: Option<u16>,
    max_speed: u16,
    min_speed: u16,
    write_percent_overrides: Option<Vec<FanSpeedPercentageOverride>>,
}

#[derive(Debug)]
/// Manages writes to the EC.
pub(crate) struct ECWriter<W: Write + Seek> {
    on_write_reg_confs: Option<Vec<RegisterWriteConfiguration>>,
    init_reg_confs: Option<Vec<RegisterWriteConfiguration>>,
    fans_write_config: Vec<FanWriteConfig>,
    write_words: bool,
    ec_dev: RcWrapper<W>,
}

type Result<T = ()> = std::result::Result<T, Error>;

impl<W: Write + Seek> ECWriter<W> {
    /// Initialize a new writer.
    pub fn new(ec_dev: RcWrapper<W>) -> Self {
        ECWriter {
            on_write_reg_confs: None,
            init_reg_confs: None,
            fans_write_config: Vec::new(),
            write_words: false,
            ec_dev,
        }
    }

    /// Refresh the configuration used for the writer.
    /// NOTE: This function does write the required values to initialize the controller (using `init_write`).
    pub fn refresh_config(
        &mut self,
        write_words: bool,
        reg_confs: Option<Vec<RegisterWriteConfiguration>>,
        fan_configs: &[FanConfiguration],
    ) -> Result {
        self.on_write_reg_confs = reg_confs.as_ref().map(|e| {
            e.iter()
                .filter(|r| r.write_occasion == Some(RegisterWriteOccasion::OnWriteFanSpeed))
                .cloned()
                .collect()
        });

        self.init_reg_confs = reg_confs.as_ref().map(|e| {
            e.iter()
                .filter(|r| r.write_occasion == Some(RegisterWriteOccasion::OnInitialization))
                .cloned()
                .collect()
        });

        self.write_words = write_words;

        self.fans_write_config = fan_configs
            .iter()
            .map(|fan| FanWriteConfig {
                write_register: fan.write_register,
                reset_required: fan.reset_required,
                reset_value: fan.fan_speed_reset_value,
                min_speed: fan.min_speed_value,
                max_speed: fan.max_speed_value,
                write_percent_overrides: fan.fan_speed_percentage_overrides.as_ref().map(|f| {
                    f.iter()
                        .filter(|e| {
                            e.target_operation == Some(OverrideTargetOperation::Write)
                                || e.target_operation == Some(OverrideTargetOperation::ReadWrite)
                        })
                        .cloned()
                        .collect()
                }),
            })
            .collect();

        self.init_write()
    }

    /// Function to call before starting to write. It initialize the EC controller so it can be used.
    fn init_write(&mut self) -> Result {
        if let Some(reg_confs) = &self.init_reg_confs {
            for reg_conf in reg_confs.iter() {
                let write_off = SeekFrom::Start(reg_conf.register as u64);
                self.write_value(false, write_off, &reg_conf.value.to_le_bytes())?
            }
        }

        for c in &self.fans_write_config {
            if let Some(value) = c.reset_value {
                let write_off = SeekFrom::Start(c.write_register as u64);
                self.write_value(self.write_words, write_off, &value.to_le_bytes())?;
            }
        }

        Ok(())
    }

    /// Reset the EC. Resets all the registers (even when it's not required) if `reset_all` is true.
    pub fn reset(&mut self, reset_all: bool) -> Result {
        if let Some(reg_confs) = &self.init_reg_confs {
            for reg_conf in reg_confs.iter() {
                if reset_all || reg_conf.reset_required {
                    let write_off = SeekFrom::Start(reg_conf.register as u64);
                    if let Some(value) = reg_conf.reset_value {
                        self.write_value(false, write_off, &value.to_le_bytes())?;
                    }
                }
            }
        }

        if let Some(reg_confs) = &self.on_write_reg_confs {
            for reg_conf in reg_confs.iter() {
                if reset_all || reg_conf.reset_required {
                    let write_off = SeekFrom::Start(reg_conf.register as u64);
                    if let Some(value) = reg_conf.reset_value {
                        self.write_value(false, write_off, &value.to_le_bytes())?;
                    }
                }
            }
        }

        for c in &self.fans_write_config {
            if reset_all || c.reset_required {
                if let Some(value) = c.reset_value {
                    let write_off = SeekFrom::Start(c.write_register as u64);
                    self.write_value(self.write_words, write_off, &value.to_le_bytes())?;
                }
            }
        }

        Ok(())
    }

    /// Write the `speed_percent` to the EC for the fan specified by `fan_index`.
    pub fn write_speed_percent(&mut self, fan_index: usize, speed_percent: f64) -> Result {
        if let Some(reg_confs) = &self.on_write_reg_confs {
            for reg_conf in reg_confs.iter() {
                let write_off = SeekFrom::Start(reg_conf.register as u64);
                self.write_value(false, write_off, &reg_conf.value.to_le_bytes())?;
            }
        }

        let fan = &self.fans_write_config[fan_index];
        let speed = if let Some(speed_value) = fan.write_percent_overrides.as_ref().and_then(|f| {
            f.iter()
                .filter(|e| (e.fan_speed_percentage as f64 - speed_percent).abs() < f64::EPSILON)
                .map(|e| e.fan_speed_value)
                .next()
        }) {
            speed_value.to_le_bytes()
        } else {
            ((fan.min_speed as f64
                + (((fan.max_speed as f64 - fan.min_speed as f64) * speed_percent) / 100.0))
                .round() as u16)
                .to_le_bytes()
        };

        let write_off = SeekFrom::Start(fan.write_register as u64);
        self.write_value(self.write_words, write_off, &speed)
    }

    /// Low-level write function.
    fn write_value(&self, write_word: bool, write_off: SeekFrom, value: &[u8]) -> Result {
        debug!(
            "Writing {:?} to offset {:?}",
            if write_word { &value } else { &value[..=0] },
            write_off
        );

        let mut dev = (*self.ec_dev).borrow_mut();

        dev.seek(write_off)?;
        dev.write_all(if write_word { &value } else { &value[..=0] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::cell::RefCell;
    use std::io::{Cursor, Read};
    use std::rc::Rc;

    static CONFIGS_PARSED: Lazy<Vec<FanControlConfigV2>> = Lazy::new(|| {
        std::fs::read_dir("nbfc_configs/Configs")
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| std::fs::File::open(e.path()).unwrap())
            .map(|mut e| {
                let mut buf = String::new();
                e.read_to_string(&mut buf).unwrap();
                buf
            })
            .map(|e| {
                quick_xml::de::from_str::<XmlFanControlConfigV2>(&e)
                    .unwrap()
                    .into()
            })
            .collect()
    });

    #[test]
    fn refresh() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            if let Some(ref on_w_confs) = writer.on_write_reg_confs {
                assert_eq!(on_w_confs.len(), c.register_write_configurations.as_ref().unwrap().iter().filter(|c| c.write_occasion == Some(RegisterWriteOccasion::OnWriteFanSpeed)).count());
            }

            if let Some(ref on_i_confs) = writer.init_reg_confs {
                assert_eq!(
                    on_i_confs.len(),
                    c.register_write_configurations
                        .as_ref()
                        .unwrap()
                        .iter()
                        .filter(
                            |c| c.write_occasion == Some(RegisterWriteOccasion::OnInitialization)
                        )
                        .count()
                );
            }

            assert_eq!(writer.fans_write_config.len(), c.fan_configurations.len());

            let mut i = 0;
            writer.fans_write_config.iter().for_each(|f| {
                assert_eq!(f.reset_required, c.fan_configurations[i].reset_required);
                assert_eq!(f.write_register, c.fan_configurations[i].write_register);
                assert_eq!(f.reset_value, c.fan_configurations[i].fan_speed_reset_value);
                assert_eq!(f.min_speed, c.fan_configurations[i].min_speed_value);
                assert_eq!(f.max_speed, c.fan_configurations[i].max_speed_value);

                if let Some(ref overrides) = f.write_percent_overrides {
                    let excepted_overrides = c.fan_configurations[i]
                        .fan_speed_percentage_overrides
                        .as_ref()
                        .unwrap();
                    assert_eq!(
                        overrides.len(),
                        excepted_overrides
                            .iter()
                            .filter(|o| o.target_operation
                                == Some(OverrideTargetOperation::ReadWrite)
                                || o.target_operation == Some(OverrideTargetOperation::Write))
                            .count()
                    );

                    overrides.iter().for_each(|o| {
                        assert!(excepted_overrides.iter().any(|e| e == o));
                    });
                }
                i += 1;
            });

            assert_eq!(writer.write_words, c.read_write_words);
        });
    }

    #[test]
    fn reset_only_required() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            ec.borrow_mut().get_mut().iter_mut().map(|x| *x = 0).count();
            writer.reset(false).unwrap();

            if let Some(reg_confs) = c.register_write_configurations.as_ref() {
                let ec = ec.borrow();

                for reg_conf in reg_confs.iter() {
                    let write_off = reg_conf.register as usize;
                    let excepted_value = if reg_conf.reset_required {
                        reg_conf.reset_value.unwrap()
                    } else {
                        0
                    };

                    let value = ec.get_ref()[write_off];
                    assert_eq!(excepted_value, value);
                }

                for fan in &c.fan_configurations {
                    if fan.reset_required {
                        let write_off = fan.write_register as usize;
                        let excepted_value = &fan.fan_speed_reset_value.unwrap().to_le_bytes();

                        let value = if c.read_write_words {
                            &ec.get_ref()[write_off..=write_off + 1]
                        } else {
                            &ec.get_ref()[write_off..=write_off]
                        };
                        assert_eq!(
                            if c.read_write_words {
                                &excepted_value[..]
                            } else {
                                &excepted_value[..=0]
                            },
                            value
                        );
                    }
                }
            }
        });
    }

    #[test]
    fn reset_all() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            ec.borrow_mut().get_mut().iter_mut().map(|x| *x = 0).count();
            writer.reset(true).unwrap();

            if let Some(reg_confs) = c.register_write_configurations.as_ref() {
                let ec = ec.borrow();

                for reg_conf in reg_confs.iter().filter(|e| e.reset_value.is_some()) {
                    let write_off = reg_conf.register as usize;
                    let excepted_value = reg_conf.reset_value.unwrap();

                    let value = ec.get_ref()[write_off];
                    assert_eq!(excepted_value, value);
                }

                for fan in &c.fan_configurations {
                    if fan.reset_required {
                        let write_off = fan.write_register as usize;
                        let excepted_value = &fan.fan_speed_reset_value.unwrap().to_le_bytes();

                        let value = if c.read_write_words {
                            &ec.get_ref()[write_off..=write_off + 1]
                        } else {
                            &ec.get_ref()[write_off..=write_off]
                        };
                        assert_eq!(
                            if c.read_write_words {
                                &excepted_value[..]
                            } else {
                                &excepted_value[..=0]
                            },
                            value
                        );
                    }
                }
            }
        });
    }

    #[test]
    fn init_write() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let mut ec = Cursor::new(vec![0; 256]);
            let mut writer = ECWriter::new(Rc::new(RefCell::new(&mut ec)));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            if let Some(reg_confs) = c.register_write_configurations.as_ref() {
                for reg_conf in reg_confs
                    .iter()
                    .filter(|e| e.write_occasion == Some(RegisterWriteOccasion::OnInitialization))
                {
                    let write_off = reg_conf.register as usize;
                    let excepted_value = reg_conf.value;

                    let value = ec.get_ref()[write_off];
                    assert_eq!(excepted_value, value);
                }

                for fan in &c.fan_configurations {
                    if fan.reset_required {
                        let write_off = fan.write_register as usize;
                        let excepted_value = &fan.fan_speed_reset_value.unwrap().to_le_bytes();

                        let value = if c.read_write_words {
                            &ec.get_ref()[write_off..=write_off + 1]
                        } else {
                            &ec.get_ref()[write_off..=write_off]
                        };
                        assert_eq!(
                            if c.read_write_words {
                                &excepted_value[..]
                            } else {
                                &excepted_value[..=0]
                            },
                            value
                        );
                    }
                }
            }
        });
    }

    #[test]
    fn write_overrides() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();
            let mut i = 0;

            for fan in &c.fan_configurations {
                if let Some(fan_override) = fan.fan_speed_percentage_overrides.as_ref() {
                    for override_s in fan_override.iter().filter(|e| {
                        e.target_operation == Some(OverrideTargetOperation::ReadWrite)
                            || e.target_operation == Some(OverrideTargetOperation::Write)
                    }) {
                        writer
                            .write_speed_percent(i, override_s.fan_speed_percentage.into())
                            .unwrap();

                        let write_off = fan.write_register as u64;
                        let excepted_value = override_s.fan_speed_value;

                        let value = {
                            let mut ec = (*ec).borrow_mut();
                            if c.read_write_words {
                                let mut buf = [0; 2];
                                ec.set_position(write_off);
                                ec.read(&mut buf).unwrap();
                                u16::from_le_bytes(buf)
                            } else {
                                let mut buf = [0; 1];
                                ec.set_position(write_off);
                                ec.read(&mut buf).unwrap();
                                buf[0] as u16
                            }
                        };

                        assert_eq!(excepted_value, value);
                    }
                }

                i += 1;
            }
        });
    }

    #[test]
    fn on_write_confs() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            writer.write_speed_percent(0, 0.0).unwrap();

            if let Some(reg_confs) = c.register_write_configurations.as_ref() {
                for reg_conf in reg_confs
                    .iter()
                    .filter(|e| e.write_occasion == Some(RegisterWriteOccasion::OnWriteFanSpeed))
                {
                    let write_off = reg_conf.register as usize;
                    let excepted_value = reg_conf.value;

                    let value = (*ec).borrow_mut().get_ref()[write_off];
                    assert_eq!(excepted_value, value);
                }
            }
        });
    }

    #[test]
    fn write_good_offset() {
        CONFIGS_PARSED.iter().for_each(|c| {
            let ec = Cursor::new(vec![0; 256]);
            let ec = Rc::new(RefCell::new(ec));
            let mut writer = ECWriter::new(Rc::clone(&ec));
            writer
                .refresh_config(
                    c.read_write_words,
                    c.register_write_configurations.clone(),
                    &c.fan_configurations,
                )
                .unwrap();

            let speed_percent = 0.0;
            let mut i = 0;

            for fan in &c.fan_configurations {
                writer.write_speed_percent(i, speed_percent).unwrap();
                let write_off = fan.write_register as u64;
                let value = if c.read_write_words {
                    let mut buf = [0; 2];
                    let mut ec = (*ec).borrow_mut();
                    ec.set_position(write_off);
                    ec.read(&mut buf).unwrap();
                    u16::from_le_bytes(buf)
                } else {
                    let mut buf = [0; 1];
                    let mut ec = (*ec).borrow_mut();
                    ec.set_position(write_off);
                    ec.read(&mut buf).unwrap();
                    buf[0] as u16
                };

                let excepted_value = if let Some(v) =
                    fan.fan_speed_percentage_overrides.as_ref().and_then(|e| {
                        e.iter()
                            .filter(|e| {
                                e.target_operation == Some(OverrideTargetOperation::ReadWrite)
                                    || e.target_operation == Some(OverrideTargetOperation::Write)
                            })
                            .filter(|e| e.fan_speed_percentage as f64 == speed_percent)
                            .next()
                    }) {
                    v.fan_speed_value
                } else {
                    (fan.min_speed_value as f64
                        + (((fan.max_speed_value as f64 - fan.min_speed_value as f64)
                            * speed_percent)
                            / 100.0))
                        .round() as u16
                };

                assert_eq!(value, excepted_value);
                i += 1;
            }
        });
    }
}
