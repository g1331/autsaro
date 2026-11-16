//! Failure-only, handle-free SYSTEM_PROCESS_INFORMATION observation (winternl.h).
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Process {
    next: u32,
    threads: u32,
    reserved1: [u8; 24],
    created: i64,
    times: [i64; 2],
    name_length: u16,
    name_maximum: u16,
    name: *const u16,
    priority: i32,
    pid: usize,
    parent: usize,
    handles: u32,
    session: u32,
    reserved3: usize,
    sizes: [usize; 12],
    reserved7: [i64; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Thread {
    times: [i64; 3],
    wait_time: u32,
    start: usize,
    pid: usize,
    tid: usize,
    priority: i32,
    base_priority: i32,
    switches: u32,
    state: u32,
    reason: u32,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(class: i32, data: *mut c_void, size: u32, used: *mut u32) -> i32;
}

pub(super) fn capture() -> Result<BTreeMap<u32, Value>, String> {
    // A single bounded query: diagnostic failure must not delay cleanup by resizing.
    let mut buffer = vec![0u64; 2 * 1024 * 1024];
    let size = buffer.len() * size_of::<u64>();
    let mut used = 0;
    // SAFETY: the buffer is writable and 8-byte aligned, with the advertised capacity.
    let status =
        unsafe { NtQuerySystemInformation(5, buffer.as_mut_ptr().cast(), size as u32, &mut used) };
    if status < 0 {
        return Err(format!(
            "NtQuerySystemInformation status=0x{:08x}",
            status as u32
        ));
    }
    let bytes = buffer.as_ptr().cast::<u8>();
    let used = used as usize;
    if used > size {
        return Err("Truncated SYSTEM_PROCESS_INFORMATION".into());
    }
    let mut result = BTreeMap::new();
    let mut offset = 0;
    while offset + size_of::<Process>() <= used {
        // SAFETY: the fixed header lies within the returned buffer. Read unaligned
        // rather than relying on the variable-length entry's alignment.
        let process = unsafe { bytes.add(offset).cast::<Process>().read_unaligned() };
        let end = if process.next == 0 {
            used
        } else {
            offset + process.next as usize
        };
        let thread_offset = offset + size_of::<Process>();
        if end > used || thread_offset + process.threads as usize * size_of::<Thread>() > end {
            return Err("Truncated SYSTEM_PROCESS_INFORMATION".into());
        }
        let name_start = process.name as usize;
        let name_end = name_start.saturating_add(process.name_length as usize);
        let image = if process.name_length == 0 {
            None
        } else if name_start >= bytes as usize
            && name_end <= bytes as usize + used
            && name_start.is_multiple_of(2)
            && process.name_length.is_multiple_of(2)
        {
            // SAFETY: the UTF-16 range and alignment were checked against the live buffer.
            Some(String::from_utf16_lossy(unsafe {
                std::slice::from_raw_parts(process.name, process.name_length as usize / 2)
            }))
        } else {
            return Err("Invalid SYSTEM_PROCESS_INFORMATION image range".into());
        };
        let threads: Vec<_> = (0..process.threads as usize)
            .map(|index| {
                // SAFETY: the complete thread array was bounded by this entry above.
                let thread = unsafe {
                    bytes
                        .add(thread_offset + index * size_of::<Thread>())
                        .cast::<Thread>()
                        .read_unaligned()
                };
                json!({"tid": thread.tid, "created_100ns": thread.times[2],
                "state": thread.state, "wait_reason": (thread.state == 5).then_some(thread.reason)})
            })
            .collect();
        result.insert(
            process.pid as u32,
            json!({"created_100ns": process.created,
            "parent_pid": process.parent, "image": image, "threads": threads}),
        );
        if process.next == 0 {
            break;
        }
        offset = end;
    }
    Ok(result)
}
