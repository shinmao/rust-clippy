#![allow(unused)]
#![warn(clippy::transmute_statistics)]

use std::convert::From;

pub struct A {
    a: i8,
    b: i32,
    c: i8,
}

pub struct B {
    a: i8,
    b: i32,
    c: i8,
}

fn sa_to_sb(sa: &A) -> &B {
    unsafe { std::mem::transmute(sa) }
}

impl From<A> for B {
    fn from(item: A) -> Self {
        unsafe { std::mem::transmute(item) }
    }
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

    let sa = A { a: 10, b: 11, c: 12 };
    let sb = sa_to_sb(&sa);

    let ano_sb = B::from(sa);
    println!("{}", ano_sb.a);
}
