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
