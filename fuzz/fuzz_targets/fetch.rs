#![no_main]
#![forbid(unsafe_code)]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    mail_engine_fuzz::fetch(data);
});
