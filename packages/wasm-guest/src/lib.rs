// MacOS and Windows seem to disallow unresolved symbols
#![cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod checkpoint;
pub mod guest;

extern "C" {
    fn __enhedron_log(data: u32, len: u32);
}

fn log(s: impl AsRef<str>) {
    let s = s.as_ref();
    unsafe {
        __enhedron_log(
            (s.as_ptr() as usize).try_into().unwrap(),
            s.len().try_into().unwrap(),
        )
    };
}
