use std::{ffi::c_void, ptr::null};

#[cfg(feature = "oodle")]
#[link(name = "oodle-network-shared")]
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

#[derive(Debug, Default)]
pub struct OodleNetwork {
    state: Vec<u8>,
    shared: Vec<u8>,
    #[allow(dead_code)] // unused in rust but required to still be available for low-level oodle
    window: Vec<u8>,
}

impl OodleNetwork {
    pub fn new() -> OodleNetwork {
        let htbits: i32 = 17;
        unsafe {
            let oodle_state_size: usize = OodleNetwork1TCP_State_Size().try_into().unwrap();
            let oodle_shared_size: usize = OodleNetwork1_Shared_Size(17).try_into().unwrap();
            let mut oodle_state = vec![0u8; oodle_state_size];
            let mut oodle_shared = vec![0u8; oodle_shared_size];
            let mut oodle_window = [0u8; 0x100000].to_vec();

            OodleNetwork1_Shared_SetWindow(
                oodle_shared.as_mut_ptr() as *mut c_void,
                htbits,
                oodle_window.as_mut_ptr() as *mut c_void,
                oodle_window.len().try_into().unwrap(),
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
            let mut out_buf: Vec<u8> = vec![0u8; decompressed_size.try_into().unwrap()];
            let mut in_buf = input.to_vec();
            let success = OodleNetwork1TCP_Decode(
                self.state.as_mut_ptr() as *mut c_void,
                self.shared.as_mut_ptr() as *mut c_void,
                in_buf.as_mut_ptr() as *const c_void,
                in_buf.len().try_into().unwrap(),
                out_buf.as_mut_ptr() as *mut c_void,
                out_buf.len().try_into().unwrap(),
            );

            if !success {
                panic!("Failed to oodle decode for an unknown reason.");
            }

            out_buf
        }
    }

    pub fn encode(&mut self, input: Vec<u8>) -> Vec<u8> {
        unsafe {
            let mut out_buf: Vec<u8> = vec![0u8; input.len()];
            let mut in_buf = input.to_vec();
            let len = OodleNetwork1TCP_Encode(
                self.state.as_mut_ptr() as *mut c_void,
                self.shared.as_mut_ptr() as *mut c_void,
                in_buf.as_mut_ptr() as *const c_void,
                in_buf.len().try_into().unwrap(),
                out_buf.as_mut_ptr() as *mut c_void,
            );

            out_buf.truncate(len as usize);
            out_buf
        }
    }
}
