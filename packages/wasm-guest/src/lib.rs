pub mod checkpoint;
pub mod guest;

extern "C" {
    fn host_func(param: i32);
}
