use bitstruct_rs::BitEnum;

#[derive(BitEnum)]
enum Missing { A = 0 }

#[derive(BitEnum)]
#[repr(i8)]
enum Signed { A = 0 }

#[derive(BitEnum)]
#[repr(u8)]
enum Implicit { A }

#[derive(BitEnum)]
#[repr(u8)]
enum Data { A(u8) = 0 }

fn main() {}
