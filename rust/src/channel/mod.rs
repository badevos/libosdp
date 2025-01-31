pub mod thread_bus;
pub mod unix_channel;

use crate::libosdp;
use std::{
    collections::hash_map::DefaultHasher,
    ffi::c_void,
    hash::{Hash, Hasher},
    io::{Read, Write},
    sync::Mutex,
};

pub trait Channel: Read + Write {
    fn get_id(&self) -> i32;
}

pub struct OsdpChannel {
    stream: Mutex<Box<dyn Channel>>,
}

unsafe extern "C" fn raw_read(data: *mut c_void, buf: *mut u8, len: i32) -> i32 {
    let channel = &mut *(data as *mut OsdpChannel);
    let mut read_buf = vec![0u8; len as usize];
    let mut stream = channel.stream.lock().unwrap();
    match stream.read(&mut read_buf) {
        Ok(n) => {
            let src_ptr = read_buf.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, buf, len as usize);
            n as i32
        }
        Err(_) => -1,
    }
}

unsafe extern "C" fn raw_write(data: *mut c_void, buf: *mut u8, len: i32) -> i32 {
    let channel = &mut *(data as *mut OsdpChannel);
    let mut write_buf = vec![0u8; len as usize];
    std::ptr::copy_nonoverlapping(buf, write_buf.as_mut_ptr(), len as usize);
    let mut stream = channel.stream.lock().unwrap();
    match stream.write(&write_buf) {
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}

unsafe extern "C" fn raw_flush(data: *mut c_void) {
    let channel = &mut *(data as *mut OsdpChannel);
    let mut stream = channel.stream.lock().unwrap();
    let _ = stream.flush();
}

impl OsdpChannel {
    pub fn new<T: Channel>(stream: Box<dyn Channel>) -> OsdpChannel {
        Self {
            stream: Mutex::new(stream),
        }
    }

    pub fn as_struct(&mut self) -> libosdp::osdp_channel {
        let id = self.stream.lock().unwrap().get_id();
        libosdp::osdp_channel {
            id,
            data: self as *mut _ as *mut c_void,
            recv: Some(raw_read),
            send: Some(raw_write),
            flush: Some(raw_flush),
        }
    }
}

pub fn str_to_channel_id(key: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let mut id = hasher.finish();
    id = (id >> 32) ^ id & 0xffffffff;
    id as i32
}
