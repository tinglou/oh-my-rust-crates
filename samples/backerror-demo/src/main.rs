use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
#[error(transparent)]
pub struct MyError1(#[from] std::io::Error);

#[backerror]
#[derive(Debug, Error)]
pub enum MyError2 {
    #[error("By MyError2: {0}")]
    MyError1(#[from] MyError1),
}

#[backerror]
#[derive(Debug, Error)]
pub enum MyError3 {
    #[error("By MyError3: {0}")]
    MyError2(#[from] MyError2),
}

fn throw_error1() -> Result<(), MyError1> {
    std::fs::File::open("blurb.txt")?;
    Ok(())
}

fn throw_error2() -> Result<(), MyError2> {
    Ok(throw_error1()?)
}

fn throw_error3() -> Result<(), MyError3> {
    Ok(throw_error2()?)
}

fn main() {
    if let Err(err3) = throw_error3() {
        println!("===========================================");
        println!("Display output:");
        println!("{}", err3);
        println!("\nDebug output:");
        println!("{:?}", err3);

        println!("===========================================");
        let MyError3::MyError2(err2) = err3;
        println!("Display output:");
        println!("{}", err2);
        println!("\nDebug output:");
        println!("{:?}", err2);

        println!("===========================================");
        let MyError2::MyError1(err1) = &*err2;
        println!("Display output:");
        println!("{}", err1);
        println!("\nDebug output:");
        println!("{:?}", err1);
    }
}
