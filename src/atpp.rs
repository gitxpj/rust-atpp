use std::io::Write;
use std::io::Cursor;

use self::AtppError::*;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug)]
pub struct AtppStartPackage {
    pub timestamp: i64,
    pub token: String,
    pub total_size: i64,
    pub slice_count: i32,
    pub slice_size: i32
}

impl AtppStartPackage {
    pub fn new(timestamp: i64, token: String, total_size: i64, slice_count: i32, slice_size: i32) -> AtppStartPackage {
        AtppStartPackage {
            timestamp: timestamp,
            token: token,
            total_size: total_size,
            slice_count: slice_count,
            slice_size: slice_size
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        // append magic head
        data.write(String::from("ATPP").as_bytes());
        data.write(&[1]);

        let mut timestamp = vec![];
        timestamp.write_i64::<BigEndian>(self.timestamp).unwrap();
        data.write(&*timestamp);
        data.write(self.token.as_bytes());

        let mut total_size = vec![];
        total_size.write_i64::<BigEndian>(self.total_size).unwrap();
        data.write(&*total_size);

        let mut slice_count = vec![];
        slice_count.write_i32::<BigEndian>(self.slice_count).unwrap();
        data.write(&*slice_count);

        let mut slice_size = vec![];
        slice_size.write_i32::<BigEndian>(self.slice_size).unwrap();
        data.write(&*slice_size);

        data
    }
}

#[derive(Debug)]
pub struct AtppDataPackage {
    pub timestamp: i64,
    pub token: String,
    pub slice_index: i32,
    pub slice_size: i32,
}

impl AtppDataPackage {
    pub fn new(timestamp: i64, token: String, slice_index: i32, slice_size: i32) -> AtppDataPackage {
        AtppDataPackage {
            timestamp: timestamp,
            token: token,
            slice_index: slice_index,
            slice_size: slice_size
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        // append magic head
        data.write(String::from("ATPP").as_bytes());
        data.write(&[2]);

        let mut timestamp = vec![];
        timestamp.write_i64::<BigEndian>(self.timestamp).unwrap();
        data.write(&*timestamp);

        data.write(self.token.as_bytes());

        let mut slice_index = vec![];
        slice_index.write_i32::<BigEndian>(self.slice_index).unwrap();
        data.write(&*slice_index);

        let mut slice_size = vec![];
        slice_size.write_i32::<BigEndian>(self.slice_size).unwrap();
        data.write(&*slice_size);

        data
    }
}

#[derive(Debug)]
pub struct AtppEndPackage {
    pub timestamp: i64,
    pub token: String,
}

impl AtppEndPackage {
    pub fn new(timestamp: i64, token: String) -> AtppEndPackage {
        AtppEndPackage {
            timestamp: timestamp,
            token: token
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        // append magic head
        data.write(String::from("ATPP").as_bytes());
        data.write(&[3]);

        let mut timestamp = vec![];
        timestamp.write_i64::<BigEndian>(self.timestamp).unwrap();
        data.write(&*timestamp);

        data.write(self.token.as_bytes());
        data
    }
}

pub trait AtppHandle<T> {
    fn OnStart(&self, stream:&mut T, pkg: AtppStartPackage);
    fn OnData(&self, stream:&mut T, pkg: AtppDataPackage, &mut Vec<u8>);
    fn OnEnd(&self, stream:&mut T, pkg: AtppEndPackage);
}

pub enum AtppError<'a> {
    BROKE_DATA(&'a str),
    NOT_ENOUGH_DATA,
}

pub struct AtppAdapter<'a, T> {
    pub handle: &'a AtppHandle<T>,
}

impl<'a, T> AtppAdapter<'a, T> {
    pub fn new(handle: &'a AtppHandle<T>) -> AtppAdapter<T> {
        AtppAdapter {
            handle: handle,
        }
    }

    pub fn unpack(&self, stream: &mut T, data: &mut Vec<u8>) -> Result<Option<Vec<u8>>, AtppError<'a>> {
        let mut buf: Vec<u8> = Vec::new();
        let mut last_buf: Vec<u8> = Vec::new();
        buf.write(&*data);

        let size: usize = buf.len();

        let mut offset = 0;

