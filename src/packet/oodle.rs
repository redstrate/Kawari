// These are triggered in non-Oodle builds
#![allow(non_snake_case)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::{ffi::c_void, ptr::null};

#[cfg(feature = "oodle")]
#[cfg_attr(
    target_os = "windows",
    link(name = "oodle-network-shared", kind = "raw-dylib")
)]
#[cfg_attr(target_os = "linux", link(name = "oodle-network-shared"))]
#[cfg_attr(target_os = "macos", link(name = "oo2netmac64"))]
unsafe extern "C" {
    pub fn OodleNetwork1TCP_State_Size() -> isize;
    pub fn OodleNetwork1_Shared_Size(htbits: i32) -> isize;
    pub fn OodleNetwork1_Shared_SetWindow(
        shared: *mut c_void,
        htbits: i32,
        window: *const c_void,
        window_size: i32,
    ) -> c_void;
    pub fn OodleNetwork1TCP_Train(
        state: *mut c_void,
        shared: *const c_void,
        training_packet_pointers: *const c_void,
        training_packet_sizes: i32,
        num_training_packets: i32,
    ) -> c_void;
    pub fn OodleNetwork1TCP_Decode(
        state: *mut c_void,
        shared: *const c_void,
        enc: *const c_void,
        enc_size: isize,
        dec: *mut c_void,
        dec_size: isize,
    ) -> bool;
    pub fn OodleNetwork1TCP_Encode(
        state: *mut c_void,
        shared: *const c_void,
        dec: *const c_void,
        dec_size: isize,
        enc: *mut c_void,
    ) -> bool;
    pub fn OodleNetwork1_CompressedBufferSizeNeeded(rawLen: isize) -> isize;
}

// dummy functions for CI mostly
#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1TCP_State_Size() -> isize {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1_Shared_Size(htbits: i32) -> isize {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1_Shared_SetWindow(
    shared: *mut c_void,
    htbits: i32,
    window: *const c_void,
    window_size: i32,
) -> c_void {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1TCP_Train(
    state: *mut c_void,
    shared: *const c_void,
    training_packet_pointers: *const c_void,
    training_packet_sizes: i32,
    num_training_packets: i32,
) -> c_void {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1TCP_Decode(
    state: *mut c_void,
    shared: *const c_void,
    enc: *const c_void,
    enc_size: isize,
    dec: *mut c_void,
    dec_size: isize,
) -> bool {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1TCP_Encode(
    state: *mut c_void,
    shared: *const c_void,
    dec: *const c_void,
    dec_size: isize,
    enc: *mut c_void,
) -> bool {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[cfg(not(feature = "oodle"))]
pub fn OodleNetwork1_CompressedBufferSizeNeeded(rawLen: isize) -> isize {
    panic!("Something is trying to use Oodle but the feature isn't enabled!")
}

#[derive(Debug, Default)]
pub struct OodleNetwork {
    state: Vec<u8>,
    shared: Vec<u8>,
    window: Vec<u8>,
}

const HT_BITS: i32 = 0x11;
const WINDOW_SIZE: usize = 0x100000;
const OODLENETWORK1_DECOMP_BUF_OVERREAD_LEN: usize = 5;

impl OodleNetwork {
    pub fn new() -> OodleNetwork {
        unsafe {
            let oodle_state_size: usize = OodleNetwork1TCP_State_Size().try_into().unwrap();
            let oodle_shared_size: usize = OodleNetwork1_Shared_Size(HT_BITS).try_into().unwrap();
            let mut oodle_state = vec![0u8; oodle_state_size];
            let mut oodle_shared = vec![0u8; oodle_shared_size];
            let mut oodle_window = [0u8; WINDOW_SIZE].to_vec();

            OodleNetwork1_Shared_SetWindow(
                oodle_shared.as_mut_ptr() as *mut c_void,
                HT_BITS,
                oodle_window.as_mut_ptr() as *mut c_void,
                WINDOW_SIZE.try_into().unwrap(),
            );
            OodleNetwork1TCP_Train(
                oodle_state.as_mut_ptr() as *mut c_void,
                oodle_shared.as_mut_ptr() as *mut c_void,
                null(),
                0,
                0,
            );

            OodleNetwork {
                state: oodle_state,
                shared: oodle_shared,
                window: oodle_window,
            }
        }
    }

    pub fn decode(&mut self, input: Vec<u8>, decompressed_size: u32) -> Vec<u8> {
        unsafe {
            let mut padded_buffer = input.clone();
            padded_buffer.resize(
                padded_buffer.len() + OODLENETWORK1_DECOMP_BUF_OVERREAD_LEN,
                0,
            );

            let mut out_buf: Vec<u8> = vec![0u8; decompressed_size.try_into().unwrap()];
            let success = OodleNetwork1TCP_Decode(
                self.state.as_mut_ptr() as *mut c_void,      // state
                self.shared.as_ptr() as *const c_void,       // shared
                padded_buffer.as_mut_ptr() as *const c_void, // comp
                input.len().try_into().unwrap(),             // compLen
                out_buf.as_mut_ptr() as *mut c_void,         // raw
                decompressed_size.try_into().unwrap(),       // rawLen
            );

            if !success {
                panic!("Failed to oodle decode for an unknown reason.");
            }

            out_buf
        }
    }

    pub fn encode(&mut self, mut input: Vec<u8>) -> Vec<u8> {
        unsafe {
            let output_size = OodleNetwork1_CompressedBufferSizeNeeded(input.len() as isize);
            let mut out_buf: Vec<u8> = vec![0u8; output_size as usize];

            let len = OodleNetwork1TCP_Encode(
                self.state.as_mut_ptr() as *mut c_void, // state
                self.shared.as_ptr() as *const c_void,  // shared
                input.as_mut_ptr() as *const c_void,    // raw
                input.len().try_into().unwrap(),        // rawLen
                out_buf.as_mut_ptr() as *mut c_void,    // comp
            );

            out_buf.truncate(len as usize);
            out_buf
        }
    }
}
