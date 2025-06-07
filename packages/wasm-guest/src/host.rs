unsafe extern "C" {
    pub fn __codastraea_log(data: u32, len: u32);
    pub fn __codastraea_register_workflow_index(
        module_data: u32,
        module_len: u32,
        name_data: u32,
        name_len: u32,
        index: u32,
    );

    pub fn __codastraea_fn_begin(module: u32, module_len: u32, name: u32, name_len: u32);
    pub fn __codastraea_fn_end(module: u32, module_len: u32, name: u32, name_len: u32);

    pub fn __codastraea_begin_if();
    pub fn __codastraea_end_if();
    pub fn __codastraea_begin_condition();
    pub fn __codastraea_end_condition();
    pub fn __codastraea_begin_then();
    pub fn __codastraea_end_then();
    pub fn __codastraea_begin_else_if();
    pub fn __codastraea_end_else_if();
    pub fn __codastraea_begin_else();
    pub fn __codastraea_end_else();
}

#[cfg(not(target_family = "wasm"))]
mod define {
    #[no_mangle]
    unsafe extern "C" fn __codastraea_log(_data: u32, _len: u32) {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_register_workflow_index(
        _module_data: u32,
        _module_len: u32,
        _name_data: u32,
        _name_len: u32,
        _index: u32,
    ) {
    }

    #[no_mangle]
    unsafe extern "C" fn __codastraea_fn_begin(
        _module: u32,
        _module_len: u32,
        _name: u32,
        _name_len: u32,
    ) {
    }

    #[no_mangle]
    unsafe extern "C" fn __codastraea_fn_end(
        _module: u32,
        _module_len: u32,
        _name: u32,
        _name_len: u32,
    ) {
    }

    #[no_mangle]
    unsafe extern "C" fn __codastraea_begin_if() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_end_if() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_begin_condition() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_end_condition() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_begin_then() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_end_then() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_begin_else_if() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_end_else_if() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_begin_else() {}

    #[no_mangle]
    unsafe extern "C" fn __codastraea_end_else() {}
}