        loop {
            if buf[offset..].len() >= 45 {
                let head: String = match String::from_utf8(buf[offset..offset + 4].to_vec()) {
                    Ok(e) => e,
                    Err(e) => {
                       return Err(AtppError::BROKE_DATA("decode head error!"))
                    }
                };
                offset += 4;
                if head == "ATPP" {
                    let atpp_type: u8 = buf[offset];
                    offset += 1;
                    match atpp_type {
                        1 => {
                            match self.unpack_start_package(&buf[offset..], &mut offset) {
                                Ok(e) => {
                                    self.handle.OnStart(stream, e);
                                },
                                Err(e) => {
                                    match e {
                                        BROKE_DATA(e) => return Err(BROKE_DATA(e)),
                                        NOT_ENOUGH_DATA => {
                                            last_buf.write(&buf[offset - 5..]);
                                            return Ok(Some(last_buf))
                                        },
                                    }
                                }
                            }
                        },
                        2 => {
                            match self.unpack_data_package(&buf[offset..], &mut offset) {
                                Ok(e) => {
                                    if size - offset > e.slice_size as usize {
                                        let mut data = Vec::new();
                                        data.write(&buf[offset..offset + e.slice_size as usize]);
                                        offset += e.slice_size as usize;
                                        self.handle.OnData(stream, e, &mut data);
                                    } else {
                                        last_buf.write(&buf[offset - 53..]);
                                        return Ok(Some(last_buf))
                                    }
                                },
                                Err(e) => {
                                    match e {
                                        BROKE_DATA(e) => return Err(BROKE_DATA(e)),
                                        NOT_ENOUGH_DATA => {
                                            last_buf.write(&buf[offset - 5..]);
                                            return Ok(Some(last_buf))
                                        },
                                    }
                                }
                            }
                        },
                        3 => {
                            match self.unpack_end_package(&buf[offset..], &mut offset) {
                                Ok(e) => {
                                    self.handle.OnEnd(stream, e);
                                },
                                Err(e) => {
                                    match e {
                                        BROKE_DATA(e) => return Err(BROKE_DATA(e)),
                                        NOT_ENOUGH_DATA => {
                                            last_buf.write(&buf[offset - 5..]);
                                            return Ok(Some(last_buf))
                                        },
                                    }
                                }
                            }
                        },
                        _ => {
                            return Err(AtppError::BROKE_DATA("type has error!"))
                        }
                    }
                } else {
                    return Err(AtppError::BROKE_DATA("head wrong!"))
                }
            } else {
                // concat package
                last_buf.write(&buf[offset..]);
                return Ok(Some(last_buf))
            }
        }

        return Ok(None)
    }

    fn unpack_start_package(&self, data: &[u8], offset: &mut usize) -> Result<AtppStartPackage, AtppError<'a>> {
        let mut oft: usize = 0;

        let mut timestamp: i64 = Default::default();
        let mut uuid: String = Default::default();
        let mut total_size: i64 = Default::default();
        let mut slice_count: i32 = Default::default();
        let mut slice_size: i32 = Default::default();

        if oft + 8 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 8].to_vec());
            timestamp = match rdr.read_i64::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `timestamp` wrong!"))
            };
            oft += 8;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 32 <= data.len() {
            uuid = match String::from_utf8(data[oft..oft + 32].to_vec()) {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `token` wrong!"))
            };

            oft += 32;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 8 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 8].to_vec());
            total_size = match rdr.read_i64::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `total_size` wrong!"))
            };

            oft += 8;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 4 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 4].to_vec());
            slice_count = match rdr.read_i32::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `slice_count` wrong!"))
            };

            oft += 4;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 4 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 4].to_vec());
            slice_size = match rdr.read_i32::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `slice_size` wrong!"))
            };

            oft += 4;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        *offset += oft;
        Ok(AtppStartPackage {
            timestamp: timestamp,
            token: uuid,
            total_size: total_size,
            slice_count: slice_count,
            slice_size: slice_size,
        })
    }

    fn unpack_data_package(&self, data: &[u8], offset: &mut usize) -> Result<AtppDataPackage, AtppError<'a>> {

        let mut oft: usize = 0;

        let mut timestamp: i64 = Default::default();
        let mut uuid: String = Default::default();
        let mut slice_index: i32 = Default::default();
        let mut slice_size: i32 = Default::default();

        if oft + 8 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 8].to_vec());
            timestamp = match rdr.read_i64::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `timestamp` wrong!"))
            };

            oft += 8;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 32 <= data.len() {
            uuid = match String::from_utf8(data[oft..oft + 32].to_vec()) {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `token` wrong!"))
            };

            oft += 32;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 4 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 4].to_vec());
            slice_index = match rdr.read_i32::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `slice_index` wrong!"))
            };

            oft += 4;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 4 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 4].to_vec());
            slice_size = match rdr.read_i32::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `slice_size` wrong!"))
            };

            oft += 4;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        *offset += oft;

        Ok(AtppDataPackage {
            timestamp: timestamp,
            token: uuid,
            slice_index: slice_index,
            slice_size: slice_size,
        })
    }

    fn unpack_end_package(&self, data: &[u8], offset: &mut usize) -> Result<AtppEndPackage, AtppError<'a>> {
        let mut oft: usize = 0;
        let mut timestamp: i64 = Default::default();
        let mut uuid: String = Default::default();

        if oft + 8 <= data.len() {
            let mut rdr = Cursor::new(data[oft..oft + 8].to_vec());
            timestamp = match rdr.read_i64::<BigEndian>() {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `timestamp` wrong!"))
            };

            oft += 8;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        if oft + 32 <= data.len() {
            uuid = match String::from_utf8(data[oft..oft + 32].to_vec()) {
                Ok(e) => e,
                Err(_) => return Err(AtppError::BROKE_DATA("unpack `token` wrong!"))
            };

            oft += 32;
        } else {
            return Err(AtppError::NOT_ENOUGH_DATA)
        }

        *offset += oft;
        Ok(AtppEndPackage {
            token: uuid,
            timestamp: timestamp
        })
    }

}


