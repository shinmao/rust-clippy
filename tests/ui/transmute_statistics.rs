#![allow(unused)]
#![warn(clippy::transmute_statistics)]

pub struct struct_a {
    a: i8,
    b: i32,
    c: i8,
}

pub struct struct_b {
    a: i8,
    b: i32,
    c: i8,
}

fn main() {
    let a = 0u32;
    let a = &a as *const u32;
    let _: &u32 = unsafe { std::mem::transmute(a) };
    let _: &u32 = unsafe { std::mem::transmute::<_, &u32>(a) };

    let b: u8 = 10;
    let _ = unsafe { std::mem::transmute::<_, &u32>(&b) };

    let c: u32 = 10;
    let _ = unsafe { std::mem::transmute::<_, &u8>(&c) };

    let sa = struct_a { a: 10, b: 11, c: 12 };
    let sb: &struct_b = unsafe { std::mem::transmute(&sa) };
}
